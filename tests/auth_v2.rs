mod common;

use axum::http::StatusCode;
use chrono::{DateTime, TimeZone, Utc};
use common::*;
use serde_json::{Value, json};
use sqlx::PgPool;

async fn login_v2(t: &TestApp, device: &str) -> Value {
    let reply = t
        .send(post_json(
            "/api/v2/auth/login",
            None,
            json!({"login": "kontrak", "password": PASSWORD, "device_name": device}),
        ))
        .await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.json);
    reply.json
}

async fn refresh(t: &TestApp, refresh_token: &str) -> Reply {
    t.send(post_json(
        "/api/v2/auth/refresh",
        None,
        json!({"refresh_token": refresh_token}),
    ))
    .await
}

fn s(v: &Value, key: &str) -> String {
    v[key].as_str().unwrap_or_default().to_string()
}

#[sqlx::test(migrations = false)]
async fn login_returns_pair_and_refresh_rotates(db: PgPool) {
    let clock = MovableClock::at(Utc.with_ymd_and_hms(2026, 10, 30, 1, 0, 0).unwrap());
    let t = setup_with(db, &[], Some(clock.clock())).await;
    let first = login_v2(&t, "hp").await;
    assert_eq!(first["user"]["username"], "kontrak");
    assert_eq!(s(&first, "token_expires_at"), "2026-11-06T08:00:00+07:00");
    assert_eq!(s(&first, "refresh_expires_at"), "2027-01-28T08:00:00+07:00");
    let (access, refresh_token) = (s(&first, "token"), s(&first, "refresh_token"));
    assert!(access.contains('|') && refresh_token.contains('|'));

    // Token akses dipakai ke endpoint v1 seperti biasa; refresh token bukan Bearer.
    assert_eq!(
        t.send(get("/api/me", Some(&access))).await.status,
        StatusCode::OK
    );
    let reply = t.send(get("/api/me", Some(&refresh_token))).await;
    assert_eq!(reply.status, StatusCode::FORBIDDEN);
    assert_eq!(reply.json, json!({"message": "Invalid ability provided."}));

    // Sehari kemudian: tukar refresh token -> pasangan baru, yang lama tidak berlaku.
    clock.advance(86_400);
    let reply = refresh(&t, &refresh_token).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.json);
    assert_eq!(
        s(&reply.json, "token_expires_at"),
        "2026-11-07T08:00:00+07:00"
    );
    let (access2, refresh2) = (s(&reply.json, "token"), s(&reply.json, "refresh_token"));
    assert_ne!(refresh2, refresh_token);
    assert_eq!(
        t.send(get("/api/me", Some(&access2))).await.status,
        StatusCode::OK
    );
    assert_eq!(
        t.send(get("/api/me", Some(&access))).await.status,
        StatusCode::UNAUTHORIZED
    );
    let reply = refresh(&t, &refresh_token).await;
    assert_eq!(reply.status, StatusCode::UNAUTHORIZED);
    assert_eq!(reply.json, json!({"message": "Unauthenticated."}));

    // Hanya satu pasangan per perangkat.
    let rows = t
        .scalar_i64("SELECT count(*) FROM personal_access_tokens WHERE name = 'hp'")
        .await;
    assert_eq!(rows, 2);
}

#[sqlx::test(migrations = false)]
async fn access_token_expires_and_refresh_token_expires(db: PgPool) {
    let start: DateTime<Utc> = Utc.with_ymd_and_hms(2026, 10, 30, 1, 0, 0).unwrap();
    let clock = MovableClock::at(start);
    let t = setup_with(
        db,
        &[("MOBILE_ACCESS_TOKEN_TTL_MINUTES", "60")],
        Some(clock.clock()),
    )
    .await;
    let pair = login_v2(&t, "hp").await;

    clock.advance(3_601);
    assert_eq!(
        t.send(get("/api/me", Some(&s(&pair, "token"))))
            .await
            .status,
        StatusCode::UNAUTHORIZED,
        "token akses kedaluwarsa setelah TTL"
    );
    let reply = refresh(&t, &s(&pair, "refresh_token")).await;
    assert_eq!(reply.status, StatusCode::OK);

    // Refresh token kedaluwarsa setelah 90 hari tanpa dipakai.
    clock.advance(91 * 86_400);
    let reply = refresh(&t, &s(&reply.json, "refresh_token")).await;
    assert_eq!(reply.status, StatusCode::UNAUTHORIZED);
}

