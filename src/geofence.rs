//! Lokasi kerja: `WorkLocationResolver` dan `LocationPolicyEvaluator` eabsensi-v2. Teks penolakan dan nilai
//! tanda bagian dari kontrak dan harus sama persis dengan Laravel.

use std::collections::HashMap;

use chrono::{DateTime, NaiveDateTime, Utc};
use serde::Serialize;
use sqlx::{PgExecutor, PgPool};
use utoipa::ToSchema;

use crate::config::Config;
use crate::error::{AppError, AppResult};
use crate::photos::Evidence;

pub const MESSAGE_LOCATION_REQUIRED: &str = "Lokasi wajib diaktifkan untuk melakukan absen.";
pub const MESSAGE_UPDATE_APP: &str = "Perbarui aplikasi ke versi terbaru untuk melakukan absen.";
pub const MESSAGE_MOCKED: &str =
    "Lokasi palsu terdeteksi. Matikan aplikasi pengubah lokasi lalu coba lagi.";
pub const MESSAGE_STALE: &str = "Lokasi sudah kedaluwarsa. Silakan coba lagi.";
pub const MESSAGE_WEAK_SIGNAL: &str = "Sinyal GPS lemah. Pindah ke area terbuka lalu coba lagi.";

pub const FLAG_OUTSIDE_RADIUS: &str = "outside_radius";
pub const FLAG_IMPOSSIBLE_TRAVEL: &str = "impossible_travel";
pub const FLAG_ROOTED_DEVICE: &str = "rooted_device";
pub const FLAG_MOCK_LOCATION: &str = "mock_location";

const EARTH_RADIUS_METERS: f64 = 6_371_000.0;

/// Lokasi yang berlaku untuk sebuah absen.
#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedLocation {
    pub id: i64,
    pub name: String,
    pub latitude: f64,
    pub longitude: f64,
    pub radius_meters: i32,
}

/// `WorkLocationResource`: lokasi yang berlaku, untuk menggambar radius di peta aplikasi.
#[derive(Debug, Clone, Serialize, PartialEq, ToSchema)]
pub struct WorkLocation {
    pub name: String,
    pub latitude: f64,
    pub longitude: f64,
    pub radius_meters: i32,
}

impl From<&ResolvedLocation> for WorkLocation {
    fn from(location: &ResolvedLocation) -> Self {
        Self {
            name: location.name.clone(),
            latitude: location.latitude,
            longitude: location.longitude,
            radius_meters: location.radius_meters,
        }
    }
}

/// Hasil pemeriksaan untuk absen yang diterima.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct LocationCheck {
    pub location: Option<ResolvedLocation>,
    pub distance_meters: Option<i32>,
    pub flags: Vec<String>,
}

impl LocationCheck {
    /// Absen tanpa lokasi yang berlaku dan tanpa tanda tidak perlu dicatat.
    fn should_be_recorded(&self) -> bool {
        self.location.is_some() || !self.flags.is_empty()
    }

    pub fn flags_json(&self) -> String {
        serde_json::to_string(&self.flags).unwrap_or_else(|_| "[]".into())
    }
}

/// Isi kolom `flags` (teks JSON) sebagai daftar tanda.
pub fn parse_flags(json: &str) -> Vec<String> {
    serde_json::from_str(json).unwrap_or_default()
}

/// Bebas Lokasi > Pegawai > Department > induk department terdekat. Lokasi nonaktif dilewati.
pub async fn for_employee(db: &PgPool, employee_id: i64) -> AppResult<Option<ResolvedLocation>> {
    let location = sqlx::query_as!(
        ResolvedLocation,
        r#"WITH RECURSIVE employee AS (
               SELECT department_id, work_location_id, location_exempt FROM employees WHERE id = $1
           ), chain(id, parent_id, work_location_id, depth) AS (
               SELECT d.id, d.parent_id, d.work_location_id, 1
               FROM departments d JOIN employee e ON d.id = e.department_id
               UNION ALL
               SELECT d.id, d.parent_id, d.work_location_id, c.depth + 1
               FROM departments d JOIN chain c ON d.id = c.parent_id
               WHERE c.depth < 33
           ), candidates(work_location_id, depth) AS (
               SELECT work_location_id, 0 FROM employee
               UNION ALL
               SELECT work_location_id, depth FROM chain
           )
           SELECT wl.id AS "id!", wl.name AS "name!", wl.latitude::float8 AS "latitude!",
                  wl.longitude::float8 AS "longitude!", wl.radius_meters AS "radius_meters!"
           FROM candidates c
           JOIN work_locations wl ON wl.id = c.work_location_id
           WHERE wl.is_active AND NOT (SELECT location_exempt FROM employee)
           ORDER BY c.depth
           LIMIT 1"#,
        employee_id
    )
    .fetch_optional(db)
    .await?;
    Ok(location)
}

