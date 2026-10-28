//! Autentikasi token kompatibel Sanctum dan endpoint login/logout/me (`AuthController` Laravel).
//!
//! Urutan per request ber-token sama dengan Laravel: `auth:sanctum` -> `abilities:mobile` ->
//! `throttle:api` (120/menit per token) -> `throttle:api-punch` (20/menit per user, endpoint aksi).
//! Request yang gagal autentikasi tidak ikut dihitung batas laju.

use axum::Json;
use axum::extract::{FromRequestParts, State};
use axum::http::request::Parts;
use axum::http::{HeaderMap, header};
use chrono::{DateTime, NaiveDateTime, TimeDelta, Utc};
use serde::Serialize;
use serde_json::{Value, json};
use utoipa::ToSchema;

use crate::employees::{self, Me, UserIdentity};
use crate::error::{AppError, AppResult};
use crate::input::Input;
use crate::net::ClientIp;
use crate::openapi::{Data, LoginRequest, Message, Unprocessable};
use crate::rate_limit::Limit;
use crate::state::AppState;
use crate::time::naive_utc;
use crate::token::{self, MOBILE_ABILITY, ParsedToken, TOKENABLE_TYPE};
use crate::validation::{ValidationErrors, msg};

pub const NO_EMPLOYEE: &str = "Akun ini belum terhubung ke data pegawai.";
const BAD_CREDENTIALS: &str = "Username atau kata sandi salah.";

/// Pengguna yang lolos `auth:sanctum` + `abilities:mobile` + `throttle:api`.
#[derive(Clone, Debug)]
pub struct AuthUser {
    pub id: i64,
    pub name: String,
    pub username: Option<String>,
    pub employee_id: Option<i64>,
    pub token_id: i64,
}

impl AuthUser {
    pub fn identity(&self) -> UserIdentity<'_> {
        UserIdentity {
            id: self.id,
            name: &self.name,
            username: self.username.as_deref(),
            employee_id: self.employee_id,
        }
    }

    /// Untuk endpoint baca yang menolak 403 bila akun belum terhubung ke pegawai.
    pub fn require_employee(&self) -> AppResult<i64> {
        self.employee_id
            .ok_or_else(|| AppError::Forbidden(NO_EMPLOYEE.into()))
    }
}

struct TokenRow {
    id: i64,
    token: String,
    abilities: Option<String>,
    last_used_at: Option<NaiveDateTime>,
    expires_at: Option<NaiveDateTime>,
    created_at: Option<NaiveDateTime>,
    user_id: Option<i64>,
    user_name: Option<String>,
    username: Option<String>,
    employee_id: Option<i64>,
}

async fn find_token(state: &AppState, parsed: &ParsedToken<'_>) -> AppResult<Option<TokenRow>> {
    let row =
        match parsed {
            ParsedToken::WithId { id, secret } => {
                let row = sqlx::query_as!(
                TokenRow,
                r#"SELECT t.id, t.token, t.abilities, t.last_used_at, t.expires_at, t.created_at,
                          u.id AS "user_id?", u.name AS "user_name?", u.username, u.employee_id
                   FROM personal_access_tokens t
                   LEFT JOIN users u ON u.id = t.tokenable_id AND t.tokenable_type = $2
                   WHERE t.id = $1"#,
                id,
                TOKENABLE_TYPE
            )
            .fetch_optional(&state.db)
            .await?;
                row.filter(|r| token::constant_time_eq(&r.token, &token::sha256_hex(secret)))
            }
            ParsedToken::Plain(secret) => sqlx::query_as!(
                TokenRow,
                r#"SELECT t.id, t.token, t.abilities, t.last_used_at, t.expires_at, t.created_at,
                          u.id AS "user_id?", u.name AS "user_name?", u.username, u.employee_id
                   FROM personal_access_tokens t
                   LEFT JOIN users u ON u.id = t.tokenable_id AND t.tokenable_type = $2
                   WHERE t.token = $1"#,
                token::sha256_hex(secret),
                TOKENABLE_TYPE
            )
            .fetch_optional(&state.db)
            .await?,
        };
    Ok(row)
}

/// `Guard::isValidAccessToken`: umur token (`SANCTUM_TOKEN_EXPIRATION`) dan `expires_at`.
fn is_valid(row: &TokenRow, now: DateTime<Utc>, expiration_minutes: i64) -> bool {
    let now = naive_utc(now);
    let young_enough = expiration_minutes <= 0
        || row
            .created_at
            .is_some_and(|created| created > now - TimeDelta::minutes(expiration_minutes));
    let not_expired = row.expires_at.is_none_or(|expires| expires > now);
    young_enough && not_expired
}

