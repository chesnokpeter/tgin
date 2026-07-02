use axum::body::Bytes;
use axum::http::{HeaderMap, HeaderName, HeaderValue, StatusCode};
use serde::{Deserialize, Serialize};
use serde::de::DeserializeOwned;

use crate::types::request::{RequestData, ResponseData};

#[derive(Serialize, Deserialize)]
pub struct RequestFrame {
    pub id: u64,
    pub method: String,
    pub uri: String,
    pub headers: Vec<(String, String)>,
}

#[derive(Serialize, Deserialize)]
pub struct ResponseFrame {
    pub id: u64,
    pub status: u16,
    pub headers: Vec<(String, String)>,
}

fn encode<F: Serialize>(frame: &F, body: &[u8]) -> Vec<u8> {
    let header = serde_json::to_vec(frame).expect("tunnel frame serialization");
    let mut message = Vec::with_capacity(4 + header.len() + body.len());
    message.extend_from_slice(&(header.len() as u32).to_be_bytes());
    message.extend_from_slice(&header);
    message.extend_from_slice(body);
    message
}

fn decode<F: DeserializeOwned>(message: &[u8]) -> Option<(F, Bytes)> {
    let length = u32::from_be_bytes(message.get(..4)?.try_into().ok()?) as usize;
    let frame = serde_json::from_slice(message.get(4..4 + length)?).ok()?;
    let body = Bytes::copy_from_slice(message.get(4 + length..)?);
    Some((frame, body))
}

fn headers_to_pairs(headers: &HeaderMap) -> Vec<(String, String)> {
    headers
        .iter()
        .filter_map(|(name, value)| Some((name.to_string(), value.to_str().ok()?.to_string())))
        .collect()
}

fn pairs_to_headers(pairs: &[(String, String)]) -> HeaderMap {
    let mut headers = HeaderMap::new();
    for (name, value) in pairs {
        let (Ok(name), Ok(value)) = (
            HeaderName::from_bytes(name.as_bytes()),
            HeaderValue::from_str(value),
        ) else {
            continue;
        };
        headers.append(name, value);
    }
    headers
}

pub fn encode_request(id: u64, data: &RequestData) -> Vec<u8> {
    let frame = RequestFrame {
        id,
        method: data.method.to_string(),
        uri: data.uri.to_string(),
        headers: headers_to_pairs(&data.headers),
    };
    encode(&frame, &data.body)
}

pub fn decode_request(message: &[u8]) -> Option<(u64, RequestData)> {
    let (frame, body): (RequestFrame, Bytes) = decode(message)?;
    Some((
        frame.id,
        RequestData {
            body,
            uri: frame.uri.parse().ok()?,
            method: frame.method.parse().ok()?,
            headers: pairs_to_headers(&frame.headers),
            client_ip: None,
        },
    ))
}

pub fn encode_response(id: u64, data: &ResponseData) -> Vec<u8> {
    let frame = ResponseFrame {
        id,
        status: data.status.as_u16(),
        headers: headers_to_pairs(&data.headers),
    };
    encode(&frame, &data.body)
}

pub fn decode_response(message: &[u8]) -> Option<(u64, ResponseData)> {
    let (frame, body): (ResponseFrame, Bytes) = decode(message)?;
    Some((
        frame.id,
        ResponseData {
            status: StatusCode::from_u16(frame.status).ok()?,
            headers: pairs_to_headers(&frame.headers),
            body,
        },
    ))
}
