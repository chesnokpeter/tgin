
use axum::http::{Method, header::CONTENT_TYPE};
use axum::{extract::{Request}, Json}; 
use serde_json::{json, Value};

use crate::dynamic::longpoll_registry::LONGPOLL_REGISTRY;

use crate::route::longpull::GetUpdatesParams;


pub async fn dynamic_handler(
    request: Request, 
) -> Json<Value> {
    let (parts, body) = request.into_parts();
    
    let method = parts.method;
    let uri = parts.uri;
    let headers = parts.headers;
    let path = uri.path().to_string();

    let body_bytes = match axum::body::to_bytes(body, usize::MAX).await {
        Ok(b) => b,
        Err(_) => return Json(json!({
            "ok": false,
            "error_code": 400,
            "description": "failed to read request body"
        })),
    };

    if method != Method::POST {
        return Json(json!({ 
            "ok": false, 
            "error_code": 405, 
            "description": "method not allowed" 
        }));
    }

    let content_type = headers
        .get(CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    let params: GetUpdatesParams = if content_type.contains("application/json") {
        match serde_json::from_slice(&body_bytes) {
            Ok(p) => p,
            Err(_) => return Json(json!({ 
                "ok": false, 
                "error_code": 400, 
                "description": "invalid json body" 
            })),
        }
    } else {
        match serde_urlencoded::from_bytes(&body_bytes) {
            Ok(p) => p,
            Err(_) => GetUpdatesParams { 
                offset: None, 
                timeout: None, 
                limit: None 
            },
        }
    };

    let route = {
        let registry = LONGPOLL_REGISTRY.read().expect("Registry lock poisoned");
        registry.get(&path).cloned()
    };

    if let Some(route) = route {
        return route.handle_request(params).await;
    }

    Json(json!({ 
        "ok": false, 
        "error_code": 404, 
        "description": format!("Path {} not found in dynamic registry", path)
    }))
}