impl FromRequestParts<AppState> for AuthUser {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        if let Some(user) = parts.extensions.get::<AuthUser>() {
            return Ok(user.clone());
        }

        let header = parts
            .headers
            .get(header::AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        let bearer = token::bearer_token(header).ok_or(AppError::Unauthenticated)?;
        let parsed = token::parse(bearer).ok_or(AppError::Unauthenticated)?;
        let row = find_token(state, &parsed)
            .await?
            .ok_or(AppError::Unauthenticated)?;

        let now = state.now();
        if !is_valid(&row, now, state.config.token_expiration_minutes) {
            return Err(AppError::Unauthenticated);
        }
        let (Some(user_id), Some(user_name)) = (row.user_id, row.user_name.clone()) else {
            return Err(AppError::Unauthenticated);
        };
        if !token::has_ability(row.abilities.as_deref(), MOBILE_ABILITY) {
            return Err(AppError::Forbidden("Invalid ability provided.".into()));
        }

        touch_last_used(state, &row, now);

        state
            .limiter
            .check(&[Limit::per_minute(
                state.config.rate_limits.api_per_token,
                format!("api:{}", token::sha256_hex(bearer)),
            )])
            .await?;

        let user = AuthUser {
            id: user_id,
            name: user_name,
            username: row.username,
            employee_id: row.employee_id,
            token_id: row.id,
        };
        parts.extensions.insert(user.clone());
        Ok(user)
    }
}

/// Perbarui `last_used_at` di latar belakang, paling sering sekali per `TOKEN_LAST_USED_DEBOUNCE_SECS`.
/// Sanctum menulis di setiap request; dengan ribuan pegawai itu satu UPDATE per request.
fn touch_last_used(state: &AppState, row: &TokenRow, now: DateTime<Utc>) {
    let debounce = TimeDelta::from_std(state.config.token_last_used_debounce).unwrap_or_default();
    let now = naive_utc(now);
    let stale = row.last_used_at.is_none_or(|last| last <= now - debounce);
    if !stale {
        return;
    }
    let db = state.db.clone();
    let id = row.id;
    tokio::spawn(async move {
        let result = sqlx::query!(
            "UPDATE personal_access_tokens SET last_used_at = $2, updated_at = $2 WHERE id = $1",
            id,
            now
        )
        .execute(&db)
        .await;
        if let Err(error) = result {
            tracing::warn!(%error, token_id = id, "gagal memperbarui last_used_at");
        }
    });
}

/// `throttle:api-punch`: 20/menit per pengguna untuk punch, check-in, dan pengajuan cuti.
pub struct ActionLimited(pub AuthUser);

impl FromRequestParts<AppState> for ActionLimited {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let user = AuthUser::from_request_parts(parts, state).await?;
        state
            .limiter
            .check(&[Limit::per_minute(
                state.config.rate_limits.actions_per_user,
                format!("action:{}", user.id),
            )])
            .await?;
        Ok(Self(user))
    }
}

#[derive(Serialize, ToSchema)]
pub struct LoginResponse {
    token: String,
    user: Me,
}

struct LoginForm {
    login: String,
    password: String,
    device_name: String,
}

fn validate_login(input: &Input) -> AppResult<LoginForm> {
    let mut errors = ValidationErrors::default();
    let mut text = |field: &str, max: usize| -> Option<String> {
        match input.get(field) {
            None => {
                errors.add(field, msg::required(field));
                None
            }
            Some(Value::String(s)) => {
                if s.chars().count() > max {
                    errors.add(field, msg::max_string(field, max));
                    None
                } else {
                    Some(s.clone())
                }
            }
            Some(Value::Array(a)) if a.is_empty() => {
                errors.add(field, msg::required(field));
                None
            }
            Some(_) => {
                errors.add(field, msg::string(field));
                None
            }
        }
    };
    let login = text("login", 255);
    let password = text("password", 255);
    let device_name = text("device_name", 100);
    errors.into_result()?;
    match (login, password, device_name) {
        (Some(login), Some(password), Some(device_name)) => Ok(LoginForm {
            login,
            password,
            device_name,
        }),
        _ => Err(AppError::internal("validasi login tidak konsisten")),
    }
}

/// `FILTER_VALIDATE_EMAIL` cukup untuk memilih kolom: satu `@`, bagian lokal dan domain tidak kosong,
/// domain bertitik, tanpa spasi.
fn looks_like_email(value: &str) -> bool {
    let Some((local, domain)) = value.split_once('@') else {
        return false;
    };
    !local.is_empty()
        && !domain.is_empty()
        && !domain.contains('@')
        && domain.contains('.')
        && !domain.starts_with('.')
        && !domain.ends_with('.')
        && !value.chars().any(char::is_whitespace)
}

