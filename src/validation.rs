//! Validasi bergaya Laravel: galat per field berurutan sesuai aturan, pesan dari `lang/id/validation.php`
//! eabsensi-v2 (APP_LOCALE=id). Kontrak hanya menjamin nama field; teks pesan dibuat semirip mungkin.

use chrono::NaiveDate;
use serde_json::{Map, Value};

use crate::error::{AppError, AppResult};

#[derive(Debug, Default, Clone, PartialEq)]
pub struct ValidationErrors {
    fields: Vec<(String, Vec<String>)>,
}

impl ValidationErrors {
    pub fn add(&mut self, field: &str, message: impl Into<String>) {
        let message = message.into();
        match self.fields.iter_mut().find(|(name, _)| name == field) {
            Some((_, messages)) => messages.push(message),
            None => self.fields.push((field.to_string(), vec![message])),
        }
    }

    pub fn has(&self, field: &str) -> bool {
        self.fields.iter().any(|(name, _)| name == field)
    }

    pub fn is_empty(&self) -> bool {
        self.fields.is_empty()
    }

    /// `ValidationException::summarize`: pesan pertama + "(dan N kesalahan lainnya)".
    pub fn summary(&self) -> String {
        let mut all = self.fields.iter().flat_map(|(_, messages)| messages.iter());
        let Some(first) = all.next() else {
            return "Data yang diberikan tidak valid.".into();
        };
        let rest = all.count();
        if rest == 0 {
            first.clone()
        } else {
            format!("{first} (dan {rest} kesalahan lainnya)")
        }
    }

    pub fn to_json(&self) -> Value {
        let mut map = Map::new();
        for (field, messages) in &self.fields {
            map.insert(
                field.clone(),
                Value::Array(messages.iter().cloned().map(Value::String).collect()),
            );
        }
        Value::Object(map)
    }

    pub fn into_result(self) -> AppResult<()> {
        if self.is_empty() {
            Ok(())
        } else {
            Err(AppError::Validation(self))
        }
    }
}

/// Nama field yang ditampilkan: terjemahan `validation.attributes` bila ada, selain itu `_` menjadi spasi.
pub fn attribute(field: &str) -> String {
    match field {
        "login" => "gabung".into(),
        "password" => "kata sandi".into(),
        "photo" => "foto".into(),
        "reason" => "alasan".into(),
        "date" => "tanggal".into(),
        other => other.replace('_', " "),
    }
}

fn ucfirst(text: &str) -> String {
    let mut chars = text.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().chain(chars).collect(),
        None => String::new(),
    }
}

fn label(field: &str) -> String {
    ucfirst(&attribute(field))
}

pub mod msg {
    use super::{attribute, label};

    pub fn required(field: &str) -> String {
        format!("{} wajib diisi.", label(field))
    }
    pub fn required_with(field: &str, other: &str) -> String {
        format!(
            "{} wajib diisi bila terdapat {}.",
            label(field),
            attribute(other)
        )
    }
    pub fn string(field: &str) -> String {
        format!("{} harus berupa string.", label(field))
    }
    pub fn max_string(field: &str, max: usize) -> String {
        format!("{} maksimal berisi {max} karakter.", label(field))
    }
    pub fn integer(field: &str) -> String {
        format!("{} harus berupa bilangan bulat.", label(field))
    }
    pub fn numeric(field: &str) -> String {
        format!("{} harus berupa angka.", label(field))
    }
    pub fn between_numeric(field: &str, min: i64, max: i64) -> String {
        format!("{} harus bernilai antara {min} sampai {max}.", label(field))
    }
    pub fn min_numeric(field: &str, min: i64) -> String {
        format!("{} minimal bernilai {min}.", label(field))
    }
    pub fn max_numeric(field: &str, max: i64) -> String {
        format!("{} maksimal bernilai {max}.", label(field))
    }
    pub fn image(field: &str) -> String {
        format!("{} harus berupa gambar.", label(field))
    }
    pub fn mimes(field: &str, values: &str) -> String {
        format!("{} harus berupa berkas berjenis: {values}.", label(field))
    }
    pub fn max_file(field: &str, kilobytes: u64) -> String {
        format!("{} maksimal berukuran {kilobytes} kilobita.", label(field))
    }
    pub fn date_format(field: &str, format: &str) -> String {
        format!("{} tidak cocok dengan format {format}.", label(field))
    }
    pub fn after_or_equal(field: &str, other: &str) -> String {
        format!(
            "{} harus berisi tanggal setelah atau sama dengan {}.",
            label(field),
            attribute(other)
        )
    }
    pub fn exists(field: &str) -> String {
        format!("{} yang dipilih tidak valid.", label(field))
    }
}

/// Aturan `required` untuk nilai non-berkas: bukan null, bukan string kosong, bukan array kosong.
pub fn is_present(value: Option<&Value>) -> bool {
    match value {
        None | Some(Value::Null) => false,
        Some(Value::String(s)) => !s.trim().is_empty(),
        Some(Value::Array(a)) => !a.is_empty(),
        Some(_) => true,
    }
}

