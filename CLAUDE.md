# eabsensi-mobile-api

REST API untuk aplikasi pegawai eAbsensi, ditulis dalam Rust. Menggantikan `/api/*` milik web Laravel (`../eabsensi-v2`) **tanpa mengubah kontrak**: aplikasi Flutter (`../eabsensi-mobile-app`) tidak boleh perlu diubah saat cutover.

Pekerjaan berurutan ada di [TASKS.md](TASKS.md). Kerjakan satu task sampai kriteria selesainya terpenuhi, commit, lalu lanjut.

## Sistem empat repo (satu database PostgreSQL)
| Repo | Folder lokal | Peran |
|---|---|---|
| `eabsensi-v2` | `../eabsensi-v2` | Web operator (Laravel). **Pemilik skema** (migration) dan sumber aturan bisnis. |
| `eabsensi-mobile-api` | repo ini | API mobile (Rust). Pemilik kontrak `/api/*` setelah Task 1. |
| `eabsensi-adms` | `../eabsensi-adms` | Server mesin ZKTeco (Rust). Tidak berbagi kode dengan repo ini. |
| `eabsensi-mobile-app` | `../eabsensi-mobile-app` | Aplikasi Flutter, klien kontrak ini. |

Rencana lengkap dan alasannya: `../eabsensi-v2/docs/rust-services-plan.md`. Baca itu sebelum Task 1. Repo `eabsensi-v2` hanya dibaca dari sini; perubahan di sana dikerjakan di sesinya sendiri.

## Aturan yang tidak boleh dilanggar
- **Tidak ada migration di repo ini.** Skema diambil dari dump `../eabsensi-v2/database/dumps/pgsql-schema.sql` versi yang dipin di `SCHEMA_VERSION`. Butuh kolom/tabel baru: minta dibuat di `eabsensi-v2` dulu (expand/contract).
- **Kontrak identik dengan Laravel**: path, kode status, bentuk JSON, format galat Laravel (`{"message"}` untuk penolakan aturan, `{"message","errors"}` untuk validasi), pesan penolakan berbahasa Indonesia sama persis. `/api/*` selalu membalas JSON, dengan atau tanpa header `Accept`.
- **Token kompatibel Sanctum**: tabel `personal_access_tokens`, token `<id>|<rahasia>`, kolom `token` = SHA-256 hex dari rahasia, `abilities` = `["mobile"]`, kedaluwarsa `expires_at` atau `created_at + SANCTUM_TOKEN_EXPIRATION` menit, perbarui `last_used_at`. Token buatan Laravel harus diterima dan sebaliknya.
- **Sandi hanya diverifikasi** (bcrypt `$2y$`, di `spawn_blocking`); repo ini tidak pernah menulis atau rehash sandi.
- **Aturan bisnis yang di-port** (sumber di `../eabsensi-v2/app/Actions`): `RecordMobilePunch`, `CheckInToEvent`, `RequestLeave` (termasuk baris `audit_logs`), dan periode terkunci (`PeriodGuard`, `EloquentPeriodRepository::lockedMonthsBetween`). Bila berbeda dari dokumen, kode Laravel yang benar; catat perbedaannya.
- **Waktu**: simpan UTC, tampilkan ISO 8601 di `ATTENDANCE_TIMEZONE` (default `Asia/Jakarta`). Waktu punch selalu dari server.
- **Foto**: tata letak `<PHOTO_ROOT>/mobile-punches/Y/m/d/<kode department>/<40 karakter acak>.<ext>`; di lokal `PHOTO_ROOT` = `../eabsensi-v2/storage/app/private` agar halaman persetujuan web bisa menampilkannya.
- **Batas laju** sama dengan `AppServiceProvider` Laravel: login 60/menit per IP dan 5/menit per akun+IP; semua endpoint ber-token 120/menit per token; punch, check-in, cuti tambahan 20/menit per user.

## Environment (Windows)
- Rust belum terpasang di mesin ini (Task 0). Toolchain MSVC.
- PostgreSQL 18 lokal `127.0.0.1:5432`, database dev `eabsensi_v2` (dipakai bersama Laravel). Kredensial ada di `../eabsensi-v2/.env`; salin ke `.env` repo ini (di-gitignore), jangan pernah di-commit. `psql`/`pg_dump` ada di `C:\Program Files\PostgreSQL\18\bin`.
- Hurl 8 di `C:\Program Files\hurl`. Node 26.12.1 di `C:\laragon\bin\nodejs\node-v26.12.1-win-x64` (untuk lint OpenAPI; jangan Node 18/25).
- PHP 8.5 untuk menjalankan Laravel pembanding: `C:\laragon\bin\php\php-8.5.11`.

## Standar kode
- `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `cargo test` hijau sebelum commit.
- Tidak ada `unwrap()`/`expect()` di jalur request; galat dipetakan ke satu tipe galat yang menghasilkan bentuk JSON Laravel.
- Query lewat `sqlx::query!`/`query_as!` (terverifikasi saat kompilasi, metadata offline `.sqlx/` di-commit), selalu dengan binding parameter.
- **Versi wajib terbaru**: toolchain Rust stable terbaru, dan setiap crate pada rilis stabil terbaru saat ditambahkan (cek crates.io/docs.rs, bukan dari ingatan). Pin di `Cargo.toml`, `Cargo.lock` di-commit. Saat mulai sesi baru, cek `cargo update --dry-run`/`cargo outdated` dan naikkan bila ada rilis baru.
- Konfigurasi hanya dari environment (nama sama dengan Laravel: `ATTENDANCE_*`, `SANCTUM_*`).

## Git
- Conventional Commits (`feat(auth): ...`, `test(contract): ...`), satu commit per task/perubahan utuh beserta test-nya.
- Jangan `git push` kecuali diminta. Jangan commit `.env`, kredensial, atau foto uji.
