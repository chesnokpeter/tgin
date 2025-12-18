
use axum::{http, extract::State, Json, response::IntoResponse};
use serde_json::Value;
use tokio::sync::mpsc::Sender;
use tokio::sync::oneshot;
use tokio;
use std::sync::Arc;

use crate::api::schemas::{AddRoute};
use crate::api::message::ApiMessage;

use crate::route::webhook::WebhookRoute;








pub async fn add_route(State(tx): State<Sender<ApiMessage>>, Json(data): Json<AddRoute>)  {
    match data {
        AddRoute::Longpull(route) => {

        },
        AddRoute::Webhook(data) => {
            let update = WebhookRoute::new(data.url);
            let route = Arc::new(update);
            let _ = tx.send(ApiMessage::AddRoute{
                sublevel: data.sublevel,
                route
            }).await;
        }
    }

}



pub async fn get_routes(State(tx): State<Sender<ApiMessage>>) -> Result<Json<Value>, impl IntoResponse> {
    let (tx_response, rx_response) = oneshot::channel();

    let _ = tx.send(ApiMessage::GetRoutes(tx_response)).await;

    match rx_response.await {
        Ok(json) => Ok(json),
        Err(_) => Err((
            http::StatusCode::INTERNAL_SERVER_ERROR
        )),
    }
}