use axum::{
    routing::{get, post},
    Router,
};
use serde_json::Value;
use tokio::sync::mpsc;
use tokio::sync::mpsc::{Receiver, Sender};

use crate::api::message::ApiMessage;
use crate::base::Serverable;

use crate::api::methods;

use async_trait::async_trait;

pub struct Api {
    base_path: String,
    tx: Sender<ApiMessage>,
    pub rx: Receiver<ApiMessage>,
}

impl Api {
    pub fn new(base_path: String) -> Self {
        let (tx, rx) = mpsc::channel::<ApiMessage>(100);
        Self { base_path, tx, rx }
    }
}

#[async_trait]
impl Serverable for Api {
    async fn set_server(&self, main_router: Router<Sender<Value>>) -> Router<Sender<Value>> {
        let router = Router::new()
            .route("/routes", get(methods::get_routes))
            .route(
                "/route",
                post(methods::add_route).delete(methods::remove_route),
            )
            .with_state(self.tx.clone());

        main_router.nest(&self.base_path, router)
    }
}
