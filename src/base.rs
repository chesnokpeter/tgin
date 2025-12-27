
use async_trait::async_trait;
use serde_json::{Value, json};

use std::sync::Arc;

use tokio::sync::mpsc::Sender;

use axum::Router;

use crate::update::base::Updater;

use crate::api::message::AddRouteType;


#[async_trait]
pub trait Routeable: Send + Sync {
    async fn process(&self, update: Value);

    async fn add_route(&self, route: AddRouteType) -> Result<(), ()>{
        Err(())
    }
}
#[async_trait]
pub trait Serverable {
    async fn set_server(&self, server: Router<Sender<Value>>) -> Router<Sender<Value>> {
        server
    }
}
#[async_trait]
pub trait Printable {
    async fn print(&self) -> String;

    async fn json_struct(&self) -> Value { 
        json!({

        })
    }
}

pub trait UpdaterComponent: Updater + Serverable + Printable + Send + Sync {}
impl<T: Updater + Serverable + Printable> UpdaterComponent for T {}

pub trait RouteableComponent: Routeable + Serverable + Printable + Send + Sync{}
impl<T: Routeable + Serverable + Printable> RouteableComponent for T {}