#[sqlx::test(migrations = false)]
async fn access_ttl_is_capped_by_sanctum_expiration(db: PgPool) {
    let clock = MovableClock::at(Utc.with_ymd_and_hms(2026, 10, 30, 1, 0, 0).unwrap());
    let t = setup_with(
        db,
        &[("SANCTUM_TOKEN_EXPIRATION", "120")],
        Some(clock.clock()),
    )
    .await;
    let pair = login_v2(&t, "hp").await;
    assert_eq!(s(&pair, "token_expires_at"), "2026-10-30T10:00:00+07:00");
}

#[sqlx::test(migrations = false)]
async fn revocation_from_web_and_deactivation(db: PgPool) {
    let t = setup(db).await;
    // Web mencabut semua token (ganti sandi): refresh token ikut mati.
    let pair = login_v2(&t, "hp").await;
    t.exec("DELETE FROM personal_access_tokens WHERE tokenable_id = (SELECT id FROM users WHERE username = 'kontrak')")
        .await;
    assert_eq!(
        refresh(&t, &s(&pair, "refresh_token")).await.status,
        StatusCode::UNAUTHORIZED
    );

    // Akun dinonaktifkan: refresh ditolak walau token masih ada.
    let pair = login_v2(&t, "hp").await;
    t.exec("UPDATE users SET is_active = false WHERE username = 'kontrak'")
        .await;
    assert_eq!(
        refresh(&t, &s(&pair, "refresh_token")).await.status,
        StatusCode::UNAUTHORIZED
    );
    t.exec("UPDATE users SET is_active = true WHERE username = 'kontrak'")
        .await;

    // Pegawai dilepas dari akun: 403 seperti login.
    let pair = login_v2(&t, "hp").await;
    t.exec("UPDATE users SET employee_id = NULL WHERE username = 'kontrak'")
        .await;
    let reply = refresh(&t, &s(&pair, "refresh_token")).await;
    assert_eq!(reply.status, StatusCode::FORBIDDEN);
    assert_eq!(
        reply.json,
        json!({"message": "Akun ini belum terhubung ke data pegawai."})
    );
}

#[sqlx::test(migrations = false)]
async fn logout_v2_and_v1_login_replace_the_pair(db: PgPool) {
    let t = setup(db).await;
    let pair = login_v2(&t, "hp").await;
    let other = login_v2(&t, "tablet").await;

    let reply = t
        .send(post_json(
            "/api/v2/auth/logout",
            Some(&s(&pair, "token")),
            json!({}),
        ))
        .await;
    assert_eq!(reply.json, json!({"message": "Berhasil keluar."}));
    assert_eq!(
        refresh(&t, &s(&pair, "refresh_token")).await.status,
        StatusCode::UNAUTHORIZED
    );
    // Perangkat lain tidak terpengaruh.
    assert_eq!(
        t.send(get("/api/me", Some(&s(&other, "token"))))
            .await
            .status,
        StatusCode::OK
    );

    // Login v1 di perangkat yang sama menggantikan pasangan v2.
    t.login("kontrak", "tablet").await;
    assert_eq!(
        refresh(&t, &s(&other, "refresh_token")).await.status,
        StatusCode::UNAUTHORIZED
    );
}

#[sqlx::test(migrations = false)]
async fn concurrent_refresh_only_one_wins(db: PgPool) {
    let t = setup(db).await;
    let pair = login_v2(&t, "hp").await;
    let token = s(&pair, "refresh_token");
    let (a, b) = tokio::join!(refresh(&t, &token), refresh(&t, &token));
    let mut statuses = [a.status.as_u16(), b.status.as_u16()];
    statuses.sort_unstable();
    assert_eq!(statuses, [200, 401]);
}

#[sqlx::test(migrations = false)]
async fn refresh_validation_and_garbage(db: PgPool) {
    let t = setup(db).await;
    let reply = t
        .send(post_json("/api/v2/auth/refresh", None, json!({})))
        .await;
    assert_eq!(reply.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(reply.json["errors"]["refresh_token"].is_array());
    for bad in ["abc", "1|abc", "x|y"] {
        assert_eq!(
            refresh(&t, bad).await.status,
            StatusCode::UNAUTHORIZED,
            "{bad}"
        );
    }
    // Token akses tidak bisa dipakai sebagai refresh token.
    let pair = login_v2(&t, "hp").await;
    assert_eq!(
        refresh(&t, &s(&pair, "token")).await.status,
        StatusCode::UNAUTHORIZED
    );
}
