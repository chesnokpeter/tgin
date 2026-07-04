use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use async_trait::async_trait;
use tgin::base::{Egress, Meta, SendError};
use tgin::batteries::lb::{All, RoundRobin};
use tgin::batteries::retry::Retry;
use tgin::batteries::route::Route;

#[derive(Clone)]
struct MockEgress {
    label: &'static str,
    calls: Arc<AtomicUsize>,
    failures: Arc<AtomicUsize>,
    permanent: bool,
}

impl MockEgress {
    fn ok(label: &'static str) -> Self {
        Self {
            label,
            calls: Arc::new(AtomicUsize::new(0)),
            failures: Arc::new(AtomicUsize::new(0)),
            permanent: false,
        }
    }

    fn failing(label: &'static str, failures: usize) -> Self {
        Self {
            label,
            calls: Arc::new(AtomicUsize::new(0)),
            failures: Arc::new(AtomicUsize::new(failures)),
            permanent: false,
        }
    }

    fn broken(label: &'static str) -> Self {
        Self {
            label,
            calls: Arc::new(AtomicUsize::new(0)),
            failures: Arc::new(AtomicUsize::new(usize::MAX)),
            permanent: true,
        }
    }

    fn calls(&self) -> usize {
        self.calls.load(Ordering::SeqCst)
    }
}

#[async_trait]
impl Egress<String> for MockEgress {
    type Output = String;

    async fn send(&self, input: String, _meta: &Meta) -> Result<String, SendError> {
        self.calls.fetch_add(1, Ordering::SeqCst);

        let remaining = self.failures.load(Ordering::SeqCst);
        if remaining > 0 {
            if remaining != usize::MAX {
                self.failures.fetch_sub(1, Ordering::SeqCst);
            }
            return match self.permanent {
                true => Err(SendError::permanent("mock permanent failure")),
                false => Err(SendError::retryable("mock retryable failure")),
            };
        }

        Ok(format!("{}:{}", self.label, input))
    }
}

#[tokio::test]
async fn route_picks_the_matching_branch() {
    let api = MockEgress::ok("api");
    let fallback = MockEgress::ok("fallback");

    let route = Route::new()
        .when(|input: &String| input.starts_with("api"), api.clone())
        .otherwise(fallback.clone());

    let matched = route.send("api/users".to_string(), &Meta::new()).await.unwrap();
    let unmatched = route.send("hello".to_string(), &Meta::new()).await.unwrap();

    assert_eq!(matched, "api:api/users");
    assert_eq!(unmatched, "fallback:hello");
    assert_eq!(api.calls(), 1);
    assert_eq!(fallback.calls(), 1);
}

#[tokio::test]
async fn route_without_a_match_is_a_permanent_error() {
    let route: Route<String, String> =
        Route::new().when(|input: &String| input.starts_with("api"), MockEgress::ok("api"));

    let error = route.send("hello".to_string(), &Meta::new()).await.unwrap_err();

    assert!(!error.is_retryable());
}

#[tokio::test]
async fn round_robin_alternates_between_egresses() {
    let first = MockEgress::ok("first");
    let second = MockEgress::ok("second");

    let balancer = RoundRobin::new().to(first.clone()).to(second.clone());

    let a = balancer.send("x".to_string(), &Meta::new()).await.unwrap();
    let b = balancer.send("x".to_string(), &Meta::new()).await.unwrap();
    let c = balancer.send("x".to_string(), &Meta::new()).await.unwrap();

    assert_eq!(a, "first:x");
    assert_eq!(b, "second:x");
    assert_eq!(c, "first:x");
}

#[tokio::test]
async fn round_robin_fails_over_on_retryable_errors() {
    let dead = MockEgress::failing("dead", usize::MAX - 1);
    let alive = MockEgress::ok("alive");

    let balancer = RoundRobin::new().to(dead.clone()).to(alive.clone());

    let a = balancer.send("x".to_string(), &Meta::new()).await.unwrap();
    let b = balancer.send("x".to_string(), &Meta::new()).await.unwrap();

    assert_eq!(a, "alive:x");
    assert_eq!(b, "alive:x");
    assert!(dead.calls() >= 1);
}

#[tokio::test]
async fn round_robin_does_not_fail_over_on_permanent_errors() {
    let broken = MockEgress::broken("broken");
    let alive = MockEgress::ok("alive");

    let balancer = RoundRobin::new().to(broken.clone()).to(alive.clone());

    let error = balancer.send("x".to_string(), &Meta::new()).await.unwrap_err();

    assert!(!error.is_retryable());
    assert_eq!(alive.calls(), 0);
}

#[tokio::test]
async fn all_requires_every_egress_to_succeed() {
    let first = MockEgress::ok("first");
    let second = MockEgress::broken("second");

    let fanout = All::new().to(first.clone()).to(second.clone());

    let error = fanout.send("x".to_string(), &Meta::new()).await.unwrap_err();

    assert!(!error.is_retryable());
    assert_eq!(first.calls(), 1);
    assert_eq!(second.calls(), 1);
}

#[tokio::test]
async fn all_returns_the_first_output_when_everyone_succeeds() {
    let first = MockEgress::ok("first");
    let second = MockEgress::ok("second");

    let fanout = All::new().to(first.clone()).to(second.clone());

    let output = fanout.send("x".to_string(), &Meta::new()).await.unwrap();

    assert_eq!(output, "first:x");
    assert_eq!(second.calls(), 1);
}

#[tokio::test]
async fn retry_retries_retryable_errors_until_success() {
    let flaky = MockEgress::failing("flaky", 2);

    let retry = Retry::new(flaky.clone())
        .attempts(3)
        .backoff(Duration::from_millis(1));

    let output = retry.send("x".to_string(), &Meta::new()).await.unwrap();

    assert_eq!(output, "flaky:x");
    assert_eq!(flaky.calls(), 3);
}

#[tokio::test]
async fn retry_gives_up_after_the_last_attempt() {
    let dead = MockEgress::failing("dead", usize::MAX - 1);

    let retry = Retry::new(dead.clone())
        .attempts(2)
        .backoff(Duration::from_millis(1));

    let error = retry.send("x".to_string(), &Meta::new()).await.unwrap_err();

    assert!(error.is_retryable());
    assert_eq!(dead.calls(), 2);
}

#[tokio::test]
async fn retry_never_repeats_permanent_errors() {
    let broken = MockEgress::broken("broken");

    let retry = Retry::new(broken.clone())
        .attempts(5)
        .backoff(Duration::from_millis(1));

    let error = retry.send("x".to_string(), &Meta::new()).await.unwrap_err();

    assert!(!error.is_retryable());
    assert_eq!(broken.calls(), 1);
}
