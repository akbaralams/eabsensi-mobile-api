# Uji beban puncak pagi

Diukur 29 Oktober 2026 di laptop pengembang (AMD Ryzen 5 6600H, 6 inti/12 thread, RAM 15 GB, Windows 11), PostgreSQL 18.6 di mesin yang sama, **satu** proses service build release, pool database 20 koneksi, batas laju memori. Generator beban berjalan di mesin yang sama (ikut memakan CPU). Angka ini titik awal; ulangi di VM staging sebelum menentukan ukuran produksi.

## Skenario

Setiap pegawai uji membuka aplikasi lalu menjalankan alur pagi: `GET /me` -> `POST /punches` (foto) -> `GET /punches` -> `GET /events` -> `POST /events/{id}/check-in` (foto). Kedatangan disebar merata dalam jendela waktu. Sebagian pegawai login ulang (bcrypt cost 12), sisanya memakai token 30 hari yang sudah ada.

Data: `scripts/loadtest/seed.sql`, pegawai di 20 department di bawah satu induk (sama dengan data uji `performance.md` eabsensi-v2), satu event "Apel Beban" untuk department induk.

```bash
psql "$DATABASE_URL" -v users=2000 -f scripts/loadtest/seed.sql
# service dijalankan dengan RATE_LIMIT_LOGIN_PER_IP=0 RATE_LIMIT_API_PER_IP=0 (semua pegawai uji dari satu IP)
LT_USERS=2000 LT_WINDOW_SECS=60 LT_LOGIN_PERCENT=10 cargo run --release --example loadtest
```

## Hasil

| Skenario | Request | Rata-rata | Galat | p99 punch | p99 check-in | p99 baca | p50 login |
|---|---:|---:|---:|---:|---:|---:|---:|
| 2.000 pegawai / 60 dtk, login 10%, foto 150 KB | 10.200 | 169 req/dtk | 0 | 5 ms | 5 ms | ≤ 2 ms | 205 ms |
| 2.000 pegawai / 10 dtk, login 10%, foto 150 KB | 10.200 | 997 req/dtk | 0 | 12 ms | 8 ms | ≤ 2 ms | 245 ms |
| 2.000 pegawai / 60 dtk, **semua login**, foto 150 KB | 12.000 | 199 req/dtk | 0 | 6 ms | 6 ms | ≤ 2 ms | 231 ms |
| **20.000 pegawai / 100 dtk**, login 10%, foto 10 KB | 102.000 | 1.018 req/dtk | 0 | 6 ms | 6 ms | ≤ 2 ms | 246 ms |
| Badai login: 3.000 login / 30 dtk (melebihi kapasitas CPU) | 10.525 | 299 req/dtk | 1.495 login 429 | 43 ms | 48 ms | ≤ 3 ms | 5,0 dtk |

Memori puncak proses: 64 MB. Koneksi database: 20 (pool).

Bacaan:
- **Alur pagi tanpa login murah**: verifikasi token adalah satu query berindeks; punch dan check-in (termasuk menulis foto) beberapa milidetik. Satu instance di laptop melayani 200 pegawai/detik (laju yang sama dengan "64.000 pegawai dalam 5 menit") tanpa galat.
- **Login adalah satu-satunya bagian mahal**: bcrypt cost 12 ±230 ms per verifikasi per inti. Kapasitas laptop ini ±43 login/detik. Saat permintaan melebihi itu, login antre maksimal `LOGIN_HASH_QUEUE_SECS` (5 dtk) lalu dibalas 429 + `Retry-After`; endpoint lain tetap cepat karena bcrypt punya slot sendiri (`LOGIN_HASH_CONCURRENCY`, default jumlah CPU - 1).
- Batas laju per IP Laravel (login 60/menit, request tanpa token 120/menit) **membatasi berapa pegawai bisa login per menit dari satu IP**. Di uji pertama 80 dari 200 login ditolak karena semua pegawai uji keluar dari satu IP. Di belakang NAT kantor, longgarkan `RATE_LIMIT_LOGIN_PER_IP` dan `RATE_LIMIT_API_PER_IP` (batas per akun tetap 5/menit).

## Perbandingan dengan Laravel

`docs/performance.md` eabsensi-v2 hanya mengukur laporan web, belum API mobile. Laravel lokal di sini berjalan dengan server bawaan PHP (satu proses, tanpa PHP-FPM), jadi tidak adil dibandingkan. Jalankan generator yang sama terhadap Laravel di staging (PHP-FPM) dengan `LT_BASE_URL` menunjuk ke sana untuk angka pembanding.

## Ukuran untuk produksi

Rumus (dari dokumen topologi, disesuaikan): login/dtk = pegawai yang harus login ÷ (jendela × 60); inti untuk bcrypt = login/dtk × 0,23 dtk ÷ 0,7 (utilisasi 70%).

| Hari upacara, 64.000 pegawai dalam 5 menit | Login/dtk | Inti bcrypt | Instance (8 vCPU) |
|---|---:|---:|---:|
| Semua login ulang | 213 | ±70 | ±10 + cadangan |
| 10% login ulang (token 30 hari masih berlaku) | 21 | ±7 | 2 (N+1) |
| Alur pagi tanpa login | 0 | 0 | 2 (N+1) |

Kesimpulannya sama dengan dokumen topologi: kunci skalabilitas adalah **berapa banyak pegawai yang harus login di jendela sempit**, bukan besar server. Token berlaku 30 hari sudah bekerja sebagai "pre-login"; lihat `docs/deployment.md` untuk risiko token yang kedaluwarsa bersamaan.
