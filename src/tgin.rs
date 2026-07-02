use std::collections::HashSet;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::{mpsc, Semaphore};
use tokio::task::{JoinHandle, JoinSet};
use tokio::time::timeout_at;
use tokio_util::sync::CancellationToken;

use crate::base::{Egress, Envelope, Ingress, Runnable, SendError};

const CHANNEL_CAPACITY: usize = 1024;
const LANE_COUNT: usize = 16;
const LANE_CAPACITY: usize = 256;
const CONCURRENCY: usize = 1024;

#[async_trait]
trait PipelineRunner: Send {
    async fn setup(&mut self);
    fn spawn(self: Box<Self>, shutdown: CancellationToken) -> Vec<JoinHandle<()>>;
}

struct Pipeline<Ing, Eg, I, O> {
    ingress: Ing,
    egress: Eg,
    tx: mpsc::Sender<Envelope<I, O>>,
    rx: mpsc::Receiver<Envelope<I, O>>,
}

#[async_trait]
impl<Ing, Eg, I, O> PipelineRunner for Pipeline<Ing, Eg, I, O>
where
    Ing: Ingress<I, O> + 'static,
    Eg: Egress<I> + Clone + 'static,
    Eg::Output: Into<O>,
    I: Send + Sync + 'static,
    O: Send + Sync + 'static,
{
    async fn setup(&mut self) {
        self.egress.setup().await;
        self.ingress.setup(self.tx.clone()).await;
    }

    fn spawn(self: Box<Self>, shutdown: CancellationToken) -> Vec<JoinHandle<()>> {
        let Pipeline { ingress, egress, tx, rx } = *self;

        let consume = tokio::spawn(async move {
            ingress.start(tx, shutdown).await;
        });

        let forward = tokio::spawn(dispatch(rx, egress));

        vec![consume, forward]
    }
}

async fn process<Eg, I, O>(egress: &Eg, envelope: Envelope<I, O>)
where
    Eg: Egress<I>,
    Eg::Output: Into<O>,
    I: Send + Sync + 'static,
    O: Send + Sync + 'static,
{
    let Envelope { data, meta, reply } = envelope;

    let result = match meta.deadline {
        Some(deadline) => match timeout_at(deadline, egress.send(data, &meta)).await {
            Ok(result) => result.map(Into::into),
            Err(_) => Err(SendError::DeadlineExceeded),
        },
        None => egress.send(data, &meta).await.map(Into::into),
    };

    if let Some(reply) = reply {
        let _ = reply.send(result);
    }
}

fn lane_index(key: &[u8]) -> usize {
    let mut hasher = DefaultHasher::new();
    key.hash(&mut hasher);
    hasher.finish() as usize % LANE_COUNT
}

async fn dispatch<Eg, I, O>(mut rx: mpsc::Receiver<Envelope<I, O>>, egress: Eg)
where
    Eg: Egress<I> + Clone + 'static,
    Eg::Output: Into<O>,
    I: Send + Sync + 'static,
    O: Send + Sync + 'static,
{
    let semaphore = Arc::new(Semaphore::new(CONCURRENCY));
    let mut pool = JoinSet::new();
    let mut lanes: Vec<Option<mpsc::Sender<Envelope<I, O>>>> = (0..LANE_COUNT).map(|_| None).collect();
    let mut lane_workers = Vec::new();

    while let Some(envelope) = rx.recv().await {
        while pool.try_join_next().is_some() {}

        match envelope.meta.key.clone() {
            Some(key) => {
                let lane = lanes[lane_index(&key)].get_or_insert_with(|| {
                    let (lane_tx, mut lane_rx) = mpsc::channel::<Envelope<I, O>>(LANE_CAPACITY);
                    let egress = egress.clone();
                    lane_workers.push(tokio::spawn(async move {
                        while let Some(envelope) = lane_rx.recv().await {
                            process(&egress, envelope).await;
                        }
                    }));
                    lane_tx
                });

                if let Err(mpsc::error::SendError(envelope)) = lane.send(envelope).await {
                    if let Some(reply) = envelope.reply {
                        let _ = reply.send(Err(SendError::Overloaded));
                    }
                }
            }
            None => {
                let permit = semaphore.clone().acquire_owned().await.expect("dispatcher semaphore closed");
                let egress = egress.clone();
                pool.spawn(async move {
                    process(&egress, envelope).await;
                    drop(permit);
                });
            }
        }
    }

    drop(lanes);
    for worker in lane_workers {
        let _ = worker.await;
    }
    while pool.join_next().await.is_some() {}

    egress.stop().await;
}

pub struct Tgin {
    services: Vec<Box<dyn Runnable>>,
    service_ids: HashSet<usize>,
    pipelines: Vec<Box<dyn PipelineRunner>>,
}

impl Tgin {
    pub fn new() -> Self {
        Self {
            services: Vec::new(),
            service_ids: HashSet::new(),
            pipelines: Vec::new(),
        }
    }

    fn push_service(&mut self, service: Box<dyn Runnable>) {
        if let Some(id) = service.id() {
            if !self.service_ids.insert(id) {
                return;
            }
        }
        self.services.push(service);
    }

    pub fn serve<S: Runnable + 'static>(mut self, service: S) -> Self {
        self.push_service(Box::new(service));
        self
    }

    pub fn pipeline<Ing, Eg, I, O>(mut self, ingress: Ing, egress: Eg) -> Self
    where
        Ing: Ingress<I, O> + 'static,
        Eg: Egress<I> + Clone + 'static,
        Eg::Output: Into<O>,
        I: Send + Sync + 'static,
        O: Send + Sync + 'static,
    {
        for service in ingress.services().into_iter().chain(egress.services()) {
            self.push_service(service);
        }

        let (tx, rx) = mpsc::channel::<Envelope<I, O>>(CHANNEL_CAPACITY);
        self.pipelines.push(Box::new(Pipeline { ingress, egress, tx, rx }));
        self
    }

    pub async fn run(mut self) {
        let shutdown = CancellationToken::new();

        let signal = shutdown.clone();
        tokio::spawn(async move {
            if tokio::signal::ctrl_c().await.is_ok() {
                signal.cancel();
            }
        });

        for pipeline in &mut self.pipelines {
            pipeline.setup().await;
        }

        let mut tasks = Vec::new();

        for pipeline in self.pipelines {
            tasks.extend(pipeline.spawn(shutdown.clone()));
        }

        for service in self.services {
            let shutdown = shutdown.clone();
            tasks.push(tokio::spawn(async move {
                service.run(shutdown).await;
            }));
        }

        futures::future::join_all(tasks).await;
    }
}
