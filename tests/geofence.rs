//! Aturan lokasi kerja, kasus yang sama dengan `WorkLocationPolicyTest` di eabsensi-v2.

mod common;

use axum::http::StatusCode;
use common::*;
use eabsensi_mobile_api::geofence::{
    MESSAGE_LOCATION_REQUIRED, MESSAGE_MOCKED, MESSAGE_STALE, MESSAGE_UPDATE_APP,
    MESSAGE_WEAK_SIGNAL,
};
use serde_json::json;
use sqlx::PgPool;

const TRUSTED: &[(&str, &str)] = &[
    ("latitude", "-6.2"),
    ("longitude", "106.8"),
    ("accuracy", "10"),
    ("is_mocked", "0"),
    ("location_age_seconds", "5"),
    ("device_rooted", "0"),
];

/// Bukti sah di titik kantor, dengan sebagian field diganti.
fn evidence<'a>(overrides: &[(&'a str, &'a str)]) -> Vec<(&'a str, &'a str)> {
    TRUSTED
        .iter()
        .map(|(key, value)| {
            overrides
                .iter()
                .find(|(k, _)| k == key)
                .copied()
                .unwrap_or((*key, *value))
        })
        .collect()
}

fn punch(token: &str, fields: &[(&str, &str)]) -> axum::http::Request<axum::body::Body> {
    post_multipart(
        "/api/punches",
        Some(token),
        fields,
        Some(("photo", PNG, "photo.png", "image/png")),
    )
}

fn check_in(
    token: &str,
    event: i64,
    fields: &[(&str, &str)],
) -> axum::http::Request<axum::body::Body> {
    post_multipart(
        &format!("/api/events/{event}/check-in"),
        Some(token),
        fields,
        Some(("photo", PNG, "photo.png", "image/png")),
    )
}

async fn apel_id(t: &TestApp) -> i64 {
    t.scalar_i64("SELECT id FROM attendance_events WHERE name = 'Apel Kontrak'")
        .await
}

#[sqlx::test(migrations = false)]
async fn profile_exposes_the_applicable_location(db: PgPool) {
    let t = setup(db).await;

    let token = t.login("kontrak.lokasi", "hp").await;
    let me = t.send(get("/api/me", Some(&token))).await;
    assert_eq!(
        me.json["data"]["work_location"],
        json!({"name": "Lokasi Kontrak", "latitude": -6.2, "longitude": 106.8, "radius_meters": 100})
    );

    let token = t.login("kontrak", "hp").await;
    let me = t.send(get("/api/me", Some(&token))).await;
    assert!(me.json["data"]["work_location"].is_null());
}

#[sqlx::test(migrations = false)]
async fn untrustworthy_evidence_is_rejected_in_order(db: PgPool) {
    let t = setup(db).await;
    let token = t.login("kontrak.lokasi", "hp").await;

    let cases: Vec<(Vec<(&str, &str)>, &str)> = vec![
        (vec![("is_mocked", "1")], MESSAGE_LOCATION_REQUIRED),
        (
            vec![
                ("latitude", "-6.2"),
                ("longitude", "106.8"),
                ("accuracy", "10"),
            ],
            MESSAGE_UPDATE_APP,
        ),
        (
            evidence(&[
                ("is_mocked", "1"),
                ("location_age_seconds", "999"),
                ("accuracy", "999"),
            ]),
            MESSAGE_MOCKED,
        ),
        (
            evidence(&[("location_age_seconds", "121"), ("accuracy", "999")]),
            MESSAGE_STALE,
        ),
        (evidence(&[("accuracy", "101")]), MESSAGE_WEAK_SIGNAL),
    ];

    for (fields, message) in cases {
        let reply = t.send(punch(&token, &fields)).await;
        assert_eq!(reply.status, StatusCode::UNPROCESSABLE_ENTITY, "{message}");
        assert_eq!(reply.json, json!({ "message": message }));
    }
    assert_eq!(
        t.scalar_i64("SELECT count(*) FROM attendance_punches")
            .await,
        0
    );
}

