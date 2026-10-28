# eabsensi-mobile-api

REST API untuk aplikasi pegawai eAbsensi (Flutter), ditulis dalam Rust. Pengganti `/api/*` milik web Laravel (`../eabsensi-v2`) dengan kontrak identik. Konteks dan aturan: [CLAUDE.md](CLAUDE.md); urutan pekerjaan: [TASKS.md](TASKS.md).

## Toolchain

| Alat | Versi (dicek 29 Oktober 2026) |
|---|---|
| rustup | 1.29.2 |
| rustc / cargo | 1.99.1 stable (MSVC) |
| sqlx-cli | 0.9.0 (fitur postgres) |
| PostgreSQL | 18.6 |
| Hurl | 8.0.1 |
| Node (lint OpenAPI) | 26.12.1 |

```bash
rustup update stable
cargo install sqlx-cli --no-default-features --features rustls,postgres
```

## Kontrak

- [docs/api.md](docs/api.md): kontrak naratif. [docs/openapi.yaml](docs/openapi.yaml): OpenAPI 3.1.
- [tests/contract](tests/contract): suite Hurl + `fixtures.sql`, runner `scripts/contract-test.sh`.
- [schema/pgsql-schema.sql](schema/pgsql-schema.sql): dump skema dari `eabsensi-v2`, versi di [SCHEMA_VERSION](SCHEMA_VERSION).
- [docs/laravel-differences.md](docs/laravel-differences.md): perbedaan dokumen vs kode Laravel, dan bug Laravel yang tidak ditiru.
