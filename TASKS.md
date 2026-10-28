# Task eabsensi-mobile-api

Urutan wajib. Tiap task selesai bila kriteria "Selesai bila" terpenuhi dan sudah di-commit. Tandai `[x]` saat selesai.

Sumber kontrak saat ini ada di `../eabsensi-v2` branch `claude/kontrak-mobile` (atau `master` setelah PR-nya di-merge): `docs/mobile-api.md`, `docs/openapi/mobile-api.yaml`, `docs/openapi/redocly.yaml`, `tests/contract/mobile/`, `scripts/contract-test.sh`, `database/seeders/ContractTestSeeder.php`, `database/dumps/pgsql-schema.sql`.

## [x] Task 0. Toolchain
- Minta user memasang Rust (rustup, toolchain stable MSVC; butuh Visual Studio Build Tools "Desktop development with C++"). Jangan memasang sendiri tanpa izin.
- Pasang `sqlx-cli` (fitur postgres saja).
- **Selesai bila**: `cargo --version`, `rustc --version`, `sqlx --version` jalan; dicatat di README.

## [x] Task 1. Pindahkan kontrak ke repo ini
- Salin dari `../eabsensi-v2`: suite Hurl + fixtures ke `tests/contract/`, `docs/mobile-api.md` ke `docs/api.md`, OpenAPI ke `docs/openapi.yaml` (+ `redocly.yaml`), dump skema ke `schema/pgsql-schema.sql`.
- `SCHEMA_VERSION` berisi commit SHA `eabsensi-v2` asal dump (ganti ke tag `vX.Y.Z` begitu repo itu punya tag rilis).
- Padankan `ContractTestSeeder` dengan `tests/contract/fixtures.sql` (SQL idempoten: department `KONTRAK`, pegawai `KTR-001`/`KTR-002`, empat akun `kontrak*` dengan hash bcrypt dari password uji `kontrak-uji-2026`, dua event hari ini, hapus jejak uji lama). Event harus memakai tanggal hari ini (`CURRENT_DATE` di zona `Asia/Jakarta`).
- `scripts/contract-test.sh`: muat fixture lewat `psql` lalu `hurl --test --jobs 1`, dengan `BASE_URL`.
- **Selesai bila**: suite yang sudah dipindah, dengan fixture SQL, hijau terhadap Laravel (`php artisan serve` di `../eabsensi-v2`, database dev). Ini membuktikan fixture setara seeder sebelum ada kode Rust.

## [x] Task 2. Kerangka service
- Crate biner `eabsensi-mobile-api`: axum + tokio, tower-http (trace, request-id, timeout, batas body 5 MB), sqlx (postgres, offline), tracing log JSON, config dari env, `/healthz` (di luar `/api`).
- Tipe galat tunggal yang menghasilkan bentuk JSON Laravel (401 `{"message":"Unauthenticated."}`, 404, 422 validasi dengan `errors`, 422 aturan, 429).
- CI GitHub Actions: service Postgres, muat `schema/pgsql-schema.sql`, `cargo sqlx prepare --check`, fmt, clippy, test, lint OpenAPI, lalu suite kontrak terhadap binary yang berjalan (boleh `continue-on-error` sampai Task 6).
- **Selesai bila**: `cargo run` melayani `/healthz`; CI hijau untuk fmt/clippy/test.

## [x] Task 3. Auth
- `POST /api/auth/login`, `POST /api/auth/logout`, `GET /api/me`; middleware token kompatibel Sanctum; rate limit login.
- Test: token yang dibuat Laravel (login lewat `../eabsensi-v2`) diterima di Rust, dan token Rust diterima Laravel.
- **Selesai bila**: `tests/contract/01-auth.hurl` hijau terhadap Rust; test interop token lulus.

## [x] Task 4. Punch
- `POST /api/punches` (multipart, foto jpg/png ≤ 4 MB dicek dari isi berkas, lat/long berpasangan, interval minimum, periode terkunci, status pending/approved), `GET /api/punches?date=`.
- **Selesai bila**: `02-punches.hurl` hijau terhadap Rust; foto tersimpan dengan tata letak yang sama dan tampil di halaman persetujuan punch web.

## [ ] Task 5. Event
- `GET /api/events`, `POST /api/events/{id}/check-in` (peserta: semua atau department pegawai beserta induknya; tanggal dan jam di zona bisnis; sekali per pegawai).
- **Selesai bila**: `03-events.hurl` hijau terhadap Rust.

## [ ] Task 6. Cuti
- `GET /api/leaves` (`data` + `types`), `POST /api/leaves` (rentang < 62 hari, tumpang tindih pending/approved, audit `leave.requested`).
- **Selesai bila**: seluruh suite (4 file) hijau terhadap Rust dan tetap hijau terhadap Laravel; job suite kontrak di CI tidak lagi `continue-on-error`.

## [ ] Task 7. OpenAPI dari kode
- Hasilkan OpenAPI dari kode (mis. `utoipa`), bandingkan dengan `docs/openapi.yaml` di CI (beda bentuk = gagal).
- **Selesai bila**: CI gagal saat bentuk respons diubah tanpa memperbarui `docs/openapi.yaml`.

## [ ] Task 8. Siap staging (bersama sesi `eabsensi-v2`)
- Uji beban puncak pagi 2.000 pegawai (bandingkan `../eabsensi-v2/docs/performance.md`), unit systemd/Windows service, contoh konfigurasi nginx (`/api` ke Rust).
- Urutan cutover dan rollback mengikuti Tahap 6-7 di `rust-services-plan.md`; perubahan dokumen di `eabsensi-v2` dikerjakan di sesinya.
