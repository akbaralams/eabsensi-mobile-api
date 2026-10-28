mod common;

use axum::http::StatusCode;
use common::*;
use eabsensi_mobile_api::token;
use serde_json::json;
use sqlx::PgPool;

#[sqlx::test(migrations = false)]
async fn login_failures_use_contract_messages(db: PgPool) {
    let t = setup(db).await;
    for (login, password, status, message) in [
        (
            "kontrak",
            "salah-sekali",
            422,
            "Username atau kata sandi salah.",
        ),
        (
            "tidak-ada",
            PASSWORD,
            422,
            "Username atau kata sandi salah.",
        ),
        (
            "kontrak.akun-nonaktif",
            PASSWORD,
            422,
            "Username atau kata sandi salah.",
        ),
        (
            "kontrak.tanpa-pegawai",
            PASSWORD,
            403,
            "Akun ini belum terhubung ke data pegawai.",
        ),
    ] {
        let reply = t
            .send(post_json(
                "/api/auth/login",
                None,
                json!({"login": login, "password": password, "device_name": "uji"}),
            ))
            .await;
        assert_eq!(reply.status.as_u16(), status, "{login}");
        assert_eq!(reply.json, json!({ "message": message }), "{login}");
    }
}

#[sqlx::test(migrations = false)]
async fn login_validation_errors(db: PgPool) {
    let t = setup(db).await;
    let reply = t.send(post_json("/api/auth/login", None, json!({}))).await;
    assert_eq!(reply.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(reply.json["message"].is_string());
    for field in ["login", "password", "device_name"] {
        assert!(reply.json["errors"][field].is_array(), "{field}");
    }
}

#[sqlx::test(migrations = false)]
async fn login_by_email_returns_token_and_profile(db: PgPool) {
    let t = setup(db).await;
    let reply = t
        .send(post_json(
            "/api/auth/login",
            None,
            json!({"login": "kontrak@kontrak.test", "password": PASSWORD, "device_name": "uji"}),
        ))
        .await;
    assert_eq!(reply.status, StatusCode::OK);
    let token = reply.json["token"].as_str().unwrap();
    let (id, secret) = token.split_once('|').unwrap();
    assert!(id.parse::<i64>().is_ok());
    assert_eq!(secret.len(), 48);
    assert!(secret.chars().all(|c| c.is_ascii_alphanumeric()));

    let user = &reply.json["user"];
    assert_eq!(user["username"], "kontrak");
    assert_eq!(user["employee"]["employee_number"], "KTR-001");
    assert_eq!(user["employee"]["name"], "Pegawai Kontrak");
    assert_eq!(user["employee"]["department"], "Department Kontrak");
    assert_eq!(user["timezone"], "Asia/Jakarta");
    assert_eq!(user["requires_photo"], true);
    assert_eq!(user["requires_approval"], true);

    // Disimpan persis seperti Sanctum.
    let row: (String, String, Option<String>, Option<chrono::NaiveDateTime>) = sqlx::query_as(
        "SELECT tokenable_type, token, abilities, expires_at FROM personal_access_tokens WHERE id = $1",
    )
    .bind(id.parse::<i64>().unwrap())
    .fetch_one(&t.db)
    .await
    .unwrap();
    assert_eq!(row.0, r"App\Models\User");
    assert_eq!(row.1, token::sha256_hex(secret));
    assert_eq!(row.2.as_deref(), Some(r#"["mobile"]"#));
    assert_eq!(row.3, None);
}

#[sqlx::test(migrations = false)]
async fn relogin_same_device_replaces_token_and_logout_revokes(db: PgPool) {
    let t = setup(db).await;
    let first = t.login("kontrak", "hp").await;
    let other_device = t.login("kontrak", "tablet").await;
    let second = t.login("kontrak", "hp").await;

    let reply = t.send(get("/api/me", Some(&first))).await;
    assert_eq!(reply.status, StatusCode::UNAUTHORIZED);
    assert_eq!(reply.json, json!({"message": "Unauthenticated."}));

    let reply = t.send(get("/api/me", Some(&second))).await;
    assert_eq!(reply.status, StatusCode::OK);
    assert_eq!(reply.json["data"]["username"], "kontrak");
    assert_eq!(reply.json["data"]["employee"]["employee_number"], "KTR-001");

    let reply = t
        .send(post_json("/api/auth/logout", Some(&second), json!({})))
        .await;
    assert_eq!(reply.status, StatusCode::OK);
    assert_eq!(reply.json, json!({"message": "Berhasil keluar."}));
    assert_eq!(
        t.send(get("/api/me", Some(&second))).await.status,
        StatusCode::UNAUTHORIZED
    );
    // Token perangkat lain tidak ikut dicabut.
    assert_eq!(
        t.send(get("/api/me", Some(&other_device))).await.status,
        StatusCode::OK
    );
}

#[sqlx::test(migrations = false)]
async fn unauthenticated_is_json_without_accept_header(db: PgPool) {
    let t = setup(db).await;
    let req = axum::http::Request::get("/api/me")
        .body(axum::body::Body::empty())
        .unwrap();
    let reply = t.send(req).await;
    assert_eq!(reply.status, StatusCode::UNAUTHORIZED);
    assert_eq!(reply.json, json!({"message": "Unauthenticated."}));
    for bad in ["x1|abc", "1|", "garbage", "999999|abcdef"] {
        assert_eq!(
            t.send(get("/api/me", Some(bad))).await.status,
            StatusCode::UNAUTHORIZED,
            "{bad}"
        );
    }
}

/// Token yang ditulis Laravel (Sanctum) diterima: baris dibuat persis seperti `createToken()`.
#[sqlx::test(migrations = false)]
async fn laravel_issued_tokens_are_accepted(db: PgPool) {
    let t = setup(db).await;
    let user_id = t
        .scalar_i64("SELECT id FROM users WHERE username = 'kontrak'")
        .await;
    let secret = token::generate_secret("");
    let id: i64 = sqlx::query_scalar(
        "INSERT INTO personal_access_tokens (tokenable_type, tokenable_id, name, token, abilities, created_at, updated_at)
         VALUES ('App\\Models\\User', $1, 'laravel', $2, '[\"mobile\"]', now() AT TIME ZONE 'UTC', now() AT TIME ZONE 'UTC')
         RETURNING id",
    )
    .bind(user_id)
    .bind(token::sha256_hex(&secret))
    .fetch_one(&t.db)
    .await
    .unwrap();

    let with_id = format!("{id}|{secret}");
    assert_eq!(
        t.send(get("/api/me", Some(&with_id))).await.status,
        StatusCode::OK
    );
    // Sanctum juga menerima rahasia tanpa id.
    assert_eq!(
        t.send(get("/api/me", Some(&secret))).await.status,
        StatusCode::OK
    );
    // Rahasia benar dengan id lain ditolak.
    let wrong_id = format!("{}|{secret}", id + 1000);
    assert_eq!(
        t.send(get("/api/me", Some(&wrong_id))).await.status,
        StatusCode::UNAUTHORIZED
    );

    // last_used_at diperbarui di latar belakang (debounce 0 di test).
    let mut used = 0;
    for _ in 0..30 {
        used = t
            .scalar_i64(&format!(
                "SELECT count(*) FROM personal_access_tokens WHERE id = {id} AND last_used_at IS NOT NULL"
            ))
            .await;
        if used == 1 {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }
    assert_eq!(used, 1);
}

#[sqlx::test(migrations = false)]
async fn expired_wrong_ability_and_foreign_tokenable(db: PgPool) {
    let t = setup(db).await;
    let user_id = t
        .scalar_i64("SELECT id FROM users WHERE username = 'kontrak'")
        .await;
    let insert = |created_offset: &str, abilities: &str, kind: &str, expires: &str| {
        let secret = token::generate_secret("");
        let sql = format!(
            "INSERT INTO personal_access_tokens (tokenable_type, tokenable_id, name, token, abilities, expires_at, created_at, updated_at)
             VALUES ('{kind}', {user_id}, 'x', '{}', '{abilities}', {expires}, (now() AT TIME ZONE 'UTC') - interval '{created_offset}', now() AT TIME ZONE 'UTC')
             RETURNING id",
            token::sha256_hex(&secret)
        );
        (sql, secret)
    };
    let cases = [
        // Umur lewat SANCTUM_TOKEN_EXPIRATION (30 hari).
        (
            insert("31 days", r#"["mobile"]"#, r"App\Models\User", "NULL"),
            StatusCode::UNAUTHORIZED,
        ),
        // expires_at sudah lewat.
        (
            insert(
                "1 hour",
                r#"["mobile"]"#,
                r"App\Models\User",
                "(now() AT TIME ZONE 'UTC') - interval '1 minute'",
            ),
            StatusCode::UNAUTHORIZED,
        ),
        // Ability lain: CheckAbilities -> 403.
        (
            insert("1 hour", r#"["web"]"#, r"App\Models\User", "NULL"),
            StatusCode::FORBIDDEN,
        ),
        // Ability wildcard.
        (
            insert("1 hour", r#"["*"]"#, r"App\Models\User", "NULL"),
            StatusCode::OK,
        ),
        // Bukan milik model User.
        (
            insert("1 hour", r#"["mobile"]"#, r"App\Models\Device", "NULL"),
            StatusCode::UNAUTHORIZED,
        ),
    ];
    for ((sql, secret), expected) in cases {
        let id = t.scalar_i64(&sql).await;
        let reply = t
            .send(get("/api/me", Some(&format!("{id}|{secret}"))))
            .await;
        assert_eq!(reply.status, expected, "{sql}");
        if expected == StatusCode::FORBIDDEN {
            assert_eq!(reply.json, json!({"message": "Invalid ability provided."}));
        }
    }
}

#[sqlx::test(migrations = false)]
async fn login_rate_limit_per_account_and_ip(db: PgPool) {
    let t = setup(db).await;
    for _ in 0..5 {
        let reply = t
            .send(post_json(
                "/api/auth/login",
                None,
                json!({"login": "KONTRAK", "password": "salah", "device_name": "x"}),
            ))
            .await;
        assert_eq!(reply.status, StatusCode::UNPROCESSABLE_ENTITY);
    }
    // Percobaan ke-6 untuk akun yang sama (tanpa membedakan huruf besar) ditolak, walau sandinya benar.
    let reply = t
        .send(post_json(
            "/api/auth/login",
            None,
            json!({"login": "kontrak", "password": PASSWORD, "device_name": "x"}),
        ))
        .await;
    assert_eq!(reply.status, StatusCode::TOO_MANY_REQUESTS);
    assert_eq!(reply.json, json!({"message": "Too Many Attempts."}));
    assert!(reply.headers.contains_key("retry-after"));
    assert_eq!(reply.headers["x-ratelimit-limit"], "5");
    // Akun lain dari IP yang sama masih boleh.
    t.login("kontrak.pegawai-nonaktif", "x").await;
}

#[sqlx::test(migrations = false)]
async fn api_rate_limit_per_token(db: PgPool) {
    let t = setup_with(db, &[("RATE_LIMIT_API_PER_TOKEN", "3")], None).await;
    let token = t.login("kontrak", "hp").await;
    for _ in 0..3 {
        assert_eq!(
            t.send(get("/api/me", Some(&token))).await.status,
            StatusCode::OK
        );
    }
    assert_eq!(
        t.send(get("/api/me", Some(&token))).await.status,
        StatusCode::TOO_MANY_REQUESTS
    );
    // Token tidak sah tidak ikut dihitung dan tetap 401.
    assert_eq!(
        t.send(get("/api/me", Some("1|salah"))).await.status,
        StatusCode::UNAUTHORIZED
    );
}

#[sqlx::test(migrations = false)]
async fn form_encoded_login_and_trailing_slash(db: PgPool) {
    let t = setup(db).await;
    let req = axum::http::Request::post("/api/auth/login/")
        .header("content-type", "application/x-www-form-urlencoded")
        .body(axum::body::Body::from(format!(
            "login=+kontrak+&password={PASSWORD}&device_name=form"
        )))
        .unwrap();
    let reply = t.send(req).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.json);
}
