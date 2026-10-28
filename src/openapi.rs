//! OpenAPI yang dibangkitkan dari kode (utoipa) dan pembanding "bentuk" dengan `docs/openapi.yaml`.
//!
//! Bentuk yang dibandingkan: operasi (method + path), parameter, keamanan, body request, kode status
//! respons, dan untuk setiap skema JSON: nama properti, wajib/tidak, tipe, nullable, array. Deskripsi,
//! contoh, format, enum, dan batas angka tidak dibandingkan. Nama komponen juga tidak: `$ref` diselesaikan
//! dulu, jadi dokumen boleh menyusun skemanya berbeda selama bentuknya sama.

use std::collections::{BTreeMap, BTreeSet, HashMap};

use serde::Serialize;
use serde_json::{Map, Value, json};
use utoipa::openapi::security::{HttpAuthScheme, HttpBuilder, SecurityScheme};
use utoipa::{Modify, OpenApi, ToSchema};

use crate::{auth, events, leaves, punches};

/// Amplop `{"data": ...}` seperti JsonResource Laravel.
#[derive(Debug, Serialize, ToSchema)]
pub struct Data<T> {
    pub data: T,
}

/// Galat atau pesan: `{"message"}`.
#[derive(Debug, Serialize, ToSchema)]
pub struct Message {
    pub message: String,
}

/// Galat validasi: `{"message","errors"}`; nama field dijamin, teks pesan informatif.
#[derive(Debug, Serialize, ToSchema)]
pub struct ValidationError {
    pub message: String,
    pub errors: HashMap<String, Vec<String>>,
}

/// 422: galat validasi atau penolakan aturan bisnis.
#[derive(Debug, Serialize, ToSchema)]
#[serde(untagged)]
pub enum Unprocessable {
    Validation(ValidationError),
    Rule(Message),
}

#[derive(Debug, Serialize, ToSchema)]
pub struct LoginRequest {
    pub login: String,
    pub password: String,
    pub device_name: String,
}

/// Field multipart punch/check-in; semuanya opsional di tingkat skema (foto wajib bila `requires_photo`).
#[derive(Debug, Serialize, ToSchema)]
pub struct EvidenceForm {
    #[serde(default)]
    #[schema(format = Binary)]
    pub photo: String,
    #[serde(default)]
    pub latitude: f64,
    #[serde(default)]
    pub longitude: f64,
    #[serde(default)]
    pub accuracy: i32,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct LeaveRequest {
    pub attendance_status_id: i64,
    pub start_date: String,
    pub end_date: String,
    pub reason: Option<String>,
}

struct BearerAuth;

impl Modify for BearerAuth {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        let components = openapi.components.get_or_insert_with(Default::default);
        components.add_security_scheme(
            "bearerAuth",
            SecurityScheme::Http(HttpBuilder::new().scheme(HttpAuthScheme::Bearer).build()),
        );
    }
}

#[derive(OpenApi)]
#[openapi(
    info(
        title = "eAbsensi API mobile",
        version = "1.0.0",
        description = "API aplikasi pegawai. Kontrak tertulis: docs/openapi.yaml dan docs/api.md.",
        license(name = "Internal"),
    ),
    servers((url = "/api")),
    paths(
        auth::login, auth::logout, auth::me,
        punches::index, punches::store,
        events::index, events::check_in,
        leaves::index, leaves::store,
    ),
    components(schemas(Message, ValidationError, Unprocessable, LoginRequest, EvidenceForm, LeaveRequest)),
    security(("bearerAuth" = [])),
    modifiers(&BearerAuth),
)]
pub struct ApiDoc;

/// Spesifikasi dari kode sebagai JSON.
pub fn generated() -> Value {
    serde_json::to_value(ApiDoc::openapi()).unwrap_or(Value::Null)
}

// ---------------------------------------------------------------------------------------------
// Bentuk kanonik
// ---------------------------------------------------------------------------------------------

/// Bentuk kanonik sebuah spesifikasi OpenAPI 3.x (JSON). Dua spesifikasi setara bila hasilnya sama.
pub fn shape(spec: &Value) -> Value {
    let global_security = security_names(spec.get("security"));
    let mut ops = BTreeMap::new();
    if let Some(paths) = spec.get("paths").and_then(Value::as_object) {
        for (path, item) in paths {
            let item = resolve(spec, item);
            for method in ["get", "put", "post", "delete", "patch", "head", "options"] {
                if let Some(op) = item.get(method) {
                    let key = format!("{} {path}", method.to_uppercase());
                    ops.insert(key, operation_shape(spec, op, &global_security));
                }
            }
        }
    }
    json!(ops)
}

fn operation_shape(spec: &Value, op: &Value, global_security: &BTreeSet<String>) -> Value {
    let security = match op.get("security") {
        Some(s) => security_names(Some(s)),
        None => global_security.clone(),
    };

    let mut params = BTreeMap::new();
    for p in op
        .get("parameters")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        let p = resolve(spec, p);
        let name = p.get("name").and_then(Value::as_str).unwrap_or_default();
        let location = p.get("in").and_then(Value::as_str).unwrap_or_default();
        let required = p.get("required").and_then(Value::as_bool).unwrap_or(false);
        let schema = p
            .get("schema")
            .map_or(json!("any"), |s| schema_shape(spec, s, 0));
        // Parameter opsional boleh ditulis nullable (Option<T>) atau tidak: yang penting tipenya.
        let schema = strip_null(schema);
        params.insert(
            format!("{location}:{name}"),
            json!({"required": required, "schema": schema}),
        );
    }

    let request = op.get("requestBody").map(|body| {
        let body = resolve(spec, body);
        json!({
            "required": body.get("required").and_then(Value::as_bool).unwrap_or(false),
            "content": content_shape(spec, body.get("content")),
        })
    });

    let mut responses = BTreeMap::new();
    if let Some(map) = op.get("responses").and_then(Value::as_object) {
        for (code, response) in map {
            let response = resolve(spec, response);
            responses.insert(code.clone(), content_shape(spec, response.get("content")));
        }
    }

    json!({
        "security": security,
        "parameters": params,
        "request": request,
        "responses": responses,
    })
}

