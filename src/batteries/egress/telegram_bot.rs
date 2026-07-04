use std::collections::BTreeMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicI64, Ordering};
use std::time::Duration;

use async_trait::async_trait;
use axum::Router;
use axum::body::Bytes;
use axum::extract::{RawQuery, State};
use axum::http::{HeaderMap, HeaderValue, StatusCode, header::CONTENT_TYPE};
use axum::response::IntoResponse;
use axum::routing::any;
use reqwest::Client;
use serde_json::Value;
use tokio::sync::{Mutex, Notify, oneshot};
use tokio::time::Instant;

use crate::base::{Egress, Meta, Runnable, SendError};
use crate::shared::client::HttpClient;
use crate::shared::server::HttpServer;
use crate::types::request::{RequestData, ResponseData};

fn classify_response(status: StatusCode, headers: HeaderMap, body: Bytes) -> Result<ResponseData, SendError> {
    if status == StatusCode::TOO_MANY_REQUESTS {
        return Err(SendError::Overloaded);
    }
    if status.is_server_error() {
        return Err(SendError::retryable(format!("responded {status}")));
    }
    if !status.is_success() {
        return Err(SendError::permanent(format!(
            "responded {status}: {}",
            String::from_utf8_lossy(&body)
        )));
    }
    Ok(ResponseData { status, headers, body })
}

#[derive(Clone)]
pub struct TelegramBotApiEgress {
    client: Client,
    base: String,
    method: Option<String>,
}

impl TelegramBotApiEgress {
    pub fn new(client: &HttpClient, token: &str) -> Self {
        Self {
            client: client.client(),
            base: format!("https://api.telegram.org/bot{token}"),
            method: None,
        }
    }

    pub fn method(client: &HttpClient, token: &str, method: &str) -> Self {
        Self {
            client: client.client(),
            base: format!("https://api.telegram.org/bot{token}"),
            method: Some(method.to_string()),
        }
    }
}

#[async_trait]
impl Egress<RequestData> for TelegramBotApiEgress {
    type Output = ResponseData;

    async fn send(&self, data: RequestData, _meta: &Meta) -> Result<ResponseData, SendError> {
        let url = match &self.method {
            Some(method) => format!("{}/{method}", self.base),
            None => format!("{}{}", self.base, data.uri),
        };

        let response = self.client
            .post(url)
            .header(CONTENT_TYPE, HeaderValue::from_static("application/json"))
            .body(data.body)
            .send()
            .await
            .map_err(SendError::retryable)?;

        let status = response.status();
        let headers = response.headers().clone();
        let body = response.bytes().await.map_err(SendError::retryable)?;

        classify_response(status, headers, body)
    }
}

#[derive(Clone)]
pub struct TelegramBotWebhookEgress {
    client: Client,
    url: String,
    secret: Option<HeaderValue>,
}

impl TelegramBotWebhookEgress {
    pub fn new(client: &HttpClient, url: &str) -> Self {
        Self {
            client: client.client(),
            url: url.to_string(),
            secret: None,
        }
    }

    pub fn secret(mut self, secret: &str) -> Self {
        self.secret = Some(
            HeaderValue::from_str(secret).expect("TelegramBotWebhookEgress: invalid secret"),
        );
        self
    }
}

#[async_trait]
impl Egress<RequestData> for TelegramBotWebhookEgress {
    type Output = ResponseData;

    async fn send(&self, data: RequestData, _meta: &Meta) -> Result<ResponseData, SendError> {
        let mut request = self.client
            .post(&self.url)
            .header(CONTENT_TYPE, HeaderValue::from_static("application/json"))
            .body(data.body);

        if let Some(secret) = &self.secret {
            request = request.header("x-telegram-bot-api-secret-token", secret.clone());
        }

        let response = request.send().await.map_err(SendError::retryable)?;

        let status = response.status();
        let headers = response.headers().clone();
        let body = response.bytes().await.map_err(SendError::retryable)?;

        classify_response(status, headers, body)
    }
}

struct PendingUpdate {
    update: Value,
    confirm: oneshot::Sender<()>,
}

struct PollingState {
    updates: Mutex<BTreeMap<i64, PendingUpdate>>,
    notify: Notify,
    counter: AtomicI64,
    registered: AtomicBool,
}

#[derive(Clone)]
pub struct TelegramBotPollingEgress {
    server: HttpServer,
    path: String,
    state: Arc<PollingState>,
}

