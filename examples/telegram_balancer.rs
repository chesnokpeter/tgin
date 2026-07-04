use std::time::Duration;

use tgin::Tgin;
use tgin::batteries::egress::telegram_bot::TelegramBotWebhookEgress;
use tgin::batteries::ingress::telegram_bot::TelegramBotPollingIngress;
use tgin::batteries::lb::RoundRobin;
use tgin::shared::client::HttpClient;

#[tokio::main]
async fn main() {
    let token = std::env::var("BOT_TOKEN").expect("BOT_TOKEN is not set");

    let http = HttpClient::builder()
        .timeout(Duration::from_secs(35))
        .build();

    Tgin::new()
        .pipeline(
            TelegramBotPollingIngress::new(&http, &token),
            RoundRobin::new()
                .to(TelegramBotWebhookEgress::new(&http, "http://127.0.0.1:3001/webhook"))
                .to(TelegramBotWebhookEgress::new(&http, "http://127.0.0.1:3002/webhook")),
        )
        .run()
        .await;
}
