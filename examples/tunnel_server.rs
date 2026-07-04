use tgin::Tgin;
use tgin::batteries::egress::tunnel::TunnelEgress;
use tgin::batteries::ingress::http::HttpIngress;
use tgin::shared::server::HttpServer;

#[tokio::main]
async fn main() {
    let token = std::env::var("TUNNEL_TOKEN").expect("TUNNEL_TOKEN is not set");

    let server = HttpServer::new("0.0.0.0:8080");

    Tgin::new()
        .pipeline(
            HttpIngress::catch_all(&server),
            TunnelEgress::new(&server, "/tunnel", &token),
        )
        .run()
        .await;
}