impl TelegramBotPollingEgress {
    pub fn new(server: &HttpServer, path: &str) -> Self {
        Self {
            server: server.clone(),
            path: path.trim_end_matches('/').to_string(),
            state: Arc::new(PollingState {
                updates: Mutex::new(BTreeMap::new()),
                notify: Notify::new(),
                counter: AtomicI64::new(1),
                registered: AtomicBool::new(false),
            }),
        }
    }
}

struct PollParams {
    offset: Option<i64>,
    limit: usize,
    timeout: u64,
}

fn parse_pairs(input: &str, params: &mut PollParams) {
    for pair in input.split('&') {
        let mut parts = pair.splitn(2, '=');
        let (Some(key), Some(value)) = (parts.next(), parts.next()) else {
            continue;
        };
        match key {
            "offset" => params.offset = value.parse().ok().or(params.offset),
            "limit" => params.limit = value.parse().unwrap_or(params.limit),
            "timeout" => params.timeout = value.parse().unwrap_or(params.timeout),
            _ => {}
        }
    }
}

fn poll_params(query: Option<&str>, body: &[u8]) -> PollParams {
    let mut params = PollParams { offset: None, limit: 100, timeout: 0 };

    if let Some(query) = query {
        parse_pairs(query, &mut params);
    }

    if let Ok(json) = serde_json::from_slice::<Value>(body) {
        params.offset = json["offset"].as_i64().or(params.offset);
        if let Some(limit) = json["limit"].as_u64() {
            params.limit = limit as usize;
        }
        if let Some(timeout) = json["timeout"].as_u64() {
            params.timeout = timeout;
        }
    } else if let Ok(body) = std::str::from_utf8(body) {
        parse_pairs(body, &mut params);
    }

    params
}

async fn poll_handler(
    State(state): State<Arc<PollingState>>,
    RawQuery(query): RawQuery,
    body: Bytes,
) -> impl IntoResponse {
    let params = poll_params(query.as_deref(), &body);

    if let Some(offset) = params.offset {
        let mut updates = state.updates.lock().await;
        let confirmed: Vec<i64> = updates.range(..offset).map(|(id, _)| *id).collect();
        for id in confirmed {
            if let Some(pending) = updates.remove(&id) {
                let _ = pending.confirm.send(());
            }
        }
    }

    let limit = params.limit.clamp(1, 100);
    let deadline = Instant::now() + Duration::from_secs(params.timeout.min(50));

    loop {
        let notified = state.notify.notified();

        {
            let updates = state.updates.lock().await;
            if !updates.is_empty() {
                let result: Vec<Value> = updates
                    .values()
                    .take(limit)
                    .map(|pending| pending.update.clone())
                    .collect();
                return axum::Json(serde_json::json!({ "ok": true, "result": result }));
            }
        }

        if Instant::now() >= deadline {
            break;
        }

        tokio::select! {
            _ = notified => {}
            _ = tokio::time::sleep_until(deadline) => break,
        }
    }

    axum::Json(serde_json::json!({ "ok": true, "result": [] }))
}

#[async_trait]
impl Egress<RequestData> for TelegramBotPollingEgress {
    type Output = ResponseData;

    fn services(&self) -> Vec<Box<dyn Runnable>> {
        vec![Box::new(self.server.clone())]
    }

    async fn setup(&mut self) {
        if self.state.registered.swap(true, Ordering::SeqCst) {
            return;
        }
        let router = Router::new()
            .route(&format!("{}/getUpdates", self.path), any(poll_handler))
            .with_state(self.state.clone());
        self.server.register(router).await;
    }

    async fn send(&self, data: RequestData, _meta: &Meta) -> Result<ResponseData, SendError> {
        let Ok(mut update) = serde_json::from_slice::<Value>(&data.body) else {
            return Err(SendError::permanent("telegram bot polling egress expects a json update"));
        };
        let Some(object) = update.as_object_mut() else {
            return Err(SendError::permanent("telegram bot polling egress expects a json object"));
        };

        let id = self.state.counter.fetch_add(1, Ordering::Relaxed);
        object.insert("update_id".to_string(), Value::from(id));

        let (confirm_tx, confirm_rx) = oneshot::channel();
        self.state.updates.lock().await.insert(id, PendingUpdate { update, confirm: confirm_tx });
        self.state.notify.notify_waiters();

        match confirm_rx.await {
            Ok(()) => Ok(ResponseData::default()),
            Err(_) => Err(SendError::retryable("update dropped before confirmation")),
        }
    }

    async fn stop(&self) {
        self.state.updates.lock().await.clear();
    }
}
