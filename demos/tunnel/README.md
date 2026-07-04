# tunnel demo — the ngrok case

`app` publishes **no ports** — it is unreachable from your machine. The tgin client sits next to it, dials out to the tgin server over one WebSocket, and the server multiplexes public traffic back through that connection. Same topology as exposing localhost behind NAT.

```
docker compose up --build
```

## walkthrough

the only published port is the tunnel server's 8080 — yet you reach the hidden app through it:

```
curl localhost:8080/anything    # hello from hidden-local-app: /anything
curl localhost:8080/            # hello from hidden-local-app: /
```

## failure drill

kill the client — the tunnel is honest about being down:

```
docker compose stop client
curl -i localhost:8080/         # HTTP/1.1 502 Bad Gateway
```

bring it back — the client reconnects by itself within a second:

```
docker compose start client
curl localhost:8080/            # hello from hidden-local-app: /
```
