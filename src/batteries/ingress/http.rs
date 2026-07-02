use std::net::SocketAddr;

use tokio::sync::mpsc::Sender;
use tokio::sync::mpsc::error::TrySendError;
use tokio::sync::oneshot;
use async_trait::async_trait;
use axum::{
    Router,
    extract::{ConnectInfo, State},
    http::{Method, StatusCode},
    routing::any,
    response::IntoResponse,
};

use crate::base::{Envelope, Ingress, Runnable, SendError};
use crate::shared::server::HttpServer;
use crate::types::request::{RequestData, ResponseData};

#[derive(Clone)]
struct HandlerState {
    tx: Sender<Envelope<RequestData, ResponseData>>,
    method: Option<Method>,
}

pub struct HttpIngress {
    server: HttpServer,
    path: Option<String>,
    method: Option<Method>,
}

impl HttpIngress {
    pub fn post(server: &HttpServer, path: &str) -> Self {
        Self { server: server.clone(), path: Some(path.to_string()), method: Some(Method::POST) }
    }

    pub fn get(server: &HttpServer, path: &str) -> Self {
        Self { server: server.clone(), path: Some(path.to_string()), method: Some(Method::GET) }
    }

    pub fn any(server: &HttpServer, path: &str) -> Self {
        Self { server: server.clone(), path: Some(path.to_string()), method: None }
    }

    pub fn catch_all(server: &HttpServer) -> Self {
        Self { server: server.clone(), path: None, method: None }
    }
}

#[async_trait]
impl Ingress<RequestData, ResponseData> for HttpIngress {
    fn services(&self) -> Vec<Box<dyn Runnable>> {
        vec![Box::new(self.server.clone())]
    }

    async fn setup(&mut self, tx: Sender<Envelope<RequestData, ResponseData>>) {
        let state = HandlerState { tx, method: self.method.clone() };
        let router = match &self.path {
            Some(path) => Router::new().route(path, any(handler)),
            None => Router::new().fallback(any(handler)),
        };
        self.server.register(router.with_state(state)).await;
    }
}

async fn handler(
    State(state): State<HandlerState>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    mut data: RequestData,
) -> impl IntoResponse {
    data.client_ip = Some(peer.ip());

    if let Some(method) = &state.method {
        if data.method != *method {
            return (StatusCode::METHOD_NOT_ALLOWED, "").into_response();
        }
    }

    let (back_tx, back_rx) = oneshot::channel();

    match state.tx.try_send(Envelope::backward(data, back_tx)) {
        Ok(()) => {}
        Err(TrySendError::Full(_)) => return (StatusCode::TOO_MANY_REQUESTS, "").into_response(),
        Err(TrySendError::Closed(_)) => return (StatusCode::SERVICE_UNAVAILABLE, "").into_response(),
    }

    match back_rx.await {
        Ok(Ok(response)) => {
            let mut builder = axum::response::Response::builder().status(response.status);

            if let Some(headers) = builder.headers_mut() {
                *headers = response.headers;
            }

            builder
                .body(axum::body::Body::from(response.body))
                .unwrap_or_else(|_| (StatusCode::INTERNAL_SERVER_ERROR, "").into_response())
        }
        Ok(Err(SendError::Overloaded)) => (StatusCode::SERVICE_UNAVAILABLE, "").into_response(),
        Ok(Err(SendError::DeadlineExceeded)) => (StatusCode::GATEWAY_TIMEOUT, "").into_response(),
        Ok(Err(_)) => (StatusCode::BAD_GATEWAY, "").into_response(),
        Err(_) => (StatusCode::BAD_GATEWAY, "").into_response(),
    }
}
