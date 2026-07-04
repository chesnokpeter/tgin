# reverse-proxy demo — the nginx case

one tgin in front of three backends: `/api/*` is balanced round-robin across backend1/backend2 with failover, everything else goes to backend3.

```
docker compose up --build
```

## walkthrough

round robin over /api:

```
curl localhost:8080/api/users   # hello from backend1: /api/users
curl localhost:8080/api/users   # hello from backend2: /api/users
curl localhost:8080/api/users   # hello from backend1: /api/users
```

everything else falls through to backend3:

```
curl localhost:8080/hello       # hello from backend3: /hello
```

## failure drill

kill one api backend — traffic fails over, clients see nothing:

```
docker compose stop backend1
curl localhost:8080/api/users   # hello from backend2 (every time, no errors)
```

kill both — tgin answers an honest 502 instead of hanging:

```
docker compose stop backend2
curl -i localhost:8080/api/users   # HTTP/1.1 502 Bad Gateway
curl localhost:8080/hello          # backend3 still fine
```

bring them back:

```
docker compose start backend1 backend2
curl localhost:8080/api/users   # balanced again
```
