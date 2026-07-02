use axum::{http::{Method, Uri, HeaderMap, StatusCode}, extract::FromRequest};
use axum::body::{Bytes};
use axum::extract::Request;

use async_trait::async_trait;


#[derive(Debug, Clone)]
pub struct RequestData {
    pub body: Bytes,
    pub uri: Uri,
    pub method: Method,
    pub headers: HeaderMap,
    pub client_ip: Option<std::net::IpAddr>,
}

#[derive(Debug, Clone)]
pub struct ResponseData {
    pub status: StatusCode,
    pub headers: HeaderMap,
    pub body: Bytes,
}


impl Default for ResponseData {
    fn default() -> Self {
        Self {
            status: StatusCode::OK,
            headers: HeaderMap::new(),
            body: Bytes::new(),
        }
    }
}

impl From<()> for ResponseData {
    fn from(_: ()) -> Self {
        Self::default()
    }
}


#[async_trait]
impl<S> FromRequest<S> for RequestData
where
    Bytes: FromRequest<S>,
    S: Send + Sync,
{
    type Rejection = StatusCode;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        let (parts, body) = req.into_parts();

        let req = Request::from_parts(parts.clone(), body);

        let body_bytes = Bytes::from_request(req, state)
            .await
            .map_err(|_| StatusCode::BAD_REQUEST)?;

        Ok(Self {
            method: parts.method,
            uri: parts.uri,
            headers: parts.headers,
            body: body_bytes,
            client_ip: None,
        })
    }
}
