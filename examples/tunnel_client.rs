use std::time::Duration;

use tgin::Tgin;
use tgin::batteries::egress::http::HttpEgress;
use tgin::batteries::ingress::tunnel::TunnelIngress;
use tgin::shared::client::HttpClient;

#[tokio::main]
async fn main() {
    let server = std::env::var("TUNNEL_SERVER").expect("TUNNEL_SERVER is not set");
    let token = std::env::var("TUNNEL_TOKEN").expect("TUNNEL_TOKEN is not set");
    let target = std::env::var("LOCAL_TARGET").unwrap_or("http://127.0.0.1:3000".to_string());

    let http = HttpClient::builder()
        .timeout(Duration::from_secs(30))
        .connect_timeout(Duration::from_secs(3))
        .build();

    Tgin::new()
        .pipeline(
            TunnelIngress::new(&server, &token).reconnect(Duration::from_secs(1)),
            HttpEgress::new(&http, &target),
        )
        .run()
        .await;
}
