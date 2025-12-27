
use crate::base::{Routeable, RouteableComponent, Serverable, Printable};

use crate::api::message::AddRouteType;

use tokio::sync::mpsc::Sender;
use axum::{Router, Json};

use tokio::sync::RwLock;

use std::mem::zeroed;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use crate::dynamic::longpoll_registry::LONGPOLL_REGISTRY;

use async_trait::async_trait;

use serde_json::{Value, json};

pub struct RoundRobinLB {
    routes: RwLock<Vec<Arc<dyn RouteableComponent>>>,
    current: AtomicUsize,
}

impl RoundRobinLB {
    pub fn new(routes: Vec<Arc<dyn RouteableComponent>>) -> Self {
        Self {
            routes:RwLock::new(routes),
            current: AtomicUsize::new(0),
        }
    }
}

#[async_trait]
impl Routeable for RoundRobinLB {
    async fn process(&self, update: Value) {
        let routes = self.routes.read().await;
        if routes.is_empty() {
            return;
        }
        let current = self.current.fetch_add(1, Ordering::Relaxed);
        let index = current % routes.len();

        let route = routes[index].clone();

        drop(routes); 

        route.process(update).await;

    }

    async fn add_route(&self, route: AddRouteType) -> Result<(), ()>{
        let mut routes = self.routes.write().await;

        match route {
            AddRouteType::Longpull(route_arc) => {
                match LONGPOLL_REGISTRY.write() {
                    Ok(mut registry) => {
                        registry.insert(route_arc.path.clone(), route_arc.clone());
                        routes.push(route_arc); 
                        Ok(())
                    }
                    Err(_) => {
                        Err(())
                    }
                }
            },
            AddRouteType::Webhook(route) => {
                routes.push(route); 
                Ok(())
            },
        }
    }
}

#[async_trait]
impl Serverable for RoundRobinLB {
    async fn set_server(&self, mut router: Router<Sender<Value>>) -> Router<Sender<Value>> {
        let routes = self.routes.read().await;
        for route in routes.iter() {
            router = route.set_server(router).await;
        }
        router
    }
}

#[async_trait]
impl Printable for RoundRobinLB {
    async fn print(&self) -> String {
        let routes = self.routes.read().await;
        let mut text = String::from("LOAD BALANCER RoundRobin\n\n");

        for route in routes.iter() {
            text.push_str(&format!("{}\n\n", route.print().await));
        }
        text
    }


    async fn json_struct(&self) -> Value {
        let routes = self.routes.read().await;
        let mut routes_json: Vec<Value> = Vec::new();
        for route in routes.iter() {
            routes_json.push(route.json_struct().await);
        }

        json!({
            "type": "load-balancer",
            "name": "round-robin",
            "routes": routes_json
        })
    }
}