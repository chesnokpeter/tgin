use async_trait::async_trait;

use tokio::sync::mpsc::Sender;

use axum::Router;


pub trait Serverable {
    fn set_router(&self, router: Router) -> Router {
        router
    }
}


#[async_trait]
pub trait Ingress: Send + Sync + Serverable {
    
    async fn start(&self, tx: Sender<Data>);

}



#[async_trait]
pub trait Egress: Send + Sync + Serverable {

    async fn process(&self, data: Data);

}

#[derive(Debug, Clone)]
pub enum Data {
    Empty
}
