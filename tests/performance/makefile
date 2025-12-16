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