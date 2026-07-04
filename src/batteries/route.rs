use std::sync::Arc;

use async_trait::async_trait;

use crate::base::{Egress, Meta, Runnable, SendError};
use crate::batteries::dyn_egress::DynEgress;

type Predicate<I> = Box<dyn Fn(&I) -> bool + Send + Sync>;
type RouteEntry<I, O> = (Predicate<I>, Box<dyn DynEgress<I, O>>);

struct Inner<I, O> {
    routes: Vec<RouteEntry<I, O>>,
    fallback: Option<Box<dyn DynEgress<I, O>>>,
}

pub struct Route<I, O> {
    inner: Arc<Inner<I, O>>,
}

impl<I, O> Clone for Route<I, O> {
    fn clone(&self) -> Self {
        Self { inner: self.inner.clone() }
    }
}

impl<I, O> Default for Route<I, O>
where
    I: Send + Sync + 'static,
    O: Send + Sync + 'static,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<I, O> Route<I, O>
where
    I: Send + Sync + 'static,
    O: Send + Sync + 'static,
{
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Inner { routes: Vec::new(), fallback: None }),
        }
    }

    pub fn when<P, E>(mut self, predicate: P, egress: E) -> Self
    where
        P: Fn(&I) -> bool + Send + Sync + 'static,
        E: Egress<I, Output = O> + 'static,
    {
        Arc::get_mut(&mut self.inner)
            .expect("Route::when must be called while building the pipeline")
            .routes
            .push((Box::new(predicate), Box::new(egress)));
        self
    }

    pub fn otherwise<E>(mut self, egress: E) -> Self
    where
        E: Egress<I, Output = O> + 'static,
    {
        Arc::get_mut(&mut self.inner)
            .expect("Route::otherwise must be called while building the pipeline")
            .fallback = Some(Box::new(egress));
        self
    }
}

#[async_trait]
impl<I, O> Egress<I> for Route<I, O>
where
    I: Send + Sync + 'static,
    O: Send + Sync + 'static,
{
    type Output = O;

    fn services(&self) -> Vec<Box<dyn Runnable>> {
        let routed = self.inner.routes.iter().flat_map(|(_, egress)| egress.services());
        let fallback = self.inner.fallback.iter().flat_map(|egress| egress.services());
        routed.chain(fallback).collect()
    }

    async fn setup(&mut self) {
        let Some(inner) = Arc::get_mut(&mut self.inner) else {
            return;
        };
        for (_, egress) in inner.routes.iter_mut() {
            egress.setup().await;
        }
        if let Some(egress) = inner.fallback.as_mut() {
            egress.setup().await;
        }
    }

    async fn send(&self, input: I, meta: &Meta) -> Result<O, SendError> {
        for (predicate, egress) in self.inner.routes.iter() {
            if predicate(&input) {
                return egress.send(input, meta).await;
            }
        }
        match self.inner.fallback.as_ref() {
            Some(egress) => egress.send(input, meta).await,
            None => Err(SendError::permanent("route has no match")),
        }
    }

    async fn stop(&self) {
        for (_, egress) in self.inner.routes.iter() {
            egress.stop().await;
        }
        if let Some(egress) = self.inner.fallback.as_ref() {
            egress.stop().await;
        }
    }
}