fn content_shape(spec: &Value, content: Option<&Value>) -> Value {
    let mut out = BTreeMap::new();
    if let Some(map) = content.and_then(Value::as_object) {
        for (media, entry) in map {
            let schema = entry
                .get("schema")
                .map_or(json!("none"), |s| schema_shape(spec, s, 0));
            out.insert(media.clone(), schema);
        }
    }
    json!(out)
}

fn security_names(value: Option<&Value>) -> BTreeSet<String> {
    value
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_object)
        .flat_map(|req| req.keys().cloned())
        .collect()
}

/// Ikuti `$ref` lokal (`#/components/...`).
fn resolve<'a>(spec: &'a Value, value: &'a Value) -> &'a Value {
    let mut current = value;
    for _ in 0..32 {
        let Some(reference) = current.get("$ref").and_then(Value::as_str) else {
            return current;
        };
        let pointer = reference.trim_start_matches('#');
        match spec.pointer(pointer) {
            Some(target) => current = target,
            None => return current,
        }
    }
    current
}

fn schema_shape(spec: &Value, schema: &Value, depth: usize) -> Value {
    if depth > 32 {
        return json!("recursive");
    }
    let schema = resolve(spec, schema);

    let mut variants: Vec<Value> = Vec::new();
    for key in ["oneOf", "anyOf"] {
        if let Some(list) = schema.get(key).and_then(Value::as_array) {
            variants.extend(list.iter().map(|s| schema_shape(spec, s, depth + 1)));
        }
    }

    match schema.get("type") {
        Some(Value::Array(types)) => {
            for t in types.iter().filter_map(Value::as_str) {
                variants.push(single_type(spec, schema, t, depth));
            }
        }
        Some(Value::String(t)) => variants.push(single_type(spec, schema, t, depth)),
        _ => {
            if schema.get("properties").is_some() {
                variants.push(single_type(spec, schema, "object", depth));
            } else if schema.get("items").is_some() {
                variants.push(single_type(spec, schema, "array", depth));
            }
        }
    }

    union(variants)
}

fn single_type(spec: &Value, schema: &Value, kind: &str, depth: usize) -> Value {
    match kind {
        "object" => {
            let required: BTreeSet<&str> = schema
                .get("required")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .filter_map(Value::as_str)
                .collect();
            let mut props = Map::new();
            if let Some(map) = schema.get("properties").and_then(Value::as_object) {
                for (name, prop) in map {
                    props.insert(
                        name.clone(),
                        json!({
                            "required": required.contains(name.as_str()),
                            "shape": schema_shape(spec, prop, depth + 1),
                        }),
                    );
                }
            }
            let mut object = Map::new();
            object.insert("object".into(), Value::Object(props));
            if let Some(extra) = schema.get("additionalProperties").filter(|v| v.is_object()) {
                object.insert("additional".into(), schema_shape(spec, extra, depth + 1));
            }
            Value::Object(object)
        }
        "array" => json!({
            "array": schema.get("items").map_or(json!("any"), |i| schema_shape(spec, i, depth + 1)),
        }),
        other => json!(other),
    }
}

/// Gabungan varian: diratakan, unik, terurut; satu varian = varian itu sendiri.
fn union(variants: Vec<Value>) -> Value {
    let mut set: BTreeMap<String, Value> = BTreeMap::new();
    for v in variants {
        match v.get("anyOf").and_then(Value::as_array) {
            Some(inner) => {
                for i in inner {
                    set.insert(i.to_string(), i.clone());
                }
            }
            None => {
                set.insert(v.to_string(), v);
            }
        }
    }
    let mut items: Vec<Value> = set.into_values().collect();
    match items.len() {
        0 => json!("any"),
        1 => items.remove(0),
        _ => json!({ "anyOf": items }),
    }
}

fn strip_null(shape: Value) -> Value {
    match shape.get("anyOf").and_then(Value::as_array) {
        Some(items) => union(
            items
                .iter()
                .filter(|v| v.as_str() != Some("null"))
                .cloned()
                .collect(),
        ),
        None => shape,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nullable_spellings_are_equivalent() {
        let spec = json!({
            "components": {"schemas": {"S": {"type": "object", "required": ["a"], "properties": {"a": {"type": "string"}}}}},
        });
        let a = schema_shape(&spec, &json!({"type": ["string", "null"]}), 0);
        let b = schema_shape(
            &spec,
            &json!({"oneOf": [{"type": "null"}, {"type": "string"}]}),
            0,
        );
        assert_eq!(a, b);
        let r = schema_shape(&spec, &json!({"$ref": "#/components/schemas/S"}), 0);
        assert_eq!(
            r,
            json!({"object": {"a": {"required": true, "shape": "string"}}})
        );
    }

    #[test]
    fn generated_spec_has_all_contract_operations() {
        let shape = shape(&generated());
        let ops: Vec<&String> = shape.as_object().unwrap().keys().collect();
        assert_eq!(
            ops,
            [
                "GET /events",
                "GET /leaves",
                "GET /me",
                "GET /punches",
                "POST /auth/login",
                "POST /auth/logout",
                "POST /events/{event}/check-in",
                "POST /leaves",
                "POST /punches",
            ]
        );
    }
}
