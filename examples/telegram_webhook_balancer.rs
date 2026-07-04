use tgin::Tgin;
use tgin::batteries::egress::telegram_bot::TelegramBotWebhookEgress;
use tgin::batteries::ingress::telegram_bot::TelegramBotWebhookIngress;
use tgin::batteries::lb::RoundRobin;
use tgin::shared::client::HttpClient;
use tgin::shared::server::HttpServer;

#[tokio::main]
async fn main() {
    let secret = std::env::var("WEBHOOK_SECRET").expect("WEBHOOK_SECRET is not set");

    let server = HttpServer::new("0.0.0.0:8080");
    let http = HttpClient::new();

    Tgin::new()
        .pipeline(
            TelegramBotWebhookIngress::new(&server, "/webhook").secret(&secret),
            RoundRobin::new()
                .to(TelegramBotWebhookEgress::new(&http, "http://127.0.0.1:3001/webhook"))
                .to(TelegramBotWebhookEgress::new(&http, "http://127.0.0.1:3002/webhook")),
        )
        .run()
        .await;
}
