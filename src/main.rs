mod base;
mod batteries;
mod shared;
mod tgin;
mod types;

use crate::batteries::egress::tunnel::TunnelEgress;
use crate::batteries::ingress::http::HttpIngress;
use crate::shared::server::HttpServer;
use crate::tgin::Tgin;

#[tokio::main]
async fn main() {
    let server = HttpServer::new("0.0.0.0:8080");
    let ingress = HttpIngress::catch_all(&server);
    let egress = TunnelEgress::new(&server, "/tunnel", "demo-token");

    Tgin::new()
        .pipeline(ingress, egress)
        .run()
        .await;
}
