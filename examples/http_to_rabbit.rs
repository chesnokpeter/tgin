use tgin::Tgin;
use tgin::batteries::egress::rabbitmq::RabbitmqEgress;
use tgin::batteries::ingress::http::HttpIngress;
use tgin::shared::rabbitmq::Rabbit;
use tgin::shared::server::HttpServer;

#[tokio::main]
async fn main() {
    let server = HttpServer::new("0.0.0.0:8080");
    let rabbit = Rabbit::new("amqp://guest:guest@127.0.0.1:5672");

    Tgin::new()
        .serve(server.clone())
        .pipeline(
            HttpIngress::post(&server, "/ingest"),
            RabbitmqEgress::new(&rabbit, "", "events"),
        )
        .run()
        .await;
}