/// `is_numeric()` PHP: angka JSON, atau string desimal/eksponen (spasi di awal/akhir diizinkan).
pub fn php_numeric(value: &Value) -> Option<f64> {
    match value {
        Value::Number(n) => n.as_f64(),
        Value::String(s) => {
            let t = s.trim();
            let valid = !t.is_empty()
                && t.chars()
                    .all(|c| c.is_ascii_digit() || matches!(c, '.' | 'e' | 'E' | '+' | '-'))
                && t.chars().any(|c| c.is_ascii_digit());
            if valid {
                t.parse::<f64>().ok().filter(|f| f.is_finite())
            } else {
                None
            }
        }
        _ => None,
    }
}

/// `filter_var($v, FILTER_VALIDATE_INT)`: bilangan bulat JSON, atau string bulat bertanda opsional tanpa nol di depan.
pub fn php_integer(value: &Value) -> Option<i64> {
    match value {
        Value::Number(n) => n.as_i64().or_else(|| {
            n.as_f64()
                .filter(|f| f.fract() == 0.0 && f.abs() < 9.0e15)
                .map(|f| f as i64)
        }),
        Value::String(s) => {
            let t = s.trim();
            let digits = t.strip_prefix(['-', '+']).unwrap_or(t);
            let valid = !digits.is_empty()
                && digits.chars().all(|c| c.is_ascii_digit())
                && (digits == "0" || !digits.starts_with('0'));
            if valid { t.parse::<i64>().ok() } else { None }
        }
        _ => None,
    }
}

/// `date_format:Y-m-d`: harus persis `YYYY-MM-DD` dan tanggalnya ada.
pub fn strict_date(value: &Value) -> Option<NaiveDate> {
    let Value::String(s) = value else {
        return None;
    };
    let b = s.as_bytes();
    let shape = b.len() == 10
        && b[4] == b'-'
        && b[7] == b'-'
        && b.iter()
            .enumerate()
            .all(|(i, c)| i == 4 || i == 7 || c.is_ascii_digit());
    if !shape {
        return None;
    }
    NaiveDate::parse_from_str(s, "%Y-%m-%d").ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn messages_follow_indonesian_lang_file() {
        assert_eq!(msg::required("login"), "Gabung wajib diisi.");
        assert_eq!(msg::required("device_name"), "Device name wajib diisi.");
        assert_eq!(
            msg::required_with("longitude", "latitude"),
            "Longitude wajib diisi bila terdapat latitude."
        );
        assert_eq!(
            msg::after_or_equal("end_date", "start_date"),
            "End date harus berisi tanggal setelah atau sama dengan start date."
        );
        assert_eq!(
            msg::mimes("photo", "jpg, jpeg, png"),
            "Foto harus berupa berkas berjenis: jpg, jpeg, png."
        );
    }

    #[test]
    fn summary_counts_remaining_messages() {
        let mut errors = ValidationErrors::default();
        assert!(errors.is_empty());
        errors.add("photo", "a");
        assert_eq!(errors.summary(), "a");
        errors.add("photo", "b");
        errors.add("latitude", "c");
        assert_eq!(errors.summary(), "a (dan 2 kesalahan lainnya)");
        assert_eq!(
            errors.to_json(),
            json!({"photo": ["a", "b"], "latitude": ["c"]})
        );
    }

    #[test]
    fn php_like_numbers() {
        assert_eq!(php_numeric(&json!("-6.2")), Some(-6.2));
        assert_eq!(php_numeric(&json!(" 106.8 ")), Some(106.8));
        assert_eq!(php_numeric(&json!("1e2")), Some(100.0));
        assert_eq!(php_numeric(&json!("inf")), None);
        assert_eq!(php_numeric(&json!("abc")), None);
        assert_eq!(php_numeric(&json!(true)), None);

        assert_eq!(php_integer(&json!("12")), Some(12));
        assert_eq!(php_integer(&json!(12)), Some(12));
        assert_eq!(php_integer(&json!(12.0)), Some(12));
        assert_eq!(php_integer(&json!("-3")), Some(-3));
        assert_eq!(php_integer(&json!("012")), None);
        assert_eq!(php_integer(&json!("1.5")), None);
        assert_eq!(php_integer(&json!(1.5)), None);
    }

    #[test]
    fn strict_dates() {
        assert!(strict_date(&json!("2099-01-05")).is_some());
        assert!(strict_date(&json!("2099-1-5")).is_none());
        assert!(strict_date(&json!("2099-02-30")).is_none());
        assert!(strict_date(&json!(20990105)).is_none());
    }

    #[test]
    fn presence() {
        assert!(!is_present(None));
        assert!(!is_present(Some(&json!(null))));
        assert!(!is_present(Some(&json!(""))));
        assert!(!is_present(Some(&json!([]))));
        assert!(is_present(Some(&json!(0))));
        assert!(is_present(Some(&json!("x"))));
    }
}
