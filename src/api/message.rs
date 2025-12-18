use crate::base::RouteableComponent;

use std::sync::Arc;

use tokio::sync::oneshot::Sender;

use axum::Json;
use serde_json::{Value, json};


pub enum ApiMessage {
    AddRoute {
        route: Arc<dyn RouteableComponent>,
        sublevel: i8
    },
    GetRoutes(Sender<Json<Value>>)
}