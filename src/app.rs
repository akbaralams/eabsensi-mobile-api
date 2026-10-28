//! Router dan middleware. Semua respons galat berbentuk JSON Laravel, juga yang dibuat middleware
//! (timeout, panic, payload terlalu besar), dengan atau tanpa header `Accept`.

use axum::body::Body;
use axum::extract::{DefaultBodyLimit, Request, State};
use axum::http::{HeaderName, HeaderValue, Method, StatusCode, Uri, header};
use axum::middleware::{self, Next};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde_json::json;
use tower::Layer;
use tower_http::catch_panic::CatchPanicLayer;
use tower_http::cors::{Any, CorsLayer};
use tower_http::normalize_path::{NormalizePath, NormalizePathLayer};
use tower_http::request_id::{MakeRequestUuid, PropagateRequestIdLayer, SetRequestIdLayer};
use tower_http::sensitive_headers::SetSensitiveRequestHeadersLayer;
use tower_http::timeout::TimeoutLayer;
use tower_http::trace::{DefaultOnResponse, TraceLayer};
use tracing::Level;

use crate::error::AppError;
use crate::state::AppState;
use crate::{auth, events, health, leaves, punches};

/// Batas body request (foto maks 4 MB + field lain).
pub const BODY_LIMIT: usize = 5 * 1024 * 1024;

const REQUEST_ID: HeaderName = HeaderName::from_static("x-request-id");

/// Aplikasi lengkap: router + middleware, dengan garis miring di akhir path diabaikan seperti Laravel.
pub fn app(state: AppState) -> NormalizePath<Router> {
    NormalizePathLayer::trim_trailing_slash().layer(router(state))
}

pub fn router(state: AppState) -> Router {
    let api = Router::new()
        .route("/auth/login", post(auth::login))
        .route("/auth/logout", post(auth::logout))
        .route("/me", get(auth::me))
        .route("/punches", get(punches::index).post(punches::store))
        .route("/events", get(events::index))
        .route("/events/{event}/check-in", post(events::check_in))
        .route("/leaves", get(leaves::index).post(leaves::store))
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            limit_concurrency,
        ));

    let timeout = state.config.request_timeout;

    Router::new()
        .nest("/api", api)
        .route("/healthz", get(health::healthz))
        .route("/livez", get(health::livez))
        .fallback(route_not_found)
        .method_not_allowed_fallback(method_not_allowed)
        .layer(DefaultBodyLimit::max(BODY_LIMIT))
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any),
        )
        .layer(TimeoutLayer::with_status_code(
            StatusCode::SERVICE_UNAVAILABLE,
            timeout,
        ))
        .layer(CatchPanicLayer::custom(|_| {
            tracing::error!("handler panik");
            AppError::internal("panic").into_response()
        }))
        .layer(middleware::from_fn_with_state(state.clone(), finalize))
        .layer(PropagateRequestIdLayer::new(REQUEST_ID))
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(|req: &Request<Body>| {
                    let id = req
                        .headers()
                        .get(&REQUEST_ID)
                        .and_then(|v| v.to_str().ok())
                        .unwrap_or("");
                    tracing::info_span!("request", method = %req.method(), path = %req.uri().path(), request_id = %id)
                })
                .on_response(DefaultOnResponse::new().level(Level::INFO)),
        )
        .layer(SetRequestIdLayer::new(REQUEST_ID, MakeRequestUuid))
        .layer(SetSensitiveRequestHeadersLayer::new([header::AUTHORIZATION]))
        .with_state(state)
}

/// Request `/api` yang diproses bersamaan dibatasi; kelebihannya menunggu giliran (bukan ditolak)
/// sampai batas waktu request. Melindungi memori (unggahan foto) dan pool database saat lonjakan.
async fn limit_concurrency(State(state): State<AppState>, req: Request, next: Next) -> Response {
    let Ok(_permit) = state.request_slots.clone().acquire_owned().await else {
        return AppError::ServiceUnavailable(
            "Server sedang sibuk. Coba lagi sebentar lagi.".into(),
        )
        .into_response();
    };
    next.run(req).await
}

/// Header keamanan (sama dengan `SecurityHeaders` Laravel untuk `/api`) dan jaminan galat selalu JSON.
async fn finalize(State(state): State<AppState>, req: Request, next: Next) -> Response {
    let (parts, body) = req.into_parts();
    let secure = crate::net::is_secure(&parts, &state.config.trusted_proxies);
    let mut response = next.run(Request::from_parts(parts, body)).await;

    let status = response.status();
    let is_json = response
        .headers()
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .is_some_and(|v| v.contains("json"));
    if status.as_u16() >= 400 && !is_json {
        let message = match status {
            StatusCode::INTERNAL_SERVER_ERROR => "Server Error",
            StatusCode::PAYLOAD_TOO_LARGE => "The POST data is too large.",
            StatusCode::SERVICE_UNAVAILABLE => "Server sedang sibuk. Coba lagi sebentar lagi.",
            other => other.canonical_reason().unwrap_or("Error"),
        };
        let mut replaced = (status, Json(json!({ "message": message }))).into_response();
        for (name, value) in response.headers() {
            if name != header::CONTENT_TYPE && name != header::CONTENT_LENGTH {
                replaced.headers_mut().insert(name.clone(), value.clone());
            }
        }
        response = replaced;
    }

    let headers = response.headers_mut();
    for (name, value) in [
        ("x-content-type-options", "nosniff"),
        ("x-frame-options", "DENY"),
        ("referrer-policy", "strict-origin-when-cross-origin"),
        (
            "permissions-policy",
            "camera=(), microphone=(), geolocation=(), payment=()",
        ),
        ("cross-origin-opener-policy", "same-origin"),
    ] {
        headers.insert(name, HeaderValue::from_static(value));
    }
    if secure {
        headers.insert(
            header::STRICT_TRANSPORT_SECURITY,
            HeaderValue::from_static("max-age=31536000; includeSubDomains"),
        );
    }
    response
}

/// `$request->path()` Laravel: tanpa garis miring di awal/akhir; root menjadi `/`.
pub fn laravel_path(uri: &Uri) -> String {
    let trimmed = uri.path().trim_matches('/');
    if trimmed.is_empty() {
        "/".into()
    } else {
        trimmed.into()
    }
}

pub fn route_not_found_error(uri: &Uri) -> AppError {
    AppError::NotFound(format!(
        "The route {} could not be found.",
        laravel_path(uri)
    ))
}

async fn route_not_found(uri: Uri) -> AppError {
    route_not_found_error(&uri)
}

async fn method_not_allowed(method: Method, uri: Uri) -> AppError {
    let path = laravel_path(&uri);
    let allowed = match path.as_str() {
        "api/auth/login" | "api/auth/logout" => "POST",
        "api/me" | "api/events" | "healthz" | "livez" => "GET, HEAD",
        "api/punches" | "api/leaves" => "GET, HEAD, POST",
        p if p.starts_with("api/events/") && p.ends_with("/check-in") => "POST",
        _ => return route_not_found_error(&uri),
    };
    AppError::MethodNotAllowed(format!(
        "The {method} method is not supported for route {path}. Supported methods: {allowed}."
    ))
}
