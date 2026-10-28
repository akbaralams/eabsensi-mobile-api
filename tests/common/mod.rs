//! Harness test integrasi: tiap test mendapat database baru (`#[sqlx::test]`) berisi dump skema
//! (`schema/pgsql-schema.sql`) dan data kontrak (`tests/contract/fixtures.sql`), lalu memanggil router
//! langsung tanpa membuka port.
#![allow(dead_code)]

use std::sync::Arc;

use axum::Router;
use axum::body::Body;
use axum::http::{HeaderMap, Request, StatusCode, header};
use chrono::{DateTime, Utc};
use eabsensi_mobile_api::config::Config;
use eabsensi_mobile_api::rate_limit::RateLimiter;
use eabsensi_mobile_api::state::Clock;
use eabsensi_mobile_api::{AppState, app};
use http_body_util::BodyExt;
use serde_json::{Value, json};
use sqlx::PgPool;
use tower::ServiceExt;

pub const PASSWORD: &str = "kontrak-uji-2026";
pub const PNG: &[u8] = include_bytes!("../contract/fixtures/photo.png");

pub struct TestApp {
    pub app: tower_http::normalize_path::NormalizePath<Router>,
    pub db: PgPool,
    pub photos: tempfile::TempDir,
    pub state: AppState,
}

/// Bagian skema saja dari dump (sebelum data `COPY ... FROM stdin`), tanpa meta-command psql.
fn schema_sql() -> String {
    let dump = include_str!("../../schema/pgsql-schema.sql");
    let schema = dump.split("\\unrestrict").next().unwrap_or(dump);
    schema
        .lines()
        .filter(|line| !line.starts_with('\\'))
        .collect::<Vec<_>>()
        .join("\n")
}

/// `fixtures.sql` tanpa meta-command psql, zona `Asia/Jakarta`.
fn fixtures_sql() -> String {
    include_str!("../contract/fixtures.sql")
        .lines()
        .filter(|line| !line.starts_with('\\'))
        .collect::<Vec<_>>()
        .join("\n")
        .replace(":'tz'", "'Asia/Jakarta'")
}

pub async fn load_database(db: &PgPool) {
    let mut conn = db.acquire().await.expect("koneksi");
    sqlx::raw_sql(sqlx::AssertSqlSafe(schema_sql()))
        .execute(&mut *conn)
        .await
        .expect("muat skema");
    sqlx::raw_sql("SET search_path TO public")
        .execute(&mut *conn)
        .await
        .expect("search_path");
    sqlx::raw_sql(sqlx::AssertSqlSafe(fixtures_sql()))
        .execute(&mut *conn)
        .await
        .expect("muat fixture");
}

pub fn config(photo_root: &std::path::Path, overrides: &[(&str, &str)]) -> Config {
    let root = photo_root.to_string_lossy().to_string();
    let mut pairs: Vec<(String, String)> = vec![
        ("DATABASE_URL".into(), "postgres://tidak-dipakai".into()),
        ("PHOTO_ROOT".into(), root),
        ("TOKEN_LAST_USED_DEBOUNCE_SECS".into(), "0".into()),
    ];
    for (k, v) in overrides {
        pairs.retain(|(key, _)| key != k);
        pairs.push(((*k).into(), (*v).into()));
    }
    Config::from_lookup(move |key| pairs.iter().find(|(k, _)| k == key).map(|(_, v)| v.clone()))
        .expect("config uji")
}

pub async fn setup(db: PgPool) -> TestApp {
    setup_with(db, &[], None).await
}

pub async fn setup_with(db: PgPool, overrides: &[(&str, &str)], clock: Option<Clock>) -> TestApp {
    load_database(&db).await;
    let photos = tempfile::tempdir().expect("folder foto");
    let mut state = AppState::new(
        db.clone(),
        config(photos.path(), overrides),
        RateLimiter::memory(),
    );
    if let Some(clock) = clock {
        state = state.with_clock(clock);
    }
    TestApp {
        app: app(state.clone()),
        db,
        photos,
        state,
    }
}

