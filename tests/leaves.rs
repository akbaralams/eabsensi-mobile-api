mod common;

use axum::http::StatusCode;
use common::*;
use serde_json::{Value, json};
use sqlx::PgPool;

fn leave(token: &str, body: Value) -> axum::http::Request<axum::body::Body> {
    post_json("/api/leaves", Some(token), body)
}

async fn first_type(t: &TestApp, token: &str) -> i64 {
    let reply = t.send(get("/api/leaves", Some(token))).await;
    reply.json["types"][0]["id"].as_i64().expect("jenis cuti")
}

#[sqlx::test(migrations = false)]
async fn list_types_and_request_flow(db: PgPool) {
    let t = setup(db).await;
    let token = t.login("kontrak", "hp").await;

    let reply = t.send(get("/api/leaves", Some(&token))).await;
    assert_eq!(reply.status, StatusCode::OK);
    assert_eq!(reply.json["data"], json!([]));
    let names: Vec<&str> = reply.json["types"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["name"].as_str().unwrap())
        .collect();
    assert_eq!(names, ["Cuti", "Dinas luar", "Izin", "Sakit"], "urut nama");
    assert!(reply.json["types"][0]["code"].is_string());
    let type_id = reply.json["types"][0]["id"].as_i64().unwrap();

    let reply = t
        .send(leave(&token, json!({"attendance_status_id": type_id, "start_date": "2099-01-05", "end_date": "2099-01-07", "reason": "  Uji kontrak  "})))
        .await;
    assert_eq!(reply.status, StatusCode::CREATED, "{}", reply.json);
    assert_eq!(
        reply.json["data"],
        json!({
            "id": reply.json["data"]["id"], "type": "Cuti", "attendance_status_id": type_id,
            "start_date": "2099-01-05", "end_date": "2099-01-07", "reason": "Uji kontrak",
            "status": "pending", "status_label": "Menunggu"
        })
    );
    let leave_id = reply.json["data"]["id"].as_i64().unwrap();

    // Audit dicatat seperti AuditLogger Laravel.
    let audit: (Option<i64>, Option<String>, String, String, Value, Option<String>) = sqlx::query_as(
        "SELECT user_id, user_name, action, description, context::jsonb, ip_address FROM audit_logs",
    )
    .fetch_one(&t.db)
    .await
    .unwrap();
    let employee_id = t
        .scalar_i64("SELECT id FROM employees WHERE employee_number = 'KTR-001'")
        .await;
    assert_eq!(audit.1.as_deref(), Some("Akun kontrak"));
    assert_eq!(audit.2, "leave.requested");
    assert_eq!(
        audit.3,
        format!("Pengajuan cuti pegawai #{employee_id}, 2099-01-05 s/d 2099-01-07")
    );
    assert_eq!(
        audit.4,
        json!({"leave_id": leave_id, "employee_id": employee_id, "attendance_status_id": type_id, "from": "2099-01-05", "to": "2099-01-07"})
    );
    assert_eq!(audit.5.as_deref(), Some("127.0.0.1"));
    let context_text: String = sqlx::query_scalar("SELECT context::text FROM audit_logs")
        .fetch_one(&t.db)
        .await
        .unwrap();
    assert!(
        context_text.starts_with("{\"leave_id\""),
        "urutan kunci seperti Laravel: {context_text}"
    );

    let reply = t
        .send(leave(&token, json!({"attendance_status_id": type_id, "start_date": "2099-01-07", "end_date": "2099-01-09"})))
        .await;
    assert_eq!(reply.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(
        reply.json,
        json!({"message": "Pegawai sudah punya pengajuan yang menunggu atau disetujui pada tanggal tersebut."})
    );

    // Pengajuan yang ditolak/dibatalkan tidak menghalangi.
    t.exec("UPDATE leave_requests SET status = 'rejected'")
        .await;
    let reply = t
        .send(leave(&token, json!({"attendance_status_id": type_id, "start_date": "2099-01-07", "end_date": "2099-01-09"})))
        .await;
    assert_eq!(reply.status, StatusCode::CREATED);

    let reply = t.send(get("/api/leaves", Some(&token))).await;
    let data = reply.json["data"].as_array().unwrap();
    assert_eq!(data.len(), 2);
    assert_eq!(
        data[0]["start_date"], "2099-01-07",
        "tanggal mulai terbaru dulu"
    );
    assert_eq!(data[1]["status_label"], "Ditolak");
}

#[sqlx::test(migrations = false)]
async fn validation_and_range_rules(db: PgPool) {
    let t = setup(db).await;
    let token = t.login("kontrak", "hp").await;
    let type_id = first_type(&t, &token).await;

    let reply = t.send(leave(&token, json!({}))).await;
    assert_eq!(reply.status, StatusCode::UNPROCESSABLE_ENTITY);
    for field in ["attendance_status_id", "start_date", "end_date"] {
        assert!(reply.json["errors"][field].is_array(), "{field}");
    }

    let reply = t
        .send(leave(&token, json!({"attendance_status_id": type_id, "start_date": "2099-01-10", "end_date": "2099-01-05"})))
        .await;
    assert_eq!(reply.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(reply.json["errors"]["end_date"].is_array());
    assert!(reply.json["errors"].get("start_date").is_none());

    for body in [
        json!({"attendance_status_id": "abc", "start_date": "2099-01-01", "end_date": "2099-01-01"}),
        json!({"attendance_status_id": 999999, "start_date": "2099-01-01", "end_date": "2099-01-01"}),
    ] {
        let reply = t.send(leave(&token, body)).await;
        assert!(reply.json["errors"]["attendance_status_id"].is_array());
    }
    let reply = t
        .send(leave(&token, json!({"attendance_status_id": type_id, "start_date": "2099-1-1", "end_date": "2099-02-30", "reason": "x".repeat(501)})))
        .await;
    for field in ["start_date", "end_date", "reason"] {
        assert!(reply.json["errors"][field].is_array(), "{field}");
    }

    // Nonaktif tidak bisa dipilih.
    t.exec("UPDATE attendance_statuses SET is_active = false WHERE code = 'SAKIT'")
        .await;
    let sakit = t
        .scalar_i64("SELECT id FROM attendance_statuses WHERE code = 'SAKIT'")
        .await;
    let reply = t
        .send(leave(&token, json!({"attendance_status_id": sakit, "start_date": "2099-01-01", "end_date": "2099-01-01"})))
        .await;
    assert!(reply.json["errors"]["attendance_status_id"].is_array());

    // 62 hari: selisih 61 hari (62 hari kalender) boleh, selisih 62 ditolak.
    let reply = t
        .send(leave(&token, json!({"attendance_status_id": type_id, "start_date": "2099-01-01", "end_date": "2099-03-04"})))
        .await;
    assert_eq!(
        reply.json,
        json!({"message": "Rentang cuti maksimal 62 hari."})
    );
    let reply = t
        .send(leave(&token, json!({"attendance_status_id": type_id, "start_date": "2099-01-01", "end_date": "2099-03-03"})))
        .await;
    assert_eq!(reply.status, StatusCode::CREATED, "{}", reply.json);
    // Id sebagai string bilangan bulat diterima (aturan integer Laravel).
    let reply = t
        .send(leave(&token, json!({"attendance_status_id": type_id.to_string(), "start_date": "2099-05-01", "end_date": "2099-05-01"})))
        .await;
    assert_eq!(reply.status, StatusCode::CREATED, "{}", reply.json);
    assert_eq!(reply.json["data"]["reason"], Value::Null);
}

#[sqlx::test(migrations = false)]
async fn account_without_employee(db: PgPool) {
    let t = setup(db).await;
    let token = t.token_for("kontrak.tanpa-pegawai").await;
    let reply = t.send(get("/api/leaves", Some(&token))).await;
    assert_eq!(reply.status, StatusCode::FORBIDDEN);
    // Validasi tetap lebih dulu.
    let reply = t.send(leave(&token, json!({}))).await;
    assert_eq!(reply.status, StatusCode::UNPROCESSABLE_ENTITY);
    let type_id = t
        .scalar_i64("SELECT id FROM attendance_statuses ORDER BY name LIMIT 1")
        .await;
    let reply = t
        .send(leave(&token, json!({"attendance_status_id": type_id, "start_date": "2099-01-01", "end_date": "2099-01-01"})))
        .await;
    assert_eq!(reply.status, StatusCode::FORBIDDEN);
    assert_eq!(
        reply.json,
        json!({"message": "Akun ini belum terhubung ke data pegawai."})
    );
}

#[sqlx::test(migrations = false)]
async fn simultaneous_overlapping_requests(db: PgPool) {
    let t = setup(db).await;
    let token = t.login("kontrak", "hp").await;
    let type_id = first_type(&t, &token).await;
    let body = json!({"attendance_status_id": type_id, "start_date": "2099-06-01", "end_date": "2099-06-03"});
    let (a, b) = tokio::join!(
        t.send(leave(&token, body.clone())),
        t.send(leave(&token, body))
    );
    let mut statuses = [a.status.as_u16(), b.status.as_u16()];
    statuses.sort_unstable();
    assert_eq!(statuses, [201, 422]);
    assert_eq!(t.scalar_i64("SELECT count(*) FROM leave_requests").await, 1);
    assert_eq!(t.scalar_i64("SELECT count(*) FROM audit_logs").await, 1);
}
