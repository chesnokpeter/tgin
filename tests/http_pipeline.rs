use std::time::Duration;

use tgin::Tgin;
use tgin::batteries::egress::http::HttpEgress;
use tgin::batteries::ingress::http::HttpIngress;
use tgin::batteries::route::Route;
use tgin::shared::client::HttpClient;
use tgin::shared::server::HttpServer;
use tgin::types::request::RequestData;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

async fn wait_until_ready(client: &reqwest::Client, url: &str) {
    for _ in 0..100 {
        if client.get(url).send().await.is_ok() {
            return;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    panic!("gateway did not become ready at {url}");
}

#[tokio::test]
async fn routes_and_proxies_through_a_real_pipeline() {
    let api_upstream = MockServer::start().await;
    let default_upstream = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/api/users"))
        .respond_with(ResponseTemplate::new(200).set_body_string("api upstream"))
        .mount(&api_upstream)
        .await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(200).set_body_string("default upstream"))
        .mount(&default_upstream)
        .await;

    let server = HttpServer::new("127.0.0.1:18471");
    let http = HttpClient::new();

    let gateway = Tgin::new().pipeline(
        HttpIngress::catch_all(&server),
        Route::new()
            .when(
                |request: &RequestData| request.uri.path().starts_with("/api"),
                HttpEgress::new(&http, &api_upstream.uri()),
            )
            .otherwise(HttpEgress::new(&http, &default_upstream.uri())),
    );

    let running = tokio::spawn(gateway.run());
    let client = reqwest::Client::new();
    wait_until_ready(&client, "http://127.0.0.1:18471/health").await;

    let api = client
        .get("http://127.0.0.1:18471/api/users")
        .send()
        .await
        .unwrap();
    assert_eq!(api.status(), 200);
    assert_eq!(api.text().await.unwrap(), "api upstream");

    let fallback = client
        .get("http://127.0.0.1:18471/anything-else")
        .send()
        .await
        .unwrap();
    assert_eq!(fallback.status(), 200);
    assert_eq!(fallback.text().await.unwrap(), "default upstream");

    running.abort();
}

#[tokio::test]
async fn a_dead_upstream_becomes_an_honest_502() {
    let server = HttpServer::new("127.0.0.1:18472");
    let http = HttpClient::builder()
        .connect_timeout(Duration::from_millis(200))
        .build();

    let gateway = Tgin::new().pipeline(
        HttpIngress::catch_all(&server),
        HttpEgress::new(&http, "http://127.0.0.1:19999"),
    );

    let running = tokio::spawn(gateway.run());
    let client = reqwest::Client::new();
    wait_until_ready(&client, "http://127.0.0.1:18472/").await;

    let response = client
        .get("http://127.0.0.1:18472/whatever")
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 502);

    running.abort();
}
