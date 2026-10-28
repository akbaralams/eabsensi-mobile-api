mod common;

use axum::http::StatusCode;
use chrono::{TimeZone, Utc};
use common::*;
use serde_json::{Value, json};
use sqlx::PgPool;

fn check_in(token: &str, id: &str, photo: bool) -> axum::http::Request<axum::body::Body> {
    post_multipart(
        &format!("/api/events/{id}/check-in"),
        Some(token),
        &[("latitude", "-6.2"), ("longitude", "106.8")],
        photo.then_some(("photo", PNG, "photo.png", "image/png")),
    )
}

fn event_named<'a>(list: &'a Value, name: &str) -> Option<&'a Value> {
    list["data"]
        .as_array()
        .and_then(|events| events.iter().find(|e| e["name"] == name))
}

#[sqlx::test(migrations = false)]
async fn list_and_check_in_flow(db: PgPool) {
    let t = setup(db).await;
    let token = t.login("kontrak", "hp").await;

    let list = t.send(get("/api/events", Some(&token))).await;
    assert_eq!(list.status, StatusCode::OK);
    let apel = event_named(&list.json, "Apel Kontrak").expect("Apel Kontrak terlihat");
    assert!(
        event_named(&list.json, "Apel Kontrak Department Lain").is_none(),
        "event department lain tidak terlihat"
    );
    assert_eq!(apel["starts_at"], "00:00");
    assert_eq!(apel["ends_at"], "23:59");
    assert_eq!(apel["checked_in"], false);
    let date = apel["event_date"].as_str().unwrap();
    assert_eq!(date.len(), 10);
    let id = apel["id"].as_i64().unwrap().to_string();

    let reply = t.send(check_in(&token, &id, false)).await;
    assert_eq!(reply.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(reply.json["errors"]["photo"].is_array());

    let reply = t.send(check_in(&token, &id, true)).await;
    assert_eq!(reply.status, StatusCode::CREATED, "{}", reply.json);
    assert_eq!(reply.json["data"]["event_id"].to_string(), id);
    let at = reply.json["data"]["checked_in_at"].as_str().unwrap();
    assert!(at.ends_with("+07:00"), "{at}");

    let reply = t.send(check_in(&token, &id, true)).await;
    assert_eq!(reply.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(
        reply.json,
        json!({"message": "Anda sudah check-in di event ini."})
    );

    let list = t.send(get("/api/events", Some(&token))).await;
    assert_eq!(
        event_named(&list.json, "Apel Kontrak").unwrap()["checked_in"],
        true
    );

    // Foto check-in: event-attendance/<tanggal UTC>/KONTRAK/..
    let path: String = sqlx::query_scalar("SELECT photo_path FROM event_attendances")
        .fetch_one(&t.db)
        .await
        .unwrap();
    assert!(path.starts_with("event-attendance/"), "{path}");
    assert!(path.contains("/KONTRAK/"), "{path}");
    assert!(t.photos.path().join(&path).exists());
}

#[sqlx::test(migrations = false)]
async fn missing_and_malformed_events(db: PgPool) {
    let t = setup(db).await;
    let token = t.login("kontrak", "hp").await;
    let reply = t.send(check_in(&token, "999999999", true)).await;
    assert_eq!(reply.status, StatusCode::NOT_FOUND);
    assert_eq!(
        reply.json,
        json!({"message": "No query results for model [App\\Models\\AttendanceEvent] 999999999"})
    );
    // Validasi form lebih dulu daripada pencarian event.
    let reply = t.send(check_in(&token, "999999999", false)).await;
    assert_eq!(reply.status, StatusCode::UNPROCESSABLE_ENTITY);
    // Id bukan angka: rute tidak ada, bahkan tanpa token.
    let req = post_multipart("/api/events/abc/check-in", None, &[], None);
    let reply = t.send(req).await;
    assert_eq!(reply.status, StatusCode::NOT_FOUND);
    assert_eq!(
        reply.json,
        json!({"message": "The route api/events/abc/check-in could not be found."})
    );
}

#[sqlx::test(migrations = false)]
async fn parent_department_events_and_eligibility(db: PgPool) {
    let t = setup(db).await;
    // INDUK -> KONTRAK (department pegawai) ; event untuk INDUK terlihat, event untuk anak KONTRAK tidak.
    t.exec(
        "INSERT INTO departments (code, name, is_active) VALUES ('INDUK', 'Induk', true), ('ANAK', 'Anak', true);
         UPDATE departments SET parent_id = (SELECT id FROM departments WHERE code = 'INDUK') WHERE code = 'KONTRAK';
         UPDATE departments SET parent_id = (SELECT id FROM departments WHERE code = 'KONTRAK') WHERE code = 'ANAK';
         INSERT INTO attendance_events (name, event_date, starts_at, ends_at, department_id, is_active)
         SELECT 'Apel Induk', (now() AT TIME ZONE 'Asia/Jakarta')::date, '00:00', '23:59', id, true FROM departments WHERE code = 'INDUK';
         INSERT INTO attendance_events (name, event_date, starts_at, ends_at, department_id, is_active)
         SELECT 'Apel Anak', (now() AT TIME ZONE 'Asia/Jakarta')::date, '00:00', '23:59', id, true FROM departments WHERE code = 'ANAK';
         INSERT INTO attendance_events (name, event_date, starts_at, ends_at, department_id, is_active)
         VALUES ('Apel Nonaktif', (now() AT TIME ZONE 'Asia/Jakarta')::date, '00:00', '23:59', NULL, false);",
    )
    .await;
    let token = t.login("kontrak", "hp").await;
    let list = t.send(get("/api/events", Some(&token))).await;
    assert!(event_named(&list.json, "Apel Induk").is_some());
    assert!(event_named(&list.json, "Apel Anak").is_none());
    assert!(event_named(&list.json, "Apel Nonaktif").is_none());

    let id_of = |name: &str| format!("SELECT id FROM attendance_events WHERE name = '{name}'");
    // Event department induk: pegawai di department turunan termasuk peserta.
    let induk = t.scalar_i64(&id_of("Apel Induk")).await.to_string();
    assert_eq!(
        t.send(check_in(&token, &induk, true)).await.status,
        StatusCode::CREATED
    );
    // Event department anak: pegawai di induknya bukan peserta.
    let anak = t.scalar_i64(&id_of("Apel Anak")).await.to_string();
    let reply = t.send(check_in(&token, &anak, true)).await;
    assert_eq!(
        reply.json,
        json!({"message": "Anda tidak termasuk peserta event ini."})
    );
    let nonaktif = t.scalar_i64(&id_of("Apel Nonaktif")).await.to_string();
    let reply = t.send(check_in(&token, &nonaktif, true)).await;
    assert_eq!(reply.json, json!({"message": "Event ini tidak aktif."}));
    // Tidak ada foto yatim dari check-in yang ditolak.
    let photos: i64 = t.scalar_i64("SELECT count(*) FROM event_attendances").await;
    assert_eq!(photos, 1);
}

#[sqlx::test(migrations = false)]
async fn date_and_time_window_in_business_zone(db: PgPool) {
    // 2026-10-29 00:30 UTC = 07:30 WIB.
    let clock = MovableClock::at(Utc.with_ymd_and_hms(2026, 10, 29, 0, 30, 0).unwrap());
    let t = setup_with(db, &[], Some(clock.clock())).await;
    t.exec(
        "INSERT INTO attendance_events (name, event_date, starts_at, ends_at, department_id, is_active) VALUES
           ('Upacara', '2026-10-29', '07:00', '08:00', NULL, true),
           ('Besok', '2026-10-30', '07:00', '08:00', NULL, true),
           ('Siang', '2026-10-29', '13:00', '14:00', NULL, true)",
    )
    .await;
    let token = t.login("kontrak", "hp").await;
    let id_of = |name: &str| format!("SELECT id FROM attendance_events WHERE name = '{name}'");

    let besok = t.scalar_i64(&id_of("Besok")).await.to_string();
    let reply = t.send(check_in(&token, &besok, true)).await;
    assert_eq!(
        reply.json,
        json!({"message": "Check-in hanya bisa dilakukan pada tanggal event."})
    );
    let siang = t.scalar_i64(&id_of("Siang")).await.to_string();
    let reply = t.send(check_in(&token, &siang, true)).await;
    assert_eq!(
        reply.json,
        json!({"message": "Check-in dibuka pukul 13:00 sampai 14:00."})
    );

    // 08:00:59 WIB masih dalam jendela "sampai 08:00" (dibandingkan per menit).
    clock.advance(30 * 60 + 59);
    let upacara = t.scalar_i64(&id_of("Upacara")).await.to_string();
    let reply = t.send(check_in(&token, &upacara, true)).await;
    assert_eq!(reply.status, StatusCode::CREATED, "{}", reply.json);
    assert_eq!(
        reply.json["data"]["checked_in_at"],
        "2026-10-29T08:00:59+07:00"
    );

    // Daftar hari ini memakai tanggal lokal dan urut jam mulai.
    let list = t.send(get("/api/events", Some(&token))).await;
    let names: Vec<&str> = list.json["data"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|e| e["name"].as_str())
        .filter(|n| ["Upacara", "Siang", "Besok"].contains(n))
        .collect();
    assert_eq!(names, ["Upacara", "Siang"]);
}

#[sqlx::test(migrations = false)]
async fn simultaneous_check_ins_create_one_row(db: PgPool) {
    let t = setup(db).await;
    let token = t.login("kontrak", "hp").await;
    let id = t
        .scalar_i64("SELECT id FROM attendance_events WHERE name = 'Apel Kontrak'")
        .await
        .to_string();
    let (a, b, c) = tokio::join!(
        t.send(check_in(&token, &id, true)),
        t.send(check_in(&token, &id, true)),
        t.send(check_in(&token, &id, true))
    );
    let created = [a.status, b.status, c.status]
        .iter()
        .filter(|s| **s == StatusCode::CREATED)
        .count();
    assert_eq!(created, 1);
    assert_eq!(
        t.scalar_i64("SELECT count(*) FROM event_attendances").await,
        1
    );
}

#[sqlx::test(migrations = false)]
async fn employee_rules(db: PgPool) {
    let t = setup(db).await;
    let id = t
        .scalar_i64("SELECT id FROM attendance_events WHERE name = 'Apel Kontrak'")
        .await
        .to_string();
    let inactive = t.login("kontrak.pegawai-nonaktif", "hp").await;
    let reply = t.send(check_in(&inactive, &id, true)).await;
    assert_eq!(reply.json, json!({"message": "Pegawai tidak aktif."}));

    let no_employee = t.token_for("kontrak.tanpa-pegawai").await;
    let reply = t.send(get("/api/events", Some(&no_employee))).await;
    assert_eq!(reply.status, StatusCode::FORBIDDEN);
    let reply = t.send(check_in(&no_employee, &id, true)).await;
    assert_eq!(reply.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(
        reply.json,
        json!({"message": "Akun ini belum terhubung ke data pegawai."})
    );
}
