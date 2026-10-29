//! `/api/v2/auth/*`: login dengan refresh token. Tambahan, tidak mengubah kontrak v1.
//!
//! - Token akses: format Sanctum, ability `mobile`, `expires_at` = sekarang + `MOBILE_ACCESS_TOKEN_TTL_MINUTES`
//!   (paling lama `SANCTUM_TOKEN_EXPIRATION`). Dipakai ke semua endpoint `/api/*` seperti token v1, juga
//!   diterima Laravel.
//! - Refresh token: format Sanctum, ability `refresh`, `expires_at` = sekarang + `MOBILE_REFRESH_TOKEN_TTL_MINUTES`.
//!   Ditukar di `/api/v2/auth/refresh` dengan pasangan baru (rotasi: yang lama langsung tidak berlaku).
//!   Sebagai Bearer di endpoint lain ditolak 403 (ability salah), di Rust maupun Laravel.
//! - Keduanya baris `personal_access_tokens` dengan `name` = `device_name`, jadi pencabutan dari web
//!   (`tokens()->delete()` saat ganti sandi/nonaktif) ikut mencabut refresh token, dan login v1 dengan
//!   perangkat yang sama menggantikan keduanya.
//!
//! Tukar refresh tidak memakai bcrypt, jadi murah walau ribuan perangkat memperbarui token bersamaan:
//! lonjakan login karena token kedaluwarsa serentak tidak terjadi lagi.

use axum::Json;
use axum::extract::State;
use axum::http::HeaderMap;
use chrono::{DateTime, TimeDelta, Utc};
use serde::Serialize;
use serde_json::Value;
use utoipa::ToSchema;

use crate::auth::{AuthUser, NO_EMPLOYEE, authenticate, issue_token, revoke_device};
use crate::employees::{self, Me};
use crate::error::{AppError, AppResult};
use crate::input::Input;
use crate::net::ClientIp;
use crate::openapi::{LoginRequest, Message, RefreshRequest, Unprocessable};
use crate::rate_limit::Limit;
use crate::state::AppState;
use crate::time::{iso8601, naive_utc};
use crate::token::{self, MOBILE_ABILITY, ParsedToken, TOKENABLE_TYPE};
use crate::validation::{ValidationErrors, msg};

pub const REFRESH_ABILITY: &str = "refresh";

/// Pasangan token baru dari login atau refresh.
#[derive(Debug, Serialize, ToSchema)]
pub struct TokenPair {
    /// Token akses untuk header `Authorization: Bearer`.
    pub token: String,
    pub token_expires_at: String,
    /// Simpan aman; hanya untuk `POST /v2/auth/refresh`.
    pub refresh_token: String,
    pub refresh_expires_at: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct LoginV2Response {
    pub token: String,
    pub token_expires_at: String,
    pub refresh_token: String,
    pub refresh_expires_at: String,
    pub user: Me,
}

fn ttl(duration: std::time::Duration) -> TimeDelta {
    TimeDelta::from_std(duration).unwrap_or(TimeDelta::days(7))
}

/// Buat pasangan token untuk satu perangkat (token lama perangkat itu sudah dicabut pemanggil).
async fn issue_pair(
    state: &AppState,
    tx: &mut sqlx::PgConnection,
    user_id: i64,
    device_name: &str,
    now: DateTime<Utc>,
) -> AppResult<TokenPair> {
    let config = &state.config;
    let mut access_expires = now + ttl(config.access_token_ttl);
    if config.token_expiration_minutes > 0 {
        // Umur token juga dibatasi SANCTUM_TOKEN_EXPIRATION (Rust dan Laravel menolaknya setelah itu).
        access_expires =
            access_expires.min(now + TimeDelta::minutes(config.token_expiration_minutes));
    }
    let refresh_expires = now + ttl(config.refresh_token_ttl);

    let token = issue_token(
        tx,
        user_id,
        device_name,
        MOBILE_ABILITY,
        Some(access_expires),
        now,
        &config.token_prefix,
    )
    .await?;
    let refresh_token = issue_token(
        tx,
        user_id,
        device_name,
        REFRESH_ABILITY,
        Some(refresh_expires),
        now,
        &config.token_prefix,
    )
    .await?;
    Ok(TokenPair {
        token,
        token_expires_at: iso8601(access_expires, config.timezone),
        refresh_token,
        refresh_expires_at: iso8601(refresh_expires, config.timezone),
    })
}

/// Login v2: aturan, pesan, dan batas laju sama dengan v1; respons menambah refresh token.
#[utoipa::path(
    post, path = "/v2/auth/login", tag = "Auth", operation_id = "loginV2",
    security(()),
    request_body(content = LoginRequest, content_type = "application/json"),
    responses(
        (status = 200, description = "Berhasil", body = LoginV2Response),
        (status = 403, description = "Akun belum terhubung ke data pegawai", body = Message),
        (status = 422, description = "Validasi gagal, atau username/sandi salah, atau akun nonaktif", body = Unprocessable),
        (status = 429, description = "Batas laju terlampaui", body = Message),
    )
)]
pub async fn login(
    State(state): State<AppState>,
    ClientIp(ip): ClientIp,
    headers: HeaderMap,
    input: Input,
) -> AppResult<Json<LoginV2Response>> {
    let (user, device_name) = authenticate(&state, ip, &headers, &input).await?;
    let now = state.now();
    let mut tx = state.db.begin().await?;
    revoke_device(&mut tx, user.id, &device_name).await?;
    let pair = issue_pair(&state, &mut tx, user.id, &device_name, now).await?;
    tx.commit().await?;

    let me = employees::me(&state.db, &state.config, user.identity()).await?;
    Ok(Json(LoginV2Response {
        token: pair.token,
        token_expires_at: pair.token_expires_at,
        refresh_token: pair.refresh_token,
        refresh_expires_at: pair.refresh_expires_at,
        user: me,
    }))
}

