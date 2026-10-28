# Deployment, cutover, dan skalabilitas

Bagian `eabsensi-v2` (migration, dump skema, rute API Laravel, `docs/deployment.md` di sana) dikerjakan di sesi repo itu. Dokumen ini mencakup sisi service Rust.

## Topologi

```
Aplikasi Flutter ── HTTPS ──▶ [F5 / nginx: TLS, satu pintu]
                                  │ /api/*  (least_conn, tanpa sticky session)
                                  ├──▶ eabsensi-mobile-api :8080 ┐  N instance, stateless
                                  ├──▶ eabsensi-mobile-api :8081 ┘  (systemd template, restart bergiliran)
                                  │ selain /api
                                  └──▶ Laravel (PHP-FPM)
eabsensi-mobile-api ──▶ PostgreSQL (pool per instance, opsional PgBouncer)
                    ──▶ Redis (opsional: batas laju bersama antar-instance)
                    ──▶ PHOTO_ROOT = storage/app/private Laravel (berkas bersama)
```

Prinsip dari dokumen "Topologi + Sizing" yang diterapkan di service ini:

| Prinsip | Di service ini |
|---|---|
| Tidak ada state di instance; instance boleh mati/ditambah tanpa pengguna logout | Token dibaca dari tabel `personal_access_tokens` di setiap request; tidak ada sesi lokal. Batas laju bisa dipindah ke Redis (`RATE_LIMIT_STORE=redis`). |
| Health check memeriksa dependensi, bukan sekadar proses hidup | `/healthz` 200 hanya bila database (dan Redis bila dipakai) menjawab; `/livez` untuk proses hidup. |
| Drain sebelum berhenti | SIGTERM: `/healthz` 503 selama `SHUTDOWN_DRAIN_SECS`, request berjalan diselesaikan, lalu berhenti. |
| Biaya hashing diisolasi | bcrypt di thread terpisah dengan slot terbatas (`LOGIN_HASH_CONCURRENCY`); kelebihan antre maks `LOGIN_HASH_QUEUE_SECS` lalu 429. Punch/check-in tidak ikut melambat. |
| Koneksi database ditahan | Pool tetap per instance (`DB_MAX_CONNECTIONS`), bukan satu koneksi per worker. 20 koneksi cukup untuk ±1.000 req/dtk (lihat `load-test.md`). |
| Batas laju bukan per IP di belakang NAT | Batas per akun (5/menit) dan per token (120/menit) tetap; batas per IP bisa dilonggarkan (`RATE_LIMIT_LOGIN_PER_IP`, `RATE_LIMIT_API_PER_IP`). |
| Tulis tanpa menahan pengguna | Tidak dipakai antrean: insert punch/check-in hanya beberapa ms di PostgreSQL dan kontrak mewajibkan 201 dengan data tersimpan. Check-in ganda dijaga unique constraint, punch ganda dijaga advisory lock. |
| Scale terjadwal, bukan autoscaling | Lihat runbook pagi upacara di bawah. |

Bagian dokumen topologi yang tidak berlaku: Oracle/DRCP/Exadata (di sini PostgreSQL), PHP-FPM/OPcache untuk API (diganti satu binary Rust), JWT + refresh token (kontrak memakai token Sanctum; refresh token butuh `/api/v2`, lihat pertanyaan terbuka).

## Kapasitas per lapis

- **Service**: satu proses memakai semua inti (tokio). Memori ±64 MB pada 1.000 req/dtk. Dua instance per host untuk restart bergiliran; host tambahan untuk HA.
- **PostgreSQL**: total koneksi = instance × `DB_MAX_CONNECTIONS` (+ Laravel + ADMS) harus di bawah `max_connections`. Dengan banyak instance, taruh PgBouncer (mode transaction, versi ≥ 1.21 dengan `max_prepared_statements` > 0 karena sqlx memakai prepared statement).
- **Redis**: data kecil (penghitung batas laju, kedaluwarsa 60 dtk). Bila Redis mati, request diloloskan (fail-open) dan dicatat di log.
- **Foto**: ±150 KB per foto, dua foto (punch + check-in) per pegawai per hari upacara: 64.000 pegawai ≈ 19 GB. Semua instance dan Laravel harus melihat folder yang sama (`PHOTO_ROOT`). Bila service dipisah ke host lain, folder itu harus berbagi (NFS) atau kedua sisi pindah ke penyimpanan S3-compatible.

## Pemasangan (Linux)

