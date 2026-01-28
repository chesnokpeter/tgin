use crate::base::{Printable, Routeable, Serverable};
use async_trait::async_trait;
use reqwest::Client;
use serde_json::{json, Value};

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

    fn diff_value(&self) -> Option<&str> {
        Some(&self.url)
    }
}

impl Serverable for WebhookRoute {}

#[async_trait]
impl Printable for WebhookRoute {
    async fn print(&self) -> String {
        format!("webhook: {}", self.url)
    }

    async fn json_struct(&self) -> Value {
        json!({
            "type": "webhook",
            "options": {
                "url": self.url
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{body_json, method};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn test_process_sends_correct_post_request() {
        let mock_server = MockServer::start().await;

        let payload = json!({
            "update_id": 12345,
            "message": { "text": "hello webhook" }
        });

        Mock::given(method("POST"))
            .and(body_json(&payload))
            .respond_with(ResponseTemplate::new(200))
            .expect(1)
            .mount(&mock_server)
            .await;

        let client = reqwest::Client::builder().no_proxy().build().unwrap();

        let mut route = WebhookRoute::new(mock_server.uri());
        route.set_client(client);

        route.process(payload).await;
    }

    #[tokio::test]
    async fn test_process_does_not_panic_on_network_error() {
        let route = WebhookRoute::new("http://localhost:9999/invalid".to_string());

        let payload = json!({"test": "data"});
        route.process(payload).await;
    }

    #[tokio::test]
    async fn test_printable_implementation() {
        let url = "http://my-bot.com/webhook";
        let route = WebhookRoute::new(url.to_string());

        assert_eq!(route.print().await, format!("webhook: {}", url));
        let json_info = route.json_struct().await;
        assert_eq!(json_info["type"], "webhook");
        assert_eq!(json_info["options"]["url"], url);
    }
}
