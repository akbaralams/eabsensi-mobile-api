//! Event (mis. upacara): `Api\EventController`, `CheckInToEvent`, `EventResource`.

use axum::Json;
use axum::extract::{FromRequestParts, OriginalUri, State};
use axum::http::StatusCode;
use axum::http::request::Parts;
use chrono::{NaiveDate, NaiveTime};
use serde::Serialize;
use utoipa::ToSchema;

use crate::app::route_not_found_error;
use crate::auth::{ActionLimited, AuthUser, NO_EMPLOYEE};
use crate::employees;
use crate::error::{AppError, AppResult};
use crate::input::Input;
use crate::openapi::{Data, EvidenceForm, Message, Unprocessable};
use crate::photos::{self, Evidence};
use crate::state::AppState;
use crate::time::{iso8601, local_date, naive_utc};

#[derive(Debug, Serialize, PartialEq, ToSchema)]
pub struct EventResource {
    pub id: i64,
    pub name: String,
    pub event_date: String,
    pub starts_at: String,
    pub ends_at: String,
    pub checked_in: bool,
}

#[derive(Debug, Serialize, PartialEq, ToSchema)]
pub struct EventCheckIn {
    pub event_id: i64,
    pub checked_in_at: String,
}

fn hhmm(time: NaiveTime) -> String {
    time.format("%H:%M").to_string()
}

struct EventListRow {
    id: i64,
    name: String,
    event_date: NaiveDate,
    starts_at: NaiveTime,
    ends_at: NaiveTime,
    checked_in: bool,
}

/// `GET /api/events`: event aktif hari ini (zona bisnis) untuk semua pegawai, atau untuk department
/// pegawai beserta seluruh induknya; urut jam mulai.
#[utoipa::path(
    get, path = "/events", tag = "Event", operation_id = "listEvents",
    responses(
        (status = 200, description = "Daftar event", body = Data<Vec<EventResource>>),
        (status = 401, description = "Token tidak sah", body = Message),
        (status = 403, description = "Akun belum terhubung ke data pegawai", body = Message),
        (status = 429, description = "Batas laju terlampaui", body = Message),
    )
)]
pub async fn index(
    State(state): State<AppState>,
    user: AuthUser,
) -> AppResult<Json<Data<Vec<EventResource>>>> {
    let employee_id = user.require_employee()?;
    let today = local_date(state.now(), state.config.timezone);

    let rows = sqlx::query_as!(
        EventListRow,
        r#"WITH RECURSIVE ancestors(id) AS (
               SELECT department_id FROM employees WHERE id = $1
               UNION
               SELECT d.parent_id FROM departments d JOIN ancestors a ON d.id = a.id
               WHERE d.parent_id IS NOT NULL
           )
           SELECT e.id, e.name, e.event_date, e.starts_at, e.ends_at,
                  EXISTS (
                      SELECT 1 FROM event_attendances ea WHERE ea.event_id = e.id AND ea.employee_id = $1
                  ) AS "checked_in!"
           FROM attendance_events e
           WHERE e.is_active
             AND e.event_date = $2
             AND (e.department_id IS NULL OR e.department_id IN (SELECT id FROM ancestors))
           ORDER BY e.starts_at, e.id"#,
        employee_id,
        today
    )
    .fetch_all(&state.db)
    .await?;

    let data = rows
        .into_iter()
        .map(|r| EventResource {
            id: r.id,
            name: r.name,
            event_date: r.event_date.format("%Y-%m-%d").to_string(),
            starts_at: hhmm(r.starts_at),
            ends_at: hhmm(r.ends_at),
            checked_in: r.checked_in,
        })
        .collect();
    Ok(Json(Data { data }))
}

/// Segmen `{event}` harus angka (`whereNumber`); selain itu rute dianggap tidak ada, sebelum autentikasi.
pub struct EventId(pub String);

impl FromRequestParts<AppState> for EventId {
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, _: &AppState) -> Result<Self, Self::Rejection> {
        let segment = parts
            .uri
            .path()
            .trim_end_matches('/')
            .trim_end_matches("/check-in")
            .rsplit('/')
            .next()
            .unwrap_or_default()
            .to_string();
        if segment.is_empty() || !segment.bytes().all(|b| b.is_ascii_digit()) {
            // Di dalam router bersarang `/api` sudah terpotong; pesan Laravel memakai path lengkap.
            let uri = parts
                .extensions
                .get::<OriginalUri>()
                .map_or_else(|| parts.uri.clone(), |o| o.0.clone());
            return Err(route_not_found_error(&uri));
        }
        Ok(Self(segment))
    }
}

struct EventRow {
    id: i64,
    event_date: NaiveDate,
    starts_at: NaiveTime,
    ends_at: NaiveTime,
    department_id: Option<i64>,
    is_active: bool,
}

