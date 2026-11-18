# Suite test kontrak API mobile

Test HTTP hitam-kotak untuk kontrak di [docs/api.md](../../docs/api.md), ditulis dengan [Hurl](https://hurl.dev) agar bisa dijalankan ke implementasi apa pun: service Rust di repo ini, dan Laravel (`../eabsensi-v2`) sebagai pembanding selama masa transisi. Implementasi dianggap setara bila suite ini hijau tanpa diubah.

Dipindahkan dari `eabsensi-v2` (`tests/contract/mobile`, commit di `SCHEMA_VERSION`). Repo ini sekarang pemiliknya.

| File | Isi |
|---|---|
| `01-auth.hurl` | login (sandi salah, akun nonaktif, tanpa pegawai, validasi, email), ganti token per perangkat, `/me`, logout, 401 |
| `02-punches.hurl` | validasi foto dan koordinat, punch 201 (waktu server `+07:00`, `pending`), interval minimum, riwayat per tanggal, pegawai nonaktif |
| `03-events.hurl` | daftar event hari ini (event department lain tidak terlihat), check-in, check-in ganda, 404 |
| `04-leaves.hurl` | jenis cuti, validasi tanggal, batas 62 hari, pengajuan 201, tumpang tindih |
| `05-location.hurl` | lokasi di `/me`, sinyal integritas, penolakan lokasi (tanpa koordinat, aplikasi lama, palsu, kedaluwarsa, akurasi), tanda di luar radius |
| `fixtures.sql` | data uji idempoten, padanan `ContractTestSeeder` (department `KONTRAK`, pegawai `KTR-001` sampai `KTR-003`, lima akun `kontrak*`, satu lokasi kerja, dua event hari ini) |

## Menjalankan

Butuh database uji khusus (fixture menghapus jejak uji akun `kontrak*` dan mengosongkan tabel `cache`), `psql`, dan `hurl`.

```bash
# service Rust (cargo run) di :8080
DATABASE_URL=postgres://eabsensi:eabsensi-lokal@127.0.0.1:5433/eabsensi_test \
BASE_URL=http://127.0.0.1:8080 bash scripts/contract-test.sh

# Laravel pembanding di :8000, memakai database yang sama (lihat README utama)
DATABASE_URL=... BASE_URL=http://127.0.0.1:8000 bash scripts/contract-test.sh
```

`scripts/contract-test.sh` memuat `fixtures.sql` lewat `psql`, lalu `hurl --test --jobs 1`. File dijalankan berurutan karena punch punya interval minimum 60 detik. Suite bisa diulang segera; foto uji tertinggal di `PHOTO_ROOT/mobile-punches/` dan boleh dihapus.

## Aturan penulisan

- Setiap request mengirim `Accept: application/json`, seperti yang dianjurkan untuk klien. Server tetap wajib membalas JSON untuk `/api/*` tanpa header itu.
- **Penolakan aturan bisnis** (422/403 dengan `message` saja) dicek teksnya persis; teks ini bagian dari kontrak dan ditampilkan apa adanya di aplikasi.
- **Galat validasi** (422 dengan `errors`) dicek nama field-nya saja; teks pesannya informatif dan boleh berbeda antar implementasi.
- Konfigurasi yang diasumsikan: nilai default `ATTENDANCE_*` (foto wajib, perlu persetujuan, interval 60 detik, zona `Asia/Jakarta`).
- Data uji hanya dari `fixtures.sql`; tanggal cuti memakai tahun 2099 agar tidak bergantung pada hari ini.

## Catatan: Laravel gagal di `02-punches.hurl` antara pukul 00:00 dan 07:00 WIB (sudah diperbaiki)

Diperbaiki di eabsensi-v2 `928950b` (branch `claude/adms-schema`); versi Laravel sebelum commit itu masih gagal di jam tersebut.

`GET /api/punches` di Laravel memakai batas hari zona bisnis tanpa konversi ke UTC, sementara sesi database UTC, jadi "hari ini" bergeser 7 jam. Punch pukul 01:00 WIB tanggal 29 muncul di `?date=28`. Ini bug di `eabsensi-v2` (`EloquentPunchRepository::forEmployeeBetween`), bukan di kontrak; service Rust menghitung batas hari dengan benar. Rincian di [docs/laravel-differences.md](../../docs/laravel-differences.md).

## Spesifikasi OpenAPI

Bentuk mesin kontrak ini ada di [docs/openapi.yaml](../../docs/openapi.yaml) (OpenAPI 3.1). Suite Hurl memeriksa perilaku; spesifikasi memeriksa bentuk. Ubah keduanya bersama.
