//! Bukti punch/check-in (`MobilePunchRequest`): foto, lokasi, akurasi. Foto disimpan dengan tata letak
//! disk `local` Laravel agar halaman persetujuan web bisa menampilkannya:
//! `<PHOTO_ROOT>/<folder>/<40 karakter acak>.<ext>`.

use std::path::{Component, Path, PathBuf};

use axum::body::Bytes;
use rand::distr::{Alphanumeric, SampleString};
use serde_json::Value;

use crate::config::Config;
use crate::error::{AppError, AppResult};
use crate::input::Input;
use crate::validation::{ValidationErrors, msg, php_integer, php_numeric};

/// `max:4096` (kilobita) pada foto.
pub const MAX_PHOTO_KB: u64 = 4096;
/// Kolom `accuracy_meters` bertipe smallint; nilai di atasnya (masih lolos validasi kontrak 0..65535)
/// disimpan sebagai batas ini. Laravel gagal 500 untuk nilai itu.
const MAX_STORED_ACCURACY: i64 = i16::MAX as i64;
/// `max:86400` pada `location_age_seconds`.
const MAX_LOCATION_AGE_SECONDS: i64 = 86_400;

#[derive(Debug, Clone)]
pub struct Photo {
    pub bytes: Bytes,
    /// Ekstensi dari isi berkas (`guessExtension` Symfony): `jpg` atau `png`.
    pub extension: &'static str,
}

#[derive(Debug, Clone, Default)]
pub struct Evidence {
    pub photo: Option<Photo>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub accuracy: Option<i16>,
    /// Sinyal integritas; `None` berarti aplikasi versi lama tidak mengirimnya.
    pub is_mocked: Option<bool>,
    pub location_age_seconds: Option<i64>,
    pub device_rooted: Option<bool>,
}

/// Jenis gambar dari magic bytes, bukan dari nama berkas atau content-type kiriman klien.
fn sniff(bytes: &[u8]) -> Option<&'static str> {
    infer::get(bytes).map(|kind| kind.mime_type())
}

fn validate_photo(
    input: &Input,
    requires_photo: bool,
    errors: &mut ValidationErrors,
) -> Option<Photo> {
    let field = "photo";
    match (input.file(field), input.get(field)) {
        (None, None) => {
            if requires_photo {
                errors.add(field, msg::required(field));
            }
            None
        }
        // Dikirim sebagai teks, bukan berkas.
        (None, Some(_)) => {
            errors.add(field, msg::image(field));
            errors.add(field, msg::mimes(field, "jpg, jpeg, png"));
            None
        }
        (Some(file), _) => {
            let mime = sniff(&file.bytes);
            let is_image = matches!(
                mime,
                Some("image/jpeg" | "image/png" | "image/gif" | "image/bmp" | "image/webp")
            );
            let extension = match mime {
                Some("image/jpeg") => Some("jpg"),
                Some("image/png") => Some("png"),
                _ => None,
            };
            if !is_image {
                errors.add(field, msg::image(field));
            }
            if extension.is_none() {
                errors.add(field, msg::mimes(field, "jpg, jpeg, png"));
            }
            if file.bytes.len() as u64 > MAX_PHOTO_KB * 1024 {
                errors.add(field, msg::max_file(field, MAX_PHOTO_KB));
            }
            match extension {
                Some(extension) if !errors.has(field) => Some(Photo {
                    bytes: file.bytes.clone(),
                    extension,
                }),
                _ => None,
            }
        }
    }
}

fn validate_coordinate(
    input: &Input,
    field: &str,
    other: &str,
    bound: i64,
    errors: &mut ValidationErrors,
) -> Option<f64> {
    match input.get(field) {
        None => {
            if input.get(other).is_some() {
                errors.add(field, msg::required_with(field, other));
            }
            None
        }
        Some(value) => match php_numeric(value) {
            None => {
                errors.add(field, msg::numeric(field));
                None
            }
            Some(n) if n < -(bound as f64) || n > bound as f64 => {
                errors.add(field, msg::between_numeric(field, -bound, bound));
                None
            }
            Some(n) => Some(n),
        },
    }
}

fn validate_accuracy(value: Option<&Value>, errors: &mut ValidationErrors) -> Option<i16> {
    let field = "accuracy";
    let value = value?;
    let Some(n) = php_integer(value) else {
        errors.add(field, msg::integer(field));
        return None;
    };
    if n < 0 {
        errors.add(field, msg::min_numeric(field, 0));
        return None;
    }
    if n > 65535 {
        errors.add(field, msg::max_numeric(field, 65535));
        return None;
    }
    Some(n.min(MAX_STORED_ACCURACY) as i16)
}

/// Aturan `nullable|integer|in:0,1`.
fn validate_flag(input: &Input, field: &str, errors: &mut ValidationErrors) -> Option<bool> {
    let value = input.get(field)?;
    let Some(n) = php_integer(value) else {
        errors.add(field, msg::integer(field));
        return None;
    };
    match n {
        0 => Some(false),
        1 => Some(true),
        _ => {
            errors.add(field, msg::exists(field));
            None
        }
    }
}