/// Lokasi yang berlaku untuk event-event seorang pegawai, dimuat sekali untuk seluruh daftar.
pub struct EventLocations {
    exempt: bool,
    employee: Option<ResolvedLocation>,
    custom: HashMap<i64, ResolvedLocation>,
}

impl EventLocations {
    pub async fn load(db: &PgPool, employee_id: i64, custom_ids: &[i64]) -> AppResult<Self> {
        let exempt = sqlx::query_scalar!(
            r#"SELECT location_exempt AS "exempt!" FROM employees WHERE id = $1"#,
            employee_id
        )
        .fetch_optional(db)
        .await?
        .unwrap_or(false);

        let custom = sqlx::query_as!(
            ResolvedLocation,
            r#"SELECT id, name, latitude::float8 AS "latitude!", longitude::float8 AS "longitude!", radius_meters
               FROM work_locations
               WHERE is_active AND id = ANY($1)"#,
            custom_ids
        )
        .fetch_all(db)
        .await?
        .into_iter()
        .map(|location| (location.id, location))
        .collect();

        Ok(Self {
            exempt,
            employee: for_employee(db, employee_id).await?,
            custom,
        })
    }

    /// Pegawai Bebas Lokasi tidak dibatasi radius, termasuk pada event berlokasi khusus.
    pub fn resolve(&self, mode: &str, custom_id: Option<i64>) -> Option<&ResolvedLocation> {
        if self.exempt {
            return None;
        }
        match mode {
            "employee" => self.employee.as_ref(),
            "custom" => custom_id.and_then(|id| self.custom.get(&id)),
            _ => None,
        }
    }
}

/// Lokasi yang berlaku untuk check-in satu event.
pub async fn for_event(
    db: &PgPool,
    employee_id: i64,
    mode: &str,
    custom_id: Option<i64>,
) -> AppResult<Option<ResolvedLocation>> {
    if mode == "none" {
        return Ok(None);
    }
    let custom_ids: Vec<i64> = custom_id.into_iter().collect();
    let locations = EventLocations::load(db, employee_id, &custom_ids).await?;
    Ok(locations.resolve(mode, custom_id).cloned())
}

/// Jarak dua titik dengan rumus Haversine, dalam meter.
pub fn distance_meters(from: (f64, f64), to: (f64, f64)) -> f64 {
    let latitude_delta = (to.0 - from.0).to_radians();
    let longitude_delta = (to.1 - from.1).to_radians();
    let a = (latitude_delta / 2.0).sin().powi(2)
        + from.0.to_radians().cos()
            * to.0.to_radians().cos()
            * (longitude_delta / 2.0).sin().powi(2);
    2.0 * EARTH_RADIUS_METERS * a.sqrt().min(1.0).asin()
}

/// Bukti lokasi wajib lengkap dan dapat dipercaya bila lokasi berlaku; hasilnya koordinat absen.
fn trusted_coordinates(evidence: &Evidence, config: &Config) -> AppResult<(f64, f64)> {
    let (Some(latitude), Some(longitude)) = (evidence.latitude, evidence.longitude) else {
        return Err(AppError::rule(MESSAGE_LOCATION_REQUIRED));
    };
    let (Some(is_mocked), Some(age_seconds), Some(_)) = (
        evidence.is_mocked,
        evidence.location_age_seconds,
        evidence.device_rooted,
    ) else {
        return Err(AppError::rule(MESSAGE_UPDATE_APP));
    };
    if is_mocked {
        return Err(AppError::rule(MESSAGE_MOCKED));
    }
    if age_seconds > config.location_max_fix_age_seconds {
        return Err(AppError::rule(MESSAGE_STALE));
    }
    match evidence.accuracy {
        Some(accuracy) if i64::from(accuracy) <= config.location_max_accuracy_meters => {
            Ok((latitude, longitude))
        }
        _ => Err(AppError::rule(MESSAGE_WEAK_SIGNAL)),
    }
}

/// Pegawai tanpa lokasi yang berlaku tidak pernah ditolak; sinyal mencurigakan hanya ditandai.
fn unenforced_flags(evidence: &Evidence) -> Vec<String> {
    let mut flags = Vec::new();
    if evidence.is_mocked == Some(true) {
        flags.push(FLAG_MOCK_LOCATION.to_string());
    }
    if evidence.device_rooted == Some(true) {
        flags.push(FLAG_ROOTED_DEVICE.to_string());
    }
    flags
}

