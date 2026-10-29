//! API mobile eAbsensi: pengganti `/api/*` Laravel dengan kontrak identik (lihat `docs/api.md`).

pub mod app;
pub mod auth;
pub mod auth_v2;
pub mod config;
pub mod employees;
pub mod error;
pub mod events;
pub mod health;
pub mod input;
pub mod leaves;
pub mod net;
pub mod openapi;
pub mod periods;
pub mod photos;
pub mod punches;
pub mod rate_limit;
pub mod state;
pub mod time;
pub mod token;
pub mod validation;

pub use app::app;
pub use state::AppState;
