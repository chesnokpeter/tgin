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

webhook-tgin: clean
	docker compose up -d tgin_webhook webhook_bot1 webhook_bot2 bench

	@sleep 10

	docker compose exec bench tgin-bench \
		--target http://10.5.0.2:3000/webhook \
		--port $(BENCH_PORT) \
		--rps $(RPS) \
		--duration $(DURATION)



longpull-tgin: clean
	docker compose up -d tgin_longpull longpull_bot1 longpull_bot2 bench

	@sleep 10

	docker compose exec bench tgin-bench \
		--mode longpoll \
		--port $(BENCH_PORT) \
		--rps $(RPS) \
		--duration $(DURATION)





longpull-direct: clean
	docker compose up -d longpull_bot_direct bench

	@sleep 10

	docker compose exec bench tgin-bench \
		--mode longpoll \
		--port $(BENCH_PORT) \
		--rps $(RPS) \
		--duration $(DURATION)





webhook-tgin-3: clean
	docker compose up -d tgin_webhook webhook_bot1 webhook_bot2 bench webhook_bot3

	@sleep 10

	curl --request POST http://10.5.0.2:3000/api/route --data '{ "type": "Webhook", "url": "http://10.5.0.13:8080/webhook"}' --header "Content-Type: application/json"

	docker compose exec bench tgin-bench \
		--target http://10.5.0.2:3000/webhook \
		--port $(BENCH_PORT) \
		--rps $(RPS) \
		--duration $(DURATION)

webhook-tgin-4: clean
	docker compose up -d tgin_webhook webhook_bot1 webhook_bot2 bench webhook_bot3 webhook_bot4

	@sleep 10

	curl --request POST http://10.5.0.2:3000/api/route --data '{ "type": "Webhook", "url": "http://10.5.0.13:8080/webhook"}' --header "Content-Type: application/json"
	curl --request POST http://10.5.0.2:3000/api/route --data '{ "type": "Webhook", "url": "http://10.5.0.14:8080/webhook"}' --header "Content-Type: application/json"

	docker compose exec bench tgin-bench \
		--target http://10.5.0.2:3000/webhook \
		--port $(BENCH_PORT) \
		--rps $(RPS) \
		--duration $(DURATION)

webhook-tgin-5: clean
	docker compose up -d tgin_webhook webhook_bot1 webhook_bot2 bench webhook_bot3 webhook_bot4 webhook_bot5

	@sleep 10

	curl --request POST http://10.5.0.2:3000/api/route --data '{ "type": "Webhook", "url": "http://10.5.0.13:8080/webhook"}' --header "Content-Type: application/json"
	curl --request POST http://10.5.0.2:3000/api/route --data '{ "type": "Webhook", "url": "http://10.5.0.14:8080/webhook"}' --header "Content-Type: application/json"
	curl --request POST http://10.5.0.2:3000/api/route --data '{ "type": "Webhook", "url": "http://10.5.0.15:8080/webhook"}' --header "Content-Type: application/json"

	docker compose exec bench tgin-bench \
		--target http://10.5.0.2:3000/webhook \
		--port $(BENCH_PORT) \
		--rps $(RPS) \
		--duration $(DURATION)



longpull-tgin-3: clean
	docker compose up -d tgin_longpull longpull_bot1 longpull_bot2 bench longpull_bot3

	@sleep 10

	curl --request POST http://10.5.0.21:3030/api/route --data '{ "type": "Longpull", "path": "/bot3/getUpdates"}' --header "Content-Type: application/json"

	docker compose exec bench tgin-bench \
		--mode longpoll \
		--port $(BENCH_PORT) \
		--rps $(RPS) \
		--duration $(DURATION)



longpull-tgin-4: clean
	docker compose up -d tgin_longpull longpull_bot1 longpull_bot2 bench longpull_bot3 longpull_bot4

	@sleep 10

	curl --request POST http://10.5.0.21:3030/api/route --data '{ "type": "Longpull", "path": "/bot3/getUpdates"}' --header "Content-Type: application/json"
	curl --request POST http://10.5.0.21:3030/api/route --data '{ "type": "Longpull", "path": "/bot4/getUpdates"}' --header "Content-Type: application/json"


	docker compose exec bench tgin-bench \
		--mode longpoll \
		--port $(BENCH_PORT) \
		--rps $(RPS) \
		--duration $(DURATION)


longpull-tgin-5: clean
	docker compose up -d tgin_longpull longpull_bot1 longpull_bot2 bench longpull_bot3 longpull_bot4 longpull_bot5

	@sleep 10


	curl --request POST http://10.5.0.21:3030/api/route --data '{ "type": "Longpull", "path": "/bot3/getUpdates"}' --header "Content-Type: application/json"
	curl --request POST http://10.5.0.21:3030/api/route --data '{ "type": "Longpull", "path": "/bot4/getUpdates"}' --header "Content-Type: application/json"
	curl --request POST http://10.5.0.21:3030/api/route --data '{ "type": "Longpull", "path": "/bot5/getUpdates"}' --header "Content-Type: application/json"

	docker compose exec bench tgin-bench \
		--mode longpoll \
		--port $(BENCH_PORT) \
		--rps $(RPS) \
		--duration $(DURATION)

