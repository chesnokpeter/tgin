# ingestion demo — guaranteed HTTP → RabbitMQ → worker

one tgin binary runs two pipelines: `POST /ingest` → durable queue (persistent messages, publisher confirms), and queue → worker with retries. The reply of every message doubles as its ack: the worker's outcome drives ack / requeue on the broker.

```
docker compose up --build
```

## walkthrough

ingest an event — 202 comes back only after the broker confirmed it:

```
curl -i -X POST localhost:8080/ingest -d '{"event":"signup","user":42}'   # HTTP/1.1 202 Accepted
```

watch it reach the worker:

```
docker compose logs processor    # processor got: {"event":"signup","user":42}
```

rabbitmq management UI: http://localhost:15672 (guest/guest), queue `events`.

## failure drill

kill the worker — ingestion keeps accepting, events pile up in the durable queue:

```
docker compose stop processor
curl -i -X POST localhost:8080/ingest -d '{"event":"while-worker-down"}'  # still 202
```

bring the worker back — everything queued is delivered, nothing lost:

```
docker compose start processor
docker compose logs -f processor   # processor got: {"event":"while-worker-down"}
```

kill the broker — tgin answers an honest 502 (nothing silently dropped), and self-heals when the broker returns:

```
docker compose stop rabbitmq
curl -i -X POST localhost:8080/ingest -d '{"event":"x"}'   # HTTP/1.1 502 Bad Gateway
docker compose start rabbitmq
curl -i -X POST localhost:8080/ingest -d '{"event":"y"}'   # 202 again (no tgin restart)
```
