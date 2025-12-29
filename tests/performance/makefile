RPS ?= 100
DURATION ?= 10
BENCH_PORT := 8090


clean:
	docker compose down


build: clean
	docker compose build

webhook-direct: clean
	docker compose up -d webhook_bot1 bench

	@sleep 10

	docker compose exec bench tgin-bench \
		--target http://10.5.0.11:8080/webhook \
		--port $(BENCH_PORT) \
		--rps $(RPS) \
		--duration $(DURATION)

	@sleep 10

webhook-tgin: clean
	docker compose up -d tgin_webhook webhook_bot1 webhook_bot2 bench

	@sleep 10

	docker compose exec bench tgin-bench \
		--target http://10.5.0.2:3000/webhook \
		--port $(BENCH_PORT) \
		--rps $(RPS) \
		--duration $(DURATION)

	@sleep 10



longpull-tgin: clean
	docker compose up -d tgin_longpull longpull_bot1 longpull_bot2 bench

	@sleep 10

	docker compose exec bench tgin-bench \
		--mode longpoll \
		--port $(BENCH_PORT) \
		--rps $(RPS) \
		--duration $(DURATION)

	@sleep 10





longpull-direct: clean
	docker compose up -d longpull_bot_direct bench

	@sleep 10

	docker compose exec bench tgin-bench \
		--mode longpoll \
		--port $(BENCH_PORT) \
		--rps $(RPS) \
		--duration $(DURATION)

	@sleep 10





webhook-tgin-3: clean
	docker compose up -d tgin_webhook webhook_bot1 webhook_bot2 bench webhook_bot3

	@sleep 10

	curl --request POST http://localhost:3000/api/route --data '{ "type": "Webhook", "url": "http://10.5.0.13:8080/webhook"}' --header "Content-Type: application/json"

	docker compose exec bench tgin-bench \
		--target http://10.5.0.2:3000/webhook \
		--port $(BENCH_PORT) \
		--rps $(RPS) \
		--duration $(DURATION)

	@sleep 10

webhook-tgin-4: clean
	docker compose up -d tgin_webhook webhook_bot1 webhook_bot2 bench webhook_bot3 webhook_bot4

	@sleep 10

	curl --request POST http://localhost:3000/api/route --data '{ "type": "Webhook", "url": "http://10.5.0.13:8080/webhook"}' --header "Content-Type: application/json"
	curl --request POST http://localhost:3000/api/route --data '{ "type": "Webhook", "url": "http://10.5.0.14:8080/webhook"}' --header "Content-Type: application/json"

	docker compose exec bench tgin-bench \
		--target http://10.5.0.2:3000/webhook \
		--port $(BENCH_PORT) \
		--rps $(RPS) \
		--duration $(DURATION)

	@sleep 10

webhook-tgin-5: clean
	docker compose up -d tgin_webhook webhook_bot1 webhook_bot2 bench webhook_bot3 webhook_bot4 webhook_bot5

	@sleep 10

	curl --request POST http://localhost:3000/api/route --data '{ "type": "Webhook", "url": "http://10.5.0.13:8080/webhook"}' --header "Content-Type: application/json"
	curl --request POST http://localhost:3000/api/route --data '{ "type": "Webhook", "url": "http://10.5.0.14:8080/webhook"}' --header "Content-Type: application/json"
	curl --request POST http://localhost:3000/api/route --data '{ "type": "Webhook", "url": "http://10.5.0.15:8080/webhook"}' --header "Content-Type: application/json"

	docker compose exec bench tgin-bench \
		--target http://10.5.0.2:3000/webhook \
		--port $(BENCH_PORT) \
		--rps $(RPS) \
		--duration $(DURATION)

	@sleep 10

webhook-tgin-10: clean
	docker compose up -d tgin_webhook webhook_bot1 webhook_bot2 bench webhook_bot3 webhook_bot4 webhook_bot5 webhook_bot6 webhook_bot7 webhook_bot8 webhook_bot9 webhook_bot10

	@sleep 10

	curl --request POST http://localhost:3000/api/route --data '{ "type": "Webhook", "url": "http://10.5.0.13:8080/webhook"}' --header "Content-Type: application/json"
	curl --request POST http://localhost:3000/api/route --data '{ "type": "Webhook", "url": "http://10.5.0.14:8080/webhook"}' --header "Content-Type: application/json"
	curl --request POST http://localhost:3000/api/route --data '{ "type": "Webhook", "url": "http://10.5.0.15:8080/webhook"}' --header "Content-Type: application/json"
	curl --request POST http://localhost:3000/api/route --data '{ "type": "Webhook", "url": "http://10.5.0.16:8080/webhook"}' --header "Content-Type: application/json"
	curl --request POST http://localhost:3000/api/route --data '{ "type": "Webhook", "url": "http://10.5.0.17:8080/webhook"}' --header "Content-Type: application/json"
	curl --request POST http://localhost:3000/api/route --data '{ "type": "Webhook", "url": "http://10.5.0.18:8080/webhook"}' --header "Content-Type: application/json"
	curl --request POST http://localhost:3000/api/route --data '{ "type": "Webhook", "url": "http://10.5.0.19:8080/webhook"}' --header "Content-Type: application/json"
	curl --request POST http://localhost:3000/api/route --data '{ "type": "Webhook", "url": "http://10.5.0.199:8080/webhook"}' --header "Content-Type: application/json"


	docker compose exec bench tgin-bench \
		--target http://10.5.0.2:3000/webhook \
		--port $(BENCH_PORT) \
		--rps $(RPS) \
		--duration $(DURATION)

	@sleep 10


