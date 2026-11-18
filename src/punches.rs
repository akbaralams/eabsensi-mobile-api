//! Punch mobile: `RecordMobilePunch`, `Api\PunchController`, `PunchResource`.

use axum::Json;
use axum::extract::State;
use axum::http::{StatusCode, Uri};
use chrono::{DateTime, NaiveDate, Utc};
use serde::Serialize;
use serde_json::Value;
use utoipa::ToSchema;

use crate::auth::{ActionLimited, AuthUser, NO_EMPLOYEE};
use crate::employees;
use crate::error::{AppError, AppResult};
use crate::geofence::{self, CheckTarget};
use crate::input::Input;
use crate::openapi::{Data, EvidenceForm, Message, Unprocessable};
use crate::periods;
use crate::photos::{self, Evidence};
use crate::state::AppState;
use crate::time::{day_bounds_utc, iso8601, local_date, naive_utc};
use crate::validation::strict_date;

/// Ruang nama kunci advisory lock per pegawai (punch). Menyerialkan punch bersamaan dari pegawai yang
/// sama, agar dua request serentak tidak sama-sama lolos cek interval minimum.
const PUNCH_LOCK_NAMESPACE: i64 = 0x5055_4E43 << 32; // "PUNC"

#[derive(Debug, Serialize, PartialEq, ToSchema)]
pub struct PunchResource {
    pub id: i64,
    pub punched_at: String,
    pub source: String,
    pub status: String,
    pub status_label: String,
    pub has_photo: bool,
    /// Tanda lokasi untuk ditinjau.
    #[schema(required = false)]
    pub location_flags: Vec<String>,
}

/// `PunchStatus::label()`.
pub fn status_label(status: &str) -> String {
    match status {
        "approved" => "Disetujui",
        "pending" => "Menunggu persetujuan",
        "rejected" => "Ditolak",
        other => other,
    }
    .to_string()
}

struct PunchRow {
    id: i64,
    punched_at: DateTime<Utc>,
    source: String,
    status: String,
    has_photo: bool,
    /// Teks JSON kolom `attendance_location_checks.flags`.
    flags: String,
}

impl PunchRow {
    fn resource(self, state: &AppState) -> PunchResource {
        PunchResource {
            id: self.id,
            punched_at: iso8601(self.punched_at, state.config.timezone),
            source: self.source,
            status_label: status_label(&self.status),
            status: self.status,
            has_photo: self.has_photo,
            location_flags: geofence::parse_flags(&self.flags),
        }
    }
}

/// `?date=YYYY-MM-DD` (zona bisnis); kosong atau tidak valid berarti hari ini.
fn requested_date(uri: &Uri, today: NaiveDate) -> NaiveDate {
    uri.query()
        .and_then(|q| {
            form_urlencoded::parse(q.as_bytes())
                .filter(|(k, _)| k == "date")
                .last()
                .map(|(_, v)| v.into_owned())
        })
        .and_then(|v| strict_date(&Value::String(v)))
        .unwrap_or(today)
}

/// `GET /api/punches`: punch milik sendiri pada satu hari, semua status, terlama dulu.
#[utoipa::path(
    get, path = "/punches", tag = "Punch", operation_id = "listPunches",
    params(("date" = Option<String>, Query, description = "Tanggal di zona bisnis (YYYY-MM-DD). Kosong atau tidak valid berarti hari ini.")),
    responses(
        (status = 200, description = "Daftar punch, terlama dulu", body = Data<Vec<PunchResource>>),
        (status = 401, description = "Token tidak sah", body = Message),
        (status = 403, description = "Akun belum terhubung ke data pegawai", body = Message),
        (status = 429, description = "Batas laju terlampaui", body = Message),
    )
)]
pub async fn index(
    State(state): State<AppState>,
    user: AuthUser,
    uri: Uri,
) -> AppResult<Json<Data<Vec<PunchResource>>>> {
    let employee_id = user.require_employee()?;
    let tz = state.config.timezone;
    let date = requested_date(&uri, local_date(state.now(), tz));
    // Batas hari zona bisnis dikonversi ke UTC (Laravel tidak mengonversinya; lihat docs/laravel-differences.md).
    let (from, to) = day_bounds_utc(date, tz);

    let rows = sqlx::query_as!(
        PunchRow,
        r#"SELECT p.id, p.punched_at, p.source, p.status, p.photo_path IS NOT NULL AS "has_photo!",
                  COALESCE(c.flags, '[]'::jsonb)::text AS "flags!"
           FROM attendance_punches p
           LEFT JOIN attendance_location_checks c ON c.attendance_punch_id = p.id
           WHERE p.employee_id = $1 AND p.punched_at >= $2 AND p.punched_at < $3
           ORDER BY p.punched_at, p.id"#,
        employee_id,
        from,
        to
    )
    .fetch_all(&state.db)
    .await?;

    let data = rows.into_iter().map(|r| r.resource(&state)).collect();
    Ok(Json(Data { data }))
}

