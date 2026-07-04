use std::time::Duration;

use async_trait::async_trait;

use crate::base::{Egress, Meta, Runnable, SendError};

#[derive(Clone)]
pub struct Retry<E> {
    inner: E,
    attempts: usize,
    backoff: Duration,
}

impl<E> Retry<E> {
    pub fn new(inner: E) -> Self {
        Self {
            inner,
            attempts: 3,
            backoff: Duration::from_millis(100),
        }
    }

    pub fn attempts(mut self, attempts: usize) -> Self {
        self.attempts = attempts.max(1);
        self
    }

    pub fn backoff(mut self, backoff: Duration) -> Self {
        self.backoff = backoff;
        self
    }
}

#[async_trait]
impl<E, I> Egress<I> for Retry<E>
where
    E: Egress<I>,
    I: Clone + Send + Sync + 'static,
{
    type Output = E::Output;

    fn services(&self) -> Vec<Box<dyn Runnable>> {
        self.inner.services()
    }

    async fn setup(&mut self) {
        self.inner.setup().await;
    }

    async fn send(&self, input: I, meta: &Meta) -> Result<Self::Output, SendError> {
        let mut delay = self.backoff;

        for _ in 1..self.attempts {
            match self.inner.send(input.clone(), meta).await {
                Err(error) if error.is_retryable() => {
                    tokio::time::sleep(delay).await;
                    delay *= 2;
                }
                result => return result,
            }
        }

        self.inner.send(input, meta).await
    }

    async fn stop(&self) {
        self.inner.stop().await;
    }
}
