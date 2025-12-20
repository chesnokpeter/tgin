use crate::base::RouteableComponent;

use std::sync::Arc;

use tokio::sync::oneshot::Sender;

use serde_json::Value;


pub enum ApiMessage {
    AddRoute {
        route: Arc<dyn RouteableComponent>,
        sublevel: i8
    },
    GetRoutes(Sender<Value>)
}