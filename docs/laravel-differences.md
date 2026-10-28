# Perbedaan dengan Laravel (eabsensi-v2)

Aturan di `CLAUDE.md`: bila dokumen dan kode Laravel berbeda, kode Laravel yang benar, dan perbedaannya dicatat di sini. Bagian kedua mencatat tempat service Rust **sengaja** berbeda dari Laravel karena perilaku Laravel adalah bug; perbaikannya perlu dikerjakan di sesi `eabsensi-v2`.

## Dokumen vs kode Laravel (Rust mengikuti kode)

| Topik | Dokumen | Kode Laravel (diikuti Rust) |
|---|---|---|
| Folder foto punch `mobile-punches/Y/m/d` | tanggal tidak disebut zonanya | tanggal **zona bisnis**: `RecordMobilePunch` memanggil `$now->setTimezone(...)` pada Carbon mutable sebelum `store()`, jadi `$now` sudah berzona bisnis saat path dibentuk |
| Folder foto check-in event | tidak didokumentasikan | `event-attendance/Y/m/d/<kode department>/`, tanggal **UTC** (`CheckInToEvent` memakai `copy()`) |
| Punch oleh akun tanpa pegawai | "Ditolak 422" | 422 dari action untuk `POST /punches` dan check-in, tetapi 403 untuk `GET /punches`, `GET /events`, `GET/POST /leaves` |
| Urutan cek check-in | - | validasi form (foto dst.) dulu, baru event dicari (404), baru aturan action |
| 404 event tidak ada | `{"message"}` | pesan Laravel apa adanya: `No query results for model [App\Models\AttendanceEvent] <id>` |

## Bug Laravel yang tidak ditiru

### `GET /api/punches?date=` bergeser 7 jam

`PunchController::index` membentuk awal/akhir hari dengan `CarbonImmutable::parse($date, 'Asia/Jakarta')`, lalu `EloquentPunchRepository::forEmployeeBetween` meneruskannya ke `whereBetween`. Query builder memformat Carbon sebagai `Y-m-d H:i:s` **tanpa zona**, sementara sesi database UTC (`config/database.php`). Akibatnya "hari ini" di Laravel adalah 07:00 WIB hari ini sampai 07:00 WIB besok.

Bukti (28-29 Oktober 2026, database uji): punch pukul `2026-10-29T01:00:13+07:00` kosong di `GET /api/punches` (hari ini) dan muncul di `?date=2026-10-28`. Suite `02-punches.hurl` gagal terhadap Laravel bila dijalankan antara 00:00 dan 07:00 WIB.

Rust membandingkan `punched_at` dengan batas hari yang dikonversi ke UTC dengan benar.

### `accuracy` 32768-65535 membuat Laravel gagal 500

`MobilePunchRequest` mengizinkan `accuracy` sampai 65535, tetapi kolom `accuracy_meters` bertipe `smallint` (maks 32767), jadi insert Laravel gagal. Rust menerima nilai sesuai kontrak dan menyimpannya dibatasi 32767 (akurasi di atas 32 km tidak bermakna). Perbaikan di Laravel: `max:32767` atau kolom `integer`.

### Tanggal tidak ada di `GET /api/punches?date=`

Laravel menerima `2026-02-30` (`Carbon::hasFormat` hanya memeriksa pola) lalu menggulirkannya ke 2 Maret. Rust memperlakukan tanggal yang tidak ada sebagai tidak valid, jadi kembali ke hari ini, sesuai dokumen ("kosong atau tidak valid berarti hari ini").

## Tambahan di Rust (tidak mengubah kontrak)

- Punch dan pengajuan cuti per pegawai diserialkan dengan advisory lock Postgres; dua request serentak tidak bisa sama-sama lolos cek interval/tumpang tindih. Check-in memakai unique constraint `(event_id, employee_id)`.
- Foto ditulis ke berkas sementara lalu di-rename, dan dihapus bila transaksi database gagal.
- Jenis foto ditentukan dari isi berkas (magic bytes), bukan dari nama berkas atau content-type kiriman klien. Perbaikan di Laravel: `->utc()` pada kedua batas di `forEmployeeBetween` (atau di controller). Pola yang sama perlu dicek di `between()` dan `inTree()` repository yang sama (kalkulator dan laporan web).
