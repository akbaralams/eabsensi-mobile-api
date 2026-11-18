//! Pegawai pengguna dan profil `MeResource`.

use serde::Serialize;
use sqlx::PgPool;
use utoipa::ToSchema;

use crate::config::Config;
use crate::error::AppResult;
use crate::geofence::{self, WorkLocation};

#[derive(Debug, Clone)]
pub struct Employee {
    pub id: i64,
    pub employee_number: String,
    pub name: String,
    pub is_active: bool,
    pub department_code: Option<String>,
    pub department_name: Option<String>,
}

/// `EmployeeRepository::findWithDepartment`.
pub async fn find_with_department(db: &PgPool, id: i64) -> AppResult<Option<Employee>> {
    let row = sqlx::query_as!(
        Employee,
        r#"SELECT e.id AS "id!", e.employee_number AS "employee_number!", e.name AS "name!", e.is_active AS "is_active!",
                  d.code AS "department_code?", d.name AS "department_name?"
           FROM employees e
           LEFT JOIN departments d ON d.id = e.department_id
           WHERE e.id = $1"#,
        id
    )
    .fetch_optional(db)
    .await?;
    Ok(row)
}

#[derive(Debug, Serialize, PartialEq, ToSchema)]
pub struct EmployeeSummary {
    pub id: i64,
    pub employee_number: String,
    pub name: String,
    /// Nama department.
    #[schema(required = true)]
    pub department: Option<String>,
}

/// `MeResource`: profil pengguna dan pengaturan aplikasi.
#[derive(Debug, Serialize, PartialEq, ToSchema)]
pub struct Me {
    pub id: i64,
    pub name: String,
    /// `null` bila akun tidak punya username (login dengan email).
    #[schema(required = true)]
    pub username: Option<String>,
    #[schema(required = true)]
    pub employee: Option<EmployeeSummary>,
    pub timezone: String,
    pub requires_photo: bool,
    pub requires_approval: bool,
    /// Lokasi yang berlaku untuk punch; `null` bila absen tidak dibatasi radius.
    pub work_location: Option<WorkLocation>,
}

pub struct UserIdentity<'a> {
    pub id: i64,
    pub name: &'a str,
    pub username: Option<&'a str>,
    pub employee_id: Option<i64>,
}

pub async fn me(db: &PgPool, config: &Config, user: UserIdentity<'_>) -> AppResult<Me> {
    let employee = match user.employee_id {
        Some(id) => find_with_department(db, id).await?,
        None => None,
    };
    let work_location = match &employee {
        Some(employee) => geofence::for_employee(db, employee.id).await?,
        None => None,
    };
    Ok(Me {
        id: user.id,
        name: user.name.to_string(),
        username: user.username.map(str::to_string),
        employee: employee.map(|e| EmployeeSummary {
            id: e.id,
            employee_number: e.employee_number,
            name: e.name,
            department: e.department_name,
        }),
        timezone: config.timezone.name().to_string(),
        requires_photo: config.requires_photo,
        requires_approval: config.requires_approval,
        work_location: work_location.as_ref().map(WorkLocation::from),
    })
}
