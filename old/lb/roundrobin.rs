use crate::base::{Printable, Routeable, RouteableComponent, Serverable};

use crate::api::message::{AddRouteType, RmRoute};

use axum::Router;
use tokio::sync::mpsc::Sender;

use tokio::sync::RwLock;

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use crate::dynamic::longpoll_registry::LONGPOLL_REGISTRY;

use async_trait::async_trait;

use serde_json::{json, Value};

pub struct RoundRobinLB {
    routes: RwLock<Vec<Arc<dyn RouteableComponent>>>,
    current: AtomicUsize,
}

impl RoundRobinLB {
    pub fn new(routes: Vec<Arc<dyn RouteableComponent>>) -> Self {
        Self {
            routes: RwLock::new(routes),
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

    async fn add_route(&self, route: AddRouteType) -> Result<(), ()> {
        let mut routes = self.routes.write().await;

        match route {
            AddRouteType::Longpull(route_arc) => match LONGPOLL_REGISTRY.write() {
                Ok(mut registry) => {
                    registry.insert(route_arc.path.clone(), route_arc.clone());
                    routes.push(route_arc);
                    Ok(())
                }
                Err(_) => Err(()),
            },
            AddRouteType::Webhook(route) => {
                routes.push(route);
                Ok(())
            }
        }
    }

    async fn remove_route(&self, route: RmRoute) -> Result<(), ()> {
        let mut routes = self.routes.write().await;
        let initial_len = routes.len();

        let target = if let Some(url) = &route.url {
            url
        } else if let Some(path) = &route.path {
            if let Ok(mut registry) = LONGPOLL_REGISTRY.write() {
                registry.remove(path);
            }
            path
        } else {
            return Err(());
        };

        routes.retain(|r| r.diff_value() != Some(&target));

        if routes.len() < initial_len {
            Ok(())
        } else {
            for i in routes.iter() {
                if i.diff_value().is_none() {
                    i.remove_route(route.clone()).await;
                };
            }
            Ok(())
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mock::routes::MockCallsRoute;
    #[tokio::test]

    async fn test_empty_routes_does_not_panic() {
        let lb = RoundRobinLB::new(vec![]);

        lb.process(json!({"update_id": 1})).await;

        let json = lb.json_struct().await;
        assert_eq!(json["routes"].as_array().unwrap().len(), 0);
    }

    #[tokio::test]
    async fn test_round_robin_distribution_even() {
        let route1 = Arc::new(MockCallsRoute::new("A"));
        let route2 = Arc::new(MockCallsRoute::new("B"));

        let lb = RoundRobinLB::new(vec![route1.clone(), route2.clone()]);

        for i in 0..4 {
            lb.process(json!({"id": i})).await;
        }

        assert_eq!(route1.count().await, 2);
        assert_eq!(route2.count().await, 2);
    }

    #[tokio::test]
    async fn test_round_robin_ordering() {
        let r1 = Arc::new(MockCallsRoute::new("1"));
        let r2 = Arc::new(MockCallsRoute::new("2"));
        let r3 = Arc::new(MockCallsRoute::new("3"));

        let lb = RoundRobinLB::new(vec![r1.clone(), r2.clone(), r3.clone()]);

        lb.process(json!("msg1")).await;
        let c1 = r1.get_calls().await;
        assert_eq!(c1.len(), 1);
        assert_eq!(c1[0], "msg1");
        assert_eq!(r2.count().await, 0);
        assert_eq!(r3.count().await, 0);

        lb.process(json!("msg2")).await;
        assert_eq!(r2.count().await, 1);

        lb.process(json!("msg3")).await;
        assert_eq!(r3.count().await, 1);

        lb.process(json!("msg4")).await;
        assert_eq!(r1.count().await, 2);
    }

    #[tokio::test]
    async fn test_json_structure_aggregation() {
        let r1 = Arc::new(MockCallsRoute::new("alpha"));
        let r2 = Arc::new(MockCallsRoute::new("beta"));

        let lb = RoundRobinLB::new(vec![r1, r2]);

        let output = lb.json_struct().await;

        assert_eq!(output["type"], "load-balancer");
        assert_eq!(output["name"], "round-robin");

        let routes_arr = output["routes"]
            .as_array()
            .expect("routes should be an array");
        assert_eq!(routes_arr.len(), 2);
        assert_eq!(routes_arr[0]["id"], "alpha");
        assert_eq!(routes_arr[1]["id"], "beta");
    }

    #[tokio::test]
    async fn test_dynamic_add_webhook_route() {
        let r1 = Arc::new(MockCallsRoute::new("static"));
        let lb = RoundRobinLB::new(vec![r1.clone()]);

        lb.process(json!(1)).await;
        assert_eq!(r1.count().await, 1);

        let r2 = Arc::new(MockCallsRoute::new("dynamic"));

        let add_res = lb.add_route(AddRouteType::Webhook(r2.clone())).await;
        assert!(add_res.is_ok());

        let json_out = lb.json_struct().await;
        assert_eq!(json_out["routes"].as_array().unwrap().len(), 2);

        lb.process(json!(2)).await;

        assert_eq!(r2.count().await, 1);
    }
}
