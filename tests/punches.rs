mod common;

use axum::http::StatusCode;
use chrono::{TimeZone, Utc};
use common::*;
use serde_json::json;
use sqlx::PgPool;

fn punch(
    token: &str,
    fields: &[(&str, &str)],
    photo: bool,
) -> axum::http::Request<axum::body::Body> {
    post_multipart(
        "/api/punches",
        Some(token),
        fields,
        photo.then_some(("photo", PNG, "photo.png", "image/png")),
    )
}

fn count_files(dir: &std::path::Path) -> usize {
    std::fs::read_dir(dir)
        .map(|entries| {
            entries
                .flatten()
                .map(|e| {
                    if e.path().is_dir() {
                        count_files(&e.path())
                    } else {
                        1
                    }
                })
                .sum()
        })
        .unwrap_or(0)
}

#[sqlx::test(migrations = false)]
async fn validation_errors_use_field_names(db: PgPool) {
    let t = setup(db).await;
    let token = t.login("kontrak", "hp").await;

    let reply = t
        .send(punch(
            &token,
            &[("latitude", "-6.2"), ("longitude", "106.8")],
            false,
        ))
        .await;
    assert_eq!(reply.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(reply.json["errors"]["photo"].is_array());

    let req = post_multipart(
        "/api/punches",
        Some(&token),
        &[],
        Some(("photo", b"bukan gambar\n", "not-an-image.txt", "text/plain")),
    );
    let reply = t.send(req).await;
    assert_eq!(reply.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(reply.json["errors"]["photo"].is_array());

    // Content-type kiriman klien tidak dipercaya: isi berkas tetap diperiksa.
    let req = post_multipart(
        "/api/punches",
        Some(&token),
        &[],
        Some(("photo", b"bukan gambar\n", "foto.png", "image/png")),
    );
    assert_eq!(t.send(req).await.status, StatusCode::UNPROCESSABLE_ENTITY);

    let reply = t.send(punch(&token, &[("latitude", "-6.2")], true)).await;
    assert_eq!(reply.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(reply.json["errors"]["longitude"].is_array());
    assert_eq!(
        t.scalar_i64("SELECT count(*) FROM attendance_punches")
            .await,
        0
    );
}

#[sqlx::test(migrations = false)]
async fn punch_is_recorded_with_server_time_and_photo_layout(db: PgPool) {
    // 2026-10-28 18:00:13 UTC = 2026-10-29 01:00:13 WIB.
    let clock = MovableClock::at(Utc.with_ymd_and_hms(2026, 10, 28, 18, 0, 13).unwrap());
    let t = setup_with(db, &[], Some(clock.clock())).await;
    let token = t.login("kontrak", "hp").await;

    let reply = t
        .send(punch(
            &token,
            &[
                ("latitude", "-6.2"),
                ("longitude", "106.8"),
                ("accuracy", "12"),
                ("punched_at", "2000-01-01T00:00:00Z"),
            ],
            true,
        ))
        .await;
    assert_eq!(reply.status, StatusCode::CREATED, "{}", reply.json);
    let data = &reply.json["data"];
    assert!(data["id"].is_i64());
    assert_eq!(data["punched_at"], "2026-10-29T01:00:13+07:00");
    assert_eq!(data["source"], "mobile");
    assert_eq!(data["status"], "pending");
    assert_eq!(data["status_label"], "Menunggu persetujuan");
    assert_eq!(data["has_photo"], true);

    let (path, lat, lng, acc): (String, f64, f64, i16) = sqlx::query_as(
        "SELECT photo_path, latitude::float8, longitude::float8, accuracy_meters FROM attendance_punches WHERE id = $1",
    )
    .bind(data["id"].as_i64().unwrap())
    .fetch_one(&t.db)
    .await
    .unwrap();
    assert!(
        path.starts_with("mobile-punches/2026/10/29/KONTRAK/"),
        "{path}"
    );
    assert!(path.ends_with(".png"));
    assert_eq!(path.rsplit('/').next().unwrap().len(), 44);
    assert_eq!(std::fs::read(t.photos.path().join(&path)).unwrap(), PNG);
    assert_eq!((lat, lng, acc), (-6.2, 106.8, 12));

    // Riwayat memakai hari lokal 29 Oktober, bukan 28 (lihat docs/laravel-differences.md).
    let reply = t.send(get("/api/punches", Some(&token))).await;
    assert_eq!(reply.json["data"].as_array().unwrap().len(), 1);
    assert_eq!(reply.json["data"][0]["status"], "pending");
    let reply = t
        .send(get("/api/punches?date=2026-10-29", Some(&token)))
        .await;
    assert_eq!(reply.json["data"].as_array().unwrap().len(), 1);
    let reply = t
        .send(get("/api/punches?date=2026-10-28", Some(&token)))
        .await;
    assert_eq!(reply.json["data"], json!([]));
    let reply = t
        .send(get("/api/punches?date=2000-01-01", Some(&token)))
        .await;
    assert_eq!(reply.json, json!({"data": []}));
}

#[sqlx::test(migrations = false)]
async fn minimum_interval_between_punches(db: PgPool) {
    let clock = MovableClock::at(Utc.with_ymd_and_hms(2026, 10, 29, 1, 0, 0).unwrap());
    let t = setup_with(db, &[], Some(clock.clock())).await;
    let token = t.login("kontrak", "hp").await;
    assert_eq!(
        t.send(punch(&token, &[], true)).await.status,
        StatusCode::CREATED
    );

    clock.advance(59);
    let reply = t.send(punch(&token, &[], true)).await;
    assert_eq!(reply.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(
        reply.json,
        json!({"message": "Tunggu 60 detik sebelum melakukan punch lagi."})
    );

    clock.advance(1);
    assert_eq!(
        t.send(punch(&token, &[], true)).await.status,
        StatusCode::CREATED
    );

    let reply = t.send(get("/api/punches", Some(&token))).await;
    let data = reply.json["data"].as_array().unwrap();
    assert_eq!(data.len(), 2);
    assert!(
        data[0]["punched_at"].as_str() < data[1]["punched_at"].as_str(),
        "terlama dulu"
    );
    // Foto dari punch yang ditolak tidak tertinggal di disk.
    assert_eq!(count_files(t.photos.path()), 2);
}

#[sqlx::test(migrations = false)]
async fn concurrent_punches_are_serialized(db: PgPool) {
    let t = setup(db).await;
    let token = t.login("kontrak", "hp").await;
    let (a, b) = tokio::join!(
        t.send(punch(&token, &[], true)),
        t.send(punch(&token, &[], true))
    );
    let mut statuses = [a.status.as_u16(), b.status.as_u16()];
    statuses.sort_unstable();
    assert_eq!(statuses, [201, 422]);
    assert_eq!(
        t.scalar_i64("SELECT count(*) FROM attendance_punches")
            .await,
        1
    );
}

#[sqlx::test(migrations = false)]
async fn rule_rejections(db: PgPool) {
    let t = setup(db).await;
    let inactive = t.login("kontrak.pegawai-nonaktif", "hp").await;
    let reply = t.send(punch(&inactive, &[], true)).await;
    assert_eq!(reply.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(reply.json, json!({"message": "Pegawai tidak aktif."}));

    let no_employee = t.token_for("kontrak.tanpa-pegawai").await;
    let reply = t.send(punch(&no_employee, &[], true)).await;
    assert_eq!(reply.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(
        reply.json,
        json!({"message": "Akun ini belum terhubung ke data pegawai."})
    );
    let reply = t.send(get("/api/punches", Some(&no_employee))).await;
    assert_eq!(reply.status, StatusCode::FORBIDDEN);
    assert_eq!(
        reply.json,
        json!({"message": "Akun ini belum terhubung ke data pegawai."})
    );
}

#[sqlx::test(migrations = false)]
async fn locked_period_rejects_punch(db: PgPool) {
    let clock = MovableClock::at(Utc.with_ymd_and_hms(2026, 10, 29, 1, 0, 0).unwrap());
    let t = setup_with(db, &[], Some(clock.clock())).await;
    let token = t.login("kontrak", "hp").await;
    t.exec("INSERT INTO periods (year, month, status) VALUES (2026, 10, 'closing')")
        .await;
    let reply = t.send(punch(&token, &[], true)).await;
    assert_eq!(reply.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(
        reply.json,
        json!({"message": "Periode bulan ini sudah ditutup."})
    );

    t.exec("UPDATE periods SET status = 'open'").await;
    assert_eq!(
        t.send(punch(&token, &[], true)).await.status,
        StatusCode::CREATED
    );
}

#[sqlx::test(migrations = false)]
async fn configuration_flags(db: PgPool) {
    let t = setup_with(
        db,
        &[
            ("ATTENDANCE_MOBILE_REQUIRES_APPROVAL", "false"),
            ("ATTENDANCE_MOBILE_REQUIRES_PHOTO", "false"),
        ],
        None,
    )
    .await;
    let token = t.login("kontrak", "hp").await;
    let me = t.send(get("/api/me", Some(&token))).await;
    assert_eq!(me.json["data"]["requires_photo"], false);
    assert_eq!(me.json["data"]["requires_approval"], false);

    let reply = t.send(punch(&token, &[], false)).await;
    assert_eq!(reply.status, StatusCode::CREATED);
    assert_eq!(reply.json["data"]["status"], "approved");
    assert_eq!(reply.json["data"]["status_label"], "Disetujui");
    assert_eq!(reply.json["data"]["has_photo"], false);
}

#[sqlx::test(migrations = false)]
async fn action_rate_limit_per_user(db: PgPool) {
    let t = setup_with(db, &[("RATE_LIMIT_ACTIONS_PER_USER", "2")], None).await;
    let token = t.login("kontrak", "hp").await;
    for _ in 0..2 {
        assert_eq!(
            t.send(punch(&token, &[], false)).await.status,
            StatusCode::UNPROCESSABLE_ENTITY
        );
    }
    let reply = t.send(punch(&token, &[], true)).await;
    assert_eq!(reply.status, StatusCode::TOO_MANY_REQUESTS);
    // Riwayat (GET) tidak terkena batas aksi.
    assert_eq!(
        t.send(get("/api/punches", Some(&token))).await.status,
        StatusCode::OK
    );
}

#[sqlx::test(migrations = false)]
async fn oversized_body_is_rejected_as_json(db: PgPool) {
    let t = setup(db).await;
    let token = t.login("kontrak", "hp").await;
    let big = vec![0u8; 6 * 1024 * 1024];
    let req = post_multipart(
        "/api/punches",
        Some(&token),
        &[],
        Some(("photo", &big, "a.png", "image/png")),
    );
    let reply = t.send(req).await;
    assert_eq!(reply.status, StatusCode::PAYLOAD_TOO_LARGE);
    assert!(reply.json["message"].is_string());

    // Di bawah batas body tetapi di atas 4 MB: galat validasi foto.
    let mut png = PNG.to_vec();
    png.resize(4 * 1024 * 1024 + 10, 0);
    let req = post_multipart(
        "/api/punches",
        Some(&token),
        &[],
        Some(("photo", &png, "a.png", "image/png")),
    );
    let reply = t.send(req).await;
    assert_eq!(reply.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(reply.json["errors"]["photo"].is_array());
}
