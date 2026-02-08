use async_trait::async_trait;
use std::sync::Arc;
use std::collections::{HashMap, VecDeque};
use tokio::sync::{Mutex, oneshot};
use axum::{Router, routing::{get, post}, extract::{State, Json}, http::StatusCode, response::IntoResponse};
use uuid::Uuid;

use crate::base::Egress;
use crate::batteries::data::request::{RequestData, ResponseData};
use crate::shared::server::HttpServer;

use crate::batteries::data::httptunnel::{TunnelRequest, TunnelResponse};


struct TunnelStateInner {
    queue: VecDeque<TunnelRequest>,
    pending: HashMap<Uuid, oneshot::Sender<ResponseData>>,
}

#[derive(Clone)]
struct TunnelState(Arc<Mutex<TunnelStateInner>>);


#[derive(Clone)] 
pub struct HttpTunnelEgress {
    state: TunnelState,
    server: Arc<Mutex<HttpServer>>, 
}

impl HttpTunnelEgress {
    pub async fn new(server: Arc<Mutex<HttpServer>>) -> Self {
        let state = TunnelState(Arc::new(Mutex::new(TunnelStateInner {
            queue: VecDeque::new(),
            pending: HashMap::new(),
        })));

        Self {
            state,
            server,
        }
    }
}

#[async_trait]
impl Egress<RequestData> for HttpTunnelEgress {
    type Output = ResponseData;

    async fn start(&self) {
        let router = Router::new()
            .route("/_httptunnel/poll", get(poll_handler))
            .route("/_httptunnel/reply", post(reply_handler))
            .with_state(self.state.clone());

        let mut server_guard = self.server.lock().await;
        server_guard.register_route(router);

    }

    async fn send(&self, data: RequestData) -> ResponseData {
        let (resp_tx, resp_rx) = oneshot::channel();
        let id = Uuid::new_v4();

        let dto = TunnelRequest {
            id,
            method: data.method.to_string(),
            uri: data.uri.to_string(),
            headers: data.headers.iter()
                .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string()))
                .collect(),
            body: data.body.to_vec(),
        };

        {
            let mut guard = self.state.0.lock().await;
            guard.queue.push_back(dto);
            guard.pending.insert(id, resp_tx);
        }

        match resp_rx.await {
            Ok(response) => response,
            Err(_) => ResponseData {
                status: StatusCode::GATEWAY_TIMEOUT,
                ..Default::default()
            }
        }
    }
}


async fn poll_handler(State(state): State<TunnelState>) -> impl IntoResponse {
    let mut guard = state.0.lock().await;
    Json(guard.queue.pop_front())
}

async fn reply_handler(
    State(state): State<TunnelState>, 
    Json(dto): Json<TunnelResponse>
) -> impl IntoResponse {
    let mut guard = state.0.lock().await;
    
    if let Some(tx) = guard.pending.remove(&dto.id) {
        let mut headers = axum::http::HeaderMap::new();
        for (k, v) in dto.headers {
            if let (Ok(kn), Ok(vn)) = (axum::http::HeaderName::from_bytes(k.as_bytes()), axum::http::HeaderValue::from_str(&v)) {
                headers.insert(kn, vn);
            }
        }

        let response = ResponseData {
            status: StatusCode::from_u16(dto.status).unwrap_or(StatusCode::OK),
            headers,
            body: axum::body::Bytes::from(dto.body),
        };

        let _ = tx.send(response);
        return StatusCode::OK;
    }

    StatusCode::NOT_FOUND
}