use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::error::Error;
use std::fmt;

use async_trait::async_trait;
use bytes::Bytes;
use tokio::sync::mpsc;
use tokio::sync::oneshot;
use tokio::time::Instant;
use tokio_util::sync::CancellationToken;

pub type ErrorSource = Box<dyn Error + Send + Sync>;

#[derive(Debug)]
pub enum SendError {
    Retryable(ErrorSource),
    Permanent(ErrorSource),
    Overloaded,
    DeadlineExceeded,
}

impl SendError {
    pub fn retryable(source: impl Into<ErrorSource>) -> Self {
        Self::Retryable(source.into())
    }

    pub fn permanent(source: impl Into<ErrorSource>) -> Self {
        Self::Permanent(source.into())
    }

    pub fn is_retryable(&self) -> bool {
        matches!(self, Self::Retryable(_) | Self::Overloaded)
    }
}

impl fmt::Display for SendError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Retryable(source) => write!(f, "retryable: {source}"),
            Self::Permanent(source) => write!(f, "permanent: {source}"),
            Self::Overloaded => write!(f, "overloaded"),
            Self::DeadlineExceeded => write!(f, "deadline exceeded"),
        }
    }
}

impl Error for SendError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Retryable(source) | Self::Permanent(source) => Some(source.as_ref()),
            _ => None,
        }
    }
}

#[derive(Default)]
pub struct Extensions {
    values: HashMap<TypeId, Box<dyn Any + Send + Sync>>,
}

impl Extensions {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert<T: Send + Sync + 'static>(&mut self, value: T) {
        self.values.insert(TypeId::of::<T>(), Box::new(value));
    }

    pub fn get<T: Send + Sync + 'static>(&self) -> Option<&T> {
        self.values.get(&TypeId::of::<T>())?.downcast_ref()
    }

    pub fn remove<T: Send + Sync + 'static>(&mut self) -> Option<T> {
        let value = self.values.remove(&TypeId::of::<T>())?;
        value.downcast().ok().map(|value| *value)
    }
}

#[derive(Default)]
pub struct Meta {
    pub key: Option<Bytes>,
    pub deadline: Option<Instant>,
    pub extensions: Extensions,
}

impl Meta {
    pub fn new() -> Self {
        Self::default()
    }
}

pub struct Envelope<I, O> {
    pub data: I,
    pub meta: Meta,
    pub reply: Option<oneshot::Sender<Result<O, SendError>>>,
}

impl<I, O> Envelope<I, O> {
    pub fn forward(data: I) -> Self {
        Self { data, meta: Meta::new(), reply: None }
    }

    pub fn backward(data: I, reply: oneshot::Sender<Result<O, SendError>>) -> Self {
        Self { data, meta: Meta::new(), reply: Some(reply) }
    }

    pub fn key(mut self, key: impl Into<Bytes>) -> Self {
        self.meta.key = Some(key.into());
        self
    }

    pub fn deadline(mut self, deadline: Instant) -> Self {
        self.meta.deadline = Some(deadline);
        self
    }
}

#[async_trait]
pub trait Runnable: Send + Sync {
    fn id(&self) -> Option<usize> {
        None
    }

    async fn run(&self, shutdown: CancellationToken);
}

#[async_trait]
pub trait Ingress<I, O>: Send + Sync
where
    I: Send + Sync + 'static,
    O: Send + Sync + 'static,
{
    fn services(&self) -> Vec<Box<dyn Runnable>> {
        Vec::new()
    }

    async fn setup(&mut self, tx: mpsc::Sender<Envelope<I, O>>) {
        let _ = tx;
    }

    async fn start(&self, tx: mpsc::Sender<Envelope<I, O>>, shutdown: CancellationToken) {
        let _ = (tx, shutdown);
    }
}

#[async_trait]
pub trait Egress<I>: Send + Sync
where
    I: Send + Sync + 'static,
{
    type Output: Send + Sync + 'static;

    fn services(&self) -> Vec<Box<dyn Runnable>> {
        Vec::new()
    }

    async fn setup(&mut self) {}

    async fn send(&self, input: I, meta: &Meta) -> Result<Self::Output, SendError>;

    async fn stop(&self) {}
}
