use std::sync::Arc;
use tokio::sync::{mpsc::{Receiver, Sender}, mpsc, Mutex, oneshot};
use async_trait::async_trait;
use axum::{
    Router, extract::State, http::{Method, StatusCode}, routing::any, response::IntoResponse
};

use crate::base::{Ingress, Envelope};
use crate::batteries::data::request::{RequestData, ResponseData};
use crate::shared::server::HttpServer;

#[derive(Clone)]
struct HandlerState {
    tx: Sender<Envelope<RequestData, ResponseData>>,
    method: Method,
}

pub struct HttpIngress {
    path: String,
    method: Method,
    server: Arc<Mutex<HttpServer>>, 
}

impl HttpIngress {
    pub fn new(
        path: &str, 
        method: Method, 
        server: Arc<Mutex<HttpServer>> 
    ) -> Self {
        Self {
            path: path.to_string(),
            method,
            server,
        }
    }
}

#[async_trait]
impl Ingress<RequestData, ResponseData> for HttpIngress {
    async fn start(&self, tx: Sender<Envelope<RequestData, ResponseData>>) {
        let state = HandlerState { 
            tx, 
            method: self.method.clone()
        };
        let router = Router::new()
            .route(&self.path, any(handler))
            .with_state(state);

        let mut shared_server = self.server.lock().await;
        shared_server.register_route(router);
    }
}




async fn handler(
    State(state): State<HandlerState>,
    data: RequestData 
) -> impl IntoResponse {
    if data.method != state.method {
        return (StatusCode::METHOD_NOT_ALLOWED, "").into_response()
    }

    let (back_tx, back_rx) = oneshot::channel();

    if state.tx.send(Envelope::backward(data, back_tx)).await.is_err() {
        return (StatusCode::INTERNAL_SERVER_ERROR, "").into_response()
    }

    match back_rx.await {
        Ok(response) => {
            let mut builder = axum::response::Response::builder()
                .status(response.status);

            if let Some(headers) = builder.headers_mut() {
                *headers = response.headers;
            }

            builder.body(axum::body::Body::from(response.body))
                .unwrap_or_else(|_| (StatusCode::INTERNAL_SERVER_ERROR, "").into_response())
        },
        Err(_) => {
            return (StatusCode::BAD_GATEWAY, "").into_response()
        }
    }
}

