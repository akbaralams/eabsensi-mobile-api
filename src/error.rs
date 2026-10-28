//! Satu tipe galat untuk seluruh jalur request, dirender dengan bentuk JSON Laravel:
//! `{"message"}` untuk penolakan aturan dan galat HTTP, `{"message","errors"}` untuk validasi.

use axum::http::{HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Response};
use serde_json::{Map, Value, json};

use crate::validation::ValidationErrors;

pub type AppResult<T> = Result<T, AppError>;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("Unauthenticated.")]
    Unauthenticated,
    /// 403 dengan pesan apa adanya (mis. akun belum terhubung ke pegawai).
    #[error("{0}")]
    Forbidden(String),
    #[error("{0}")]
    NotFound(String),
    #[error("{0}")]
    MethodNotAllowed(String),
    #[error("validasi gagal")]
    Validation(ValidationErrors),
    /// Penolakan aturan bisnis: 422 dengan `message` saja; teksnya bagian dari kontrak.
    #[error("{0}")]
    Rule(String),
    #[error("Too Many Attempts.")]
    TooManyRequests {
        limit: u32,
        retry_after_secs: u64,
        reset_at_unix: i64,
    },
    #[error("payload terlalu besar")]
    PayloadTooLarge,
    #[error("{0}")]
    ServiceUnavailable(String),
    /// Galat tak terduga. Pesannya hanya dicatat di log, klien menerima "Server Error" seperti Laravel produksi.
    #[error("internal: {0}")]
    Internal(String),
}

impl AppError {
    pub fn internal(error: impl std::fmt::Display) -> Self {
        Self::Internal(error.to_string())
    }

    pub fn rule(message: impl Into<String>) -> Self {
        Self::Rule(message.into())
    }

    pub fn status(&self) -> StatusCode {
        match self {
            Self::Unauthenticated => StatusCode::UNAUTHORIZED,
            Self::Forbidden(_) => StatusCode::FORBIDDEN,
            Self::NotFound(_) => StatusCode::NOT_FOUND,
            Self::MethodNotAllowed(_) => StatusCode::METHOD_NOT_ALLOWED,
            Self::Validation(_) | Self::Rule(_) => StatusCode::UNPROCESSABLE_ENTITY,
            Self::TooManyRequests { .. } => StatusCode::TOO_MANY_REQUESTS,
            Self::PayloadTooLarge => StatusCode::PAYLOAD_TOO_LARGE,
            Self::ServiceUnavailable(_) => StatusCode::SERVICE_UNAVAILABLE,
            Self::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

impl From<sqlx::Error> for AppError {
    fn from(error: sqlx::Error) -> Self {
        match error {
            sqlx::Error::PoolTimedOut => {
                tracing::error!(error = %error, "pool database habis");
                Self::ServiceUnavailable("Server sedang sibuk. Coba lagi sebentar lagi.".into())
            }
            other => Self::internal(format!("database: {other}")),
        }
    }
}

impl From<std::io::Error> for AppError {
    fn from(error: std::io::Error) -> Self {
        Self::internal(format!("io: {error}"))
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let status = self.status();
        let body = match &self {
            Self::Validation(errors) => {
                let mut map = Map::new();
                map.insert("message".into(), Value::String(errors.summary()));
                map.insert("errors".into(), errors.to_json());
                Value::Object(map)
            }
            Self::Internal(detail) => {
                tracing::error!(error = %detail, "galat internal");
                json!({ "message": "Server Error" })
            }
            Self::PayloadTooLarge => json!({ "message": "The POST data is too large." }),
            other => json!({ "message": other.to_string() }),
        };

        let mut response = (status, axum::Json(body)).into_response();
        if let Self::TooManyRequests {
            limit,
            retry_after_secs,
            reset_at_unix,
        } = self
        {
            let headers = response.headers_mut();
            for (name, value) in [
                ("retry-after", retry_after_secs.to_string()),
                ("x-ratelimit-limit", limit.to_string()),
                ("x-ratelimit-remaining", "0".to_string()),
                ("x-ratelimit-reset", reset_at_unix.to_string()),
            ] {
                if let Ok(value) = HeaderValue::from_str(&value) {
                    headers.insert(name, value);
                }
            }
        }
        if status == StatusCode::SERVICE_UNAVAILABLE {
            response
                .headers_mut()
                .insert(header::RETRY_AFTER, HeaderValue::from_static("5"));
        }
        response
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use http_body_util::BodyExt;

    async fn body(error: AppError) -> (StatusCode, Value) {
        let response = error.into_response();
        let status = response.status();
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        (status, serde_json::from_slice(&bytes).unwrap())
    }

    #[tokio::test]
    async fn laravel_shapes() {
        assert_eq!(
            body(AppError::Unauthenticated).await,
            (
                StatusCode::UNAUTHORIZED,
                json!({"message": "Unauthenticated."})
            )
        );
        assert_eq!(
            body(AppError::rule("Pegawai tidak aktif.")).await,
            (
                StatusCode::UNPROCESSABLE_ENTITY,
                json!({"message": "Pegawai tidak aktif."})
            )
        );
        assert_eq!(
            body(AppError::internal("rahasia")).await,
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                json!({"message": "Server Error"})
            )
        );
    }

    #[tokio::test]
    async fn validation_shape() {
        let mut errors = ValidationErrors::default();
        errors.add("login", "Gabung wajib diisi.");
        errors.add("password", "Kata sandi wajib diisi.");
        let (status, value) = body(AppError::Validation(errors)).await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(
            value,
            json!({
                "message": "Gabung wajib diisi. (dan 1 kesalahan lainnya)",
                "errors": {"login": ["Gabung wajib diisi."], "password": ["Kata sandi wajib diisi."]}
            })
        );
    }

    #[tokio::test]
    async fn too_many_requests_headers() {
        let response = AppError::TooManyRequests {
            limit: 5,
            retry_after_secs: 42,
            reset_at_unix: 1_800_000_000,
        }
        .into_response();
        assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
        assert_eq!(response.headers()["retry-after"], "42");
        assert_eq!(response.headers()["x-ratelimit-limit"], "5");
    }
}
