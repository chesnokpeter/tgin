use crate::base::{Printable, Routeable, Serverable};
use async_trait::async_trait;

use std::collections::VecDeque;

use axum::{extract::Form, routing::post, Json, Router};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc::Sender;
use tokio::sync::{Mutex, Notify};
use tokio::time::timeout as tokio_timeout;

#[derive(Serialize, Deserialize, Debug)]
pub struct GetUpdatesParams {
    #[serde(default)]
    pub offset: Option<i64>,
    #[serde(default)]
    pub timeout: Option<u64>,
    #[serde(default)]
    pub limit: Option<u64>,
}

#[derive(Clone)]
pub struct LongPollRoute {
    updates: Arc<Mutex<VecDeque<Value>>>,
    notify: Arc<Notify>,
    pub path: String,
}

impl LongPollRoute {
    pub fn new(path: String) -> Self {
        Self {
            updates: Arc::new(Mutex::new(VecDeque::new())),
            notify: Arc::new(Notify::new()),
            path,
        }
    }

    pub async fn handle_request(&self, params: GetUpdatesParams) -> Json<Value> {
        let updates = self.updates.clone();
        let notify = self.notify.clone();

        let timeout_sec = params.timeout.unwrap_or(0);
        let start_time = tokio::time::Instant::now();
        let duration = Duration::from_secs(timeout_sec);

        loop {
            {
                let mut lock = updates.lock().await;

                if !lock.is_empty() {
                    let mut batch = Vec::new();

                    let limit = params.limit.unwrap_or(1000) as usize;

                    while batch.len() < limit {
                        if let Some(upd) = lock.pop_front() {
                            batch.push(upd);
                        } else {
                            break;
                        }
                    }

                    return Json(json!({
                        "ok": true,
                        "result": batch
                    }));
                }
            }

            if timeout_sec == 0 || start_time.elapsed() >= duration {
                return Json(json!({
                    "ok": true,
                    "result": []
                }));
            }

            let remaining = duration.saturating_sub(start_time.elapsed());
            let _ = tokio_timeout(remaining, notify.notified()).await;
        }
    }
}

#[async_trait]
impl Routeable for LongPollRoute {
    async fn process(&self, update: Value) {
        let mut lock = self.updates.lock().await;
        lock.push_back(update);
        self.notify.notify_waiters();
    }

    fn diff_value(&self) -> Option<&str> {
        Some(&self.path)
    }
}

#[async_trait]
impl Serverable for LongPollRoute {
    async fn set_server(&self, router: Router<Sender<Value>>) -> Router<Sender<Value>> {
        let this = self.clone();
        let path = self.path.clone();

        let handler = move |Form(params): Form<GetUpdatesParams>| {
            let this = this.clone();

            async move { this.handle_request(params).await }
        };

        router.route(&path, post(handler))
    }
}

#[async_trait]
impl Printable for LongPollRoute {
    async fn print(&self) -> String {
        format!("longpull: http://0.0.0.0{}", self.path)
    }

    async fn json_struct(&self) -> Value {
        json!({
            "type": "longpoll",
            "options": {
                "path": self.path
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use tower::ServiceExt;

    fn default_params() -> GetUpdatesParams {
        GetUpdatesParams {
            offset: None,
            timeout: Some(0),
            limit: Some(100),
        }
    }

    #[tokio::test]
    async fn test_basic_process_and_retrieve() {
        let route = LongPollRoute::new("/bot/updates".to_string());

        route.process(json!({"update_id": 1})).await;
        route.process(json!({"update_id": 2})).await;

        let response = route.handle_request(default_params()).await;

        let body: Value = serde_json::to_value(response.0).unwrap();
        let results = body.get("result").unwrap().as_array().unwrap();

        assert_eq!(results.len(), 2);
        assert_eq!(results[0]["update_id"], 1);
        assert_eq!(results[1]["update_id"], 2);
    }

    #[tokio::test]
    async fn test_limit_batching() {
        let route = LongPollRoute::new("/test".to_string());

        for i in 0..10 {
            route.process(json!({"id": i})).await;
        }
        // Запрашиваем только 4
        let params = GetUpdatesParams {
            limit: Some(4),
            ..default_params()
        };

        let response = route.handle_request(params).await;
        let body: Value = serde_json::to_value(response.0).unwrap();
        let results = body.get("result").unwrap().as_array().unwrap();

        assert_eq!(results.len(), 4);
        assert_eq!(results[0]["id"], 0);
        assert_eq!(results[3]["id"], 3);

        let remaining = route.handle_request(default_params()).await;
        let rem_body: Value = serde_json::to_value(remaining.0).unwrap();
        assert_eq!(rem_body["result"].as_array().unwrap().len(), 6);
    }

    #[tokio::test]
    async fn test_timeout_empty_queue() {
        let route = LongPollRoute::new("/test".to_string());

        let start = tokio::time::Instant::now();

        let params = GetUpdatesParams {
            timeout: Some(1),
            ..default_params()
        };

        let response = route.handle_request(params).await;
        let duration = start.elapsed();

        let body: Value = serde_json::to_value(response.0).unwrap();
        let results = body.get("result").unwrap().as_array().unwrap();

        assert_eq!(results.len(), 0);
        assert!(duration.as_millis() >= 1000);
    }

    #[tokio::test]
    async fn test_longpoll_notification() {
        let route = Arc::new(LongPollRoute::new("/test".to_string()));
        let route_clone = route.clone();

        let handle = tokio::spawn(async move {
            let params = GetUpdatesParams {
                timeout: Some(5),
                ..default_params()
            };
            route_clone.handle_request(params).await
        });

        tokio::time::sleep(Duration::from_millis(100)).await;

        route.process(json!({"msg": "hello"})).await;
        let response = handle.await.unwrap();
        let body: Value = serde_json::to_value(response.0).unwrap();
        let results = body.get("result").unwrap().as_array().unwrap();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0]["msg"], "hello");
    }

    #[tokio::test]
    async fn test_axum_server_integration() {
        let path = "/bot123/getUpdates";
        let route = LongPollRoute::new(path.to_string());

        route.process(json!({"ok": true})).await;

        let app = Router::new();
        let app = route.set_server(app).await;

        let request = Request::builder()
            .method("POST")
            .uri(path)
            .header("content-type", "application/x-www-form-urlencoded")
            .body(Body::from("timeout=0&limit=10"))
            .unwrap();

        let (tx, _rx) = tokio::sync::mpsc::channel(1);
        let app = app.with_state(tx);

        let response = app.oneshot(request).await.unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let body_json: Value = serde_json::from_slice(&body_bytes).unwrap();

        assert_eq!(body_json["result"].as_array().unwrap().len(), 1);
    }

    #[tokio::test]
    async fn test_printable_json_struct() {
        let route = LongPollRoute::new("/my/path".to_string());
        let json = route.json_struct().await;

        assert_eq!(json["type"], "longpoll");
        assert_eq!(json["options"]["path"], "/my/path");
    }
}
