use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use async_trait::async_trait;
use axum::body::Bytes;
use axum::http::{HeaderMap, Method, Uri};
use rdkafka::Offset;
use rdkafka::config::ClientConfig;
use rdkafka::consumer::{CommitMode, Consumer, StreamConsumer};
use rdkafka::message::Message;
use tokio::sync::mpsc::{self, Sender};
use tokio::sync::oneshot;
use tokio::task::JoinSet;
use tokio_util::sync::CancellationToken;

use crate::base::{Envelope, Ingress, SendError};
use crate::types::request::{RequestData, ResponseData};

const PARTITION_INFLIGHT: usize = 256;

type AckEntry = (u64, i64, oneshot::Receiver<Result<ResponseData, SendError>>);

struct PartitionAcker {
    tx: mpsc::Sender<AckEntry>,
    epoch: Arc<AtomicU64>,
}

pub struct KafkaIngress {
    brokers: String,
    group: String,
    topic: String,
    reconnect: Duration,
}

impl KafkaIngress {
    pub fn new(brokers: &str, group: &str, topic: &str) -> Self {
        Self {
            brokers: brokers.to_string(),
            group: group.to_string(),
            topic: topic.to_string(),
            reconnect: Duration::from_secs(5),
        }
    }

    pub fn reconnect(mut self, delay: Duration) -> Self {
        self.reconnect = delay;
        self
    }

    fn spawn_acker(
        &self,
        ackers: &mut JoinSet<()>,
        consumer: Arc<StreamConsumer>,
        partition: i32,
    ) -> PartitionAcker {
        let (ack_tx, mut ack_rx) = mpsc::channel::<AckEntry>(PARTITION_INFLIGHT);
        let epoch = Arc::new(AtomicU64::new(0));
        let topic = self.topic.clone();
        let acker_epoch = epoch.clone();

        ackers.spawn(async move {
            while let Some((sent_epoch, offset, reply)) = ack_rx.recv().await {
                if sent_epoch < acker_epoch.load(Ordering::Acquire) {
                    continue;
                }
                let delivered = matches!(reply.await, Ok(Ok(_)) | Ok(Err(SendError::Permanent(_))));
                if delivered {
                    let _ = consumer.store_offset(&topic, partition, offset + 1);
                } else {
                    acker_epoch.fetch_add(1, Ordering::Release);
                    let _ = consumer.seek(&topic, partition, Offset::Offset(offset), Duration::from_secs(5));
                }
            }
        });

        PartitionAcker { tx: ack_tx, epoch }
    }

    async fn consume(&self, tx: &Sender<Envelope<RequestData, ResponseData>>, shutdown: &CancellationToken) {
        let consumer: StreamConsumer = match ClientConfig::new()
            .set("bootstrap.servers", &self.brokers)
            .set("group.id", &self.group)
            .set("enable.auto.commit", "true")
            .set("auto.commit.interval.ms", "5000")
            .set("enable.auto.offset.store", "false")
            .set("auto.offset.reset", "earliest")
            .create()
        {
            Ok(consumer) => consumer,
            Err(_) => return,
        };

        if consumer.subscribe(&[&self.topic]).is_err() {
            return;
        }

        let consumer = Arc::new(consumer);
        let mut ackers = JoinSet::new();
        let mut partitions: HashMap<i32, PartitionAcker> = HashMap::new();

        loop {
            let message = tokio::select! {
                _ = shutdown.cancelled() => break,
                message = consumer.recv() => match message {
                    Ok(message) => message,
                    Err(_) => continue,
                },
            };

            let partition = message.partition();
            let offset = message.offset();

            let request = RequestData {
                body: Bytes::copy_from_slice(message.payload().unwrap_or_default()),
                uri: Uri::from_static("/"),
                method: Method::POST,
                headers: HeaderMap::new(),
                client_ip: None,
            };

            let key = match message.key() {
                Some(key) => Bytes::copy_from_slice(key),
                None => Bytes::from(partition.to_string()),
            };

            let (reply_tx, reply_rx) = oneshot::channel();

            if tx.send(Envelope::backward(request, reply_tx).key(key)).await.is_err() {
                break;
            }

            let acker = match partitions.get(&partition) {
                Some(acker) => acker,
                None => {
                    let acker = self.spawn_acker(&mut ackers, consumer.clone(), partition);
                    partitions.entry(partition).or_insert(acker)
                }
            };

            let sent_epoch = acker.epoch.load(Ordering::Acquire);
            if acker.tx.send((sent_epoch, offset, reply_rx)).await.is_err() {
                break;
            }
        }

        partitions.clear();
        while ackers.join_next().await.is_some() {}
        let _ = consumer.commit_consumer_state(CommitMode::Sync);
    }
}

#[async_trait]
impl Ingress<RequestData, ResponseData> for KafkaIngress {
    async fn start(&self, tx: Sender<Envelope<RequestData, ResponseData>>, shutdown: CancellationToken) {
        loop {
            if shutdown.is_cancelled() {
                return;
            }
            self.consume(&tx, &shutdown).await;
            tokio::select! {
                _ = shutdown.cancelled() => return,
                _ = tokio::time::sleep(self.reconnect) => {}
            }
        }
    }
}
