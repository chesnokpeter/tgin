use std::net::IpAddr;

use reqwest::Client;
use async_trait::async_trait;
use axum::http::{
    HeaderMap, HeaderValue,
    header::{
        HeaderName, CONNECTION, CONTENT_LENGTH, HOST, PROXY_AUTHENTICATE, PROXY_AUTHORIZATION,
        TE, TRAILER, TRANSFER_ENCODING, UPGRADE,
    },
};

use crate::base::{Egress, Meta, SendError};
use crate::shared::client::HttpClient;
use crate::types::request::{RequestData, ResponseData};

#[derive(Clone)]
pub struct HttpEgress {
    url: String,
    client: Client,
}

impl HttpEgress {
    pub fn new(client: &HttpClient, url: &str) -> Self {
        Self { url: url.to_string(), client: client.client() }
    }
}

fn forward_headers(mut headers: HeaderMap, client_ip: Option<IpAddr>) -> HeaderMap {
    let mut listed: Vec<HeaderName> = Vec::new();
    if let Some(connection) = headers.get(CONNECTION) {
        if let Ok(value) = connection.to_str() {
            for name in value.split(',') {
                if let Ok(name) = HeaderName::from_bytes(name.trim().as_bytes()) {
                    listed.push(name);
                }
            }
        }
    }
    for name in listed {
        headers.remove(name);
    }

    for name in [
        HOST, CONNECTION, CONTENT_LENGTH, TE, TRAILER, TRANSFER_ENCODING, UPGRADE,
        PROXY_AUTHENTICATE, PROXY_AUTHORIZATION,
    ] {
        headers.remove(name);
    }
    headers.remove(HeaderName::from_static("keep-alive"));
    headers.remove(HeaderName::from_static("proxy-connection"));

    if let Some(ip) = client_ip {
        let forwarded = HeaderName::from_static("x-forwarded-for");
        let value = match headers.get(&forwarded).and_then(|v| v.to_str().ok()) {
            Some(existing) => format!("{existing}, {ip}"),
            None => ip.to_string(),
        };
        if let Ok(value) = HeaderValue::from_str(&value) {
            headers.insert(forwarded, value);
        }
    }

    headers
}

#[async_trait]
impl Egress<RequestData> for HttpEgress {
    type Output = ResponseData;

    async fn send(&self, data: RequestData, _meta: &Meta) -> Result<ResponseData, SendError> {
        let url = format!("{}{}", self.url, data.uri);
        let headers = forward_headers(data.headers, data.client_ip);

        let response = self.client
            .request(data.method, url)
            .body(data.body)
            .headers(headers)
            .send()
            .await
            .map_err(SendError::retryable)?;

        let status = response.status();
        let headers = response.headers().clone();
        let body = response.bytes().await.map_err(SendError::retryable)?;

        Ok(ResponseData { status, headers, body })
    }
}
