use std::time::Duration;

use tgin::Tgin;
use tgin::batteries::egress::telegram_bot::TelegramBotApiEgress;
use tgin::batteries::ingress::rabbitmq::RabbitmqIngress;
use tgin::batteries::retry::Retry;
use tgin::shared::client::HttpClient;
use tgin::shared::rabbitmq::Rabbit;

#[tokio::main]
async fn main() {
    let token = std::env::var("BOT_TOKEN").expect("BOT_TOKEN is not set");

    let http = HttpClient::new();
    let rabbit = Rabbit::new("amqp://guest:guest@127.0.0.1:5672");

    Tgin::new()
        .pipeline(
            RabbitmqIngress::new(&rabbit, "notifications"),
            Retry::new(TelegramBotApiEgress::method(&http, &token, "sendMessage"))
                .attempts(5)
                .backoff(Duration::from_millis(200)),
        )
        .run()
        .await;
}
