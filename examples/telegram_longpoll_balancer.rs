use std::time::Duration;

use tgin::Tgin;
use tgin::batteries::egress::telegram_bot::TelegramBotPollingEgress;
use tgin::batteries::ingress::telegram_bot::TelegramBotPollingIngress;
use tgin::batteries::lb::RoundRobin;
use tgin::shared::client::HttpClient;
use tgin::shared::server::HttpServer;

#[tokio::main]
async fn main() {
    let token = std::env::var("BOT_TOKEN").expect("BOT_TOKEN is not set");

    let server = HttpServer::new("0.0.0.0:8080");
    let http = HttpClient::builder()
        .timeout(Duration::from_secs(35))
        .build();

    Tgin::new()
        .pipeline(
            TelegramBotPollingIngress::new(&http, &token),
            RoundRobin::new()
                .to(TelegramBotPollingEgress::new(&server, "/bot1"))
                .to(TelegramBotPollingEgress::new(&server, "/bot2")),
        )
        .run()
        .await;
}
