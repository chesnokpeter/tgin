use async_trait::async_trait;
use tokio::sync::{mpsc, oneshot};
use reqwest::Client;
use std::collections::HashMap;
use std::time::Duration;
use axum::http::{Method, Uri, StatusCode, HeaderMap, HeaderName, HeaderValue};
use axum::body::Bytes;

use crate::base::{Ingress, Envelope};
use crate::batteries::data::request::{RequestData, ResponseData};

use crate::batteries::data::httptunnel::{TunnelRequest, TunnelResponse};


pub struct HttpTunnelIngress {
    control_plane_url: String,
    client: Client,
    poll_interval: Duration,
}

impl HttpTunnelIngress {
    pub fn new(control_plane_url: String) -> Self {
        Self {
            control_plane_url: control_plane_url.trim_end_matches('/').to_string(),
            client: Client::new(),
            poll_interval: Duration::from_millis(100), 
        }
    }
}

#[async_trait]
impl Ingress<RequestData, ResponseData> for HttpTunnelIngress {
    async fn start(&self, tx: mpsc::Sender<Envelope<RequestData, ResponseData>>) {
        let poll_url = format!("{}/_httptunnel/poll", self.control_plane_url);
        let reply_url = format!("{}/_httptunnel/reply", self.control_plane_url);

        loop {
            match self.client.get(&poll_url).send().await {
                Ok(resp) => {
                    match resp.json::<Option<TunnelRequest>>().await {
                        Ok(Some(task)) => {
                            let method = Method::from_bytes(task.method.as_bytes()).unwrap_or(Method::GET);
                            let uri = task.uri.parse::<Uri>().unwrap_or_else(|_| Uri::from_static("/"));
                            
                            let mut headers = HeaderMap::new();
                            for (k, v) in task.headers {
                                if let (Ok(kn), Ok(vn)) = (HeaderName::from_bytes(k.as_bytes()), HeaderValue::from_str(&v)) {
                                    headers.insert(kn, vn);
                                }
                            }

                            let request_data = RequestData {
                                method,
                                uri,
                                headers,
                                body: Bytes::from(task.body),
                            };

                            let (reply_tx, reply_rx) = oneshot::channel();
                            
                            let envelope = Envelope::backward(request_data, reply_tx);
                            
                            if tx.send(envelope).await.is_ok() {
                                let client = self.client.clone();
                                let r_url = reply_url.clone();
                                let task_id = task.id;

                                tokio::spawn(async move {
                                    if let Ok(local_resp) = reply_rx.await {
                                        let headers_map: HashMap<String, String> = local_resp.headers.iter()
                                            .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string()))
                                            .collect();

                                        let resp_dto = TunnelResponse {
                                            id: task_id,
                                            status: local_resp.status.as_u16(),
                                            headers: headers_map,
                                            body: local_resp.body.to_vec(),
                                        };

                                        if let Err(e) = client.post(&r_url).json(&resp_dto).send().await {
                                            eprintln!("Failed to send tunnel reply: {}", e);
                                        }
                                    }
                                });
                            }
                        }
                        Ok(None) => {
                            tokio::time::sleep(self.poll_interval).await;
                        }
                        Err(e) => {
                            eprintln!("Failed to parse tunnel task: {}", e);
                            tokio::time::sleep(Duration::from_secs(1)).await;
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Tunnel connection error: {}", e);
                    tokio::time::sleep(Duration::from_secs(2)).await;
                }
            }
        }
    }
}