/// Aturan `nullable|integer|min:0|max:86400`.
fn validate_location_age(input: &Input, errors: &mut ValidationErrors) -> Option<i64> {
    let field = "location_age_seconds";
    let value = input.get(field)?;
    let Some(n) = php_integer(value) else {
        errors.add(field, msg::integer(field));
        return None;
    };
    if n < 0 {
        errors.add(field, msg::min_numeric(field, 0));
        return None;
    }
    if n > MAX_LOCATION_AGE_SECONDS {
        errors.add(field, msg::max_numeric(field, MAX_LOCATION_AGE_SECONDS));
        return None;
    }
    Some(n)
}

/// Aturan `MobilePunchRequest`, galat per field dengan urutan photo, latitude, longitude, accuracy,
/// is_mocked, location_age_seconds, device_rooted.
pub fn validate_evidence(input: &Input, requires_photo: bool) -> AppResult<Evidence> {
    let mut errors = ValidationErrors::default();
    let photo = validate_photo(input, requires_photo, &mut errors);
    let latitude = validate_coordinate(input, "latitude", "longitude", 90, &mut errors);
    let longitude = validate_coordinate(input, "longitude", "latitude", 180, &mut errors);
    let accuracy = validate_accuracy(input.get("accuracy"), &mut errors);
    let is_mocked = validate_flag(input, "is_mocked", &mut errors);
    let location_age_seconds = validate_location_age(input, &mut errors);
    let device_rooted = validate_flag(input, "device_rooted", &mut errors);
    errors.into_result()?;
    Ok(Evidence {
        photo,
        latitude,
        longitude,
        accuracy,
        is_mocked,
        location_age_seconds,
        device_rooted,
    })
}

/// Satu segmen folder yang aman (kode department berasal dari data, bukan dari klien, tetapi tetap dijaga).
pub fn safe_segment(value: &str) -> String {
    let cleaned: String = value
        .chars()
        .map(|c| {
            if matches!(c, '/' | '\\' | ':' | '\0') {
                '_'
            } else {
                c
            }
        })
        .collect();
    match cleaned.as_str() {
        "" | "." | ".." => "_".into(),
        _ => cleaned,
    }
}

/// Simpan foto ke `<PHOTO_ROOT>/<folder>/<acak>.<ext>`; hasilnya path relatif (nilai kolom `photo_path`).
/// Ditulis ke berkas sementara lalu di-rename, jadi halaman web tidak pernah membaca berkas setengah jadi.
pub async fn store(config: &Config, folder: &str, photo: &Photo) -> AppResult<String> {
    let name = format!(
        "{}.{}",
        Alphanumeric.sample_string(&mut rand::rng(), 40),
        photo.extension
    );
    let relative = format!("{folder}/{name}");
    let target = resolve(&config.photo_root, &relative)?;
    let bytes = photo.bytes.clone();
    let (file_mode, dir_mode) = (config.photo_file_mode, config.photo_dir_mode);

    tokio::task::spawn_blocking(move || write_atomic(&target, &bytes, file_mode, dir_mode))
        .await
        .map_err(AppError::internal)??;
    Ok(relative)
}

/// Hapus foto yang sudah tersimpan bila transaksi database gagal (best effort).
pub async fn remove(config: &Config, relative: &str) {
    if let Ok(path) = resolve(&config.photo_root, relative)
        && let Err(error) = tokio::fs::remove_file(&path).await
    {
        tracing::warn!(%error, path = %path.display(), "gagal menghapus foto yatim");
    }
}

fn resolve(root: &Path, relative: &str) -> AppResult<PathBuf> {
    let rel = Path::new(relative);
    if rel.components().any(|c| !matches!(c, Component::Normal(_))) {
        return Err(AppError::internal(format!(
            "path foto tidak aman: {relative}"
        )));
    }
    Ok(root.join(rel))
}

