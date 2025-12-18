
use async_trait::async_trait;
use serde_json::{Value, json};

use tokio::sync::mpsc::Sender;

use axum::{Json, Router};

use crate::update::base::Updater;


#[async_trait]
pub trait Routeable: Send + Sync {
    async fn process(&self, update: Value);
}

pub trait Serverable {
    fn set_server(&self, server: Router<Sender<Value>>) -> Router<Sender<Value>> {
        server
    }
}

pub trait Printable {
    fn print(&self) -> String;

    fn json_struct(&self) -> Json<Value> { 
        Json(json!({

        }))
    }
}

pub trait UpdaterComponent: Updater + Serverable + Printable {}
impl<T: Updater + Serverable + Printable> UpdaterComponent for T {}

pub trait RouteableComponent: Routeable + Serverable + Printable {}
impl<T: Routeable + Serverable + Printable> RouteableComponent for T {}

