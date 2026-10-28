//! Waktu: disimpan UTC, ditampilkan ISO 8601 di zona bisnis (`ATTENDANCE_TIMEZONE`).

use chrono::{DateTime, NaiveDate, NaiveDateTime, TimeZone, Utc};
use chrono_tz::Tz;

/// Seperti Carbon `toIso8601String()`: `2026-10-25T07:58:12+07:00`.
pub fn iso8601(at: DateTime<Utc>, tz: Tz) -> String {
    at.with_timezone(&tz)
        .format("%Y-%m-%dT%H:%M:%S%:z")
        .to_string()
}

pub fn local_date(at: DateTime<Utc>, tz: Tz) -> NaiveDate {
    at.with_timezone(&tz).date_naive()
}

/// Kolom `timestamp without time zone` Laravel berisi waktu UTC (app.timezone = UTC).
pub fn naive_utc(at: DateTime<Utc>) -> NaiveDateTime {
    at.naive_utc()
}

/// Awal hari `date` di zona bisnis dan awal hari berikutnya, dalam UTC: rentang setengah-terbuka [awal, akhir).
pub fn day_bounds_utc(date: NaiveDate, tz: Tz) -> (DateTime<Utc>, DateTime<Utc>) {
    let start_of = |d: NaiveDate| {
        let midnight = d.and_hms_opt(0, 0, 0).unwrap_or_default();
        tz.from_local_datetime(&midnight)
            .earliest()
            // Zona dengan DST yang melompati tengah malam: ambil waktu UTC naif sebagai cadangan.
            .map_or_else(
                || Utc.from_utc_datetime(&midnight),
                |t| t.with_timezone(&Utc),
            )
    };
    let next = date.succ_opt().unwrap_or(date);
    (start_of(date), start_of(next))
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono_tz::Asia::Jakarta;

    #[test]
    fn iso_in_business_zone() {
        let at = Utc.with_ymd_and_hms(2026, 10, 28, 18, 0, 13).unwrap();
        assert_eq!(iso8601(at, Jakarta), "2026-10-29T01:00:13+07:00");
        assert_eq!(iso8601(at, chrono_tz::UTC), "2026-10-28T18:00:13+00:00");
        assert_eq!(
            local_date(at, Jakarta),
            NaiveDate::from_ymd_opt(2026, 10, 29).unwrap()
        );
    }

    #[test]
    fn day_bounds_are_converted_to_utc() {
        let (start, end) = day_bounds_utc(NaiveDate::from_ymd_opt(2026, 10, 29).unwrap(), Jakarta);
        assert_eq!(start, Utc.with_ymd_and_hms(2026, 10, 28, 17, 0, 0).unwrap());
        assert_eq!(end, Utc.with_ymd_and_hms(2026, 10, 29, 17, 0, 0).unwrap());
    }
}
