# kafka-bridge demo — HTTP → Kafka → worker

one tgin binary runs two pipelines: `POST /ingest` → Kafka topic `events` (acks=all), and a consumer group → worker with retries. Offsets are committed only after the worker confirmed a contiguous prefix — a failure seeks back and redelivers instead of losing the message.

```
docker compose up --build
```

kafka (kraft, single node) takes ~20s to become healthy on first start.

## walkthrough

ingest an event — 202 comes back only after the broker acknowledged the produce:

```
curl -i -X POST localhost:8080/ingest -d '{"event":"signup","user":42}'   # HTTP/1.1 202 Accepted
```

watch it come out the other side:

```
docker compose logs processor    # processor got: {"event":"signup","user":42}
```

## failure drill

kill the worker — events keep landing in kafka, offsets stop advancing:

```
docker compose stop processor
curl -i -X POST localhost:8080/ingest -d '{"event":"while-worker-down"}'  # still 202
```

bring it back — the consumer group resumes from the last committed offset, nothing lost:

```
docker compose start processor
docker compose logs -f processor   # processor got: {"event":"while-worker-down"}
```

restart tgin itself — the consumer group picks up exactly where the committed offsets left off:

```
docker compose restart tgin
curl -X POST localhost:8080/ingest -d '{"event":"after-restart"}'
docker compose logs -f processor
```
