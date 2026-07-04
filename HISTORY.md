# from Telegram Gateway Interface to Traffic Gateway Interface

tgin did not start universal.

The first tgin was a **Telegram Gateway Interface** — a dedicated routing layer for Telegram bot infrastructure: receive updates via longpoll or webhook, balance them across multiple bot instances. "NGINX for Telegram's Bot API ecosystem" was the whole pitch. That project lives on in the `master` branch history.

Building it made one thing obvious: none of the hard parts were about Telegram. Accepting traffic, buffering it, balancing it, retrying it, acknowledging it honestly, shutting down gracefully — the same problems appear for any protocol. Telegram just happened to be the first shape of traffic passing through.

So the core was rebuilt around the general shape — ingress → envelope → egress — and the name now reads **Traffic Gateway Interface**. HTTP, RabbitMQ, Kafka, WebSocket tunnels and Telegram bots are all batteries on the same frozen core.

## the original positioning still works

Telegram was not dropped — it was promoted to batteries. Everything the old tgin did, the new one does on top of the universal core, and does more honestly:

- `TelegramBotPollingIngress` / `TelegramBotWebhookIngress` — updates in, longpoll or webhook
- `TelegramBotWebhookEgress` / `TelegramBotPollingEgress` — updates out to bot instances: push them like Telegram does, or host a `getUpdates` endpoint that unmodified longpoll bots poll as if tgin were Telegram itself
- `TelegramBotApiEgress` — Bot API calls out, honest 429/5xx/4xx handling
- per-chat ordering for free, offsets advance only after real delivery, dead instances get failover

If you came here for the old use case, start with [examples/telegram_balancer.rs](examples/telegram_balancer.rs), [examples/telegram_webhook_balancer.rs](examples/telegram_webhook_balancer.rs) or [examples/telegram_longpoll_balancer.rs](examples/telegram_longpoll_balancer.rs).