pub fn fixed_clock(at: DateTime<Utc>) -> Clock {
    Arc::new(move || at)
}

pub struct Reply {
    pub status: StatusCode,
    pub headers: HeaderMap,
    pub json: Value,
}

impl TestApp {
    pub async fn send(&self, req: Request<Body>) -> Reply {
        let response = self.app.clone().oneshot(req).await.expect("respons");
        let status = response.status();
        let headers = response.headers().clone();
        let bytes = response
            .into_body()
            .collect()
            .await
            .expect("body")
            .to_bytes();
        let json = if bytes.is_empty() {
            Value::Null
        } else {
            serde_json::from_slice(&bytes)
                .unwrap_or_else(|_| panic!("bukan JSON: {}", String::from_utf8_lossy(&bytes)))
        };
        Reply {
            status,
            headers,
            json,
        }
    }

    pub async fn login(&self, login: &str, device: &str) -> String {
        let reply = self
            .send(post_json(
                "/api/auth/login",
                None,
                json!({"login": login, "password": PASSWORD, "device_name": device}),
            ))
            .await;
        assert_eq!(reply.status, StatusCode::OK, "{}", reply.json);
        reply.json["token"].as_str().expect("token").to_string()
    }

    pub async fn scalar_i64(&self, sql: &str) -> i64 {
        sqlx::query_scalar::<_, i64>(sqlx::AssertSqlSafe(sql.to_string()))
            .fetch_one(&self.db)
            .await
            .expect("query")
    }

    pub async fn exec(&self, sql: &str) {
        sqlx::raw_sql(sqlx::AssertSqlSafe(sql.to_string()))
            .execute(&self.db)
            .await
            .expect("exec");
    }
}

fn with_auth(
    builder: axum::http::request::Builder,
    token: Option<&str>,
) -> axum::http::request::Builder {
    let builder = builder.header(header::ACCEPT, "application/json");
    match token {
        Some(t) => builder.header(header::AUTHORIZATION, format!("Bearer {t}")),
        None => builder,
    }
}

pub fn get(uri: &str, token: Option<&str>) -> Request<Body> {
    with_auth(Request::get(uri), token)
        .body(Body::empty())
        .expect("request")
}

pub fn post_json(uri: &str, token: Option<&str>, body: Value) -> Request<Body> {
    with_auth(Request::post(uri), token)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(body.to_string()))
        .expect("request")
}

/// Multipart: field teks dan berkas opsional (nama field, isi, nama berkas, content-type).
pub fn post_multipart(
    uri: &str,
    token: Option<&str>,
    fields: &[(&str, &str)],
    file: Option<(&str, &[u8], &str, &str)>,
) -> Request<Body> {
    let boundary = "----eabsensi-uji";
    let mut body: Vec<u8> = Vec::new();
    for (name, value) in fields {
        body.extend_from_slice(
            format!(
                "--{boundary}\r\nContent-Disposition: form-data; name=\"{name}\"\r\n\r\n{value}\r\n"
            )
            .as_bytes(),
        );
    }
    if let Some((name, bytes, filename, content_type)) = file {
        body.extend_from_slice(
            format!(
                "--{boundary}\r\nContent-Disposition: form-data; name=\"{name}\"; filename=\"{filename}\"\r\nContent-Type: {content_type}\r\n\r\n"
            )
            .as_bytes(),
        );
        body.extend_from_slice(bytes);
        body.extend_from_slice(b"\r\n");
    }
    body.extend_from_slice(format!("--{boundary}--\r\n").as_bytes());
    with_auth(Request::post(uri), token)
        .header(
            header::CONTENT_TYPE,
            format!("multipart/form-data; boundary={boundary}"),
        )
        .body(Body::from(body))
        .expect("request")
}
