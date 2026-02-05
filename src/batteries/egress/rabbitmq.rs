use async_trait::async_trait;
use lapin::{Channel, BasicProperties, options::BasicPublishOptions};
use crate::base::Egress;
use crate::batteries::data::request::{RequestData, ResponseData};

#[derive(Clone)]
pub struct RabbitMqEgress {
    channel: Channel,
    exchange: String,
    routing_key: String,
}

impl RabbitMqEgress {
    pub fn new(channel: Channel, exchange: String, routing_key: String) -> Self {
        Self {
            channel,
            exchange,
            routing_key,
        }
    }
}

#[async_trait]
impl Egress<RequestData> for RabbitMqEgress {
    type Output = ResponseData; 

    async fn send(&self, data: RequestData) -> Self::Output {
        let payload = data.body.to_vec();

        let confirm = self.channel
            .basic_publish(
                &self.exchange,
                &self.routing_key,
                BasicPublishOptions::default(),
                &payload,
                BasicProperties::default(),
            )
            .await;

        match confirm {
            Ok(_) => {
                ResponseData::default() 
            },
            Err(e) => {
                println!("RabbitMQ publish error: {:?}", e);
                let mut err_resp = ResponseData::default();
                err_resp.status = axum::http::StatusCode::INTERNAL_SERVER_ERROR;
                err_resp
            }
        }
    }
}