//! Uji beban "puncak pagi": setiap pegawai uji membuka aplikasi lalu
//! `GET /me` -> `POST /punches` (foto) -> `GET /punches` -> `GET /events` -> `POST check-in` (foto).
//! Kedatangan pegawai disebar merata dalam jendela waktu; sebagian login ulang (bcrypt), sisanya memakai
//! token 30 hari yang sudah ada (lihat scripts/loadtest/seed.sql).
//!
//!   psql ... -f scripts/loadtest/seed.sql
//!   cargo run --release --example loadtest
//!
//! Variabel: LT_BASE_URL (http://127.0.0.1:8080), LT_USERS (2000), LT_WINDOW_SECS (60),
//! LT_LOGIN_PERCENT (10), LT_PHOTO_KB (150), LT_PASSWORD (kontrak-uji-2026).

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use reqwest::multipart::{Form, Part};
use serde_json::Value;

fn env<T: std::str::FromStr>(name: &str, default: T) -> T {
    std::env::var(name)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

#[derive(Default)]
struct Stats {
    latencies: BTreeMap<&'static str, Vec<Duration>>,
    statuses: BTreeMap<(&'static str, String), u64>,
}

type Shared = Arc<Mutex<Stats>>;

fn record(stats: &Shared, name: &'static str, started: Instant, status: Result<u16, String>) {
    let elapsed = started.elapsed();
    let mut s = stats
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    s.latencies.entry(name).or_default().push(elapsed);
    let key = match status {
        Ok(code) => code.to_string(),
        Err(e) => format!("galat: {e}"),
    };
    *s.statuses.entry((name, key)).or_default() += 1;
}

/// PNG valid (header + IHDR + IDAT + IEND dari fixture) yang dipadatkan sampai ukuran foto selfie.
fn photo(kb: usize) -> Vec<u8> {
    let mut bytes = include_bytes!("../tests/contract/fixtures/photo.png").to_vec();
    bytes.resize(kb * 1024, 0);
    bytes
}

fn evidence(photo: &[u8]) -> Form {
    Form::new()
        .text("latitude", "-6.2")
        .text("longitude", "106.8")
        .text("accuracy", "12")
        .part(
            "photo",
            Part::bytes(photo.to_vec())
                .file_name("selfie.png")
                .mime_str("image/png")
                .unwrap_or_else(|_| Part::bytes(Vec::new())),
        )
}

async fn send(
    stats: &Shared,
    name: &'static str,
    request: reqwest::RequestBuilder,
) -> Option<Value> {
    let started = Instant::now();
    match request.send().await {
        Ok(response) => {
            let status = response.status().as_u16();
            let body = response.json::<Value>().await.ok();
            record(stats, name, started, Ok(status));
            body
        }
        Err(error) => {
            record(stats, name, started, Err(error.to_string()));
            None
        }
    }
}

async fn employee_morning(
    client: reqwest::Client,
    base: String,
    n: usize,
    login: bool,
    password: String,
    photo: Arc<Vec<u8>>,
    stats: Shared,
) {
    let token = if login {
        let body = serde_json::json!({
            "login": format!("loadtest-{n:04}"),
            "password": password,
            "device_name": "loadtest",
        });
        let reply = send(
            &stats,
            "POST /auth/login",
            client.post(format!("{base}/api/auth/login")).json(&body),
        )
        .await;
        match reply.and_then(|v| v["token"].as_str().map(str::to_string)) {
            Some(token) => token,
            None => return,
        }
    } else {
        format!("{}|lt{n:046}", 900_000_000 + n)
    };
    let auth = format!("Bearer {token}");
    let get = |path: &str| {
        client
            .get(format!("{base}{path}"))
            .header("Authorization", &auth)
            .header("Accept", "application/json")
    };

    send(&stats, "GET /me", get("/api/me")).await;
    send(
        &stats,
        "POST /punches",
        client
            .post(format!("{base}/api/punches"))
            .header("Authorization", &auth)
            .header("Accept", "application/json")
            .multipart(evidence(&photo)),
    )
    .await;
    send(&stats, "GET /punches", get("/api/punches")).await;
    let events = send(&stats, "GET /events", get("/api/events")).await;
    let event_id = events.and_then(|v| {
        v["data"]
            .as_array()?
            .iter()
            .find(|e| e["name"] == "Apel Beban")?["id"]
            .as_i64()
    });
    if let Some(id) = event_id {
        send(
            &stats,
            "POST /events/{id}/check-in",
            client
                .post(format!("{base}/api/events/{id}/check-in"))
                .header("Authorization", &auth)
                .header("Accept", "application/json")
                .multipart(evidence(&photo)),
        )
        .await;
    }
}

fn percentile(sorted: &[Duration], p: f64) -> Duration {
    if sorted.is_empty() {
        return Duration::ZERO;
    }
    let index = ((sorted.len() as f64 - 1.0) * p).round() as usize;
    sorted[index.min(sorted.len() - 1)]
}

fn ms(d: Duration) -> String {
    format!("{:.0}", d.as_secs_f64() * 1000.0)
}

#[tokio::main]
async fn main() {
    let base: String = env("LT_BASE_URL", "http://127.0.0.1:8080".to_string());
    let users: usize = env("LT_USERS", 2000);
    let window = Duration::from_secs(env("LT_WINDOW_SECS", 60));
    let login_percent: usize = env("LT_LOGIN_PERCENT", 10);
    let photo_kb: usize = env("LT_PHOTO_KB", 150);
    let password: String = env("LT_PASSWORD", "kontrak-uji-2026".to_string());

    let client = match reqwest::Client::builder()
        .timeout(Duration::from_secs(120))
        .pool_max_idle_per_host(1024)
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            eprintln!("client: {e}");
            std::process::exit(1);
        }
    };
    let photo = Arc::new(photo(photo_kb));
    let stats: Shared = Arc::default();

    println!(
        "{users} pegawai, jendela {}s, login ulang {login_percent}%, foto {photo_kb} KB -> {base}",
        window.as_secs()
    );
    let start = Instant::now();
    let mut tasks = Vec::with_capacity(users);
    for n in 1..=users {
        let offset = window.mul_f64((n - 1) as f64 / users as f64);
        let (client, base, password, photo, stats) = (
            client.clone(),
            base.clone(),
            password.clone(),
            photo.clone(),
            stats.clone(),
        );
        // Sebar pegawai yang login ulang merata (bukan n pertama saja).
        let login = (n * 7919) % 100 < login_percent;
        tasks.push(tokio::spawn(async move {
            tokio::time::sleep_until(tokio::time::Instant::from_std(start + offset)).await;
            employee_morning(client, base, n, login, password, photo, stats).await;
        }));
    }
    for task in tasks {
        let _ = task.await;
    }
    let elapsed = start.elapsed();

    let stats = stats
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let total: usize = stats.latencies.values().map(Vec::len).sum();
    println!(
        "\nSelesai dalam {:.1}s, {total} request ({:.0} req/s rata-rata)\n",
        elapsed.as_secs_f64(),
        total as f64 / elapsed.as_secs_f64()
    );
    println!("| Endpoint | Request | p50 ms | p95 ms | p99 ms | maks ms |");
    println!("|---|---:|---:|---:|---:|---:|");
    for (name, latencies) in &stats.latencies {
        let mut sorted = latencies.clone();
        sorted.sort_unstable();
        println!(
            "| {name} | {} | {} | {} | {} | {} |",
            sorted.len(),
            ms(percentile(&sorted, 0.50)),
            ms(percentile(&sorted, 0.95)),
            ms(percentile(&sorted, 0.99)),
            ms(*sorted.last().unwrap_or(&Duration::ZERO))
        );
    }
    println!("\nStatus:");
    for ((name, status), count) in &stats.statuses {
        println!("  {name}: {status} x{count}");
    }
}
