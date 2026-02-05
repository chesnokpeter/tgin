use tokio::sync::{mpsc, oneshot};
use async_trait::async_trait;
use futures_lite::stream::StreamExt;
use lapin::options::{BasicAckOptions, BasicConsumeOptions};
use lapin::types::{FieldTable};

use crate::base::{Ingress, Envelope};
use crate::shared::rabbitmq::RabbitmqBroker;

pub struct RabbitmqIngress {
    broker: RabbitmqBroker,
    queue_name: String,
    consumer_name: String,

    wait_for_ack: bool
}

impl RabbitmqIngress {
    pub fn new(broker: RabbitmqBroker, queue_name: String, consumer_name: String, wait_for_ack: bool) -> Self {
        Self {
            broker,
            queue_name,
            consumer_name,
            wait_for_ack
        }
    }
}

#[async_trait]
impl Ingress<String, ()> for RabbitmqIngress {
    async fn start(&self, tx: mpsc::Sender<Envelope<String, ()>>) {
        let conn = self.broker.get_connection().await;

        let channel = conn.create_channel().await.expect("Failed to create rabbitmq channel");

        let mut consumer = channel
            .basic_consume(
                &self.queue_name,
                &self.consumer_name,
                BasicConsumeOptions::default(),
                FieldTable::default(),
            )
            .await
            .expect("Failed to consume rabbitmq channel");

        while let Some(delivery) = consumer.next().await {
            if let Ok(delivery) = delivery {
                let payload = String::from_utf8_lossy(&delivery.data).to_string();

                match self.wait_for_ack {
                    true => {
                        let (ack_tx, ack_rx) = oneshot::channel();
                        let envelope = Envelope::backward(payload, ack_tx);
                        if tx.send(envelope).await.is_err() {
                            break;
                        }
                        match ack_rx.await {
                            Ok(_) => {
                                delivery
                                    .ack(BasicAckOptions::default())
                                    .await
                                    .expect("Failed to ack");
                            },
                            Err(_) => {}
                        }
                    }
                    false => {
                        let envelope = Envelope::forward(payload);
                        if tx.send(envelope).await.is_err() {
                            break;
                        }
                    }
                }

            }
        }
    }
}