/// `POST /api/punches` (multipart): validasi form dulu, baru aturan bisnis.
#[utoipa::path(
    post, path = "/punches", tag = "Punch", operation_id = "createPunch",
    request_body(content = EvidenceForm, content_type = "multipart/form-data"),
    responses(
        (status = 201, description = "Tercatat", body = Data<PunchResource>),
        (status = 401, description = "Token tidak sah", body = Message),
        (status = 422, description = "Galat validasi atau penolakan aturan", body = Unprocessable),
        (status = 429, description = "Batas laju terlampaui", body = Message),
    )
)]
pub async fn store(
    State(state): State<AppState>,
    ActionLimited(user): ActionLimited,
    input: Input,
) -> AppResult<(StatusCode, Json<Data<PunchResource>>)> {
    let evidence = photos::validate_evidence(&input, state.config.requires_photo)?;
    let punch = record_mobile_punch(&state, &user, evidence).await?;
    Ok((StatusCode::CREATED, Json(Data { data: punch })))
}

/// `RecordMobilePunch::handle`. Waktu selalu dari server.
pub async fn record_mobile_punch(
    state: &AppState,
    user: &AuthUser,
    evidence: Evidence,
) -> AppResult<PunchResource> {
    let config = &state.config;
    let employee_id = user
        .employee_id
        .ok_or_else(|| AppError::rule(NO_EMPLOYEE))?;
    let employee = employees::find_with_department(&state.db, employee_id)
        .await?
        .ok_or_else(|| {
            AppError::NotFound(format!(
                "No query results for model [App\\Models\\Employee] {employee_id}"
            ))
        })?;
    if !employee.is_active {
        return Err(AppError::rule("Pegawai tidak aktif."));
    }

    // Dibaca sebelum transaksi agar request tidak memegang dua koneksi sekaligus.
    let location = geofence::for_employee(&state.db, employee.id).await?;

    let now = state.now();
    let mut tx = state.db.begin().await?;
    sqlx::query!(
        "SELECT pg_advisory_xact_lock($1)",
        PUNCH_LOCK_NAMESPACE | employee.id
    )
    .execute(&mut *tx)
    .await?;

    // Punch terakhir dari sumber mana pun dan status apa pun (termasuk mesin).
    let latest = sqlx::query_scalar!(
        "SELECT max(punched_at) FROM attendance_punches WHERE employee_id = $1",
        employee.id
    )
    .fetch_one(&mut *tx)
    .await?;
    let interval = config.min_interval_seconds;
    if let Some(latest) = latest
        && (now - latest).num_seconds().abs() < interval
    {
        return Err(AppError::rule(format!(
            "Tunggu {interval} detik sebelum melakukan punch lagi."
        )));
    }

    let today = local_date(now, config.timezone);
    if !periods::locked_months_between(&mut *tx, today, today, true)
        .await?
        .is_empty()
    {
        return Err(AppError::rule("Periode bulan ini sudah ditutup."));
    }

    let check = geofence::evaluate(&mut *tx, config, location, &evidence, employee.id, now).await?;

    let department_code = employee
        .department_code
        .as_deref()
        .ok_or_else(|| AppError::internal(format!("pegawai {} tanpa department", employee.id)))?;
    // Laravel membentuk folder setelah `$now` diubah ke zona bisnis, jadi tanggalnya tanggal lokal.
    let folder = format!(
        "mobile-punches/{}/{}",
        now.with_timezone(&config.timezone).format("%Y/%m/%d"),
        photos::safe_segment(department_code)
    );
    let photo_path = match &evidence.photo {
        Some(photo) => Some(photos::store(config, &folder, photo).await?),
        None => None,
    };

    let status = if config.requires_approval {
        "pending"
    } else {
        "approved"
    };
    let created = naive_utc(now);
    let inserted = async {
        let id = sqlx::query_scalar!(
            r#"INSERT INTO attendance_punches
                   (employee_id, punched_at, source, status, photo_path, latitude, longitude, accuracy_meters, created_at, updated_at)
               VALUES ($1, $2, 'mobile', $3, $4, $5::float8, $6::float8, $7, $8, $8)
               RETURNING id"#,
            employee.id,
            now,
            status,
            photo_path.as_deref(),
            evidence.latitude,
            evidence.longitude,
            evidence.accuracy,
            created
        )
        .fetch_one(&mut *tx)
        .await?;
        geofence::record(&mut *tx, CheckTarget::Punch(id), &check, created).await?;
        tx.commit().await?;
        Ok::<i64, AppError>(id)
    }
    .await;

    let id = match inserted {
        Ok(id) => id,
        Err(error) => {
            if let Some(path) = &photo_path {
                photos::remove(config, path).await;
            }
            return Err(error);
        }
    };

    Ok(PunchRow {
        id,
        punched_at: now,
        source: "mobile".into(),
        status: status.into(),
        has_photo: photo_path.is_some(),
        flags: check.flags_json(),
    }
    .resource(state))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn date_parameter() {
        let today = NaiveDate::from_ymd_opt(2026, 10, 29).unwrap();
        let uri = |s: &str| s.parse::<Uri>().unwrap();
        assert_eq!(requested_date(&uri("/api/punches"), today), today);
        assert_eq!(
            requested_date(&uri("/api/punches?date=2000-01-01"), today),
            NaiveDate::from_ymd_opt(2000, 1, 1).unwrap()
        );
        assert_eq!(
            requested_date(&uri("/api/punches?date=kemarin"), today),
            today
        );
        assert_eq!(
            requested_date(&uri("/api/punches?date=2026-02-30"), today),
            today
        );
    }

    #[test]
    fn labels() {
        assert_eq!(status_label("pending"), "Menunggu persetujuan");
        assert_eq!(status_label("approved"), "Disetujui");
        assert_eq!(status_label("rejected"), "Ditolak");
    }
}
