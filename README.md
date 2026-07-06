```
 __                          
/\ \__         __            
\ \ ,_\    __ /\_\    ___    
 \ \ \/  /'_ `\/\ \ /' _ `\  
  \ \ \_/\ \L\ \ \ \/\ \/\ \ 
   \ \__\ \____ \ \_\ \_\ \_\
    \/__/\/___L\ \/_/\/_/\/_/
           /\____/           
           \_/__/
```

#### any traffic in - any traffic out. A Rust construction kit for building gateways of your own shape: one core, endless batteries

[![ci](https://github.com/chesnokpeter/tgin/actions/workflows/ci.yml/badge.svg)](https://github.com/chesnokpeter/tgin/actions/workflows/ci.yml)
[![crates.io](https://img.shields.io/crates/v/tgin.svg)](https://crates.io/crates/tgin)
[![docs.rs](https://docs.rs/tgin/badge.svg)](https://docs.rs/tgin)

[HISTORY](HISTORY.md) | [DEMOS](demos/README.md) | [EXAMPLES](examples/) | [DOCS](https://docs.rs/tgin)

> [!IMPORTANT]
> active development is continuing: there is no stable version yet

### Why Tgin?
- Composability: every gateway is `ingress → egress`; balancers, routers and retries are decorators that wrap egresses — new behavior without touching the core

- Type safety: an incompatible ingress/egress pair is a failure to compile

- Batteries included: HTTP server & client, Telegram Bot API (longpoll and webhook), RabbitMQ, Kafka, WebSocket tunnels, round-robin / fan-out balancers, predicate router, retry with backoff

- Honest delivery: the reply of every message doubles as its ack — RabbitMQ ack/nack/requeue and Telegram offset advancement are driven by the real downstream outcome

- Backpressure end to end: full channels turn into HTTP 429 and broker qos instead of unbounded memory

- Per-key ordering: messages with the same key (one Telegram chat, one entity) stay ordered, different keys run in parallel

- Graceful shutdown: SIGINT drains in-flight messages, then stops

- Embeddable: tgin is a library, not a daemon — your gateway is a ~20 line Rust binary you fully control

### Architecture Overview
```
   ingress                     egress
HTTP | Telegram | RabbitMQ | Kafka | Tunnel
     ↓
   Envelope<I, O>  (data + meta + reply)
     ↓
 [ Route | RoundRobin | All | Retry ]
     ↓
HTTP | Telegram | RabbitMQ | Kafka | Tunnel
```

### Quick start
```
# Clone the repository
git clone https://github.com/chesnokpeter/tgin.git
cd tgin

# Run a case
cargo run --example reverse_proxy
BOT_TOKEN=... cargo run --example telegram_balancer
TUNNEL_TOKEN=... cargo run --example tunnel_server

# Or bring up a full live stack (broker + backends + tgin)
cd demos/kafka-bridge && docker compose up --build
```

### Use as a library
tgin is a library crate — your gateway is your own ~20 line binary:
```
cargo add tgin
```
or in Cargo.toml:
```toml
[dependencies]
tgin = "0.1"
```

### Configuration
Your gateway is a Rust builder — every piece is a value you construct and compose:
```rust
let server = HttpServer::new("0.0.0.0:8080");
let http = HttpClient::new();

Tgin::new()
    .pipeline(
        HttpIngress::catch_all(&server),
        Route::new()
            .when(
                |request: &RequestData| request.uri.path().starts_with("/api"),
                RoundRobin::new()
                    .to(HttpEgress::new(&http, "http://127.0.0.1:3001"))
                    .to(HttpEgress::new(&http, "http://127.0.0.1:3002")),
            )
            .otherwise(HttpEgress::new(&http, "http://127.0.0.1:3000")),
    )
    .run()
    .await;
