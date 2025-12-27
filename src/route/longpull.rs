use crate::base::{Routeable, Serverable, Printable};
use async_trait::async_trait;

use std::collections::VecDeque;

use axum::{extract::Form, routing::post, Json, Router}; 
use serde::Deserialize;
use serde_json::{json, Value};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{Mutex, Notify};
use tokio::sync::mpsc::Sender;
use tokio::time::timeout as tokio_timeout;

#[derive(Deserialize, Debug)]
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

    pub async fn handle_request(&self, params: GetUpdatesParams) -> Json<Value>{

        let updates = self.updates.clone();
        let notify = self.notify.clone();

        let timeout_sec = params.timeout.unwrap_or(0);
        let start_time = tokio::time::Instant::now();
        let duration = Duration::from_secs(timeout_sec);

        loop {
            {
                let mut lock = updates.lock().await;

                if !lock.is_empty() {

                    println!("📤 [LP Queue {}] Popping batch. Size before: {}", self.path, lock.len());
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
        println!("📥 [LP Queue {}] Pushed. Size: {}", self.path, lock.len());
        lock.push_back(update);
        self.notify.notify_waiters();
    }
}

#[async_trait]
impl Serverable for LongPollRoute {
    async fn set_server(&self, router: Router<Sender<Value>>) -> Router<Sender<Value>> {
        let this = self.clone(); 
        let path = self.path.clone();

        let handler = move |Form(params): Form<GetUpdatesParams>| {
            let this = this.clone();
            
            async move {
                this.handle_request(params).await
            }
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