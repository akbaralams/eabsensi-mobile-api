//! Cuti: `Api\LeaveController`, `LeaveRequestRequest`, `RequestLeave`, `LeaveResource`.

use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use chrono::NaiveDate;
use serde::Serialize;
use serde_json::{Value, json};

use crate::auth::{ActionLimited, AuthUser};
use crate::error::{AppError, AppResult};
use crate::input::Input;
use crate::net::ClientIp;
use crate::state::AppState;
use crate::time::naive_utc;
use crate::validation::{ValidationErrors, msg, php_integer, strict_date};

/// `RequestLeave::MAX_DAYS`: selisih hari (akhir - awal) harus kurang dari ini.
pub const MAX_DAYS: i64 = 62;
const LEAVE_LOCK_NAMESPACE: i64 = 0x4C45_4156 << 32; // "LEAV"

#[derive(Debug, Serialize, PartialEq)]
pub struct LeaveResource {
    pub id: i64,
    #[serde(rename = "type")]
    pub type_name: String,
    pub attendance_status_id: i64,
    pub start_date: String,
    pub end_date: String,
    pub reason: Option<String>,
    pub status: String,
    pub status_label: String,
}

/// `LeaveStatus::label()`.
pub fn status_label(status: &str) -> String {
    match status {
        "pending" => "Menunggu",
        "approved" => "Disetujui",
        "rejected" => "Ditolak",
        "cancelled" => "Dibatalkan",
        other => other,
    }
    .to_string()
}

struct LeaveRow {
    id: i64,
    type_name: Option<String>,
    attendance_status_id: i64,
    start_date: NaiveDate,
    end_date: NaiveDate,
    reason: Option<String>,
    status: String,
}

impl From<LeaveRow> for LeaveResource {
    fn from(r: LeaveRow) -> Self {
        Self {
            id: r.id,
            type_name: r.type_name.unwrap_or_default(),
            attendance_status_id: r.attendance_status_id,
            start_date: r.start_date.format("%Y-%m-%d").to_string(),
            end_date: r.end_date.format("%Y-%m-%d").to_string(),
            reason: r.reason,
            status_label: status_label(&r.status),
            status: r.status,
        }
    }
}

#[derive(Debug, Serialize)]
struct LeaveType {
    id: i64,
    code: String,
    name: String,
}

/// `GET /api/leaves`: 50 pengajuan terakhir milik sendiri (tanggal mulai terbaru dulu) dan jenis aktif.
pub async fn index(State(state): State<AppState>, user: AuthUser) -> AppResult<Json<Value>> {
    let employee_id = user.require_employee()?;
    let rows = sqlx::query_as!(
        LeaveRow,
        r#"SELECT l.id, s.name AS "type_name?", l.attendance_status_id, l.start_date, l.end_date, l.reason, l.status
           FROM leave_requests l
           LEFT JOIN attendance_statuses s ON s.id = l.attendance_status_id
           WHERE l.employee_id = $1
           ORDER BY l.start_date DESC, l.id DESC
           LIMIT 50"#,
        employee_id
    )
    .fetch_all(&state.db)
    .await?;
    let types = sqlx::query_as!(
        LeaveType,
        "SELECT id, code, name FROM attendance_statuses WHERE is_active ORDER BY name"
    )
    .fetch_all(&state.db)
    .await?;

    let data: Vec<LeaveResource> = rows.into_iter().map(Into::into).collect();
    Ok(Json(json!({ "data": data, "types": types })))
}

struct LeaveForm {
    attendance_status_id: i64,
    start_date: NaiveDate,
    end_date: NaiveDate,
    reason: Option<String>,
}

/// Aturan `LeaveRequestRequest`. `exists` hanya diperiksa bila nilainya bilangan bulat.
async fn validate(state: &AppState, input: &Input) -> AppResult<LeaveForm> {
    let mut errors = ValidationErrors::default();

    let status_field = "attendance_status_id";
    let status_id = match input.get(status_field) {
        None => {
            errors.add(status_field, msg::required(status_field));
            None
        }
        Some(value) => match php_integer(value) {
            None => {
                errors.add(status_field, msg::integer(status_field));
                None
            }
            Some(id) => {
                let exists = sqlx::query_scalar!(
                    r#"SELECT EXISTS (SELECT 1 FROM attendance_statuses WHERE id = $1 AND is_active) AS "exists!""#,
                    id
                )
                .fetch_one(&state.db)
                .await?;
                if exists {
                    Some(id)
                } else {
                    errors.add(status_field, msg::exists(status_field));
                    None
                }
            }
        },
    };

    let mut date = |field: &str| match input.get(field) {
        None => {
            errors.add(field, msg::required(field));
            None
        }
        Some(value) => {
            let parsed = strict_date(value);
            if parsed.is_none() {
                errors.add(field, msg::date_format(field, "Y-m-d"));
            }
            parsed
        }
    };
    let start = date("start_date");
    let end = date("end_date");
    if input.get("end_date").is_some() {
        match (start, end) {
            (Some(s), Some(e)) if e >= s => {}
            _ => errors.add("end_date", msg::after_or_equal("end_date", "start_date")),
        }
    }

    let reason = match input.get("reason") {
        None => None,
        Some(Value::String(s)) if s.chars().count() > 500 => {
            errors.add("reason", msg::max_string("reason", 500));
            None
        }
        Some(Value::String(s)) => Some(s.clone()),
        Some(_) => {
            errors.add("reason", msg::string("reason"));
            None
        }
    };

    errors.into_result()?;
    match (status_id, start, end) {
        (Some(attendance_status_id), Some(start_date), Some(end_date)) => Ok(LeaveForm {
            attendance_status_id,
            start_date,
            end_date,
            reason,
        }),
        _ => Err(AppError::internal("validasi cuti tidak konsisten")),
    }
}

