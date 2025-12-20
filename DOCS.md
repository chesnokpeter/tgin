# TGIN Mini Documentation

## Overview
TGIN is a dedicated routing layer that sits between Telegram Bot API and one or more bot instances. It receives updates via long polling or inbound webhooks, applies a routing strategy (direct routes or load balancers), and forwards each update to downstream bots. TGIN is configured through a single RON file (`tgin.ron`) and can optionally expose a management API and TLS.

## Capabilities
- **Hybrid ingress:** consume Telegram updates via long polling (`LongPollUpdate`) or a http api endpoint (`WebhookUpdate`).
- **Flexible routing:** forward updates to downstream long-poll consumers (`LongPollRoute`) or HTTP request webhook (`WebhookRoute`), or use hierarchical load balancers (`RoundRobinLB`, `AllLB`).
- **Hot reconfiguration API:** optional HTTP API allows you to change the current configuration at runtime.
- **Built-in TLS:** serve update ingestion over HTTPS.
- **CLI & Docker ready:** ships with a cli-app and Docker assets for containerized deployments.

### Key terms in practice
- **LongPollUpdate** – inbound adapter that keeps asking Telegram for fresh updates via `getUpdates`.  
- **WebhookUpdate** – inbound adapter that accepts Telegram requests over HTTP (and can self-register).  
- **LongPollRoute** – outbound endpoint that downstream bots can poll (effectively recreating Telegram semantics inside your cluster).  
- **WebhookRoute** – outbound push that POSTs updates to another HTTP service.  
- **RoundRobinLB** – load balancer that rotates through child routes, sending each update to exactly one target.  
- **AllLB** – load balancer that fans out every update to every child route.


## Running the binary
```bash
git clone https://github.com/chesnokpeter/tgin.git
cd tgin

cargo build --release
./target/release/tgin -f tgin.ron
```
The `-f/--file` flag selects the configuration file (defaults to `tgin.ron`). Environment variables referenced as `${VAR}` inside the config are substituted before parsing.

## Configuration Reference
Top-level structure loaded from `tgin.ron` (`src/config/schema.rs`):

| Field | Type | Example | Description |
| ----- | ---- | ------- | ----------- |
| `dark_threads` | `usize` (default 4) | `dark_threads: 6` | Worker threads allocated to the Tokio runtime. Increase for higher concurrency. |
| `server_port` | `Option<u16>` | `server_port: Some(3000)` | When set, TGIN hosts all ingress routes (long poll, webhook, API) under `0.0.0.0:<port>`. When `None`, only outbound behavior (e.g., `LongPollUpdate` and `WebhookRoute`) runs. |
| `ssl` | `Option<SslConfig{ cert: String, key: String }>` | `ssl: Some(SslConfig(cert: "/cert.pem", key: "/privkey.pem" ))` | Optional TLS certificate and private key (PEM files) for HTTPS. |
| `updates` | `Vec<UpdaterComponent>` | see below | Ingress providers that pull updates from Telegram. |
| `route` | `RouteableComponent` | see below | Outgoing route (single route or nested load balancer tree) that receives each update pulled from Telegram. |
| `api` | `Option<ApiConfig{ base_path: String }>` |  `api : Some(ApiConfig(base_path: "/api"))` | Optional management API base path (e.g., `"/api"`). |

### Update providers
`updates` control how TGIN receives Telegram traffic. Several providers can coexist, in which case tgin will receive updates from all of them.

- **`LongPollUpdate`**  
  Fields:  
  - `token` (required): Telegram bot token (`123456:ABC`).  
  - `url` (optional): Override for the Telegram API endpoint (defaults to `https://api.telegram.org`).  
  - `default_timeout_sleep` (default `100` ms): Cool-down between successful polling iterations.  
  - `error_timeout_sleep` (default `100` ms): Backoff duration after JSON or network errors.  
  Behavior: periodically calls `getUpdates` with an ever-increasing offset and forwards every update into the routing layer.

- **`WebhookUpdate`**  
  Fields:  
  - `path` (required): Local path that Telegram should post updates to (e.g., `/bot/pull`).  
  - `registration` (optional): `{ public_ip: String, token: String, set_webhook_url: Option<String> }` used for automatic webhook registration against Telegram. (The plumbing hook is present in `WebhookUpdate` but not yet wired inside `build_updates`, so manual registration or extending the builder is currently required.)  
  Behavior: exposes an HTTP endpoint on the configured `server_port` and pushes incoming JSON bodies into the routing pipeline.

