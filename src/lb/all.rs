
use crate::base::{Routeable, RouteableComponent, Serverable, Printable};

use tokio::sync::mpsc::Sender;
use axum::{Router, Json};

use std::sync::Arc;


use async_trait::async_trait;

use serde_json::{Value, json};

pub struct AllLB {
    routes: Vec<Arc<dyn RouteableComponent>>,
}

impl AllLB {
    pub fn new(routes: Vec<Arc<dyn RouteableComponent>>) -> Self {
        Self {
            routes,
        }
    }
}

#[async_trait]
impl Routeable for AllLB {
    async fn process(&self, update: Value) {
        for route in &self.routes {

            let route = route.clone();
            let update = update.clone();

            tokio::spawn(async move {
                route.process(update).await;
            });
        }
    }
}

impl Serverable for AllLB {
    fn set_server(&self, mut router: Router<Sender<Value>>) -> Router<Sender<Value>> {
        for route in &self.routes {
            router = route.set_server(router);
        }
        router
    }
}

impl Printable for AllLB {
    fn print(&self) -> String {

        let mut text = String::from("LOAD BALANCER AllLB");

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
            "name": "all",
            "routes": routes_json
        }))
    }


}