/// Kecepatan dari titik sebelumnya melebihi batas wajar.
fn is_impossible_travel(
    previous: (DateTime<Utc>, f64, f64),
    current: (f64, f64),
    now: DateTime<Utc>,
    max_speed_kmh: i64,
) -> bool {
    let (at, latitude, longitude) = previous;
    let seconds = (now - at).num_milliseconds().abs() as f64 / 1000.0;
    if seconds <= 0.0 {
        return false;
    }
    let distance = distance_meters((latitude, longitude), current);
    distance / seconds * 3.6 > max_speed_kmh as f64
}

struct PreviousFix {
    punched_at: DateTime<Utc>,
    latitude: f64,
    longitude: f64,
}

/// `LocationPolicyEvaluator::evaluate`: menolak bukti yang tidak dapat dipercaya, selain itu memberi tanda.
pub async fn evaluate<'c>(
    executor: impl PgExecutor<'c>,
    config: &Config,
    location: Option<ResolvedLocation>,
    evidence: &Evidence,
    employee_id: i64,
    now: DateTime<Utc>,
) -> AppResult<LocationCheck> {
    let Some(location) = location else {
        return Ok(LocationCheck {
            flags: unenforced_flags(evidence),
            ..LocationCheck::default()
        });
    };

    let current = trusted_coordinates(evidence, config)?;
    let distance = distance_meters(current, (location.latitude, location.longitude));
    let mut flags = Vec::new();
    if distance > f64::from(location.radius_meters) {
        flags.push(FLAG_OUTSIDE_RADIUS.to_string());
    }

    // Punch mobile terakhir yang punya koordinat, status apa pun.
    let previous = sqlx::query_as!(
        PreviousFix,
        r#"SELECT punched_at, latitude::float8 AS "latitude!", longitude::float8 AS "longitude!"
           FROM attendance_punches
           WHERE employee_id = $1 AND source = 'mobile' AND latitude IS NOT NULL AND longitude IS NOT NULL
           ORDER BY punched_at DESC, id DESC
           LIMIT 1"#,
        employee_id
    )
    .fetch_optional(executor)
    .await?;
    if let Some(previous) = previous
        && is_impossible_travel(
            (previous.punched_at, previous.latitude, previous.longitude),
            current,
            now,
            config.location_max_speed_kmh,
        )
    {
        flags.push(FLAG_IMPOSSIBLE_TRAVEL.to_string());
    }

    if evidence.device_rooted == Some(true) {
        flags.push(FLAG_ROOTED_DEVICE.to_string());
    }

    Ok(LocationCheck {
        location: Some(location),
        distance_meters: Some(distance.round() as i32),
        flags,
    })
}

/// Baris yang diperiksa: punch mobile atau check-in event.
#[derive(Debug, Clone, Copy)]
pub enum CheckTarget {
    Punch(i64),
    EventAttendance(i64),
}

