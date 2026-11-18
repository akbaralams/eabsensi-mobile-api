# eabsensi-mobile-api

REST API untuk aplikasi pegawai eAbsensi (Flutter, `../eabsensi-mobile-app`), ditulis dalam Rust. Pengganti `/api/*` milik web Laravel (`../eabsensi-v2`) dengan **kontrak identik**: path, status, bentuk JSON, pesan Indonesia, dan token Sanctum yang sama, sehingga aplikasi tidak perlu diubah saat cutover dan rollback cukup dengan mengubah nginx.

Konteks dan aturan: [CLAUDE.md](CLAUDE.md). Urutan pekerjaan: [TASKS.md](TASKS.md).

## Endpoint

| Endpoint | Isi |
|---|---|
| `POST /api/auth/login`, `POST /api/auth/logout`, `GET /api/me` | Token per perangkat (format Sanctum), profil |
| `GET /api/punches?date=`, `POST /api/punches` | Riwayat per hari, punch dengan foto dan lokasi (waktu dari server) |
| `GET /api/events`, `POST /api/events/{id}/check-in` | Event hari ini dan check-in sekali per pegawai |
| `GET /api/leaves`, `POST /api/leaves` | Pengajuan cuti, audit `leave.requested` |
| `POST /api/v2/auth/login`, `/refresh`, `/logout` | Tambahan v2: token akses + refresh token berotasi (khusus Rust) |
| `GET /healthz`, `GET /livez` | Health check untuk load balancer (di luar `/api`) |

Kontrak: [docs/api.md](docs/api.md) (naratif) dan [docs/openapi.yaml](docs/openapi.yaml) (OpenAPI 3.1, dijaga sebentuk dengan kode oleh `tests/openapi.rs`; `cargo run --bin openapi` mencetak versi dari kode).

## Menjalankan lokal

```bash
cp .env.example .env          # isi DATABASE_URL dan PHOTO_ROOT
cargo run                     # http://127.0.0.1:8080/healthz
```

Database: skema dari `schema/pgsql-schema.sql` (dump `eabsensi-v2` versi `SCHEMA_VERSION`); repo ini tidak punya migration. Untuk database uji kosong:

```bash
psql "$DATABASE_URL" -f schema/pgsql-schema.sql
psql "$DATABASE_URL" -f tests/contract/fixtures.sql     # akun kontrak*, sandi kontrak-uji-2026
```

Di mesin pengembang saat ini service Windows PostgreSQL 18 berstatus *Disabled*, jadi pengujian memakai cluster terpisah di `.local/pgdata` (port 5433, di-gitignore):

```powershell
& 'C:\Program Files\PostgreSQL\18\bin\pg_ctl.exe' -D .local\pgdata -l .local\pg.log start
```

## Test

| Perintah | Isi |
|---|---|
| `cargo test` | Unit + integrasi. Test integrasi (`tests/*.rs`) membuat database baru per test dari dump skema dan `fixtures.sql`, jadi `DATABASE_URL` harus milik role dengan `CREATEDB`. `TEST_REDIS_URL` (opsional) menyalakan test Redis. |
| `bash scripts/contract-test.sh` | Suite kontrak Hurl terhadap server yang berjalan (`BASE_URL`, `DATABASE_URL`). Hijau terhadap Rust; terhadap Laravel lihat [tests/contract/README.md](tests/contract/README.md). |
| `bash scripts/token-interop.sh` | Token Laravel dipakai di Rust dan sebaliknya, termasuk pencabutan silang. |
| `cargo run --release --example loadtest` | Uji beban puncak pagi ([docs/load-test.md](docs/load-test.md)). |

Sebelum commit: `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `cargo test`, dan `cargo sqlx prepare -- --all-targets` bila ada query yang berubah (metadata offline `.sqlx/` di-commit; CI memeriksanya).

## Konfigurasi

Semua dari environment; daftar lengkap dan default di [.env.example](.env.example). Nilai yang **harus sama dengan `.env` Laravel**: `ATTENDANCE_TIMEZONE`, `ATTENDANCE_MOBILE_REQUIRES_APPROVAL`, `ATTENDANCE_MOBILE_REQUIRES_PHOTO`, `ATTENDANCE_MOBILE_MIN_INTERVAL`, `ATTENDANCE_LOCATION_MAX_ACCURACY`, `ATTENDANCE_LOCATION_MAX_FIX_AGE`, `ATTENDANCE_LOCATION_MAX_SPEED_KMH`, `SANCTUM_TOKEN_EXPIRATION`, `SANCTUM_TOKEN_PREFIX`. `PHOTO_ROOT` harus folder `storage/app/private` Laravel.

## Aturan bersama dengan eabsensi-v2

Aturan berikut ditulis di dua tempat (Laravel dan repo ini). Mengubahnya butuh PR di kedua repo, dirilis berurutan:

- **Format token**: Sanctum v4, `<id>|<prefix><40 alfanumerik><crc32b>`, kolom `token` = SHA-256 rahasia, `abilities` `["mobile"]`, kedaluwarsa `created_at + SANCTUM_TOKEN_EXPIRATION` atau `expires_at`.
- **Periode terkunci**: punch ditolak bila bulan berjalan berstatus `closing`/`closed` (`PeriodGuard`).
- **Punch mobile**: interval minimum dari punch terakhir semua sumber; status `pending` bila perlu persetujuan; tata letak foto `mobile-punches/Y/m/d/<kode department>/`.
- **Event**: peserta = semua pegawai atau subtree department event; tanggal dan jam di zona bisnis; satu check-in per pegawai.
- **Cuti**: selisih tanggal < 62 hari; tidak tumpang tindih dengan pengajuan `pending`/`approved`; audit `leave.requested`.

Rancangan fitur berikutnya (rekap bulanan, menu dinamis, persetujuan, notifikasi, QR, lampiran cuti): [docs/api-v2-design.md](docs/api-v2-design.md). Kebutuhan server untuk RAB: [docs/kebutuhan-infrastruktur.md](docs/kebutuhan-infrastruktur.md).

Perbedaan yang ditemukan (dokumen vs kode Laravel, dan bug Laravel yang tidak ditiru): [docs/laravel-differences.md](docs/laravel-differences.md).

## Operasional

- [docs/deployment.md](docs/deployment.md): topologi dan skalabilitas, pemasangan, cutover dan rollback, runbook pagi upacara.
- [deploy/](deploy): contoh nginx, unit systemd template, skrip Windows service.
- [scripts/db-grants.sql](scripts/db-grants.sql): role database hak minimum.

## Toolchain

| Alat | Versi (dicek 29 Oktober 2026) |
|---|---|
| rustup | 1.29.2 |
| rustc / cargo | 1.99.1 stable (MSVC) |
| sqlx-cli | 0.9.0 (fitur postgres) |
| PostgreSQL | 18.6 |
| Hurl | 8.0.1 lokal, 8.1.0 di CI |
| Node (lint OpenAPI) | 26.12.1 lokal, 24 di CI; `@redocly/cli` 2.72.0 |

```bash
rustup update stable
cargo install sqlx-cli --no-default-features --features rustls,postgres
```
