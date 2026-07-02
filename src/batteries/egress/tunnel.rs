use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use async_trait::async_trait;
use axum::{
    Router,
    extract::{
        State, WebSocketUpgrade,
        ws::{Message, WebSocket},
    },
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    routing::get,
};
use dashmap::DashMap;
use futures::{SinkExt, StreamExt};
use tokio::sync::{Mutex, mpsc, oneshot};

use crate::base::{Egress, Meta, Runnable, SendError};
use crate::shared::server::HttpServer;
use crate::types::request::{RequestData, ResponseData};
use crate::types::tunnel::{decode_response, encode_request};

struct TunnelState {
    token: String,
    agent: Mutex<Option<mpsc::Sender<Vec<u8>>>>,
    pending: DashMap<u64, oneshot::Sender<ResponseData>>,
    counter: AtomicU64,
    registered: AtomicBool,
}

#[derive(Clone)]
pub struct TunnelEgress {
    server: HttpServer,
    path: String,
    state: Arc<TunnelState>,
}

impl TunnelEgress {
    pub fn new(server: &HttpServer, path: &str, token: &str) -> Self {
        Self {
            server: server.clone(),
            path: path.to_string(),
            state: Arc::new(TunnelState {
                token: token.to_string(),
                agent: Mutex::new(None),
                pending: DashMap::new(),
                counter: AtomicU64::new(0),
                registered: AtomicBool::new(false),
            }),
        }
    }
}

async fn handler(
    State(state): State<Arc<TunnelState>>,
    headers: HeaderMap,
    upgrade: WebSocketUpgrade,
) -> impl IntoResponse {
    let authorized = headers
        .get("authorization")
        .and_then(|value| value.to_str().ok())
        .map(|value| value == format!("Bearer {}", state.token))
        .unwrap_or(false);

    if !authorized {
        return StatusCode::UNAUTHORIZED.into_response();
    }

    upgrade.on_upgrade(move |socket| agent_session(state, socket))
}

async fn agent_session(state: Arc<TunnelState>, socket: WebSocket) {
    let (mut sink, mut stream) = socket.split();
    let (tx, mut rx) = mpsc::channel::<Vec<u8>>(256);
    let session = tx.clone();

    *state.agent.lock().await = Some(tx);

    let writer = tokio::spawn(async move {
        while let Some(message) = rx.recv().await {
            if sink.send(Message::Binary(message)).await.is_err() {
                return;
            }
        }
    });

    while let Some(Ok(message)) = stream.next().await {
        let Message::Binary(payload) = message else {
            continue;
        };
        let Some((id, response)) = decode_response(&payload) else {
            continue;
        };
        if let Some((_, reply)) = state.pending.remove(&id) {
            let _ = reply.send(response);
        }
    }

    let mut guard = state.agent.lock().await;
    if guard.as_ref().is_some_and(|current| current.same_channel(&session)) {
        *guard = None;
        drop(guard);
        state.pending.clear();
    }

    writer.abort();
}

#[async_trait]
impl Egress<RequestData> for TunnelEgress {
    type Output = ResponseData;

    fn services(&self) -> Vec<Box<dyn Runnable>> {
        vec![Box::new(self.server.clone())]
    }

    async fn setup(&mut self) {
        if self.state.registered.swap(true, Ordering::SeqCst) {
            return;
        }
        let router: Router = Router::new()
            .route(&self.path, get(handler))
            .with_state(self.state.clone());
        self.server.register(router).await;
    }

    async fn send(&self, data: RequestData, _meta: &Meta) -> Result<ResponseData, SendError> {
        let Some(agent) = self.state.agent.lock().await.clone() else {
            return Err(SendError::retryable("no tunnel agent connected"));
        };

        let id = self.state.counter.fetch_add(1, Ordering::Relaxed);
        let (reply_tx, reply_rx) = oneshot::channel();
        self.state.pending.insert(id, reply_tx);

        if agent.send(encode_request(id, &data)).await.is_err() {
            self.state.pending.remove(&id);
            return Err(SendError::retryable("tunnel agent disconnected"));
        }

        match reply_rx.await {
            Ok(response) => Ok(response),
            Err(_) => Err(SendError::retryable("tunnel agent dropped the request")),
        }
    }
}
