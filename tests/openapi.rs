//! `docs/openapi.yaml` (kontrak tertulis) harus sebentuk dengan OpenAPI yang dibangkitkan dari kode.
//! Gagal di CI bila bentuk respons/request diubah di kode tanpa memperbarui dokumen, atau sebaliknya.
//! Lihat `src/openapi.rs` untuk apa yang termasuk "bentuk".

use eabsensi_mobile_api::openapi::{generated, shape};
use serde_json::Value;

fn documented() -> Value {
    let yaml = include_str!("../docs/openapi.yaml");
    yaml_serde::from_str::<Value>(yaml).expect("docs/openapi.yaml harus YAML yang valid")
}

#[test]
fn documented_and_generated_shapes_match() {
    let doc = shape(&documented());
    let code = shape(&generated());
    if doc == code {
        return;
    }

    let (Some(doc_ops), Some(code_ops)) = (doc.as_object(), code.as_object()) else {
        panic!("bentuk tidak berupa objek");
    };
    let mut report = String::new();
    let keys: std::collections::BTreeSet<&String> = doc_ops.keys().chain(code_ops.keys()).collect();
    for key in keys {
        match (doc_ops.get(key), code_ops.get(key)) {
            (Some(a), Some(b)) if a == b => {}
            (Some(_), None) => {
                report.push_str(&format!("\n- {key}: ada di dokumen, tidak ada di kode"))
            }
            (None, Some(_)) => {
                report.push_str(&format!("\n- {key}: ada di kode, tidak ada di dokumen"))
            }
            (Some(a), Some(b)) => {
                for part in ["security", "parameters", "request", "responses"] {
                    if a.get(part) != b.get(part) {
                        report.push_str(&format!(
                            "\n- {key} [{part}]\n  dokumen: {}\n  kode:    {}",
                            a.get(part).cloned().unwrap_or_default(),
                            b.get(part).cloned().unwrap_or_default()
                        ));
                    }
                }
            }
            (None, None) => {}
        }
    }
    panic!(
        "docs/openapi.yaml tidak sebentuk dengan OpenAPI dari kode. Perbarui keduanya bersama.{report}"
    );
}
