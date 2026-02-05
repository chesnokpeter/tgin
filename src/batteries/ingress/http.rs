use std::sync::Arc;
use tokio::sync::{mpsc::{Receiver, Sender}, mpsc, Mutex};
use async_trait::async_trait;
use axum::{
    Router, extract::State, http::{Method, method}, routing::any
};

use crate::base::Ingress;
use crate::batteries::data::request::RequestData;
use crate::shared::server::HttpServer;

#[derive(Clone)]
struct HandlerState {
    tx: Sender<RequestData>,
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
impl Ingress<RequestData> for HttpIngress {
    async fn start(&self, tx: Sender<RequestData>) {
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
) {
    if state.method == data.method {
        let _ = state.tx.send(data).await;
    }
}

