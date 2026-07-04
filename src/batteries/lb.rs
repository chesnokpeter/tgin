use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use async_trait::async_trait;

use crate::base::{Egress, Meta, Runnable, SendError};
use crate::batteries::dyn_egress::{DynEgress, collect_services, setup_children, stop_children};

pub struct RoundRobin<I, O> {
    egresses: Arc<Vec<Box<dyn DynEgress<I, O>>>>,
    cursor: Arc<AtomicUsize>,
}

impl<I, O> Clone for RoundRobin<I, O> {
    fn clone(&self) -> Self {
        Self {
            egresses: self.egresses.clone(),
            cursor: self.cursor.clone(),
        }
    }
}

impl<I, O> Default for RoundRobin<I, O>
where
    I: Send + Sync + 'static,
    O: Send + Sync + 'static,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<I, O> RoundRobin<I, O>
where
    I: Send + Sync + 'static,
    O: Send + Sync + 'static,
{
    pub fn new() -> Self {
        Self {
            egresses: Arc::new(Vec::new()),
            cursor: Arc::new(AtomicUsize::new(0)),
        }
    }

    pub fn to<E>(mut self, egress: E) -> Self
    where
        E: Egress<I, Output = O> + 'static,
    {
        Arc::get_mut(&mut self.egresses)
            .expect("RoundRobin::to must be called while building the pipeline")
            .push(Box::new(egress));
        self
    }
}

#[async_trait]
impl<I, O> Egress<I> for RoundRobin<I, O>
where
    I: Clone + Send + Sync + 'static,
    O: Send + Sync + 'static,
{
    type Output = O;

    fn services(&self) -> Vec<Box<dyn Runnable>> {
        collect_services(&self.egresses)
    }

    async fn setup(&mut self) {
        setup_children(&mut self.egresses).await;
    }

    async fn send(&self, input: I, meta: &Meta) -> Result<O, SendError> {
        let count = self.egresses.len();
        if count == 0 {
            return Err(SendError::permanent("round robin has no egresses"));
        }

        let start = self.cursor.fetch_add(1, Ordering::Relaxed);
        let mut last = None;

        for attempt in 0..count {
            let egress = &self.egresses[(start + attempt) % count];
            match egress.send(input.clone(), meta).await {
                Ok(output) => return Ok(output),
                Err(error) if error.is_retryable() => last = Some(error),
                Err(error) => return Err(error),
            }
        }

        Err(last.expect("round robin exhausted egresses without an error"))
    }

    async fn stop(&self) {
        stop_children(&self.egresses).await;
    }
}

pub struct All<I, O> {
    egresses: Arc<Vec<Box<dyn DynEgress<I, O>>>>,
}

impl<I, O> Clone for All<I, O> {
    fn clone(&self) -> Self {
        Self {
            egresses: self.egresses.clone(),
        }
    }
}

impl<I, O> Default for All<I, O>
where
    I: Send + Sync + 'static,
    O: Send + Sync + 'static,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<I, O> All<I, O>
where
    I: Send + Sync + 'static,
    O: Send + Sync + 'static,
{
    pub fn new() -> Self {
        Self {
            egresses: Arc::new(Vec::new()),
        }
    }

    pub fn to<E>(mut self, egress: E) -> Self
    where
        E: Egress<I, Output = O> + 'static,
    {
        Arc::get_mut(&mut self.egresses)
            .expect("All::to must be called while building the pipeline")
            .push(Box::new(egress));
        self
    }
}

#[async_trait]
impl<I, O> Egress<I> for All<I, O>
where
    I: Clone + Send + Sync + 'static,
    O: Send + Sync + 'static,
{
    type Output = O;

    fn services(&self) -> Vec<Box<dyn Runnable>> {
        collect_services(&self.egresses)
    }

    async fn setup(&mut self) {
        setup_children(&mut self.egresses).await;
    }

    async fn send(&self, input: I, meta: &Meta) -> Result<O, SendError> {
        if self.egresses.is_empty() {
            return Err(SendError::permanent("all has no egresses"));
        }

        let sends = self.egresses.iter().map(|egress| egress.send(input.clone(), meta));
        let outputs = futures::future::join_all(sends).await;

        let mut first = None;
        for output in outputs {
            match output {
                Ok(output) => {
                    if first.is_none() {
                        first = Some(output);
                    }
                }
                Err(error) => return Err(error),
            }
        }

        Ok(first.expect("all egresses succeeded without an output"))
    }

    async fn stop(&self) {
        stop_children(&self.egresses).await;
    }
}
