use tgin::Tgin;
use tgin::batteries::egress::http::HttpEgress;
use tgin::batteries::ingress::http::HttpIngress;
use tgin::batteries::lb::RoundRobin;
use tgin::batteries::route::Route;
use tgin::shared::client::HttpClient;
use tgin::shared::server::HttpServer;
use tgin::types::request::RequestData;

fn env_or(key: &str, default: &str) -> String {
    std::env::var(key).unwrap_or_else(|_| default.to_string())
}

#[tokio::main]
async fn main() {
    let api_backend_1 = env_or("API_BACKEND_1", "http://127.0.0.1:3001");
    let api_backend_2 = env_or("API_BACKEND_2", "http://127.0.0.1:3002");
    let default_backend = env_or("DEFAULT_BACKEND", "http://127.0.0.1:3000");

    let server = HttpServer::new("0.0.0.0:8080");
    let http = HttpClient::new();

    Tgin::new()
        .pipeline(
            HttpIngress::catch_all(&server),
            Route::new()
                .when(
                    |request: &RequestData| request.uri.path().starts_with("/api"),
                    RoundRobin::new()
                        .to(HttpEgress::new(&http, &api_backend_1))
                        .to(HttpEgress::new(&http, &api_backend_2)),
                )
                .otherwise(HttpEgress::new(&http, &default_backend)),
        )
        .run()
        .await;
}
