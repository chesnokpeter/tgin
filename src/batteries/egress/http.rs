use reqwest::{Client};

use crate::base::Egress;

use crate::batteries::data::request::{RequestData, ResponseData};

use async_trait::async_trait;

#[derive(Clone)] 
pub struct HttpEgress {
    url: String,
    client: Client
}


impl HttpEgress {
    pub fn new(url: String) -> Self {
        Self {
            url,
            client: Client::new()
        }
    }

    pub fn with_client(url: String, client: Client) -> Self {
        Self {
            url,
            client
        }
    }
}

#[async_trait]
impl Egress<RequestData> for HttpEgress {
    type Output = ResponseData;

    async fn send(&self, data: RequestData) -> Self::Output{
        let url = format!("{}{}", self.url, data.uri);
        let response = self.client.request(data.method, url)
            .body(data.body)
            .headers(data.headers)
            .send()
            .await;

        match response {
            Ok(resp) => {
                ResponseData {
                    status: resp.status(),
                    headers: resp.headers().clone(),
                    body: resp.bytes().await.unwrap_or_default(),
                }
            },
            Err(e) => {
                ResponseData {
                    status: reqwest::StatusCode::BAD_GATEWAY,
                    headers: reqwest::header::HeaderMap::new(),
                    body: format!("").into(),
                }
            }
        }

    }
}



