# tgin demos

live compose stacks: real brokers, real backends, failure drills. The tgin "config" in every demo is a plain Rust example from [../examples](../examples) — the compose file only wires services together with env vars.

| demo | case | stack |
| ---- | ---- | ----- |
| [reverse-proxy](reverse-proxy/) | nginx case: routing + balancing + failover | tgin + 3 python backends |
| [tunnel](tunnel/) | ngrok case: expose a container that publishes no ports | tgin server + tgin client + hidden python app |
| [ingestion](ingestion/) | guaranteed ingestion: HTTP → RabbitMQ → worker, at-least-once | tgin + rabbitmq + python worker |
| [kafka-bridge](kafka-bridge/) | broker bridge: HTTP → Kafka → worker, honest offset commits | tgin + kafka (kraft) + python worker |

every demo:

```
cd <demo>
docker compose up --build
```

then follow the README walkthrough inside the folder.
