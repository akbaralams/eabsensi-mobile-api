# API mobile eAbsensi

Dasar URL: `/api`. Semua respons JSON (juga galat, termasuk tanpa header `Accept`; klien tetap sebaiknya mengirim `Accept: application/json`). Spesifikasi mesin: [openapi.yaml](openapi.yaml). Semua endpoint ber-token dibatasi 120/menit per token. Autentikasi memakai token Sanctum (`Authorization: Bearer <token>`).
Token (ability `mobile`) berlaku 30 hari (`SANCTUM_TOKEN_EXPIRATION`, menit). Satu perangkat satu token: login ulang dengan `device_name` yang sama menggantikan token lama.

## Login
`POST /api/auth/login`

| Field | Wajib | Keterangan |
|---|---|---|
| `login` | ya | username atau email |
| `password` | ya | |
| `device_name` | ya | nama perangkat, maks 100 karakter |

Berhasil (200): `{ "token": "...", "user": { id, name, username, employee: { id, employee_number, name, department }, timezone, requires_photo, requires_approval } }`.
Gagal: 422 (username/sandi salah atau akun nonaktif, pesan sama), 403 (akun belum terhubung ke pegawai), 429 (terlalu banyak percobaan: 60/menit per IP, 5/menit per akun+IP).

## Profil dan keluar
- `GET /api/me`: profil seperti di atas, dibungkus `data`.
- `POST /api/auth/logout`: mencabut token yang sedang dipakai.

## Punch
`POST /api/punches` (multipart/form-data, maks 20/menit)

| Field | Wajib | Keterangan |
|---|---|---|
| `photo` | bila `requires_photo` | jpg/png, maks 4 MB |
| `latitude`, `longitude` | tidak | harus berpasangan |
| `accuracy` | tidak | meter, bilangan bulat |

Waktu punch selalu dari server (field waktu dari klien diabaikan). Berhasil: 201 `{ "data": { id, punched_at (ISO 8601, zona waktu bisnis), source, status, status_label, has_photo } }`.
`status` bernilai `pending` bila `requires_approval` aktif (menunggu persetujuan atasan, belum dihitung sebagai kehadiran) atau `approved`.

Ditolak 422 dengan `message`: akun tanpa pegawai, pegawai nonaktif, punch lagi sebelum `ATTENDANCE_MOBILE_MIN_INTERVAL` detik (default 60), atau periode bulan berjalan sudah ditutup. Validasi lain memakai format Laravel (`errors`).

`GET /api/punches?date=YYYY-MM-DD`: punch milik sendiri pada satu hari (default hari ini), semua status.

## Event (mis. upacara)
- `GET /api/events`: event aktif **hari ini** yang boleh diikuti pegawai (untuk semua pegawai, atau department induknya), beserta `checked_in`.
- `POST /api/events/{id}/check-in` (multipart, field sama dengan punch: `photo`, `latitude`, `longitude`, `accuracy`): 201 `{ "data": { event_id, checked_in_at } }`.
  Ditolak 422 dengan `message` bila: bukan peserta, event nonaktif, bukan tanggal event, di luar jam mulai–selesai (zona waktu bisnis), atau sudah check-in. Satu pegawai satu check-in per event.

## Cuti
- `GET /api/leaves`: pengajuan milik sendiri (terbaru dulu) dan `types` (jenis yang bisa dipilih: status kehadiran aktif).
- `POST /api/leaves` (JSON): `attendance_status_id`, `start_date`, `end_date` (YYYY-MM-DD), `reason` (opsional). 201 dengan `data` berstatus `pending`.
  Ditolak 422 bila rentang lebih dari 62 hari atau tumpang tindih dengan pengajuan aktif (menunggu atau disetujui). Persetujuan dilakukan atasan di web; saat disetujui, status cuti terisi di roster.

## Konfigurasi (`config/attendance.php`)
`ATTENDANCE_TIMEZONE` (default Asia/Jakarta), `ATTENDANCE_MOBILE_REQUIRES_APPROVAL` (true), `ATTENDANCE_MOBILE_REQUIRES_PHOTO` (true), `ATTENDANCE_MOBILE_MIN_INTERVAL` (60).

## Catatan operasional
Foto disimpan di disk `local` pada `mobile-punches/Y/m/d/<kode department>/` dan hanya bisa dilihat lewat halaman persetujuan oleh user yang berwenang atas pegawainya.
Hubungkan akun pengguna ke pegawai lewat kolom `users.employee_id` (UI pengelolaan pengguna menyusul).