#[sqlx::test(migrations = false)]
async fn punch_inside_the_radius_records_a_clean_check(db: PgPool) {
    let t = setup(db).await;
    let token = t.login("kontrak.lokasi", "hp").await;

    let reply = t.send(punch(&token, TRUSTED)).await;
    assert_eq!(reply.status, StatusCode::CREATED, "{}", reply.json);
    assert_eq!(reply.json["data"]["location_flags"], json!([]));

    assert_eq!(
        t.scalar_i64(
            "SELECT count(*) FROM attendance_location_checks
             WHERE location_name = 'Lokasi Kontrak' AND radius_meters = 100 AND distance_meters = 0
               AND NOT is_flagged AND attendance_punch_id IS NOT NULL"
        )
        .await,
        1
    );
}

#[sqlx::test(migrations = false)]
async fn punch_outside_the_radius_is_accepted_and_flagged(db: PgPool) {
    let t = setup(db).await;
    let token = t.login("kontrak.lokasi", "hp").await;

    let fields = evidence(&[("latitude", "-6.21"), ("device_rooted", "1")]);
    let reply = t.send(punch(&token, &fields)).await;
    assert_eq!(reply.status, StatusCode::CREATED, "{}", reply.json);
    assert_eq!(reply.json["data"]["status"], "pending");
    assert_eq!(
        reply.json["data"]["location_flags"],
        json!(["outside_radius", "rooted_device"])
    );

    let list = t.send(get("/api/punches", Some(&token))).await;
    assert_eq!(
        list.json["data"][0]["location_flags"],
        json!(["outside_radius", "rooted_device"])
    );
    assert_eq!(
        t.scalar_i64(
            "SELECT distance_meters::bigint FROM attendance_location_checks WHERE is_flagged"
        )
        .await,
        1112
    );
}

#[sqlx::test(migrations = false)]
async fn impossible_travel_since_the_last_mobile_punch_is_flagged(db: PgPool) {
    let t = setup_with(db, &[("ATTENDANCE_MOBILE_MIN_INTERVAL", "0")], None).await;
    let token = t.login("kontrak.lokasi", "hp").await;
    t.exec(
        "INSERT INTO attendance_punches (employee_id, punched_at, source, status, latitude, longitude, created_at, updated_at)
         SELECT id, now() - interval '5 minutes', 'mobile', 'approved', -6.9, 107.6, now(), now()
         FROM employees WHERE employee_number = 'KTR-003'",
    )
    .await;

    let reply = t.send(punch(&token, TRUSTED)).await;
    assert_eq!(reply.status, StatusCode::CREATED, "{}", reply.json);
    assert_eq!(
        reply.json["data"]["location_flags"],
        json!(["impossible_travel"])
    );
}

#[sqlx::test(migrations = false)]
async fn department_location_is_inherited_from_the_nearest_parent(db: PgPool) {
    let t = setup(db).await;
    t.exec(
        "INSERT INTO work_locations (name, latitude, longitude, radius_meters, created_at, updated_at)
         VALUES ('Nonaktif', 1, 1, 100, now(), now()), ('Induk', 2, 2, 200, now(), now());
         UPDATE work_locations SET is_active = false WHERE name = 'Nonaktif';
         INSERT INTO departments (code, name, created_at, updated_at, work_location_id)
         SELECT 'GEO-INDUK', 'Induk', now(), now(), id FROM work_locations WHERE name = 'Induk';
         UPDATE departments SET parent_id = (SELECT id FROM departments WHERE code = 'GEO-INDUK'),
                                work_location_id = (SELECT id FROM work_locations WHERE name = 'Nonaktif')
         WHERE code = 'KONTRAK';",
    )
    .await;

    let token = t.login("kontrak", "hp").await;
    let me = t.send(get("/api/me", Some(&token))).await;
    assert_eq!(me.json["data"]["work_location"]["name"], "Induk");
}

#[sqlx::test(migrations = false)]
async fn exempt_employees_are_never_rejected_but_still_flagged(db: PgPool) {
    let t = setup(db).await;
    t.exec("UPDATE employees SET location_exempt = true WHERE employee_number = 'KTR-003'")
        .await;
    let token = t.login("kontrak.lokasi", "hp").await;

    let me = t.send(get("/api/me", Some(&token))).await;
    assert!(me.json["data"]["work_location"].is_null());

    let fields = [("is_mocked", "1"), ("device_rooted", "1")];
    let reply = t.send(punch(&token, &fields)).await;
    assert_eq!(reply.status, StatusCode::CREATED, "{}", reply.json);
    assert_eq!(
        reply.json["data"]["location_flags"],
        json!(["mock_location", "rooted_device"])
    );
}

