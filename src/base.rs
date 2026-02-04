use async_trait::async_trait;

use tokio::sync::mpsc::Sender;



#[async_trait]
pub trait Ingress<I>: Send + Sync 
    where I: Send + Sync + 'static
{
    async fn start(&self, tx: Sender<I>);

}

#[async_trait]
pub trait Egress<O>: Send + Sync
    where O: Send + Sync + 'static 
{
    async fn process(&self, data: O);

}

