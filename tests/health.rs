mod common;

use axum::http::StatusCode;
use common::*;
use eabsensi_mobile_api::rate_limit::{Limit, RateLimiter};
use serde_json::json;
use sqlx::PgPool;

#[sqlx::test(migrations = false)]
async fn healthz_checks_database_and_reports_draining(db: PgPool) {
    let t = setup(db).await;
    let reply = t.send(get("/healthz", None)).await;
    assert_eq!(reply.status, StatusCode::OK);
    assert_eq!(
        reply.json,
        json!({"status": "ok", "database": true, "redis": null})
    );
    assert!(reply.headers.contains_key("x-request-id"));
    assert_eq!(reply.headers["x-content-type-options"], "nosniff");

    t.state.start_draining();
    let reply = t.send(get("/healthz", None)).await;
    assert_eq!(reply.status, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(reply.json, json!({"status": "draining"}));
    // Request API tetap dilayani selama drain.
    let token = t.login("kontrak", "hp").await;
    assert_eq!(
        t.send(get("/api/me", Some(&token))).await.status,
        StatusCode::OK
    );
}

#[sqlx::test(migrations = false)]
async fn healthz_fails_when_database_is_unreachable(db: PgPool) {
    let t = setup(db).await;
    t.db.close().await;
    let reply = t.send(get("/healthz", None)).await;
    assert_eq!(reply.status, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(reply.json["database"], false);
}

/// Penyimpanan Redis dipakai bersama beberapa instance. Hanya berjalan bila `TEST_REDIS_URL` diisi
/// (mis. `redis://127.0.0.1:6379/15`); database Redis itu boleh dikosongkan oleh test.
#[tokio::test]
async fn redis_store_is_shared_between_instances() {
    let Ok(url) = std::env::var("TEST_REDIS_URL") else {
        eprintln!("TEST_REDIS_URL kosong: test Redis dilewati");
        return;
    };
    let a = RateLimiter::redis(&url).await.expect("redis a");
    let b = RateLimiter::redis(&url).await.expect("redis b");
    assert!(a.ping().await);
    let key = format!("uji-{}", std::process::id());
    let limit = [Limit::per_minute(3, key)];
    assert!(a.check(&limit).await.is_ok());
    assert!(b.check(&limit).await.is_ok());
    assert!(a.check(&limit).await.is_ok());
    // Percobaan ke-4 ditolak, dari instance mana pun.
    assert!(b.check(&limit).await.is_err());
    assert!(a.check(&limit).await.is_err());
}
