use crate::base::RouteableComponent;

use crate::route::longpull::LongPollRoute;

use std::sync::Arc;

use tokio::sync::oneshot::Sender;

use serde_json::Value;


pub enum AddRouteType {
    Longpull(Arc<LongPollRoute>),
    Webhook(Arc<dyn RouteableComponent>),
}

pub enum ApiMessage {
    AddRoute {
        route: AddRouteType,
        sublevel: i8
    },
    GetRoutes(Sender<Value>)
}