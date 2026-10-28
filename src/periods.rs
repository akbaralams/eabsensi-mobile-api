//! Periode terkunci (`PeriodGuard::lockedMonths`, `EloquentPeriodRepository::lockedMonthsBetween`).

use chrono::{Datelike, NaiveDate};
use sqlx::PgExecutor;

use crate::error::AppResult;

/// Bulan (`YYYY-MM`) berstatus `closing`/`closed` (atau hanya `closed`) dalam rentang tanggal.
pub async fn locked_months_between<'e>(
    db: impl PgExecutor<'e>,
    from: NaiveDate,
    to: NaiveDate,
    include_closing: bool,
) -> AppResult<Vec<String>> {
    let (years, months): (Vec<i16>, Vec<i16>) = months_between(from, to).into_iter().unzip();
    let statuses: Vec<String> = if include_closing {
        vec!["closing".into(), "closed".into()]
    } else {
        vec!["closed".into()]
    };
    let rows = sqlx::query!(
        r#"SELECT p.year, p.month
           FROM periods p
           JOIN UNNEST($1::int2[], $2::int2[]) AS m(year, month) ON m.year = p.year AND m.month = p.month
           WHERE p.status = ANY($3)
           ORDER BY p.year, p.month"#,
        &years,
        &months,
        &statuses
    )
    .fetch_all(db)
    .await?;
    Ok(rows
        .into_iter()
        .map(|r| format!("{:04}-{:02}", r.year, r.month))
        .collect())
}

/// Setiap (tahun, bulan) dari bulan `from` sampai `to`.
fn months_between(from: NaiveDate, to: NaiveDate) -> Vec<(i16, i16)> {
    let mut out = Vec::new();
    let (mut year, mut month) = (from.year(), from.month());
    while (year, month) <= (to.year(), to.month()) {
        out.push((year as i16, month as i16));
        if month == 12 {
            year += 1;
            month = 1;
        } else {
            month += 1;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn months_cover_the_range() {
        let d = |y, m, day| NaiveDate::from_ymd_opt(y, m, day).unwrap();
        assert_eq!(
            months_between(d(2026, 10, 29), d(2026, 10, 29)),
            vec![(2026, 10)]
        );
        assert_eq!(
            months_between(d(2026, 11, 30), d(2027, 2, 1)),
            vec![(2026, 11), (2026, 12), (2027, 1), (2027, 2)]
        );
        assert!(months_between(d(2027, 1, 1), d(2026, 1, 1)).is_empty());
    }
}
