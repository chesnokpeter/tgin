use tgin::Tgin;
use tgin::batteries::egress::telegram_bot::TelegramBotApiEgress;
use tgin::batteries::ingress::http::HttpIngress;
use tgin::shared::client::HttpClient;
use tgin::shared::server::HttpServer;

#[tokio::main]
async fn main() {
    let token = std::env::var("BOT_TOKEN").expect("BOT_TOKEN is not set");

    let server = HttpServer::new("0.0.0.0:8080");
    let http = HttpClient::new();

    Tgin::new()
        .pipeline(
            HttpIngress::catch_all(&server),
            TelegramBotApiEgress::new(&http, &token),
        )
        .run()
        .await;
}