struct LoginUser {
    id: i64,
    name: String,
    username: Option<String>,
    password: String,
    employee_id: Option<i64>,
    is_active: bool,
}

/// `UserRepository::findByLogin`: email bila berformat email, selain itu username.
async fn find_by_login(state: &AppState, login: &str) -> AppResult<Option<LoginUser>> {
    let row = if looks_like_email(login) {
        sqlx::query_as!(
            LoginUser,
            "SELECT id, name, username, password, employee_id, is_active FROM users WHERE email = $1 LIMIT 1",
            login
        )
        .fetch_optional(&state.db)
        .await?
    } else {
        sqlx::query_as!(
            LoginUser,
            "SELECT id, name, username, password, employee_id, is_active FROM users WHERE username = $1 LIMIT 1",
            login
        )
        .fetch_optional(&state.db)
        .await?
    };
    Ok(row)
}

/// Verifikasi bcrypt (`$2y$`, cost dari hash) di thread blocking, dengan jumlah bersamaan dibatasi
/// agar lonjakan login tidak menghabiskan CPU untuk request lain. Sandi tidak pernah ditulis/di-rehash.
async fn verify_password(state: &AppState, password: String, hash: String) -> AppResult<bool> {
    let permit = tokio::time::timeout(
        state.config.hash_queue_timeout,
        state.hash_slots.clone().acquire_owned(),
    )
    .await
    .map_err(|_| {
        AppError::ServiceUnavailable("Server sedang sibuk. Coba lagi sebentar lagi.".into())
    })?
    .map_err(AppError::internal)?;

    tokio::task::spawn_blocking(move || {
        let _permit = permit;
        match bcrypt::verify(password.as_bytes(), &hash) {
            Ok(matches) => matches,
            Err(error) => {
                // Hash bukan bcrypt (mis. argon): Laravel melempar galat; di sini dianggap tidak cocok.
                tracing::warn!(%error, "hash sandi tidak bisa diverifikasi");
                false
            }
        }
    })
    .await
    .map_err(AppError::internal)
}

/// Masuk dan dapatkan token perangkat. Satu perangkat satu token: login ulang dengan `device_name` yang
/// sama mencabut token lama.
#[utoipa::path(
    post, path = "/auth/login", tag = "Auth", operation_id = "login",
    security(()),
    request_body(content = LoginRequest, content_type = "application/json"),
    responses(
        (status = 200, description = "Berhasil", body = LoginResponse),
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
) -> AppResult<Json<LoginResponse>> {
    let limits = &state.config.rate_limits;
    // throttle:api (grup api): per token bila ada header Bearer, selain itu per IP.
    let api_key = headers
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(token::bearer_token)
        .map_or_else(
            || format!("api-ip:{ip}"),
            |t| format!("api:{}", token::sha256_hex(t)),
        );
    let login_key = match input.get("login") {
        Some(Value::String(s)) => s.to_lowercase(),
        Some(other) => other.to_string().to_lowercase(),
        None => String::new(),
    };
    state
        .limiter
        .check(&[
            Limit::per_minute(limits.api_per_token, api_key),
            Limit::per_minute(limits.login_per_ip, format!("login-ip:{ip}")),
            Limit::per_minute(limits.login_per_account, format!("login:{login_key}|{ip}")),
        ])
        .await?;

    let form = validate_login(&input)?;

    // Pesan sama untuk user tidak ada, akun nonaktif, dan sandi salah agar tidak membocorkan akun.
    let user = match find_by_login(&state, &form.login).await? {
        Some(user) if user.is_active => user,
        _ => return Err(AppError::rule(BAD_CREDENTIALS)),
    };
    if !verify_password(&state, form.password, user.password.clone()).await? {
        return Err(AppError::rule(BAD_CREDENTIALS));
    }
    if user.employee_id.is_none() {
        return Err(AppError::Forbidden(NO_EMPLOYEE.into()));
    }

    let secret = token::generate_secret(&state.config.token_prefix);
    let now = naive_utc(state.now());
    let mut tx = state.db.begin().await?;
    // Satu perangkat satu token: token lama dengan device_name yang sama dicabut.
    sqlx::query!(
        "DELETE FROM personal_access_tokens WHERE tokenable_type = $1 AND tokenable_id = $2 AND name = $3",
        TOKENABLE_TYPE,
        user.id,
        form.device_name
    )
    .execute(&mut *tx)
    .await?;
    let token_id = sqlx::query_scalar!(
        r#"INSERT INTO personal_access_tokens (tokenable_type, tokenable_id, name, token, abilities, expires_at, created_at, updated_at)
           VALUES ($1, $2, $3, $4, $5, NULL, $6, $6)
           RETURNING id"#,
        TOKENABLE_TYPE,
        user.id,
        form.device_name,
        token::sha256_hex(&secret),
        json!([MOBILE_ABILITY]).to_string(),
        now
    )
    .fetch_one(&mut *tx)
    .await?;
    tx.commit().await?;

    let me = employees::me(
        &state.db,
        &state.config,
        UserIdentity {
            id: user.id,
            name: &user.name,
            username: user.username.as_deref(),
            employee_id: user.employee_id,
        },
    )
    .await?;

    Ok(Json(LoginResponse {
        token: format!("{token_id}|{secret}"),
        user: me,
    }))
}

