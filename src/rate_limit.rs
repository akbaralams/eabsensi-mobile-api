//! Batas laju seperti `ThrottleRequests` Laravel: jendela tetap per kunci; request ditolak bila jumlah
//! percobaan sudah mencapai batas (penolakan tidak ikut dihitung). Dua penyimpanan:
//! - memori: cukup untuk satu instance;
//! - Redis: dipakai bersama oleh beberapa instance di belakang load balancer. Bila Redis tidak bisa
//!   dihubungi, request **diloloskan** (fail-open) dan dicatat di log, agar gangguan Redis tidak
//!   mematikan absensi.

use std::collections::HashMap;
use std::hash::{BuildHasher, RandomState};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use chrono::Utc;
use redis::aio::ConnectionManager;

use crate::error::AppError;

const SHARDS: usize = 32;
const WINDOW: Duration = Duration::from_secs(60);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Limit {
    pub key: String,
    /// Maksimum per menit; 0 berarti nonaktif.
    pub max: u32,
}

impl Limit {
    pub fn per_minute(max: u32, key: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            max,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Outcome {
    Allowed,
    Denied { retry_after: Duration },
}

#[derive(Clone)]
pub struct RateLimiter {
    store: Arc<Store>,
}

enum Store {
    Memory(MemoryStore),
    Redis {
        conn: ConnectionManager,
        script: redis::Script,
    },
}

impl RateLimiter {
    pub fn memory() -> Self {
        Self {
            store: Arc::new(Store::Memory(MemoryStore::default())),
        }
    }

    pub async fn redis(url: &str) -> Result<Self, redis::RedisError> {
        let client = redis::Client::open(url)?;
        let conn = ConnectionManager::new(client).await?;
        Ok(Self {
            store: Arc::new(Store::Redis {
                conn,
                script: redis::Script::new(REDIS_SCRIPT),
            }),
        })
    }

    /// Periksa lalu catat setiap batas berurutan; berhenti pada batas pertama yang terlampaui.
    pub async fn check(&self, limits: &[Limit]) -> Result<(), AppError> {
        for limit in limits.iter().filter(|l| l.max > 0) {
            if let Outcome::Denied { retry_after } = self.attempt(limit).await {
                let secs = retry_after.as_secs().max(1);
                return Err(AppError::TooManyRequests {
                    limit: limit.max,
                    retry_after_secs: secs,
                    reset_at_unix: Utc::now().timestamp() + secs as i64,
                });
            }
        }
        Ok(())
    }

    async fn attempt(&self, limit: &Limit) -> Outcome {
        match &*self.store {
            Store::Memory(store) => store.attempt(&limit.key, limit.max, Instant::now()),
            Store::Redis { conn, script } => {
                let mut conn = conn.clone();
                let result: Result<(i64, i64), _> = script
                    .key(format!("eabsensi:rl:{}", limit.key))
                    .arg(limit.max)
                    .arg(WINDOW.as_millis() as u64)
                    .invoke_async(&mut conn)
                    .await;
                match result {
                    Ok((1, _)) => Outcome::Allowed,
                    Ok((_, ttl_ms)) => Outcome::Denied {
                        retry_after: Duration::from_millis(ttl_ms.max(0) as u64),
                    },
                    Err(error) => {
                        tracing::warn!(%error, "redis batas laju gagal; request diloloskan");
                        Outcome::Allowed
                    }
                }
            }
        }
    }

    /// Hapus kunci yang jendelanya sudah lewat (hanya penyimpanan memori).
    pub fn purge_expired(&self) {
        if let Store::Memory(store) = &*self.store {
            store.purge(Instant::now());
        }
    }

    /// Untuk health check: Redis bisa dihubungi (memori selalu sehat).
    pub async fn ping(&self) -> bool {
        match &*self.store {
            Store::Memory(_) => true,
            Store::Redis { conn, .. } => {
                let mut conn = conn.clone();
                redis::cmd("PING")
                    .query_async::<String>(&mut conn)
                    .await
                    .is_ok()
            }
        }
    }

    pub fn uses_redis(&self) -> bool {
        matches!(&*self.store, Store::Redis { .. })
    }
}

/// KEYS[1] = kunci, ARGV[1] = maksimum, ARGV[2] = jendela (ms). Hasil: {diizinkan(1/0), ttl ms}.
const REDIS_SCRIPT: &str = r"
local current = tonumber(redis.call('GET', KEYS[1]) or '0')
if current >= tonumber(ARGV[1]) then
  local ttl = redis.call('PTTL', KEYS[1])
  if ttl < 0 then
    redis.call('PEXPIRE', KEYS[1], ARGV[2])
    ttl = tonumber(ARGV[2])
  end
  return {0, ttl}
end
current = redis.call('INCR', KEYS[1])
if current == 1 or redis.call('PTTL', KEYS[1]) < 0 then
  redis.call('PEXPIRE', KEYS[1], ARGV[2])
end
return {1, redis.call('PTTL', KEYS[1])}
";

struct MemoryStore {
    shards: Vec<Mutex<HashMap<String, Window>>>,
    hasher: RandomState,
}

#[derive(Clone, Copy)]
struct Window {
    count: u32,
    reset_at: Instant,
}

impl Default for MemoryStore {
    fn default() -> Self {
        Self {
            shards: (0..SHARDS).map(|_| Mutex::new(HashMap::new())).collect(),
            hasher: RandomState::new(),
        }
    }
}

impl MemoryStore {
    fn shard(&self, key: &str) -> &Mutex<HashMap<String, Window>> {
        let index = (self.hasher.hash_one(key) as usize) % SHARDS;
        &self.shards[index]
    }

    fn attempt(&self, key: &str, max: u32, now: Instant) -> Outcome {
        // Mutex yang "poisoned" hanya terjadi bila thread lain panik saat memegangnya; isinya tetap utuh.
        let mut map = self
            .shard(key)
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let window = map.entry(key.to_string()).or_insert(Window {
            count: 0,
            reset_at: now + WINDOW,
        });
        if window.reset_at <= now {
            *window = Window {
                count: 0,
                reset_at: now + WINDOW,
            };
        }
        if window.count >= max {
            return Outcome::Denied {
                retry_after: window.reset_at - now,
            };
        }
        window.count += 1;
        Outcome::Allowed
    }

    fn purge(&self, now: Instant) {
        for shard in &self.shards {
            shard
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .retain(|_, w| w.reset_at > now);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixed_window_like_laravel() {
        let store = MemoryStore::default();
        let start = Instant::now();
        for _ in 0..5 {
            assert_eq!(store.attempt("k", 5, start), Outcome::Allowed);
        }
        let denied = store.attempt("k", 5, start + Duration::from_secs(10));
        assert_eq!(
            denied,
            Outcome::Denied {
                retry_after: Duration::from_secs(50)
            }
        );
        // Kunci lain tidak terpengaruh.
        assert_eq!(store.attempt("lain", 5, start), Outcome::Allowed);
        // Jendela baru setelah 60 detik.
        assert_eq!(
            store.attempt("k", 5, start + Duration::from_secs(61)),
            Outcome::Allowed
        );
    }

    #[test]
    fn purge_removes_expired_windows() {
        let store = MemoryStore::default();
        let start = Instant::now();
        store.attempt("k", 5, start);
        store.purge(start + Duration::from_secs(61));
        assert!(store.shards.iter().all(|s| s.lock().unwrap().is_empty()));
    }

    #[tokio::test]
    async fn check_reports_first_exceeded_limit_and_skips_disabled() {
        let limiter = RateLimiter::memory();
        let limits = [Limit::per_minute(0, "nonaktif"), Limit::per_minute(1, "a")];
        assert!(limiter.check(&limits).await.is_ok());
        match limiter.check(&limits).await {
            Err(AppError::TooManyRequests {
                limit,
                retry_after_secs,
                ..
            }) => {
                assert_eq!(limit, 1);
                assert!(retry_after_secs >= 59);
            }
            other => panic!("harus 429, dapat {other:?}"),
        }
    }
}