1. Build: `cargo build --release --locked`, salin `target/release/eabsensi-mobile-api` ke `/opt/eabsensi-mobile-api/`.
2. User sistem `eabsensi-api`, anggota grup PHP-FPM (`www-data`). `PHOTO_FILE_MODE=640` dan `PHOTO_DIR_MODE=750` bila ingin foto tidak terbaca user lain.
3. Role database hak minimum: `psql "$ADMIN_URL" -v password=... -f scripts/db-grants.sql` (diuji: seluruh suite kontrak hijau dengan role ini).
4. `/etc/eabsensi/mobile-api.env` dari `.env.example` (`chmod 600`). Nilai `ATTENDANCE_*`, `SANCTUM_TOKEN_EXPIRATION`, `SANCTUM_TOKEN_PREFIX` **sama** dengan `.env` Laravel. `PHOTO_ROOT=/var/www/eabsensi-v2/storage/app/private`. `TRUSTED_PROXIES` = alamat nginx/F5.
5. `deploy/systemd/eabsensi-mobile-api@.service` ke `/etc/systemd/system/`, lalu `systemctl enable --now eabsensi-mobile-api@8080 eabsensi-mobile-api@8081`.
6. nginx: `deploy/nginx/eabsensi.conf` (blok `/api/` belum diaktifkan sampai cutover).

Windows: `deploy/windows/install-service.ps1` (NSSM), satu service per port.

## Urutan rilis bila ada migration

Sesuai aturan expand/contract: migration di `eabsensi-v2` dirilis dulu (kolom/tabel baru nullable atau berdefault), lalu `SCHEMA_VERSION` + dump di repo ini dinaikkan dalam PR sendiri (CI memeriksa query terhadap dump baru), lalu service dirilis. Penghapusan kolom yang dibaca service baru boleh setelah service tidak lagi memakainya.

## Cutover (Tahap 6-7 `rust-services-plan.md`)

**Staging (Tahap 6)**
1. Pasang service di staging dengan database staging; `/healthz` 200.
2. Arahkan `/api` di nginx staging ke Rust.
3. Suite kontrak terhadap staging: `DATABASE_URL=<staging> BASE_URL=https://staging... bash scripts/contract-test.sh` (fixture membuat akun `kontrak*` di database staging).
4. Interop token: `LARAVEL_URL=... RUST_URL=... bash scripts/token-interop.sh` (Laravel staging masih melayani `/api` di port/host internal).
5. Uji beban dengan `examples/loadtest.rs` di VM staging; catat angkanya di `load-test.md`.
6. Uji aplikasi Flutter sungguhan (build dengan `API_BASE_URL` staging): login, punch dengan foto, foto tampil di halaman persetujuan web, event, cuti.

**Produksi (Tahap 7)**
1. Pasang dan jalankan service (belum menerima trafik). Cek `/healthz` dan satu `GET /api/me` langsung ke port service dengan token pengguna uji.
2. Aktifkan blok `location /api/` di nginx, `nginx -t && systemctl reload nginx`.
3. Pantau 30-60 menit: rasio 5xx, latensi p95, log `galat internal`, jumlah punch masuk dibanding hari sebelumnya, foto muncul di halaman persetujuan.
4. **Rollback** kapan saja: komentari `location /api/`, reload nginx. Token tetap berlaku (format Sanctum di tabel yang sama), pengguna tidak perlu login ulang, punch yang sudah masuk tetap di tabel yang sama.
5. Setelah stabil satu periode (mis. satu bulan tutup buku), sesi `eabsensi-v2` menghapus rute/controller/test API Laravel. Paket Sanctum dan tabelnya tetap (dipakai service ini dan pencabutan token dari web).

## Runbook pagi upacara (scale terjadwal)

Upacara dijadwalkan jauh hari, jadi penggantinya autoscaler adalah kalender:

| Waktu | Langkah |
|---|---|
| H-3 | Pastikan pegawai sudah login di aplikasi dalam 30 hari terakhir (token masih berlaku); umumkan agar membuka aplikasi sehari sebelumnya. Event dibuat di web dengan jam yang benar. |
| H-1 | Cek kapasitas: `select count(*) from personal_access_tokens where created_at > now() - interval '29 days'` dibanding jumlah pegawai, untuk memperkirakan berapa yang harus login ulang. Bila banyak, siapkan instance tambahan (lihat tabel di `load-test.md`). |
| T-60 mnt | `/healthz` semua instance 200; PostgreSQL tanpa lock menggantung; ruang disk foto cukup. |
| T-30 mnt | Nyalakan instance tambahan, tunggu `/healthz` 200 dan masuk upstream. |
| T-10 mnt | Bekukan deploy dan perubahan konfigurasi. Pantau: req/dtk, p95, 429 login, koneksi database. |
| T-0 | Bila 429 login melonjak: umumkan check-in bertahap per unit. Jangan menurunkan cost bcrypt. |
| T+30 mnt | Cocokkan jumlah check-in di web dengan peserta. |
| T+60 mnt | Matikan instance tambahan dengan `systemctl stop` (drain otomatis), catat angka puncak untuk upacara berikutnya. |