#[sqlx::test(migrations = false)]
async fn employees_without_a_location_keep_the_old_behaviour(db: PgPool) {
    let t = setup(db).await;
    let token = t.login("kontrak", "hp").await;

    let reply = t.send(punch(&token, &[])).await;
    assert_eq!(reply.status, StatusCode::CREATED, "{}", reply.json);
    assert_eq!(reply.json["data"]["location_flags"], json!([]));
    assert_eq!(
        t.scalar_i64("SELECT count(*) FROM attendance_location_checks")
            .await,
        0
    );
}

#[sqlx::test(migrations = false)]
async fn events_without_a_location_rule_are_not_limited(db: PgPool) {
    let t = setup(db).await;
    let token = t.login("kontrak.lokasi", "hp").await;
    let event = apel_id(&t).await;

    let reply = t.send(check_in(&token, event, &[])).await;
    assert_eq!(reply.status, StatusCode::CREATED, "{}", reply.json);
    assert_eq!(reply.json["data"]["location_flags"], json!([]));
}

#[sqlx::test(migrations = false)]
async fn events_can_follow_the_employee_location(db: PgPool) {
    let t = setup(db).await;
    t.exec("UPDATE attendance_events SET location_mode = 'employee' WHERE name = 'Apel Kontrak'")
        .await;
    let token = t.login("kontrak.lokasi", "hp").await;
    let event = apel_id(&t).await;

    let reply = t.send(check_in(&token, event, &[])).await;
    assert_eq!(reply.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(reply.json, json!({ "message": MESSAGE_LOCATION_REQUIRED }));

    let fields = evidence(&[("latitude", "-6.21")]);
    let reply = t.send(check_in(&token, event, &fields)).await;
    assert_eq!(reply.status, StatusCode::CREATED, "{}", reply.json);
    assert_eq!(
        reply.json["data"]["location_flags"],
        json!(["outside_radius"])
    );
    assert_eq!(
        t.scalar_i64(
            "SELECT count(*) FROM attendance_location_checks c
             JOIN event_attendances a ON a.id = c.event_attendance_id"
        )
        .await,
        1
    );
}

#[sqlx::test(migrations = false)]
async fn custom_event_location_replaces_the_employee_location(db: PgPool) {
    let t = setup(db).await;
    t.exec(
        "INSERT INTO work_locations (name, latitude, longitude, radius_meters, created_at, updated_at)
         VALUES ('Lapangan', -6.3, 106.9, 100, now(), now());
         UPDATE attendance_events
         SET location_mode = 'custom', work_location_id = (SELECT id FROM work_locations WHERE name = 'Lapangan')
         WHERE name = 'Apel Kontrak';",
    )
    .await;
    let token = t.login("kontrak.lokasi", "hp").await;
    let event = apel_id(&t).await;

    let list = t.send(get("/api/events", Some(&token))).await;
    let apel = list.json["data"]
        .as_array()
        .and_then(|events| events.iter().find(|e| e["name"] == "Apel Kontrak"))
        .expect("Apel Kontrak terlihat");
    assert_eq!(apel["work_location"]["name"], "Lapangan");

    let fields = evidence(&[("latitude", "-6.3"), ("longitude", "106.9")]);
    let reply = t.send(check_in(&token, event, &fields)).await;
    assert_eq!(reply.status, StatusCode::CREATED, "{}", reply.json);
    assert_eq!(reply.json["data"]["location_flags"], json!([]));
}

#[sqlx::test(migrations = false)]
async fn exempt_employees_are_not_limited_on_custom_events(db: PgPool) {
    let t = setup(db).await;
    t.exec(
        "UPDATE employees SET location_exempt = true WHERE employee_number = 'KTR-003';
         UPDATE attendance_events
         SET location_mode = 'custom', work_location_id = (SELECT id FROM work_locations WHERE name = 'Lokasi Kontrak')
         WHERE name = 'Apel Kontrak';",
    )
    .await;
    let token = t.login("kontrak.lokasi", "hp").await;
    let event = apel_id(&t).await;

    let reply = t.send(check_in(&token, event, &[])).await;
    assert_eq!(reply.status, StatusCode::CREATED, "{}", reply.json);
}
