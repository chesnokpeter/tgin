use std::time::Duration;

use async_trait::async_trait;
use axum::http::StatusCode;
use rdkafka::config::ClientConfig;
use rdkafka::error::KafkaError;
use rdkafka::producer::{FutureProducer, FutureRecord};
use rdkafka::types::RDKafkaErrorCode;

use crate::base::{Egress, Meta, SendError};
use crate::types::request::{RequestData, ResponseData};

#[derive(Clone)]
pub struct KafkaEgress {
    producer: FutureProducer,
    topic: String,
    timeout: Duration,
}

impl KafkaEgress {
    pub fn new(brokers: &str, topic: &str) -> Self {
        let producer = ClientConfig::new()
            .set("bootstrap.servers", brokers)
            .set("acks", "all")
            .set("message.timeout.ms", "10000")
            .create()
            .expect("KafkaEgress: invalid configuration");

        Self {
            producer,
            topic: topic.to_string(),
            timeout: Duration::from_secs(10),
        }
    }

    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }
}

#[async_trait]
impl Egress<RequestData> for KafkaEgress {
    type Output = ResponseData;

    async fn send(&self, data: RequestData, meta: &Meta) -> Result<ResponseData, SendError> {
        let record = FutureRecord::to(&self.topic).payload(data.body.as_ref());

        let delivery = match &meta.key {
            Some(key) => self.producer.send(record.key(key.as_ref()), self.timeout).await,
            None => self.producer.send(record, self.timeout).await,
        };

        match delivery {
            Ok(_) => Ok(ResponseData { status: StatusCode::ACCEPTED, ..Default::default() }),
            Err((KafkaError::MessageProduction(RDKafkaErrorCode::MessageSizeTooLarge), _)) => {
                Err(SendError::permanent("kafka rejected the message: too large"))
            }
            Err((KafkaError::MessageProduction(RDKafkaErrorCode::QueueFull), _)) => Err(SendError::Overloaded),
            Err((error, _)) => Err(SendError::retryable(error)),
        }
    }
}
