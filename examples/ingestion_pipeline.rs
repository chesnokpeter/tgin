use std::time::Duration;

use tgin::Tgin;
use tgin::batteries::egress::http::HttpEgress;
use tgin::batteries::egress::rabbitmq::RabbitmqEgress;
use tgin::batteries::ingress::http::HttpIngress;
use tgin::batteries::ingress::rabbitmq::RabbitmqIngress;
use tgin::batteries::retry::Retry;
use tgin::shared::client::HttpClient;
use tgin::shared::rabbitmq::Rabbit;
use tgin::shared::server::HttpServer;

fn env_or(key: &str, default: &str) -> String {
    std::env::var(key).unwrap_or_else(|_| default.to_string())
}

#[tokio::main]
async fn main() {
    let rabbit_url = env_or("RABBIT_URL", "amqp://guest:guest@127.0.0.1:5672");
    let processor_url = env_or("PROCESSOR_URL", "http://127.0.0.1:3000");

    let server = HttpServer::new("0.0.0.0:8080");
    let http = HttpClient::new();
    let rabbit = Rabbit::new(&rabbit_url);

    Tgin::new()
        .pipeline(
            HttpIngress::post(&server, "/ingest"),
            RabbitmqEgress::new(&rabbit, "", "events"),
        )
        .pipeline(
            RabbitmqIngress::new(&rabbit, "events"),
            Retry::new(HttpEgress::new(&http, &processor_url))
                .attempts(3)
                .backoff(Duration::from_millis(300)),
        )
        .run()
        .await;
}
