
use axum::{http, extract::State, Json, response::IntoResponse};
use serde_json::Value;
use tokio::sync::mpsc::Sender;
use tokio::sync::oneshot;
use tokio;
use std::sync::Arc;

use crate::api::schemas::{AddRoute, RouteType};
use crate::api::message::{ApiMessage, AddRouteType};

use crate::route::webhook::WebhookRoute;
use crate::route::longpull::LongPollRoute;







pub async fn add_route(State(tx): State<Sender<ApiMessage>>, Json(data): Json<AddRoute>)  {
    let route = match data.typee {
        RouteType::Longpull(route) => {
            let update = LongPollRoute::new(route.path);
            AddRouteType::Longpull(Arc::new(update))
        },
        RouteType::Webhook(route) => {
            let update = WebhookRoute::new(route.url);
            AddRouteType::Webhook(Arc::new(update))
        }
    };

    let _ = tx.send(ApiMessage::AddRoute{
        sublevel: data.sublevel,
        route
        }).await;

}



pub async fn get_routes(State(tx): State<Sender<ApiMessage>>) -> Result<Json<Value>, impl IntoResponse> {
    let (tx_response, rx_response) = oneshot::channel();

    let _ = tx.send(ApiMessage::GetRoutes(tx_response)).await;

    match rx_response.await {
        Ok(json) => Ok(Json::from(json)),
        Err(_) => Err(
            http::StatusCode::INTERNAL_SERVER_ERROR
        ),
    }
}