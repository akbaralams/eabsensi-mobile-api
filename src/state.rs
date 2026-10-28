use std::sync::Arc;

use chrono::{DateTime, SubsecRound, Utc};
use sqlx::PgPool;
use tokio::sync::Semaphore;

use crate::config::Config;
use crate::rate_limit::RateLimiter;

/// Sumber waktu. Produksi memakai jam sistem; test bisa memasang waktu tetap.
pub type Clock = Arc<dyn Fn() -> DateTime<Utc> + Send + Sync>;

#[derive(Clone)]
pub struct AppState {
    pub db: PgPool,
    pub config: Arc<Config>,
    pub limiter: RateLimiter,
    /// Slot verifikasi bcrypt bersamaan (lihat `LOGIN_HASH_CONCURRENCY`).
    pub hash_slots: Arc<Semaphore>,
    /// Slot request `/api` bersamaan (lihat `MAX_CONCURRENT_REQUESTS`); sisanya antre.
    pub request_slots: Arc<Semaphore>,
    pub clock: Clock,
}

impl AppState {
    pub fn new(db: PgPool, config: Config, limiter: RateLimiter) -> Self {
        let hash_slots = config.hash_concurrency.max(1);
        let request_slots = config.max_concurrent_requests.max(1);
        Self {
            db,
            config: Arc::new(config),
            limiter,
            hash_slots: Arc::new(Semaphore::new(hash_slots)),
            request_slots: Arc::new(Semaphore::new(request_slots)),
            clock: Arc::new(Utc::now),
        }
    }

    pub fn with_clock(mut self, clock: Clock) -> Self {
        self.clock = clock;
        self
    }

    /// Waktu server, dibulatkan ke bawah ke detik (kolom `timestamp(0)`; Laravel memotong, bukan membulatkan).
    pub fn now(&self) -> DateTime<Utc> {
        (self.clock)().trunc_subsecs(0)
    }
}
