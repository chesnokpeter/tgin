use std::sync::Arc;

use async_trait::async_trait;

use crate::base::{Egress, Meta, Runnable, SendError};

#[async_trait]
pub(crate) trait DynEgress<I, O>: Send + Sync {
    fn services(&self) -> Vec<Box<dyn Runnable>>;
    async fn setup(&mut self);
    async fn send(&self, input: I, meta: &Meta) -> Result<O, SendError>;
    async fn stop(&self);
}

#[async_trait]
impl<E, I, O> DynEgress<I, O> for E
where
    E: Egress<I, Output = O> + 'static,
    I: Send + Sync + 'static,
    O: Send + Sync + 'static,
{
    fn services(&self) -> Vec<Box<dyn Runnable>> {
        Egress::services(self)
    }

    async fn setup(&mut self) {
        Egress::setup(self).await;
    }

    async fn send(&self, input: I, meta: &Meta) -> Result<O, SendError> {
        Egress::send(self, input, meta).await
    }

    async fn stop(&self) {
        Egress::stop(self).await;
    }
}

pub(crate) fn collect_services<I, O>(egresses: &[Box<dyn DynEgress<I, O>>]) -> Vec<Box<dyn Runnable>> {
    egresses.iter().flat_map(|egress| egress.services()).collect()
}

pub(crate) async fn setup_children<I, O>(egresses: &mut Arc<Vec<Box<dyn DynEgress<I, O>>>>) {
    if let Some(egresses) = Arc::get_mut(egresses) {
        for egress in egresses.iter_mut() {
            egress.setup().await;
        }
    }
}

pub(crate) async fn stop_children<I, O>(egresses: &Arc<Vec<Box<dyn DynEgress<I, O>>>>) {
    for egress in egresses.iter() {
        egress.stop().await;
    }
}
