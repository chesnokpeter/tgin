
use crate::base::{Routeable, RouteableComponent, Serverable, Printable};

use tokio::sync::mpsc::Sender;
use axum::{Router, Json};

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;


use async_trait::async_trait;

use serde_json::{Value, json};

pub struct RoundRobinLB {
    routes: Vec<Arc<dyn RouteableComponent>>,
    current: AtomicUsize,
}

impl RoundRobinLB {
    pub fn new(routes: Vec<Arc<dyn RouteableComponent>>) -> Self {
        Self {
            routes,
            current: AtomicUsize::new(0),
        }
    }
}

#[async_trait]
impl Routeable for RoundRobinLB {
    async fn process(&self, update: Value) {
        if self.routes.is_empty() {
            return;
        }
        let current = self.current.fetch_add(1, Ordering::Relaxed);
        let index = current % self.routes.len();
        self.routes[index].process(update).await;
    }
}

impl Serverable for RoundRobinLB {
    fn set_server(&self, mut router: Router<Sender<Value>>) -> Router<Sender<Value>> {
        for route in &self.routes {
            router = route.set_server(router);
        }
        router
    }
}

impl Printable for RoundRobinLB {
    fn print(&self) -> String {

        let mut text = String::from("LOAD BALANCER RoundRobin\n\n");

        for route in &self.routes {
            text.push_str(&format!("{}\n\n", &route.print()));
        }

        text
    }

    fn json_struct(&self) -> Json<Value> {
        let routes_json: Vec<Value> = self.routes
            .iter()
            .map(|route| route.json_struct().0) 
            .collect();

        Json(json!({
            "type": "load-balancer",
            "name": "round-robin",
            "routes": routes_json
        }))
    }
}