use std::time::Duration;

use tgin::Tgin;
use tgin::batteries::egress::http::HttpEgress;
use tgin::batteries::egress::kafka::KafkaEgress;
use tgin::batteries::ingress::http::HttpIngress;
use tgin::batteries::ingress::kafka::KafkaIngress;
use tgin::batteries::retry::Retry;
use tgin::shared::client::HttpClient;
use tgin::shared::server::HttpServer;

fn env_or(key: &str, default: &str) -> String {
    std::env::var(key).unwrap_or_else(|_| default.to_string())
}

#[tokio::main]
async fn main() {
    let brokers = env_or("KAFKA_BROKERS", "127.0.0.1:9092");
    let processor_url = env_or("PROCESSOR_URL", "http://127.0.0.1:3000");

    let server = HttpServer::new("0.0.0.0:8080");
    let http = HttpClient::new();

    Tgin::new()
        .pipeline(
            HttpIngress::post(&server, "/ingest"),
            KafkaEgress::new(&brokers, "events"),
        )
        .pipeline(
            KafkaIngress::new(&brokers, "tgin", "events"),
            Retry::new(HttpEgress::new(&http, &processor_url))
                .attempts(3)
                .backoff(Duration::from_millis(300)),
        )
        .run()
        .await;
}