longpull-tgin-3: clean
	docker compose up -d tgin_longpull longpull_bot1 longpull_bot2 bench longpull_bot3

	@sleep 10

	curl --request POST http://localhost:3030/api/route --data '{ "type": "Longpull", "path": "/bot3/getUpdates"}' --header "Content-Type: application/json"

	docker compose exec bench tgin-bench \
		--mode longpoll \
		--port $(BENCH_PORT) \
		--rps $(RPS) \
		--duration $(DURATION)

	@sleep 10



longpull-tgin-4: clean
	docker compose up -d tgin_longpull longpull_bot1 longpull_bot2 bench longpull_bot3 longpull_bot4

	@sleep 10

	curl --request POST http://localhost:3030/api/route --data '{ "type": "Longpull", "path": "/bot3/getUpdates"}' --header "Content-Type: application/json"
	curl --request POST http://localhost:3030/api/route --data '{ "type": "Longpull", "path": "/bot4/getUpdates"}' --header "Content-Type: application/json"


	docker compose exec bench tgin-bench \
		--mode longpoll \
		--port $(BENCH_PORT) \
		--rps $(RPS) \
		--duration $(DURATION)

	@sleep 10


longpull-tgin-5: clean
	docker compose up -d tgin_longpull longpull_bot1 longpull_bot2 bench longpull_bot3 longpull_bot4 longpull_bot5

	@sleep 10


	curl --request POST http://localhost:3030/api/route --data '{ "type": "Longpull", "path": "/bot3/getUpdates"}' --header "Content-Type: application/json"
	curl --request POST http://localhost:3030/api/route --data '{ "type": "Longpull", "path": "/bot4/getUpdates"}' --header "Content-Type: application/json"
	curl --request POST http://localhost:3030/api/route --data '{ "type": "Longpull", "path": "/bot5/getUpdates"}' --header "Content-Type: application/json"


	docker compose exec bench tgin-bench \
		--mode longpoll \
		--port $(BENCH_PORT) \
		--rps $(RPS) \
		--duration $(DURATION)

	@sleep 10


longpull-tgin-10: clean
	docker compose up -d tgin_longpull longpull_bot1 longpull_bot2 bench longpull_bot3 longpull_bot4 longpull_bot5 longpull_bot6 longpull_bot7 longpull_bot8 longpull_bot9 longpull_bot10

	@sleep 10


	curl --request POST http://localhost:3030/api/route --data '{ "type": "Longpull", "path": "/bot3/getUpdates"}' --header "Content-Type: application/json"
	curl --request POST http://localhost:3030/api/route --data '{ "type": "Longpull", "path": "/bot4/getUpdates"}' --header "Content-Type: application/json"
	curl --request POST http://localhost:3030/api/route --data '{ "type": "Longpull", "path": "/bot5/getUpdates"}' --header "Content-Type: application/json"
	curl --request POST http://localhost:3030/api/route --data '{ "type": "Longpull", "path": "/bot6/getUpdates"}' --header "Content-Type: application/json"
	curl --request POST http://localhost:3030/api/route --data '{ "type": "Longpull", "path": "/bot7/getUpdates"}' --header "Content-Type: application/json"
	curl --request POST http://localhost:3030/api/route --data '{ "type": "Longpull", "path": "/bot8/getUpdates"}' --header "Content-Type: application/json"
	curl --request POST http://localhost:3030/api/route --data '{ "type": "Longpull", "path": "/bot9/getUpdates"}' --header "Content-Type: application/json"
	curl --request POST http://localhost:3030/api/route --data '{ "type": "Longpull", "path": "/bot10/getUpdates"}' --header "Content-Type: application/json"


	docker compose exec bench tgin-bench \
		--mode longpoll \
		--port $(BENCH_PORT) \
		--rps $(RPS) \
		--duration $(DURATION)

	@sleep 10

