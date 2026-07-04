use std::time::Duration;

use tokio::sync::mpsc::Sender;
use tokio::sync::oneshot;
use tokio_util::sync::CancellationToken;
use async_trait::async_trait;
use futures_lite::stream::StreamExt;
use lapin::{
    message::Delivery,
    options::{BasicAckOptions, BasicConsumeOptions, BasicNackOptions, BasicQosOptions, QueueDeclareOptions},
    types::FieldTable,
};
use axum::body::Bytes;
use axum::http::{Method, Uri, HeaderMap};

use crate::base::{Envelope, Ingress, SendError};
use crate::shared::rabbitmq::Rabbit;
use crate::types::request::{RequestData, ResponseData};

pub struct RabbitmqIngress {
    rabbit: Rabbit,
    queue: String,
    reconnect: Duration,
    prefetch: u16,
}

impl RabbitmqIngress {
    pub fn new(rabbit: &Rabbit, queue: &str) -> Self {
        Self {
            rabbit: rabbit.clone(),
            queue: queue.to_string(),
            reconnect: Duration::from_secs(5),
            prefetch: 64,
        }
    }

    pub fn reconnect(mut self, delay: Duration) -> Self {
        self.reconnect = delay;
        self
    }

    pub fn prefetch(mut self, prefetch: u16) -> Self {
        self.prefetch = prefetch;
        self
    }

    async fn consume(&self, tx: &Sender<Envelope<RequestData, ResponseData>>, shutdown: &CancellationToken) {
        let Some(connection) = self.rabbit.connection().await else {
            return;
        };
        let Ok(channel) = connection.create_channel().await else {
            return;
        };
        if channel.basic_qos(self.prefetch, BasicQosOptions::default()).await.is_err() {
            return;
        }
        let _ = channel
            .queue_declare(
                &self.queue,
                QueueDeclareOptions { durable: true, ..QueueDeclareOptions::default() },
                FieldTable::default(),
            )
            .await;

        let mut consumer = match channel
            .basic_consume(&self.queue, "tgin", BasicConsumeOptions::default(), FieldTable::default())
            .await
        {
            Ok(consumer) => consumer,
            Err(_) => return,
        };

        loop {
            let delivery = tokio::select! {
                _ = shutdown.cancelled() => return,
                next = consumer.next() => match next {
                    Some(Ok(delivery)) => delivery,
                    Some(Err(_)) => continue,
                    None => return,
                },
            };

            let Delivery { data, acker, .. } = delivery;

            let request = RequestData {
                body: Bytes::from(data),
                uri: Uri::from_static("/"),
                method: Method::POST,
                headers: HeaderMap::new(),
                client_ip: None,
            };

            let (ack_tx, ack_rx) = oneshot::channel();

            if tx.send(Envelope::backward(request, ack_tx)).await.is_err() {
                let _ = acker.nack(BasicNackOptions { requeue: true, ..Default::default() }).await;
                return;
            }

            tokio::spawn(async move {
                let requeue = match ack_rx.await {
                    Ok(Ok(_)) => {
                        let _ = acker.ack(BasicAckOptions::default()).await;
                        return;
                    }
                    Ok(Err(SendError::Permanent(_))) => false,
                    _ => true,
                };
                let _ = acker.nack(BasicNackOptions { requeue, ..Default::default() }).await;
            });
        }
    }
}

#[async_trait]
impl Ingress<RequestData, ResponseData> for RabbitmqIngress {
    async fn start(&self, tx: Sender<Envelope<RequestData, ResponseData>>, shutdown: CancellationToken) {
        loop {
            if shutdown.is_cancelled() {
                return;
            }
            self.consume(&tx, &shutdown).await;
            tokio::select! {
                _ = shutdown.cancelled() => return,
                _ = tokio::time::sleep(self.reconnect) => {}
            }
        }
    }
}
