use std::time::Duration;

use async_trait::async_trait;
use axum::Router;
use axum::body::Bytes;
use axum::extract::State;
use axum::http::{HeaderMap, HeaderValue, Method, StatusCode, Uri, header::CONTENT_TYPE};
use axum::response::IntoResponse;
use axum::routing::post;
use reqwest::Client;
use serde_json::Value;
use tokio::sync::mpsc::Sender;
use tokio::sync::mpsc::error::TrySendError;
use tokio::sync::oneshot;
use tokio_util::sync::CancellationToken;

use crate::base::{Envelope, Ingress, Runnable, SendError};
use crate::shared::client::HttpClient;
use crate::shared::server::HttpServer;
use crate::types::request::{RequestData, ResponseData};

fn chat_key(update: &Value) -> Option<i64> {
    for field in ["message", "edited_message", "channel_post", "edited_channel_post"] {
        if let Some(id) = update[field]["chat"]["id"].as_i64() {
            return Some(id);
        }
    }
    if let Some(id) = update["callback_query"]["message"]["chat"]["id"].as_i64() {
        return Some(id);
    }
    for field in ["my_chat_member", "chat_member", "chat_join_request"] {
        if let Some(id) = update[field]["chat"]["id"].as_i64() {
            return Some(id);
        }
    }
    for field in ["inline_query", "chosen_inline_result", "shipping_query", "pre_checkout_query"] {
        if let Some(id) = update[field]["from"]["id"].as_i64() {
            return Some(id);
        }
    }
    None
}

fn update_request(update: &Value) -> RequestData {
    let mut headers = HeaderMap::new();
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
    RequestData {
        body: Bytes::from(serde_json::to_vec(update).unwrap_or_default()),
        uri: Uri::from_static("/"),
        method: Method::POST,
        headers,
        client_ip: None,
    }
}

fn update_envelope(
    update: &Value,
    reply: oneshot::Sender<Result<ResponseData, SendError>>,
) -> Envelope<RequestData, ResponseData> {
    let envelope = Envelope::backward(update_request(update), reply);
    match chat_key(update) {
        Some(chat) => envelope.key(chat.to_string()),
        None => envelope,
    }
}

pub struct TelegramBotPollingIngress {
    client: Client,
    base: String,
    poll_timeout: u64,
    reconnect: Duration,
}

impl TelegramBotPollingIngress {
    pub fn new(client: &HttpClient, token: &str) -> Self {
        Self {
            client: client.client(),
            base: format!("https://api.telegram.org/bot{token}"),
            poll_timeout: 25,
            reconnect: Duration::from_secs(5),
        }
    }

    pub fn poll_timeout(mut self, seconds: u64) -> Self {
        self.poll_timeout = seconds;
        self
    }

    pub fn reconnect(mut self, delay: Duration) -> Self {
        self.reconnect = delay;
        self
    }

    async fn poll(&self, offset: Option<i64>) -> Option<Vec<Value>> {
        let mut url = format!("{}/getUpdates?timeout={}", self.base, self.poll_timeout);
        if let Some(offset) = offset {
            url.push_str(&format!("&offset={offset}"));
        }

        let response = self.client.get(url).send().await.ok()?;
        if !response.status().is_success() {
            return None;
        }

        let payload: Value = response.json().await.ok()?;
        if payload["ok"].as_bool() != Some(true) {
            return None;
        }
        payload["result"].as_array().cloned()
    }

    async fn consume(
        &self,
        tx: &Sender<Envelope<RequestData, ResponseData>>,
        shutdown: &CancellationToken,
        offset: &mut Option<i64>,
    ) {
        loop {
            let updates = tokio::select! {
                _ = shutdown.cancelled() => return,
                updates = self.poll(*offset) => match updates {
                    Some(updates) => updates,
                    None => return,
                },
            };

            let mut replies = Vec::with_capacity(updates.len());

            for update in &updates {
                let Some(id) = update["update_id"].as_i64() else {
                    continue;
                };
                let (reply_tx, reply_rx) = oneshot::channel();
                if tx.send(update_envelope(update, reply_tx)).await.is_err() {
                    return;
                }
                replies.push((id, reply_rx));
            }

            for (id, reply) in replies {
                let delivered = tokio::select! {
                    _ = shutdown.cancelled() => return,
                    outcome = reply => matches!(outcome, Ok(Ok(_)) | Ok(Err(SendError::Permanent(_)))),
                };
                if !delivered {
                    break;
                }
                *offset = Some(id + 1);
            }
        }
    }
}

#[async_trait]
impl Ingress<RequestData, ResponseData> for TelegramBotPollingIngress {
    async fn start(&self, tx: Sender<Envelope<RequestData, ResponseData>>, shutdown: CancellationToken) {
        let mut offset = None;

        loop {
            if shutdown.is_cancelled() {
                return;
            }
            self.consume(&tx, &shutdown, &mut offset).await;
            tokio::select! {
                _ = shutdown.cancelled() => return,
                _ = tokio::time::sleep(self.reconnect) => {}
            }
        }
    }
}

#[derive(Clone)]
struct WebhookState {
    tx: Sender<Envelope<RequestData, ResponseData>>,
    secret: Option<String>,
}

pub struct TelegramBotWebhookIngress {
    server: HttpServer,
    path: String,
    secret: Option<String>,
}

impl TelegramBotWebhookIngress {
    pub fn new(server: &HttpServer, path: &str) -> Self {
        Self {
            server: server.clone(),
            path: path.to_string(),
            secret: None,
        }
    }

    pub fn secret(mut self, secret: &str) -> Self {
        self.secret = Some(secret.to_string());
        self
    }
}

#[async_trait]
impl Ingress<RequestData, ResponseData> for TelegramBotWebhookIngress {
    fn services(&self) -> Vec<Box<dyn Runnable>> {
        vec![Box::new(self.server.clone())]
    }

    async fn setup(&mut self, tx: Sender<Envelope<RequestData, ResponseData>>) {
        let state = WebhookState { tx, secret: self.secret.clone() };
        let router = Router::new().route(&self.path, post(webhook_handler)).with_state(state);
        self.server.register(router).await;
    }
}

async fn webhook_handler(
    State(state): State<WebhookState>,
    headers: HeaderMap,
    body: Bytes,
) -> impl IntoResponse {
    if let Some(secret) = &state.secret {
        let provided = headers
            .get("x-telegram-bot-api-secret-token")
            .and_then(|value| value.to_str().ok());
        if provided != Some(secret.as_str()) {
            return StatusCode::UNAUTHORIZED.into_response();
        }
    }

    let Ok(update) = serde_json::from_slice::<Value>(&body) else {
        return StatusCode::BAD_REQUEST.into_response();
    };

    let (reply_tx, reply_rx) = oneshot::channel();

    match state.tx.try_send(update_envelope(&update, reply_tx)) {
        Ok(()) => {}
        Err(TrySendError::Full(_)) => return StatusCode::TOO_MANY_REQUESTS.into_response(),
        Err(TrySendError::Closed(_)) => return StatusCode::SERVICE_UNAVAILABLE.into_response(),
    }

    match reply_rx.await {
        Ok(Ok(_)) => StatusCode::OK.into_response(),
        Ok(Err(SendError::Overloaded)) => StatusCode::SERVICE_UNAVAILABLE.into_response(),
        Ok(Err(SendError::DeadlineExceeded)) => StatusCode::GATEWAY_TIMEOUT.into_response(),
        _ => StatusCode::BAD_GATEWAY.into_response(),
    }
}
