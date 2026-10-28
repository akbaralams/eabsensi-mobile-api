//! Cetak OpenAPI yang dibangkitkan dari kode (JSON) ke stdout:
//! `cargo run --bin openapi > target/openapi.json`
//! Kontrak tertulis tetap `docs/openapi.yaml`; test `tests/openapi.rs` menjaga keduanya sebentuk.

fn main() {
    let spec = eabsensi_mobile_api::openapi::generated();
    match serde_json::to_string_pretty(&spec) {
        Ok(text) => println!("{text}"),
        Err(error) => {
            eprintln!("gagal membuat OpenAPI: {error}");
            std::process::exit(1);
        }
    }
}
