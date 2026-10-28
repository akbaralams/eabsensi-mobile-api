//! `/healthz` untuk health monitor load balancer: 200 hanya bila database (dan Redis, bila dipakai)
//! benar-benar bisa dipakai, sehingga instance yang dependensinya putus dikeluarkan dari pool sebelum
//! menerima request. `/livez` hanya menandakan proses hidup.

use std::time::Duration;

use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde_json::json;

use crate::state::AppState;

pub async fn healthz(State(state): State<AppState>) -> Response {
    if state.is_draining() {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({ "status": "draining" })),
        )
            .into_response();
    }
    let database = tokio::time::timeout(
        Duration::from_secs(2),
        sqlx::query_scalar!("SELECT 1 AS \"one!\"").fetch_one(&state.db),
    )
    .await
    .is_ok_and(|r| r.is_ok());

    let redis = if state.limiter.uses_redis() {
        Some(
            tokio::time::timeout(Duration::from_secs(1), state.limiter.ping())
                .await
                .unwrap_or(false),
        )
    } else {
        None
    };

    let ok = database && redis.unwrap_or(true);
    let status = if ok {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };
    (
        status,
        Json(json!({
            "status": if ok { "ok" } else { "fail" },
            "database": database,
            "redis": redis,
        })),
    )
        .into_response()
}

pub async fn livez() -> Json<serde_json::Value> {
    Json(json!({ "status": "ok" }))
}