struct RefreshRow {
    token: String,
    abilities: Option<String>,
    expires_at: Option<chrono::NaiveDateTime>,
    name: String,
    user_id: i64,
    is_active: bool,
    employee_id: Option<i64>,
}

/// Tukar refresh token dengan pasangan baru. Refresh token tidak sah, kedaluwarsa, sudah dipakai, atau
/// milik akun nonaktif: 401 (aplikasi kembali ke layar login).
#[utoipa::path(
    post, path = "/v2/auth/refresh", tag = "Auth", operation_id = "refreshToken",
    security(()),
    request_body(content = RefreshRequest, content_type = "application/json"),
    responses(
        (status = 200, description = "Pasangan token baru", body = TokenPair),
        (status = 401, description = "Refresh token tidak sah, kedaluwarsa, atau sudah dipakai", body = Message),
        (status = 403, description = "Akun belum terhubung ke data pegawai", body = Message),
        (status = 422, description = "Validasi gagal", body = Unprocessable),
        (status = 429, description = "Batas laju terlampaui", body = Message),
    )
)]
pub async fn refresh(
    State(state): State<AppState>,
    ClientIp(ip): ClientIp,
    input: Input,
) -> AppResult<Json<TokenPair>> {
    let limits = &state.config.rate_limits;
    let field = "refresh_token";
    let mut errors = ValidationErrors::default();
    let raw = match input.get(field) {
        None => {
            errors.add(field, msg::required(field));
            None
        }
        Some(Value::String(s)) if s.chars().count() > 255 => {
            errors.add(field, msg::max_string(field, 255));
            None
        }
        Some(Value::String(s)) => Some(s.clone()),
        Some(_) => {
            errors.add(field, msg::string(field));
            None
        }
    };
    state
        .limiter
        .check(&[
            Limit::per_minute(limits.api_per_ip, format!("api-ip:{ip}")),
            Limit::per_minute(
                limits.login_per_account,
                format!(
                    "refresh:{}",
                    token::sha256_hex(raw.as_deref().unwrap_or(""))
                ),
            ),
        ])
        .await?;
    errors.into_result()?;
    let raw = raw.unwrap_or_default();

    let Some(ParsedToken::WithId { id, secret }) = token::parse(&raw) else {
        return Err(AppError::Unauthenticated);
    };

    let now = state.now();
    let mut tx = state.db.begin().await?;
    // FOR UPDATE: dua refresh bersamaan dengan token yang sama, hanya satu yang berhasil.
    let row = sqlx::query_as!(
        RefreshRow,
        r#"SELECT t.token AS "token!", t.abilities, t.expires_at, t.name AS "name!",
                  u.id AS "user_id!", u.is_active AS "is_active!", u.employee_id
           FROM personal_access_tokens t
           JOIN users u ON u.id = t.tokenable_id AND t.tokenable_type = $2
           WHERE t.id = $1
           FOR UPDATE OF t"#,
        id,
        TOKENABLE_TYPE
    )
    .fetch_optional(&mut *tx)
    .await?
    .ok_or(AppError::Unauthenticated)?;

    let is_refresh = row
        .abilities
        .as_deref()
        .and_then(|a| serde_json::from_str::<Vec<String>>(a).ok())
        .is_some_and(|list| list.iter().any(|a| a == REFRESH_ABILITY));
    let valid = token::constant_time_eq(&row.token, &token::sha256_hex(secret))
        && is_refresh
        && row.expires_at.is_some_and(|e| e > naive_utc(now))
        && row.is_active;
    if !valid {
        return Err(AppError::Unauthenticated);
    }
    if row.employee_id.is_none() {
        return Err(AppError::Forbidden(NO_EMPLOYEE.into()));
    }

    revoke_device(&mut tx, row.user_id, &row.name).await?;
    let pair = issue_pair(&state, &mut tx, row.user_id, &row.name, now).await?;
    tx.commit().await?;
    Ok(Json(pair))
}

/// Keluar dari perangkat ini: token akses dan refresh token perangkat itu dicabut.
#[utoipa::path(
    post, path = "/v2/auth/logout", tag = "Auth", operation_id = "logoutV2",
    responses(
        (status = 200, description = "Berhasil", body = Message),
        (status = 401, description = "Token tidak sah", body = Message),
        (status = 429, description = "Batas laju terlampaui", body = Message),
    )
)]
pub async fn logout(State(state): State<AppState>, user: AuthUser) -> AppResult<Json<Message>> {
    let mut tx = state.db.begin().await?;
    let name = sqlx::query_scalar!(
        "SELECT name FROM personal_access_tokens WHERE id = $1",
        user.token_id
    )
    .fetch_optional(&mut *tx)
    .await?;
    match name {
        Some(name) => revoke_device(&mut tx, user.id, &name).await?,
        None => return Err(AppError::Unauthenticated),
    }
    tx.commit().await?;
    Ok(Json(Message {
        message: "Berhasil keluar.".into(),
    }))
}