/// Simpan snapshot pemeriksaan ke `attendance_location_checks` bila ada yang perlu dicatat.
pub async fn record<'c>(
    executor: impl PgExecutor<'c>,
    target: CheckTarget,
    check: &LocationCheck,
    created: NaiveDateTime,
) -> AppResult<()> {
    if !check.should_be_recorded() {
        return Ok(());
    }
    let (punch_id, attendance_id) = match target {
        CheckTarget::Punch(id) => (Some(id), None),
        CheckTarget::EventAttendance(id) => (None, Some(id)),
    };
    sqlx::query!(
        r#"INSERT INTO attendance_location_checks
               (attendance_punch_id, event_attendance_id, work_location_id, location_name, radius_meters,
                distance_meters, flags, is_flagged, created_at, updated_at)
           VALUES ($1, $2, $3, $4, $5, $6, $7::text::jsonb, $8, $9, $9)"#,
        punch_id,
        attendance_id,
        check.location.as_ref().map(|location| location.id),
        check.location.as_ref().map(|location| location.name.as_str()),
        check.location.as_ref().map(|location| location.radius_meters),
        check.distance_meters,
        check.flags_json(),
        !check.flags.is_empty(),
        created
    )
    .execute(executor)
    .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn config() -> Config {
        Config::from_lookup(|key| (key == "DATABASE_URL").then(|| "postgres://x".to_string()))
            .unwrap()
    }

    fn trusted() -> Evidence {
        Evidence {
            latitude: Some(-6.2),
            longitude: Some(106.8),
            accuracy: Some(10),
            is_mocked: Some(false),
            location_age_seconds: Some(5),
            device_rooted: Some(false),
            ..Evidence::default()
        }
    }

    fn rejection(evidence: &Evidence) -> String {
        match trusted_coordinates(evidence, &config()) {
            Err(AppError::Rule(message)) => message,
            other => panic!("harus ditolak, dapat {other:?}"),
        }
    }

    #[test]
    fn distance_matches_the_shared_test_vector() {
        let distance = distance_meters((-6.2, 106.8), (-6.21, 106.8));
        assert!((distance - 1111.95).abs() < 0.01, "{distance}");
        assert_eq!(distance_meters((-6.2, 106.8), (-6.2, 106.8)), 0.0);
    }

    #[test]
    fn untrustworthy_evidence_is_rejected_in_order() {
        let no_coordinates = Evidence {
            is_mocked: Some(true),
            ..Evidence::default()
        };
        assert_eq!(rejection(&no_coordinates), MESSAGE_LOCATION_REQUIRED);

        let old_app = Evidence {
            is_mocked: None,
            ..trusted()
        };
        assert_eq!(rejection(&old_app), MESSAGE_UPDATE_APP);

        let mocked = Evidence {
            is_mocked: Some(true),
            location_age_seconds: Some(999),
            accuracy: Some(999),
            ..trusted()
        };
        assert_eq!(rejection(&mocked), MESSAGE_MOCKED);

        let stale = Evidence {
            location_age_seconds: Some(121),
            accuracy: Some(999),
            ..trusted()
        };
        assert_eq!(rejection(&stale), MESSAGE_STALE);

        let weak = Evidence {
            accuracy: Some(101),
            ..trusted()
        };
        assert_eq!(rejection(&weak), MESSAGE_WEAK_SIGNAL);

        let no_accuracy = Evidence {
            accuracy: None,
            ..trusted()
        };
        assert_eq!(rejection(&no_accuracy), MESSAGE_WEAK_SIGNAL);
    }

    #[test]
    fn evidence_at_the_limits_is_accepted() {
        let evidence = Evidence {
            location_age_seconds: Some(120),
            accuracy: Some(100),
            ..trusted()
        };
        assert_eq!(
            trusted_coordinates(&evidence, &config()).unwrap(),
            (-6.2, 106.8)
        );
    }

    #[test]
    fn unenforced_signals_are_only_flagged() {
        let evidence = Evidence {
            is_mocked: Some(true),
            device_rooted: Some(true),
            ..Evidence::default()
        };
        assert_eq!(
            unenforced_flags(&evidence),
            [FLAG_MOCK_LOCATION, FLAG_ROOTED_DEVICE]
        );
        assert!(unenforced_flags(&Evidence::default()).is_empty());
    }

    #[test]
    fn travel_speed_uses_the_elapsed_time() {
        let now = Utc.with_ymd_and_hms(2026, 8, 17, 0, 30, 0).unwrap();
        let bandung = (now - chrono::TimeDelta::minutes(5), -6.9, 107.6);
        assert!(is_impossible_travel(bandung, (-6.2, 106.8), now, 250));

        let hours_ago = (now - chrono::TimeDelta::hours(3), -6.9, 107.6);
        assert!(!is_impossible_travel(hours_ago, (-6.2, 106.8), now, 250));
        assert!(!is_impossible_travel(
            (now, -6.9, 107.6),
            (-6.2, 106.8),
            now,
            250
        ));
    }

    #[test]
    fn event_locations_follow_the_mode() {
        let office = ResolvedLocation {
            id: 1,
            name: "Kantor".into(),
            latitude: -6.2,
            longitude: 106.8,
            radius_meters: 100,
        };
        let field = ResolvedLocation {
            id: 2,
            name: "Lapangan".into(),
            ..office.clone()
        };
        let locations = EventLocations {
            exempt: false,
            employee: Some(office.clone()),
            custom: HashMap::from([(2, field.clone())]),
        };
        assert_eq!(locations.resolve("none", None), None);
        assert_eq!(locations.resolve("employee", None), Some(&office));
        assert_eq!(locations.resolve("custom", Some(2)), Some(&field));
        assert_eq!(locations.resolve("custom", Some(9)), None);

        let exempt = EventLocations {
            exempt: true,
            ..locations
        };
        assert_eq!(exempt.resolve("custom", Some(2)), None);
    }
}