/// `POST /api/leaves`: validasi, 403 tanpa pegawai, lalu `RequestLeave`.
pub async fn store(
    State(state): State<AppState>,
    ActionLimited(user): ActionLimited,
    ClientIp(ip): ClientIp,
    input: Input,
) -> AppResult<(StatusCode, Json<Value>)> {
    let form = validate(&state, &input).await?;
    let employee_id = user.require_employee()?;
    let leave = request_leave(&state, &user, employee_id, form, &ip.to_string()).await?;
    Ok((StatusCode::CREATED, Json(json!({ "data": leave }))))
}

async fn request_leave(
    state: &AppState,
    actor: &AuthUser,
    employee_id: i64,
    form: LeaveForm,
    ip: &str,
) -> AppResult<LeaveResource> {
    if (form.end_date - form.start_date).num_days() >= MAX_DAYS {
        return Err(AppError::rule(format!(
            "Rentang cuti maksimal {MAX_DAYS} hari."
        )));
    }

    let now = state.now();
    let mut tx = state.db.begin().await?;
    // Pengajuan bersamaan dari pegawai yang sama tidak boleh sama-sama lolos cek tumpang tindih.
    sqlx::query!(
        "SELECT pg_advisory_xact_lock($1)",
        LEAVE_LOCK_NAMESPACE | employee_id
    )
    .execute(&mut *tx)
    .await?;

    let overlap = sqlx::query_scalar!(
        r#"SELECT EXISTS (
               SELECT 1 FROM leave_requests
               WHERE employee_id = $1 AND status IN ('pending', 'approved')
                 AND start_date <= $3 AND end_date >= $2
           ) AS "overlap!""#,
        employee_id,
        form.start_date,
        form.end_date
    )
    .fetch_one(&mut *tx)
    .await?;
    if overlap {
        return Err(AppError::rule(
            "Pegawai sudah punya pengajuan yang menunggu atau disetujui pada tanggal tersebut.",
        ));
    }

    let created = naive_utc(now);
    let id = sqlx::query_scalar!(
        r#"INSERT INTO leave_requests
               (employee_id, attendance_status_id, start_date, end_date, reason, status, requested_by, created_at, updated_at)
           VALUES ($1, $2, $3, $4, $5, 'pending', $6, $7, $7)
           RETURNING id"#,
        employee_id,
        form.attendance_status_id,
        form.start_date,
        form.end_date,
        form.reason.as_deref(),
        actor.id,
        created
    )
    .fetch_one(&mut *tx)
    .await?;

    let (from, to) = (
        form.start_date.format("%Y-%m-%d").to_string(),
        form.end_date.format("%Y-%m-%d").to_string(),
    );
    // AuditLogger::log(AuditAction::LeaveRequested, ...): nama pengguna disalin, konteks tanpa data sensitif.
    let context = json!({
        "leave_id": id,
        "employee_id": employee_id,
        "attendance_status_id": form.attendance_status_id,
        "from": from,
        "to": to,
    });
    sqlx::query!(
        r#"INSERT INTO audit_logs (user_id, user_name, action, description, context, ip_address, created_at)
           VALUES ($1, $2, 'leave.requested', $3, $4::text::json, $5, $6)"#,
        actor.id,
        actor.name,
        format!("Pengajuan cuti pegawai #{employee_id}, {from} s/d {to}"),
        // Dikirim sebagai teks: biner JSONB akan mengurutkan ulang kunci, Laravel menyimpan urutan aslinya.
        context.to_string(),
        ip,
        now
    )
    .execute(&mut *tx)
    .await?;

    let row = sqlx::query_as!(
        LeaveRow,
        r#"SELECT l.id, s.name AS "type_name?", l.attendance_status_id, l.start_date, l.end_date, l.reason, l.status
           FROM leave_requests l
           LEFT JOIN attendance_statuses s ON s.id = l.attendance_status_id
           WHERE l.id = $1"#,
        id
    )
    .fetch_one(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(row.into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn labels() {
        assert_eq!(status_label("pending"), "Menunggu");
        assert_eq!(status_label("cancelled"), "Dibatalkan");
    }

    #[test]
    fn resource_shape() {
        let resource: LeaveResource = LeaveRow {
            id: 1,
            type_name: Some("Cuti".into()),
            attendance_status_id: 3,
            start_date: NaiveDate::from_ymd_opt(2099, 1, 5).unwrap(),
            end_date: NaiveDate::from_ymd_opt(2099, 1, 7).unwrap(),
            reason: None,
            status: "pending".into(),
        }
        .into();
        assert_eq!(
            serde_json::to_value(resource).unwrap(),
            json!({
                "id": 1, "type": "Cuti", "attendance_status_id": 3, "start_date": "2099-01-05",
                "end_date": "2099-01-07", "reason": null, "status": "pending", "status_label": "Menunggu"
            })
        );
    }
}
