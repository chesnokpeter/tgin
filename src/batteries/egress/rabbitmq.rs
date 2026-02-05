use async_trait::async_trait;
use lapin::{Channel, BasicProperties, options::BasicPublishOptions};
use crate::base::Egress;
use crate::batteries::data::request::{RequestData, ResponseData};

#[derive(Clone)]
pub struct RabbitmqEgress {
    channel: Channel,
    exchange: String,
    routing_key: String,
}

impl RabbitmqEgress {
    pub fn new(channel: Channel, exchange: String, routing_key: String) -> Self {
        Self {
            channel,
            exchange,
            routing_key,
        }
    }
}

#[async_trait]
impl Egress<RequestData> for RabbitmqEgress {
    type Output = ResponseData; 

    async fn send(&self, data: RequestData) -> Self::Output {
        let payload = data.body.to_vec();

        let _ = self.channel
            .basic_publish(
                &self.exchange,
                &self.routing_key,
                BasicPublishOptions::default(),
                &payload,
                BasicProperties::default(),
            )
            .await;

        ResponseData::default()

    }
}