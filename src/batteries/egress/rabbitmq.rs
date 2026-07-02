use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::Mutex;
use lapin::{Channel, BasicProperties, options::{BasicPublishOptions, ConfirmSelectOptions}};
use axum::http::StatusCode;

use crate::base::{Egress, Meta, SendError};
use crate::shared::rabbitmq::Rabbit;
use crate::types::request::{RequestData, ResponseData};

#[derive(Clone)]
pub struct RabbitmqEgress {
    rabbit: Rabbit,
    exchange: String,
    routing_key: String,
    channel: Arc<Mutex<Option<Channel>>>,
}

impl RabbitmqEgress {
    pub fn new(rabbit: &Rabbit, exchange: &str, routing_key: &str) -> Self {
        Self {
            rabbit: rabbit.clone(),
            exchange: exchange.to_string(),
            routing_key: routing_key.to_string(),
            channel: Arc::new(Mutex::new(None)),
        }
    }

    async fn channel(&self) -> Option<Channel> {
        let mut guard = self.channel.lock().await;

        if let Some(channel) = guard.as_ref() {
            if channel.status().connected() {
                return Some(channel.clone());
            }
        }

        let connection = self.rabbit.connection().await?;
        let channel = connection.create_channel().await.ok()?;
        channel.confirm_select(ConfirmSelectOptions::default()).await.ok()?;

        *guard = Some(channel.clone());
        Some(channel)
    }
}

#[async_trait]
impl Egress<RequestData> for RabbitmqEgress {
    type Output = ResponseData;

    async fn send(&self, data: RequestData, _meta: &Meta) -> Result<ResponseData, SendError> {
        let channel = self.channel().await
            .ok_or_else(|| SendError::retryable("rabbitmq connection unavailable"))?;

        let confirm = channel
            .basic_publish(
                &self.exchange,
                &self.routing_key,
                BasicPublishOptions::default(),
                data.body.as_ref(),
                BasicProperties::default(),
            )
            .await
            .map_err(SendError::retryable)?
            .await
            .map_err(SendError::retryable)?;

        if confirm.is_nack() {
            return Err(SendError::retryable("broker nacked publish"));
        }

        Ok(ResponseData { status: StatusCode::ACCEPTED, ..Default::default() })
    }

    async fn stop(&self) {
        let channel = self.channel.lock().await.take();
        if let Some(channel) = channel {
            let _ = channel.close(200, "shutdown").await;
        }
    }
}
