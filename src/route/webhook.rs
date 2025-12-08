use crate::base::{Routeable, Serverable, Printable};
use async_trait::async_trait;
use reqwest::Client;
use serde_json::Value;

pub struct WebhookRoute {
    client: Client,
    url: String,
}

impl WebhookRoute {
    pub fn new(url: String) -> Self {
        Self {
            client: Client::new(),
            url,
        }
    }

    pub fn set_client(&mut self, client: Client) {
        self.client = client;
    }
}

#[async_trait]
impl Routeable for WebhookRoute {
    async fn process(&self, update: Value) {
        let _ = self.client.post(&self.url).json(&update).send().await;
    }
}

impl Serverable for WebhookRoute {}


impl Printable for WebhookRoute {
    fn print(&self) -> String {
        format!("webhook: {}", self.url)
    }
}