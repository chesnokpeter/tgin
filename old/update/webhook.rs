use crate::base::{Printable, Serverable};
use crate::update::base::Updater;

use crate::utils::defaults::TELEGRAM_TOKEN_REGEX;

use async_trait::async_trait;
use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    routing::post,
    Json, Router,
};
use serde_json::{json, Value};

use reqwest::Client;

use tokio::sync::mpsc::Sender;

use regex::Regex;

pub struct RegistrationWebhookConfig {
    public_ip: String,
    client: Client,
    set_webhook_url: String,

    token_regex: Regex,
}

impl RegistrationWebhookConfig {
    pub fn new(token: String, public_ip: String) -> Self {
        Self {
            public_ip,
            client: Client::new(),
            set_webhook_url: format!("https://api.telegram.org/bot{}/setWebhook", token),
            token_regex: Regex::new(TELEGRAM_TOKEN_REGEX).unwrap(),
        }
    }

    pub fn set_client(&mut self, client: Client) {
        self.client = client;
    }

    pub fn set_webhook_url(&mut self, set_webhook_url: String) {
        self.set_webhook_url = set_webhook_url;
    }

    pub fn set_regex_token(&mut self, regex: Regex) {
        self.token_regex = regex;
    }
}

#[derive(Clone)]
struct WebhookState {
    tx: Sender<Value>,
    secret_token: Option<String>,
}

pub struct WebhookUpdate {
    path: String,
    secret_token: Option<String>,
    registration: Option<RegistrationWebhookConfig>,
}

impl WebhookUpdate {
    pub fn new(path: String) -> Self {
        Self {
            path,
            secret_token: None,
            registration: None,
        }
    }

    pub fn set_secret_token(&mut self, token: String) {
        self.secret_token = Some(token);
    }

    pub async fn register_webhook(&self, config: &RegistrationWebhookConfig) {
        let full_url = format!("{}{}", config.public_ip.trim_end_matches('/'), self.path);

        let params = json!({ "url": full_url });

        match config
            .client
            .post(&config.set_webhook_url)
            .json(&params)
            .send()
            .await
        {
            Ok(resp) => {
                if resp.status().is_success() {
                    println!("Webhook set successfully for path: {}", self.path);
                } else {
                    eprintln!("Failed to set webhook. Status: {}", resp.status());
                }
            }
            Err(e) => eprintln!("Network error setting webhook: {}", e),
        }
    }
}

#[async_trait]
impl Updater for WebhookUpdate {
    async fn start(&self, _tx: Sender<Value>) {
        if let Some(config) = &self.registration {
            self.register_webhook(config).await;
        } else {
            println!(
                "Webhook started in passive mode (no auto-registration) for {}",
                self.path
            );
        }
    }
}

#[async_trait]
impl Serverable for WebhookUpdate {
    async fn set_server(&self, router: Router<Sender<Value>>) -> Router<Sender<Value>> {
        let path = self.path.clone();
        let secret = self.secret_token.clone();

        let handler_func = move |State(tx): State<Sender<Value>>,
                                 headers: HeaderMap,
                                 Json(update): Json<Value>| {
            let secret = secret.clone();
            async move {
                if let Some(sec) = secret {
                    if let Some(header_val) = headers.get("x-telegram-bot-api-secret-token") {
                        if header_val.to_str().unwrap_or("") != sec {
                            return StatusCode::UNAUTHORIZED;
                        }
                    } else {
                        return StatusCode::UNAUTHORIZED;
                    }
                }
                let _ = tx.send(update).await;
                StatusCode::OK
            }
        };

        router.route(&path, post(handler_func))
    }
}

#[async_trait]
impl Printable for WebhookUpdate {
    async fn print(&self) -> String {
        let reg_text = match &self.registration {
            Some(reg) => format!(
                "REGISTRATED ON {}",
                &reg.token_regex.replace_all(&reg.set_webhook_url, "#####")
            ),
            None => "".to_string(),
        };
        format!("webhook: 0.0.0.0{} {}", self.path, reg_text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use tokio::sync::mpsc;
    use tower::ServiceExt;
    use wiremock::matchers::method;
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn test_webhook_registers_correctly() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(json!({"ok": true, "result": true})),
            )
            .expect(1)
            .mount(&mock_server)
            .await;

        let my_ip = "https://my-server.com";
        let token = "TOKEN123";

        let mut reg_config = RegistrationWebhookConfig::new(token.to_string(), my_ip.to_string());

        let client = Client::builder().no_proxy().build().unwrap();
        reg_config.set_client(client);

        reg_config.set_webhook_url(format!("{}/setWebhook", mock_server.uri()));

        let mut updater = WebhookUpdate::new("/webhook".to_string());
        updater.registration = Some(reg_config);

        let (tx, _) = mpsc::channel(1);

        updater.start(tx).await;
    }

    #[tokio::test]
    async fn test_webhook_handler_receives_json_and_sends_to_channel() {
        let updater = WebhookUpdate::new("/bot/update".to_string());

        let (tx, mut rx) = mpsc::channel(10);

        let app = Router::new();
        let app = updater.set_server(app).await.with_state(tx);

        let incoming_payload = json!({
            "update_id": 999,
            "message": { "text": "Hello via Webhook" }
        });

        let request = Request::builder()
            .method("POST")
            .uri("/bot/update")
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_vec(&incoming_payload).unwrap()))
            .unwrap();

        let response = app.oneshot(request).await.unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let received = rx.recv().await.expect("Channel should receive update");

        assert_eq!(received["update_id"], 999);
        assert_eq!(received["message"]["text"], "Hello via Webhook");
    }
}
