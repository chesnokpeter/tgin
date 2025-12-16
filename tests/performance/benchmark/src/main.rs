use axum::{
    body::Bytes,
    extract::{Query, State},
    routing::{get, post},
    Json, Router,
};
use clap::{Parser, ValueEnum};
use dashmap::DashMap;
use hdrhistogram::Histogram;
use reqwest::Client;
use serde::Deserialize;
use serde_json::{json, Value};
use std::{
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use tokio::{
    sync::{Mutex, Notify},
    time::interval,
};
use uuid::Uuid;

#[derive(ValueEnum, Clone, Debug)]
enum BenchMode {
    Webhook,
    Longpoll,
}

#[derive(Parser, Debug)]
#[command(author, version, about)]
struct Args {
    #[arg(short, long, default_value = "http://127.0.0.1:3000/webhook")]
    target: String,
    #[arg(short, long, default_value_t = 1000)]
    rps: u64,
    #[arg(short, long, default_value_t = 10)]
    duration: u64,
    #[arg(short, long, default_value_t = 8090)]
    port: u16,
    #[arg(value_enum, short, long, default_value_t = BenchMode::Webhook)]
    mode: BenchMode,
}

struct BenchState {
    pending: DashMap<String, Instant>,
    histogram: Mutex<Histogram<u64>>,
    sent_count: AtomicUsize,
    received_count: AtomicUsize,
    errors_count: AtomicUsize,
    lp_queue: Mutex<Vec<Value>>,
    notify: Notify,
}

#[derive(Deserialize)]
struct GetUpdatesParams {
    #[allow(dead_code)]
    offset: Option<i64>,
    #[allow(dead_code)]
    timeout: Option<u64>,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();

    let state = Arc::new(BenchState {
        pending: DashMap::new(),
        histogram: Mutex::new(Histogram::<u64>::new(3).unwrap()),
        sent_count: AtomicUsize::new(0),
        received_count: AtomicUsize::new(0),
        errors_count: AtomicUsize::new(0),
        lp_queue: Mutex::new(Vec::new()),
        notify: Notify::new(),
    });

    println!("🚀 Starting Benchmark ({:?})", args.mode);

    let server_state = state.clone();
    let app = Router::new()
        .route("/bot:token/sendMessage", post(handle_send_message))
        .route("/bot:token/getMe", get(handle_get_me).post(handle_get_me))
        .route("/bot:token/getUpdates", get(handle_get_updates).post(handle_get_updates))
        .with_state(server_state);

    let addr = std::net::SocketAddr::from(([0, 0, 0, 0], args.port));
    


    tokio::spawn(async move {
        let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
        axum::serve(listener, app).await.unwrap();
    });
    
    tokio::time::sleep(Duration::from_secs(1)).await;

    let gen_state = state.clone();
    let target_url = args.target.clone();
    let rps = args.rps;
    let duration_sec = args.duration;
    let mode = args.mode.clone();
    let client = Client::builder().pool_max_idle_per_host(1000).build().unwrap();

    let generator_handle = tokio::spawn(async move {
        let start_test = Instant::now();
        let interval_micros = if rps > 0 { 1_000_000 / rps } else { 100_000 };
        let mut ticker = interval(Duration::from_micros(interval_micros));
        let mut update_id_counter = 100000;

        while start_test.elapsed().as_secs() < duration_sec {
            ticker.tick().await;
            let uuid = Uuid::new_v4().to_string();
            let s = gen_state.clone();
            s.pending.insert(uuid.clone(), Instant::now());
            s.sent_count.fetch_add(1, Ordering::Relaxed);

            let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
            update_id_counter += 1;

            let update = json!({
                "update_id": update_id_counter,
                "message": {
                    "message_id": 123,
                    "date": timestamp,
                    "chat": { "id": 1, "type": "private" },
                    "from": { "id": 1, "is_bot": false, "first_name": "Bench" },
                    "text": uuid
                }
            });

            match mode {
                BenchMode::Webhook => {
                    let c = client.clone();
                    let u = target_url.clone();
                    tokio::spawn(async move {
                        if c.post(&u).json(&update).send().await.is_err() {
                            s.errors_count.fetch_add(1, Ordering::Relaxed);
                        }
                    });
                }
                BenchMode::Longpoll => {
                    let mut q = s.lp_queue.lock().await;
                    q.push(update);
                    s.notify.notify_waiters();
                }
            }
        }
    });

    let _stats_handle = tokio::spawn(async move {
        let mut interval = interval(Duration::from_secs(1));
        loop { interval.tick().await; }
    });

    generator_handle.await.unwrap();
    println!("🏁 Sending finished. Waiting 2s for trailing responses...");
    tokio::time::sleep(Duration::from_secs(2)).await;
    print_report(&state).await;
}


async fn handle_get_me() -> Json<Value> {
    Json(json!({
        "ok": true,
        "result": {
            "id": 123456789,
            "is_bot": true,
            "first_name": "Tgin Bench Bot",
            "username": "bench_bot",
            "can_join_groups": true,
            "can_read_all_group_messages": false,
            "supports_inline_queries": false
        }
    }))
}

async fn handle_send_message(State(state): State<Arc<BenchState>>, body: Bytes) -> Json<Value> {
    let payload: Value = if let Ok(json) = serde_json::from_slice(&body) { json } 
                         else if let Ok(form) = serde_urlencoded::from_bytes(&body) { form } 
                         else { json!({}) };

    if let Some(text) = payload.get("text").and_then(|t| t.as_str()) {
        if let Some((_, start_time)) = state.pending.remove(text) {
            let elapsed = start_time.elapsed().as_micros() as u64;
            let mut hist = state.histogram.lock().await;
            let _ = hist.record(elapsed);
            state.received_count.fetch_add(1, Ordering::Relaxed);
        }
    }
    let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
    Json(json!({ "ok": true, "result": { "message_id": 123, "date": timestamp, "chat": { "id": 1, "type": "private" }, "text": "ok" } }))
}

async fn handle_get_updates(State(state): State<Arc<BenchState>>, _query: Option<Query<GetUpdatesParams>>) -> Json<Value> {
    let updates = {
        let mut q = state.lp_queue.lock().await;
        let batch: Vec<Value> = q.drain(..).collect();
        batch
    };
    if updates.is_empty() {
        let _ = tokio::time::timeout(Duration::from_secs(1), state.notify.notified()).await;
        let mut q = state.lp_queue.lock().await;
        let batch: Vec<Value> = q.drain(..).collect();
        return Json(json!({ "ok": true, "result": batch }));
    }
    Json(json!({ "ok": true, "result": updates }))
}

async fn print_report(state: &BenchState) {
    let sent = state.sent_count.load(Ordering::Relaxed);
    let received = state.received_count.load(Ordering::Relaxed);
    let errors = state.errors_count.load(Ordering::Relaxed);
    let hist = state.histogram.lock().await;
    let loss_rate = if sent > 0 { 100.0 * (sent.saturating_sub(received)) as f64 / sent as f64 } else { 0.0 };

    println!("\n==========================================");
    println!("📊 BENCHMARK RESULTS");
    println!("==========================================");
    println!("Requests Sent:     {}", sent);
    println!("Responses Recv:    {}", received);
    println!("Errors (Net):      {}", errors);
    println!("Loss Rate:         {:.2}%", loss_rate);
    println!("------------------------------------------");
    println!("LATENCY (Round-Trip Time):");
    println!("  Min:    {:.2} ms", hist.min() as f64 / 1000.0);
    println!("  Mean:   {:.2} ms", hist.mean() / 1000.0);
    println!("  p50:    {:.2} ms", hist.value_at_quantile(0.5) as f64 / 1000.0);
    println!("  p95:    {:.2} ms", hist.value_at_quantile(0.95) as f64 / 1000.0);
    println!("  p99:    {:.2} ms", hist.value_at_quantile(0.99) as f64 / 1000.0);
    println!("  Max:    {:.2} ms", hist.max() as f64 / 1000.0);
    println!("==========================================");
}