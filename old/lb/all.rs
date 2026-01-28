use crate::base::{Printable, Routeable, RouteableComponent, Serverable};

use axum::Router;
use tokio::sync::{mpsc::Sender, RwLock};

use std::sync::Arc;

use crate::api::message::{AddRouteType, RmRoute};
use crate::dynamic::longpoll_registry::LONGPOLL_REGISTRY;

use async_trait::async_trait;

use serde_json::{json, Value};

pub struct AllLB {
    routes: RwLock<Vec<Arc<dyn RouteableComponent>>>,
}

impl AllLB {
    pub fn new(routes: Vec<Arc<dyn RouteableComponent>>) -> Self {
        Self {
            routes: RwLock::new(routes),
        }
    }
}

#[async_trait]
impl Routeable for AllLB {
    async fn process(&self, update: Value) {
        let routes = self.routes.read().await;
        for route in routes.iter() {
            let route = route.clone();
            let update = update.clone();

            tokio::spawn(async move {
                route.process(update).await;
            });
        }
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
impl Serverable for AllLB {
    async fn set_server(&self, mut router: Router<Sender<Value>>) -> Router<Sender<Value>> {
        let routes = self.routes.read().await;
        for route in routes.iter() {
            router = route.set_server(router).await;
        }
        router
    }
}

#[async_trait]
impl Printable for AllLB {
    async fn print(&self) -> String {
        let routes = self.routes.read().await;
        let mut text = String::from("LOAD BALANCER All\n\n");

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
            "name": "all",
            "routes": routes_json
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mock::routes::MockCallsRoute;
    use std::time::Duration;

    #[tokio::test]
    async fn test_empty_routes_does_not_panic() {
        let lb = AllLB::new(vec![]);

        lb.process(json!({"update_id": 1})).await;

        let json = lb.json_struct().await;
        assert_eq!(json["routes"].as_array().unwrap().len(), 0);
    }

    #[tokio::test]
    async fn test_all_broadcast_distribution() {
        let route1 = Arc::new(MockCallsRoute::new("A"));
        let route2 = Arc::new(MockCallsRoute::new("B"));

        let lb = AllLB::new(vec![route1.clone(), route2.clone()]);

        lb.process(json!({"msg": "hello"})).await;

        tokio::time::sleep(Duration::from_millis(50)).await;

        assert_eq!(route1.count().await, 1);
        assert_eq!(route2.count().await, 1);
        let c1 = route1.get_calls().await;
        let c2 = route2.get_calls().await;
        assert_eq!(c1[0]["msg"], "hello");
        assert_eq!(c2[0]["msg"], "hello");
    }

    #[tokio::test]
    async fn test_multiple_messages_broadcast() {
        let r1 = Arc::new(MockCallsRoute::new("1"));
        let r2 = Arc::new(MockCallsRoute::new("2"));
        let r3 = Arc::new(MockCallsRoute::new("3"));

        let lb = AllLB::new(vec![r1.clone(), r2.clone(), r3.clone()]);

        lb.process(json!("m1")).await;
        lb.process(json!("m2")).await;
        lb.process(json!("m3")).await;

        tokio::time::sleep(Duration::from_millis(50)).await;

        assert_eq!(r1.count().await, 3);
        assert_eq!(r2.count().await, 3);
        assert_eq!(r3.count().await, 3);
    }

    #[tokio::test]
    async fn test_json_structure() {
        let r1 = Arc::new(MockCallsRoute::new("x"));
        let lb = AllLB::new(vec![r1]);

        let output = lb.json_struct().await;

        assert_eq!(output["type"], "load-balancer");
        assert_eq!(output["name"], "all");
        assert_eq!(output["routes"].as_array().unwrap().len(), 1);
    }

    #[tokio::test]
    async fn test_dynamic_add_route_broadcast() {
        let r1 = Arc::new(MockCallsRoute::new("static"));
        let lb = AllLB::new(vec![r1.clone()]);

        lb.process(json!(1)).await;
        tokio::time::sleep(Duration::from_millis(20)).await;
        assert_eq!(r1.count().await, 1);

        let r2 = Arc::new(MockCallsRoute::new("dynamic"));
        let add_res = lb.add_route(AddRouteType::Webhook(r2.clone())).await;
        assert!(add_res.is_ok());

        let json_out = lb.json_struct().await;
        assert_eq!(json_out["routes"].as_array().unwrap().len(), 2);

        lb.process(json!(2)).await;
        tokio::time::sleep(Duration::from_millis(20)).await;

        assert_eq!(r1.count().await, 2);
        assert_eq!(r2.count().await, 1);
    }
}