/// Cabut token yang sedang dipakai.
#[utoipa::path(
    post, path = "/auth/logout", tag = "Auth", operation_id = "logout",
    responses(
        (status = 200, description = "Berhasil", body = Message),
        (status = 401, description = "Token tidak sah", body = Message),
        (status = 429, description = "Batas laju terlampaui", body = Message),
    )
)]
pub async fn logout(State(state): State<AppState>, user: AuthUser) -> AppResult<Json<Message>> {
    sqlx::query!(
        "DELETE FROM personal_access_tokens WHERE id = $1",
        user.token_id
    )
    .execute(&state.db)
    .await?;
    Ok(Json(Message {
        message: "Berhasil keluar.".into(),
    }))
}

/// Profil pengguna dan pengaturan aplikasi.
#[utoipa::path(
    get, path = "/me", tag = "Auth", operation_id = "me",
    responses(
        (status = 200, description = "Profil", body = Data<Me>),
        (status = 401, description = "Token tidak sah", body = Message),
        (status = 429, description = "Batas laju terlampaui", body = Message),
    )
)]
pub async fn me(State(state): State<AppState>, user: AuthUser) -> AppResult<Json<Data<Me>>> {
    let me = employees::me(&state.db, &state.config, user.identity()).await?;
    Ok(Json(Data { data: me }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn row(created: Option<NaiveDateTime>, expires: Option<NaiveDateTime>) -> TokenRow {
        TokenRow {
            id: 1,
            token: String::new(),
            abilities: None,
            last_used_at: None,
            expires_at: expires,
            created_at: created,
            user_id: Some(1),
            user_name: Some("a".into()),
            username: None,
            employee_id: None,
        }
    }

    #[test]
    fn token_validity_like_sanctum_guard() {
        let now = Utc.with_ymd_and_hms(2026, 10, 29, 0, 0, 0).unwrap();
        let n = now.naive_utc();
        assert!(is_valid(
            &row(Some(n - TimeDelta::days(29)), None),
            now,
            43200
        ));
        assert!(!is_valid(
            &row(Some(n - TimeDelta::days(30)), None),
            now,
            43200
        ));
        assert!(is_valid(&row(Some(n - TimeDelta::days(400)), None), now, 0));
        assert!(!is_valid(
            &row(Some(n), Some(n - TimeDelta::seconds(1))),
            now,
            43200
        ));
        assert!(is_valid(
            &row(Some(n), Some(n + TimeDelta::seconds(1))),
            now,
            43200
        ));
        assert!(!is_valid(&row(None, None), now, 43200));
    }

    #[test]
    fn email_detection() {
        assert!(looks_like_email("kontrak@kontrak.test"));
        assert!(!looks_like_email("kontrak"));
        assert!(!looks_like_email("kontrak.akun-nonaktif"));
        assert!(!looks_like_email("a@b"));
        assert!(!looks_like_email("a b@c.d"));
    }

    #[test]
    fn login_validation_lists_all_missing_fields() {
        let input = Input::from_json(json!({}));
        let Err(AppError::Validation(errors)) = validate_login(&input) else {
            panic!("harus galat validasi");
        };
        assert!(errors.has("login") && errors.has("password") && errors.has("device_name"));
        assert_eq!(
            errors.summary(),
            "Gabung wajib diisi. (dan 2 kesalahan lainnya)"
        );

        let long = "x".repeat(101);
        let input = Input::from_json(json!({"login": "a", "password": "b", "device_name": long}));
        let Err(AppError::Validation(errors)) = validate_login(&input) else {
            panic!("harus galat validasi");
        };
        assert!(errors.has("device_name") && !errors.has("login"));
    }
}
