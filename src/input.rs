//! Input request seperti `$request->input()` Laravel: query string digabung dengan body JSON,
//! form-urlencoded, atau multipart (body menang). Middleware global Laravel ikut ditiru:
//! `TrimStrings` (kecuali field sandi) dan `ConvertEmptyStringsToNull`. Body yang tidak bisa
//! diurai dianggap kosong, sehingga berakhir sebagai galat validasi seperti di Laravel.

use std::collections::HashMap;

use axum::body::Bytes;
use axum::extract::{FromRequest, Multipart, Request};
use axum::http::{StatusCode, header};
use serde_json::{Map, Value};

use crate::error::AppError;

/// Berkas unggahan (hanya bagian multipart yang punya `filename`, seperti `$request->file()`).
#[derive(Clone, Debug)]
pub struct UploadedFile {
    pub bytes: Bytes,
}

#[derive(Debug, Default)]
pub struct Input {
    fields: Map<String, Value>,
    files: HashMap<String, UploadedFile>,
}

const UNTRIMMED: [&str; 3] = ["password", "password_confirmation", "current_password"];

impl Input {
    pub fn get(&self, field: &str) -> Option<&Value> {
        self.fields.get(field).filter(|v| !v.is_null())
    }

    pub fn file(&self, field: &str) -> Option<&UploadedFile> {
        self.files.get(field)
    }

    #[cfg(test)]
    pub fn from_json(value: Value) -> Self {
        let mut input = Self::default();
        if let Value::Object(map) = value {
            input.merge(map);
        }
        input
    }

    fn merge(&mut self, map: Map<String, Value>) {
        for (key, value) in map {
            let value = normalize(&key, value);
            self.fields.insert(key, value);
        }
    }

    fn merge_pairs(&mut self, pairs: impl Iterator<Item = (String, String)>) {
        let mut map = Map::new();
        for (key, value) in pairs {
            map.insert(key, Value::String(value));
        }
        self.merge(map);
    }
}

fn normalize(key: &str, value: Value) -> Value {
    match value {
        Value::String(s) => {
            let s = if UNTRIMMED.contains(&key) {
                s
            } else {
                s.trim().to_string()
            };
            if s.is_empty() {
                Value::Null
            } else {
                Value::String(s)
            }
        }
        Value::Array(items) => Value::Array(items.into_iter().map(|v| normalize("", v)).collect()),
        Value::Object(map) => Value::Object(
            map.into_iter()
                .map(|(k, v)| (k.clone(), normalize(&k, v)))
                .collect(),
        ),
        other => other,
    }
}

fn body_error(status: StatusCode) -> Option<AppError> {
    (status == StatusCode::PAYLOAD_TOO_LARGE).then_some(AppError::PayloadTooLarge)
}

impl<S: Send + Sync> FromRequest<S> for Input {
    type Rejection = AppError;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        let mut input = Input::default();
        if let Some(query) = req.uri().query() {
            input.merge_pairs(form_urlencoded::parse(query.as_bytes()).into_owned());
        }

        let content_type = req
            .headers()
            .get(header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .to_ascii_lowercase();

        if content_type.starts_with("multipart/form-data") {
            let Ok(mut multipart) = Multipart::from_request(req, state).await else {
                return Ok(input);
            };
            let mut text = Vec::new();
            loop {
                match multipart.next_field().await {
                    Ok(Some(field)) => {
                        let name = field.name().unwrap_or_default().to_string();
                        // PHP: bagian dengan filename kosong berarti "tidak ada berkas".
                        let filename = field.file_name().map(str::to_string);
                        let data = field.bytes().await.map_err(|e| {
                            body_error(e.status())
                                .unwrap_or(AppError::rule("Unggahan tidak lengkap."))
                        })?;
                        match filename {
                            Some(f) if !f.is_empty() => {
                                input.files.insert(name, UploadedFile { bytes: data });
                            }
                            Some(_) => {}
                            None => text.push((name, String::from_utf8_lossy(&data).into_owned())),
                        }
                    }
                    Ok(None) => break,
                    Err(e) => {
                        if let Some(err) = body_error(e.status()) {
                            return Err(err);
                        }
                        break;
                    }
                }
            }
            input.merge_pairs(text.into_iter());
            return Ok(input);
        }

        let bytes = match Bytes::from_request(req, state).await {
            Ok(bytes) => bytes,
            Err(rejection) => {
                return match body_error(rejection.status()) {
                    Some(err) => Err(err),
                    None => Ok(input),
                };
            }
        };

        if content_type.contains("json") {
            if let Ok(Value::Object(map)) = serde_json::from_slice::<Value>(&bytes) {
                input.merge(map);
            }
        } else if content_type.starts_with("application/x-www-form-urlencoded") {
            input.merge_pairs(form_urlencoded::parse(&bytes).into_owned());
        }
        Ok(input)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use serde_json::json;

    async fn extract(req: Request) -> Input {
        Input::from_request(req, &()).await.unwrap()
    }

    #[tokio::test]
    async fn json_body_is_trimmed_and_empty_becomes_null() {
        let req = Request::builder()
            .method("POST")
            .uri("/x?device_name=query")
            .header("content-type", "application/json")
            .body(Body::from(
                r#"{"login":"  kontrak ","password":" rahasia ","device_name":"   ","n":5}"#,
            ))
            .unwrap();
        let input = extract(req).await;
        assert_eq!(input.get("login"), Some(&json!("kontrak")));
        assert_eq!(input.get("password"), Some(&json!(" rahasia ")));
        assert_eq!(
            input.get("device_name"),
            None,
            "body menang atas query, lalu dikosongkan"
        );
        assert_eq!(input.get("n"), Some(&json!(5)));
    }

    #[tokio::test]
    async fn invalid_json_is_empty_input() {
        let req = Request::builder()
            .method("POST")
            .uri("/x")
            .header("content-type", "application/json")
            .body(Body::from("{oops"))
            .unwrap();
        assert!(extract(req).await.get("login").is_none());
    }

    #[tokio::test]
    async fn multipart_files_and_text() {
        let body = "--B\r\nContent-Disposition: form-data; name=\"latitude\"\r\n\r\n -6.2 \r\n\
                    --B\r\nContent-Disposition: form-data; name=\"photo\"; filename=\"a.png\"\r\nContent-Type: image/png\r\n\r\nPNGDATA\r\n\
                    --B\r\nContent-Disposition: form-data; name=\"empty\"; filename=\"\"\r\n\r\n\r\n--B--\r\n";
        let req = Request::builder()
            .method("POST")
            .uri("/x")
            .header("content-type", "multipart/form-data; boundary=B")
            .body(Body::from(body))
            .unwrap();
        let input = extract(req).await;
        assert_eq!(input.get("latitude"), Some(&json!("-6.2")));
        assert_eq!(input.file("photo").unwrap().bytes.as_ref(), b"PNGDATA");
        assert!(input.file("empty").is_none());
    }

    #[tokio::test]
    async fn form_urlencoded() {
        let req = Request::builder()
            .method("POST")
            .uri("/x")
            .header("content-type", "application/x-www-form-urlencoded")
            .body(Body::from("login=a%40b.test&password=+x+"))
            .unwrap();
        let input = extract(req).await;
        assert_eq!(input.get("login"), Some(&json!("a@b.test")));
        assert_eq!(input.get("password"), Some(&json!(" x ")));
    }
}