### Routing targets
`route` declares where ingested updates get forwarded. Routes can be nested inside load balancers to build complex trees.

- **`LongPollRoute { path }`**  
  Exposes a `/bot`-style endpoint that downstream bots can poll. Updates are buffered in memory until a client calls the route using an HTTP-request (`application/x-www-form-urlencoded`) with Telegram-compatible `offset`/`timeout` parameters. `offset` filtering follows Telegram semantics so multiple bots can safely read from the buffer.

- **`WebhookRoute { url }`**  
  Push-based forwarder: every update triggers an HTTP POST with the original JSON payload to the target `url` (e.g., `http://internal-bot:8080/bot`). HTTP errors are ignored after logging, so ensure downstream services are resilient.

### Load balancers
Load balancers compose multiple routes.

- **`RoundRobinLB { routes }`** (`src/lb/roundrobin.rs`)  
  Keeps an atomic cursor and forwards each update to the next route in sequence. Useful for horizontal scaling across stateless webhook handlers or long-poll queues. Routes can be heterogeneous (e.g., a webhook and a long-poll route mixed together). 

- **`AllLB { routes }`** (`src/lb/all.rs`)  
  Broadcast strategy: clones every update and dispatches it to all child routes concurrently. Ideal when multiple specialized services must see the full update stream (analytics, moderation, etc.). Beware of downstream backpressure because each update is processed `N` times.

## HTTP Management API
Enable the API by adding an `api` block to your config:

```ron
api: Some((
    base_path: "/api",
)),
```

Routes are nested under `base_path` and share the same listener as your ingress endpoints.

| Endpoint | Method | Body | Description |
| -------- | ------ | ---- | ----------- |
| `/api/routes` | GET | — | Returns the current routing tree as JSON (source: `Routeable::json_struct`). |
| `/api/route` | POST | `{ "type": "...", "path/url": "...", "sublevel": 0 }` | Adds a new route dynamically. `type` accepts `Webhook` or `Longpull`. `sublevel` is reserved for future hierarchical insertion (currently a placeholder). |

Example request:
```bash
curl -X POST http://localhost:3000/api/route \
  -H 'Content-Type: application/json' \
  -d '{ "type": "Webhook", "url": "http://bot-b:9000/bot" }'
```

The API communicates with the routing core via an in-memory channel (see `src/api/router.rs` and `src/api/methods.rs`).

## SSL/TLS Setup
TGIN can use TLS itself with using Rustls (`axum_server::tls_rustls`).

1. **Obtain certificates**  
   Use your CA-issued files or create temporary self-signed assets:  
   ```bash
   openssl req -x509 -nodes -days 365 -newkey rsa:2048 \
     -keyout tls/key.pem -out tls/cert.pem \
     -subj "/CN=your.domain"
   ```
2. **Update the config**  
   ```ron
   ssl: Some(SslConfig(
       cert: "tls/cert.pem",
       key: "tls/key.pem",
   )),
   server_port: Some(3000),
   ```
3. **Run TGIN**  
   The Axum server listens on `0.0.0.0:<server_port>` and serves HTTPS using the supplied certificate. Long-poll routes, webhook ingress, and the management API automatically use TLS.

## Example
**You can set any environment variables in the config using the syntax `${VAR}`**
```ron
(
    dark_threads: 6,
    server_port: Some(3000),

//    api : Some(ApiConfig(base_path: "/api")),

//    ssl: Some(SslConfig(cert: "/cert.pem", key: "/privkey.pem" )),

    updates: [
        LongPollUpdate(
            token: "${TOKEN}",
        ),
//        WebhookUpdate(
//            path: "/bot/pull"
//        )

    ],

    route: RoundRobinLB(
        routes: [
            LongPollRoute(path: "/bot2/getUpdates"),
            WebhookRoute(url: "http://127.0.0.1:8080/bot2"),
        ]
    )

)
```

`// - this is a comment`


## Additional resources
- `README.md` – high-level motivation and quick start instructions.
- `examples/simple` – docker-compose scenario demonstrating multiple downstream bots and a sample `tgin.ron`.