/// `POST /api/events/{event}/check-in`: validasi form, cari event (404), lalu aturan `CheckInToEvent`.
#[utoipa::path(
    post, path = "/events/{event}/check-in", tag = "Event", operation_id = "checkInEvent",
    params(("event" = i64, Path, description = "Id event")),
    request_body(content = EvidenceForm, content_type = "multipart/form-data"),
    responses(
        (status = 201, description = "Tercatat", body = Data<EventCheckIn>),
        (status = 401, description = "Token tidak sah", body = Message),
        (status = 404, description = "Event tidak ditemukan", body = Message),
        (status = 422, description = "Galat validasi atau penolakan aturan", body = Unprocessable),
        (status = 429, description = "Batas laju terlampaui", body = Message),
    )
)]
pub async fn check_in(
    EventId(segment): EventId,
    State(state): State<AppState>,
    ActionLimited(user): ActionLimited,
    input: Input,
) -> AppResult<(StatusCode, Json<Data<EventCheckIn>>)> {
    let evidence = photos::validate_evidence(&input, state.config.requires_photo)?;

    let not_found = || {
        AppError::NotFound(format!(
            "No query results for model [App\\Models\\AttendanceEvent] {segment}"
        ))
    };
    let id: i64 = segment.parse().map_err(|_| not_found())?;
    let event = sqlx::query_as!(
        EventRow,
        "SELECT id, event_date, starts_at, ends_at, department_id, is_active FROM attendance_events WHERE id = $1",
        id
    )
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(not_found)?;

    let checked_in_at = record_check_in(&state, &user, &event, evidence).await?;
    Ok((
        StatusCode::CREATED,
        Json(Data {
            data: EventCheckIn {
                event_id: event.id,
                checked_in_at,
            },
        }),
    ))
}

const ALREADY: &str = "Anda sudah check-in di event ini.";

async fn record_check_in(
    state: &AppState,
    user: &AuthUser,
    event: &EventRow,
    evidence: Evidence,
) -> AppResult<String> {
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
    if !event.is_active {
        return Err(AppError::rule("Event ini tidak aktif."));
    }
    if let Some(department_id) = event.department_id {
        // Pegawai harus berada di department event atau turunannya.
        let eligible = sqlx::query_scalar!(
            r#"WITH RECURSIVE tree(id) AS (
                   SELECT id FROM departments WHERE id = $1
                   UNION
                   SELECT d.id FROM departments d JOIN tree t ON d.parent_id = t.id
               )
               SELECT EXISTS (
                   SELECT 1 FROM employees WHERE id = $2 AND department_id IN (SELECT id FROM tree)
               ) AS "eligible!""#,
            department_id,
            employee.id
        )
        .fetch_one(&state.db)
        .await?;
        if !eligible {
            return Err(AppError::rule("Anda tidak termasuk peserta event ini."));
        }
    }

    let now = state.now();
    let local = now.with_timezone(&config.timezone);
    if local.date_naive() != event.event_date {
        return Err(AppError::rule(
            "Check-in hanya bisa dilakukan pada tanggal event.",
        ));
    }
    // Dibandingkan per menit (H:i), seperti Laravel: pukul 09:00:59 masih termasuk "sampai 09:00".
    let (start, end, current) = (
        hhmm(event.starts_at),
        hhmm(event.ends_at),
        local.format("%H:%M").to_string(),
    );
    if current < start || current > end {
        return Err(AppError::rule(format!(
            "Check-in dibuka pukul {start} sampai {end}."
        )));
    }

    let already = sqlx::query_scalar!(
        r#"SELECT EXISTS (SELECT 1 FROM event_attendances WHERE event_id = $1 AND employee_id = $2) AS "exists!""#,
        event.id,
        employee.id
    )
    .fetch_one(&state.db)
    .await?;
    if already {
        return Err(AppError::rule(ALREADY));
    }

    let department_code = employee
        .department_code
        .as_deref()
        .ok_or_else(|| AppError::internal(format!("pegawai {} tanpa department", employee.id)))?;
    // CheckInToEvent memakai `$now` UTC untuk folder (berbeda dengan punch).
    let folder = format!(
        "event-attendance/{}/{}",
        now.format("%Y/%m/%d"),
        photos::safe_segment(department_code)
    );
    let photo_path = match &evidence.photo {
        Some(photo) => Some(photos::store(config, &folder, photo).await?),
        None => None,
    };

    let created = naive_utc(now);
    // Unique (event_id, employee_id) menjaga satu check-in per pegawai, juga saat ribuan pegawai
    // check-in bersamaan: request kedua yang lolos cek di atas tetap tidak membuat baris ganda.
    let inserted = sqlx::query_scalar!(
        r#"INSERT INTO event_attendances
               (event_id, employee_id, checked_in_at, photo_path, latitude, longitude, accuracy_meters, created_at, updated_at)
           VALUES ($1, $2, $3, $4, $5::float8, $6::float8, $7, $8, $8)
           ON CONFLICT (event_id, employee_id) DO NOTHING
           RETURNING id"#,
        event.id,
        employee.id,
        now,
        photo_path.as_deref(),
        evidence.latitude,
        evidence.longitude,
        evidence.accuracy,
        created
    )
    .fetch_optional(&state.db)
    .await;

    match inserted {
        Ok(Some(_)) => Ok(iso8601(now, config.timezone)),
        Ok(None) => {
            if let Some(path) = &photo_path {
                photos::remove(config, path).await;
            }
            Err(AppError::rule(ALREADY))
        }
        Err(error) => {
            if let Some(path) = &photo_path {
                photos::remove(config, path).await;
            }
            Err(error.into())
        }
    }
}