```

### Cases
one core, different hands:

**nginx case** — reverse proxy with routing, balancing and failover → [examples/reverse_proxy.rs](examples/reverse_proxy.rs)

**ngrok case** — expose localhost to the internet through a WebSocket tunnel (in NAT) → [examples/tunnel_server.rs](examples/tunnel_server.rs) + [examples/tunnel_client.rs](examples/tunnel_client.rs)


**telegram case** — the original tgin: balance updates across bot instances, webhook or longpoll on both sides, per-chat ordering for free. Bots can even poll tgin itself — it recreates Telegram `getUpdates` semantics inside your cluster, offsets included → [examples/telegram_balancer.rs](examples/telegram_balancer.rs), [examples/telegram_webhook_balancer.rs](examples/telegram_webhook_balancer.rs), [examples/telegram_longpoll_balancer.rs](examples/telegram_longpoll_balancer.rs)

**bridge case** — HTTP in, broker out; queue in, Telegram message out. One binary can run several pipelines at once → [examples/http_to_rabbit.rs](examples/http_to_rabbit.rs), [examples/rabbit_to_telegram.rs](examples/rabbit_to_telegram.rs), [examples/ingestion_pipeline.rs](examples/ingestion_pipeline.rs), [examples/kafka_bridge.rs](examples/kafka_bridge.rs)


**gateway case** — hide your bot token behind your own API → [examples/telegram_api_gateway.rs](examples/telegram_api_gateway.rs)

### Demos
live compose stacks under [demos/](demos/) — real brokers, real backends, failure drills to run yourself:

- [demos/reverse-proxy](demos/reverse-proxy/) — routing + balancing, kill a backend and watch failover
- [demos/tunnel](demos/tunnel/) — reach a container that publishes no ports, kill the agent and watch it reconnect
- [demos/ingestion](demos/ingestion/) — HTTP → RabbitMQ at-least-once, kill the worker and nothing is lost
- [demos/kafka-bridge](demos/kafka-bridge/) — HTTP → Kafka → worker with honest offset commits

each is one `docker compose up --build` plus a README walkthrough

### Batteries
| kind | battery | what it does |
| ---- | ------- | ------------ |
| ingress | `HttpIngress` | serve a path, a method or catch-all on a shared `HttpServer` |
| ingress | `KafkaIngress` | consume a topic in a consumer group; offsets commit only after downstream confirms, failures seek back and redeliver |
| ingress | `TelegramBotPollingIngress` | `getUpdates` longpoll from Telegram, offset advances only after delivery, per-chat key |
| ingress | `TelegramBotWebhookIngress` | webhook endpoint with `x-telegram-bot-api-secret-token` validation |
| ingress | `RabbitmqIngress` | consume a durable queue, qos prefetch, ack/nack/requeue by outcome |
| ingress | `TunnelIngress` | agent side of the tunnel: outbound connect + reconnect |
| egress | `HttpEgress` | forward to an upstream with proper proxy headers |
| egress | `KafkaEgress` | produce with acks=all; `meta.key` becomes the partition key, so per-chat / per-entity order survives the hop |
| egress | `TelegramBotWebhookEgress` | push updates to a bot instance the way Telegram does, secret header included |
| egress | `TelegramBotPollingEgress` | host a `getUpdates` endpoint — unmodified longpoll bots poll tgin as if it were Telegram, ack by offset |
| egress | `TelegramBotApiEgress` | call the Bot API: fixed method or pass-through, 429/5xx/4xx mapped honestly |
| egress | `RabbitmqEgress` | publish persistent messages with publisher confirms |
| egress | `TunnelEgress` | server side of the tunnel: multiplex requests over one WebSocket |
| decorator | `Route` | pick an egress by predicate, `otherwise` fallback |
| decorator | `RoundRobin` | rotate across egresses, failover on retryable errors |
| decorator | `All` | fan out every message to every egress |
| decorator | `Retry` | retry retryable errors with exponential backoff |
| shared | `HttpServer` / `HttpClient` / `Rabbit` | resources you construct once and share across batteries |

### Future features
- Config file on top of compiled batteries — declare pipelines in a config instead of Rust, validated at startup, invalid pairs rejected before serving (Envoy/Vector style)
- Hot config reload — swap pipeline wiring on the fly, without restarting the process
- Transform / processing decorators — map, filter and enrich messages between ingress and egress
- Metrics, tracing, logging
- TLS termination
- Streaming bodies & WebSocket passthrough
- More batteries: NATS, Redis, gRPC
- Performance harness

### Main Goal
**Provide a universal, embeddable toolkit where any traffic can be taken in, routed, distributed and transformed on the way out — one frozen core, endless batteries**
