use std::time::Duration;

use async_trait::async_trait;
use axum::http::StatusCode;
use futures::{SinkExt, StreamExt};
use tokio::sync::mpsc::{self, Sender};
use tokio::sync::oneshot;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_util::sync::CancellationToken;

use crate::base::{Envelope, Ingress, SendError};
use crate::types::request::{RequestData, ResponseData};
use crate::types::tunnel::{decode_request, encode_response};

pub struct TunnelIngress {
    url: String,
    token: String,
    reconnect: Duration,
}

impl TunnelIngress {
    pub fn new(url: &str, token: &str) -> Self {
        Self {
            url: url.to_string(),
            token: token.to_string(),
            reconnect: Duration::from_secs(5),
        }
    }

    pub fn reconnect(mut self, delay: Duration) -> Self {
        self.reconnect = delay;
        self
    }

    async fn session(&self, tx: &Sender<Envelope<RequestData, ResponseData>>, shutdown: &CancellationToken) {
        let Ok(mut request) = self.url.as_str().into_client_request() else {
            return;
        };
        let Ok(authorization) = format!("Bearer {}", self.token).parse() else {
            return;
        };
        request.headers_mut().insert("authorization", authorization);

        let Ok((socket, _)) = connect_async(request).await else {
            return;
        };
        let (mut sink, mut stream) = socket.split();
        let (out_tx, mut out_rx) = mpsc::channel::<Vec<u8>>(256);

        let writer = tokio::spawn(async move {
            while let Some(message) = out_rx.recv().await {
                if sink.send(Message::Binary(message)).await.is_err() {
                    return;
                }
            }
        });

        loop {
            let message = tokio::select! {
                _ = shutdown.cancelled() => break,
                next = stream.next() => match next {
                    Some(Ok(message)) => message,
                    _ => break,
                },
            };

            let Message::Binary(payload) = message else {
                continue;
            };
            let Some((id, data)) = decode_request(&payload) else {
                continue;
            };

            let (reply_tx, reply_rx) = oneshot::channel();

            if tx.send(Envelope::backward(data, reply_tx)).await.is_err() {
                break;
            }

            let out = out_tx.clone();
            tokio::spawn(async move {
                let response = match reply_rx.await {
                    Ok(Ok(response)) => response,
                    Ok(Err(SendError::DeadlineExceeded)) => ResponseData {
                        status: StatusCode::GATEWAY_TIMEOUT,
                        ..Default::default()
                    },
                    _ => ResponseData {
                        status: StatusCode::BAD_GATEWAY,
                        ..Default::default()
                    },
                };
                let _ = out.send(encode_response(id, &response)).await;
            });
        }

        writer.abort();
    }
}

#[async_trait]
impl Ingress<RequestData, ResponseData> for TunnelIngress {
    async fn start(&self, tx: Sender<Envelope<RequestData, ResponseData>>, shutdown: CancellationToken) {
        loop {
            if shutdown.is_cancelled() {
                return;
            }
            self.session(&tx, &shutdown).await;
            tokio::select! {
                _ = shutdown.cancelled() => return,
                _ = tokio::time::sleep(self.reconnect) => {}
            }
        }
    }
}
