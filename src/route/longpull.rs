use crate::base::{Routeable, Serverable, Printable};
use async_trait::async_trait;

use axum::{extract::Form, routing::post, Json, Router}; 
use serde::Deserialize;
use serde_json::{json, Value};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{Mutex, Notify};
use tokio::sync::mpsc::Sender;
use tokio::time::timeout as tokio_timeout;

pub struct LongPollRoute {
    updates: Arc<Mutex<Vec<Value>>>,
    notify: Arc<Notify>,
    path: String,
}

impl LongPollRoute {
    pub fn new(path: String) -> Self {
        Self {
            updates: Arc::new(Mutex::new(Vec::new())),
            notify: Arc::new(Notify::new()),
            path,
        }
    }
}

#[derive(Deserialize, Debug)]
struct GetUpdatesParams {
    #[serde(default)]
    offset: Option<i64>,
    #[serde(default)]
    timeout: Option<u64>,
}

#[async_trait]
impl Routeable for LongPollRoute {
    async fn process(&self, update: Value) {
        let mut lock = self.updates.lock().await;
        lock.push(update);
        self.notify.notify_waiters();
    }
}

impl Serverable for LongPollRoute {
    fn set_server(&self, router: Router<Sender<Value>>) -> Router<Sender<Value>> {
        let updates = self.updates.clone();
        let notify = self.notify.clone();

        let handler = move |Form(params): Form<GetUpdatesParams>| async move {
            let offset = params.offset.unwrap_or(0);
            let timeout_sec = params.timeout.unwrap_or(0);
            
            let start_time = tokio::time::Instant::now();
            let duration = Duration::from_secs(timeout_sec);

            loop {
                {
                    let mut lock = updates.lock().await;

                    if offset > 0 {
                        lock.retain(|u| {
                            u.get("update_id")
                                .and_then(|id| id.as_i64())
                                .map(|id| id >= offset)
                                .unwrap_or(true) 
                        });
                    }

                    let result: Vec<Value> = lock.iter()
                        .filter(|u| {
                            u.get("update_id")
                                .and_then(|id| id.as_i64())
                                .map(|id| id >= offset)
                                .unwrap_or(false)
                        })
                        .cloned()
                        .collect();

                    if !result.is_empty() {
                        return Json(json!({
                            "ok": true,
                            "result": result
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
        };

        router.route(&self.path, post(handler))
    }
}





impl Printable for LongPollRoute {
    fn print(&self) -> String {
        format!("longpull: http://0.0.0.0{}", self.path)
    }

    fn json_struct(&self) -> Json<Value> {
        Json(json!({
            "type": "longpoll",
            "options": {
                "path": self.path
            }
        }))
    }
}