fn write_atomic(target: &Path, bytes: &[u8], file_mode: u32, dir_mode: u32) -> std::io::Result<()> {
    let dir = target
        .parent()
        .ok_or_else(|| std::io::Error::other("path foto tanpa folder"))?;
    let mut builder = std::fs::DirBuilder::new();
    builder.recursive(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;
        builder.mode(dir_mode);
    }
    #[cfg(not(unix))]
    let _ = dir_mode;
    builder.create(dir)?;

    let file_name = target
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("foto");
    let temp = dir.join(format!(".{file_name}.tmp"));
    std::fs::write(&temp, bytes)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&temp, std::fs::Permissions::from_mode(file_mode))?;
    }
    #[cfg(not(unix))]
    let _ = file_mode;
    if let Err(error) = std::fs::rename(&temp, target) {
        let _ = std::fs::remove_file(&temp);
        return Err(error);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::validation::ValidationErrors;
    use serde_json::json;

    const PNG: &[u8] = include_bytes!("../tests/contract/fixtures/photo.png");
    const JPEG: &[u8] = &[0xFF, 0xD8, 0xFF, 0xE0, 0, 0x10, b'J', b'F', b'I', b'F', 0];
    const GIF: &[u8] = b"GIF89a\x01\x00\x01\x00\x00\x00\x00";

    fn errors_of(result: AppResult<Evidence>) -> ValidationErrors {
        match result {
            Err(AppError::Validation(errors)) => errors,
            other => panic!("harus galat validasi, dapat {other:?}"),
        }
    }

    #[test]
    fn photo_type_comes_from_content() {
        assert_eq!(sniff(PNG), Some("image/png"));
        assert_eq!(sniff(JPEG), Some("image/jpeg"));
        assert_eq!(sniff(b"bukan gambar"), None);
    }

    #[test]
    fn coordinates_must_come_in_pairs_and_be_in_range() {
        let input = Input::from_json(json!({"latitude": "-6.2"}));
        let errors = errors_of(validate_evidence(&input, false));
        assert!(errors.has("longitude") && !errors.has("latitude"));

        let input =
            Input::from_json(json!({"latitude": "91", "longitude": "abc", "accuracy": "-1"}));
        let errors = errors_of(validate_evidence(&input, false));
        assert!(errors.has("latitude") && errors.has("longitude") && errors.has("accuracy"));

        let input = Input::from_json(
            json!({"latitude": "-6.2", "longitude": "106.8", "accuracy": "40000"}),
        );
        let evidence = validate_evidence(&input, false).unwrap();
        assert_eq!(evidence.latitude, Some(-6.2));
        assert_eq!(evidence.accuracy, Some(i16::MAX));
    }

    #[test]
    fn integrity_signals_are_optional_and_validated() {
        let evidence = validate_evidence(&Input::from_json(json!({})), false).unwrap();
        assert_eq!(evidence.is_mocked, None);
        assert_eq!(evidence.location_age_seconds, None);
        assert_eq!(evidence.device_rooted, None);

        let input = Input::from_json(
            json!({"is_mocked": "1", "location_age_seconds": "12", "device_rooted": "0"}),
        );
        let evidence = validate_evidence(&input, false).unwrap();
        assert_eq!(evidence.is_mocked, Some(true));
        assert_eq!(evidence.location_age_seconds, Some(12));
        assert_eq!(evidence.device_rooted, Some(false));

        let input = Input::from_json(
            json!({"is_mocked": "2", "location_age_seconds": "-1", "device_rooted": "ya"}),
        );
        let errors = errors_of(validate_evidence(&input, false));
        assert!(errors.has("is_mocked"));
        assert!(errors.has("location_age_seconds"));
        assert!(errors.has("device_rooted"));

        let input = Input::from_json(json!({"location_age_seconds": "86401"}));
        assert!(errors_of(validate_evidence(&input, false)).has("location_age_seconds"));
    }

    #[test]
    fn photo_required_when_configured() {
        let errors = errors_of(validate_evidence(&Input::from_json(json!({})), true));
        assert!(errors.has("photo"));
        assert!(validate_evidence(&Input::from_json(json!({})), false).is_ok());
        // Teks di field foto bukan berkas.
        let errors = errors_of(validate_evidence(
            &Input::from_json(json!({"photo": "x"})),
            false,
        ));
        assert!(errors.has("photo"));
    }

    #[test]
    fn gif_is_an_image_but_not_an_allowed_type() {
        assert!(matches!(sniff(GIF), Some("image/gif")));
    }

    #[test]
    fn segments_and_paths_are_safe() {
        assert_eq!(safe_segment("KONTRAK-LAIN"), "KONTRAK-LAIN");
        assert_eq!(safe_segment("../x"), ".._x");
        assert_eq!(safe_segment(".."), "_");
        assert!(resolve(Path::new("/r"), "a/../b").is_err());
        assert!(resolve(Path::new("/r"), "/etc/passwd").is_err());
        assert!(resolve(Path::new("/r"), "mobile-punches/2026/10/29/K/x.png").is_ok());
    }

    #[tokio::test]
    async fn store_writes_under_root() {
        let dir = tempfile::tempdir().unwrap();
        let config = crate::config::Config::from_lookup(|k| match k {
            "DATABASE_URL" => Some("postgres://x".into()),
            "PHOTO_ROOT" => Some(dir.path().to_string_lossy().into()),
            _ => None,
        })
        .unwrap();
        let photo = Photo {
            bytes: Bytes::from_static(PNG),
            extension: "png",
        };
        let relative = store(&config, "mobile-punches/2026/10/29/KONTRAK", &photo)
            .await
            .unwrap();
        assert!(relative.starts_with("mobile-punches/2026/10/29/KONTRAK/"));
        let name = relative.rsplit('/').next().unwrap();
        assert_eq!(name.len(), 44);
        assert_eq!(std::fs::read(dir.path().join(&relative)).unwrap(), PNG);
        remove(&config, &relative).await;
        assert!(!dir.path().join(&relative).exists());
    }
}
