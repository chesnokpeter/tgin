use async_trait::async_trait;

use tokio::sync::mpsc;
use tokio::sync::oneshot;


pub struct Envelope<I, O> {
    pub data: I,
    pub reply: Option<oneshot::Sender<O>>,
}


impl<I, O> Envelope<I, O> {
    pub fn forward(data: I) -> Self {
        Self { data, reply: None }
    }

    pub fn backward(data: I, reply: oneshot::Sender<O>) -> Self {
        Self { data, reply: Some(reply) }
    }
}



#[async_trait]
pub trait Forwardable<E>: Send + Sync + 'static {
    async fn process(self, egress: &E);
}

#[async_trait]
impl<E, I, O> Forwardable<E> for Envelope<I, O>
where
    E: Egress<I>,
    E::Output: Into<O> + Send + Sync, 
    I: Send + Sync + 'static,
    O: Send + Sync + 'static,
{
    async fn process(self, egress: &E) {
        let reverse = egress.send(self.data).await;
        if let Some(tx) = self.reply {
            let _ = tx.send(reverse.into());
        }
    }
}


#[async_trait]
pub trait Ingress<I, O>: Send + Sync 
where 
    I: Send + Sync + 'static,
    O: Send + Sync + 'static,
{
    async fn start(&self, tx: mpsc::Sender<Envelope<I, O>>);
}

#[async_trait]
pub trait Egress<I>: Send + Sync 
where 
    I: Send + Sync + 'static,
{
    type Output: Send + Sync + 'static; 
    async fn send(&self, input: I) -> Self::Output;
    async fn start(&self) {}
}


#[async_trait]
pub trait Runnable: Send + Sync + 'static {
    async fn run(&self);
}