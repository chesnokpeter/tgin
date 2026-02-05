mod base;
mod batteries;
mod shared;

use crate::batteries::{data::request::{RequestData, ResponseData}, egress::http::HttpEgress, ingress::http::HttpIngress};
use crate::base::{Ingress, Egress, Forwardable, Envelope};
use crate::shared::server::HttpServer;

use axum::http::Method;

use tokio::sync::{mpsc, Mutex};

use std::sync::Arc;

#[tokio::main]
async fn main() {
    let server = HttpServer::new("127.0.0.1".to_string(), 8080);
    let server_shared = Arc::new(Mutex::new(server));

    let ingress = HttpIngress::new("/test", Method::POST, server_shared.clone());

    let egress = HttpEgress::new("http://127.0.0.1:8090".to_string());

    let (tx, mut rx) = mpsc::channel::<Envelope<RequestData, ResponseData>>(1000000);


    let ingress_task = tokio::spawn(async move {
        ingress.start(tx).await;
    });

    let _ = ingress_task.await; 

    let server_task = tokio::spawn(async move {
        let server = {
            let mut guard = server_shared.lock().await;
            guard.run().await;
        };
    });


    while let Some(update) = rx.recv().await {
        let route_clone = egress.clone(); 
        tokio::spawn(async move {
            update.process(&route_clone).await;
        });

    }


}