//! API mobile eAbsensi: pengganti `/api/*` Laravel dengan kontrak identik (lihat `docs/api.md`).

pub mod app;
pub mod config;
pub mod error;
pub mod health;
pub mod input;
pub mod net;
pub mod rate_limit;
pub mod state;
pub mod time;
pub mod validation;

pub use app::app;
pub use state::AppState;
