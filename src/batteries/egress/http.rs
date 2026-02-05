use reqwest::{Client};

use crate::base::Egress;

use crate::batteries::data::request::RequestData;

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
    async fn send(&self, data: RequestData){
        let url = format!("{}{}", self.url, data.uri);
        let _ = self.client.request(data.method, url)
            .body(data.body)
            .headers(data.headers)
            .send()
            .await;
    }
}



