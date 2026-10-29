# Rancangan API v2: fitur aplikasi yang belum ada di kontrak

`/api/v2` adalah **versi 2 API mobile** (endpoint tambahan untuk aplikasi pegawai), tidak berhubungan dengan repo web `eabsensi-v2`. Semua endpoint di sini untuk aplikasi mobile; kaitannya dengan `eabsensi-v2` hanya karena keduanya memakai database yang sama, jadi tabel/kolom baru tetap dibuat lewat migration di sana.

Status: **rancangan, belum dikerjakan** (30 Oktober 2026). Sumber kebutuhan: domain aplikasi Flutter (`../eabsensi-mobile-app/lib/features/*/domain`, catatan "belum ada di kontrak") dan `docs/design/DEVIATIONS.md` di repo itu.

Prinsip:
- **Tambahan, tidak memutus**: endpoint baru di `/api/v2/*`; field baru di respons v1 hanya bila opsional. Aplikasi lama tetap jalan.
- **Skema tetap milik `eabsensi-v2`**: tabel/kolom baru dibuat lewat migration di sana dulu (expand), baru dipakai di sini.
- **Aturan bisnis tidak dikarang**: bila aturan sudah ada di Laravel, di-port dengan test kasus bersama (pola `tests/contract/adms/outcome-cases.json`), atau Laravel yang mengerjakan.

## Ringkasan

| Fitur | Endpoint | Butuh skema baru di eabsensi-v2 | Fase |
|---|---|---|---|
| Peran atasan di profil | `GET /api/v2/me` | tidak | 1 |
| Rekap riwayat bulanan | `GET /api/v2/attendance/monthly` | tidak (port kalkulator) | 1 |
| Menu dinamis | `GET /api/v2/menus` | tabel `mobile_menus` (fase 1 memakai menu bawaan) | 1 → 2 |
| QR check-in event | field `qr_code` di check-in, `requires_qr` di event | kolom di `attendance_events` | 2 |
| Lampiran cuti | `POST /api/v2/leaves` (multipart), `GET .../attachment` | kolom di `leave_requests` | 2 |
| Persetujuan punch | `GET /api/v2/approvals`, `POST .../punches/{id}` | tidak | 3 |
| Persetujuan cuti | `POST .../leaves/{id}` | tidak (efek samping roster, lihat catatan) | 3 |
| Notifikasi | `GET /api/v2/notifications`, tandai dibaca | tabel `user_notifications` | 3 |

## Fase 1

### `GET /api/v2/me`

Sama dengan `/api/me` ditambah kemampuan, supaya aplikasi tidak menebak peran:

```json
{ "data": { "...field /api/me...",
  "abilities": { "approve_punches": true, "approve_leaves": false } } }
```

Sumber: permission Spatie `punches.approve` / `leaves.approve` milik role pengguna (`model_has_roles`, `role_has_permissions`, `model_has_permissions`). Menggantikan `UserProfile.isSupervisor` di aplikasi.

### `GET /api/v2/attendance/monthly?month=YYYY-MM`

Untuk layar Riwayat (tile Hadir/Terlambat/Tidak hadir, daftar per hari):

```json
{ "data": {
  "month": "2026-10",
  "summary": { "present": 18, "late": 2, "absent": 1, "leave": 1, "off": 8 },
  "days": [
    { "date": "2026-10-29", "status": "present", "status_label": "Hadir", "shift": "PAGI",
      "first_in": "2026-10-29T07:52:10+07:00", "last_out": null,
      "late_minutes": 0, "early_minutes": 0, "leave_type": null, "pending_punches": 1 } ] } }
```

`status` mengikuti `DailyStatus` Laravel (`present`, `absent`, `leave`, `off`, `unscheduled`); "terlambat" = `present` dengan `late_minutes > 0`. Hanya punch berstatus `approved` yang dihitung (aturan Laravel); `pending_punches` memberi tahu aplikasi ada punch yang menunggu persetujuan.

Sumber data, dua pilihan:

| Pilihan | Cara | Untung | Rugi |
|---|---|---|---|
| **A (usulan)** | Port `AttendanceCalculator` (136 baris) + pengumpul data `AttendanceService::forRange` (roster, shift, libur, punch approved); bulan tertutup dibaca dari snapshot `attendance_daily` | Data hari ini langsung benar; tidak ada perubahan di Laravel | Aturan kalkulator ditulis di dua tempat: wajib berkas kasus bersama yang dijalankan kedua repo di CI |
| B | Laravel menulis hasil harian juga untuk periode terbuka (job malam + setelah persetujuan), Rust hanya membaca | Satu sumber aturan | `attendance_daily.period_id` wajib diisi (perlu perubahan skema), data tertinggal sampai job jalan |

### `GET /api/v2/menus`

Bentuk mengikuti model aplikasi (`MenuItemData`):

```json
{ "inactive_mode": "locked",
  "data": [ { "id": "cuti", "label": "Cuti", "icon": "calendar-x", "tone": "mint",
              "is_active": true, "sort": 10,
              "action": { "type": "leave_form", "category": "cuti", "type_code": null } } ] }
```

`action.type`: `leave_form`, `events`, `coming_soon`, `url`. Fase 1: daftar bawaan dari berkas konfigurasi service (sama dengan menu statis aplikasi sekarang). Fase 2: tabel `mobile_menus` yang dikelola operator di web.

## Fase 2 (setelah migration di eabsensi-v2)

### QR check-in

- Migration: `attendance_events.requires_qr boolean default false`, `attendance_events.qr_secret varchar(64) null`.
- Web menampilkan QR berganti setiap 30 detik di lokasi upacara: `kode = HMAC-SHA256(qr_secret, floor(unix/30))` dipotong 10 karakter (seperti TOTP), jadi foto QR yang dibagikan cepat basi.
- `GET /api/events`: field tambahan `requires_qr` (opsional, aplikasi lama mengabaikannya).
- `POST /api/events/{id}/check-in`: field tambahan `qr_code`. Bila event `requires_qr`, kode harus cocok dengan jendela sekarang atau satu sebelumnya, selain itu 422 `Kode QR tidak valid atau sudah kedaluwarsa.` Event tanpa `requires_qr` tidak berubah.
- Aturan bersama baru (Laravel membuat QR, Rust memeriksa): algoritma dan panjang kode masuk daftar aturan bersama di README kedua repo.

### Lampiran cuti

- Migration: `leave_requests.attachment_path varchar(255) null`.
- `POST /api/v2/leaves`: multipart, field sama dengan v1 + `attachment` opsional (pdf/jpg/png maks 5 MB, jenis dari isi berkas). Disimpan `leave-attachments/Y/m/<kode department>/<40 acak>.<ext>` di folder yang sama dengan foto.
- Respons cuti v2 menambah `has_attachment`. `GET /api/v2/leaves/{id}/attachment`: berkas milik sendiri.
- Web menampilkan lampiran di halaman persetujuan cuti.

### Menu dari database

- Migration + CRUD web: `mobile_menus (id, key, label, icon, tone, action_type, action_payload json, sort, is_active, timestamps)`.

## Fase 3

### Persetujuan

- `GET /api/v2/approvals?type=punch|leave`: pengajuan `pending` milik pegawai dalam cakupan data atasan (`data_scope` global/region/department + subtree, sama dengan `Employee::visibleTo`), terlama dulu, dengan nomor pegawai, nama, department, waktu, dan untuk punch `has_photo`.
- `GET /api/v2/approvals/punches/{id}/photo`: foto punch untuk atasan berwenang.
- `POST /api/v2/approvals/punches/{id}` `{ "decision": "approve" | "reject" }`: port `ReviewPunches` (+ `AttendancePunchPolicy`).
- `POST /api/v2/approvals/leaves/{id}` `{ "decision", "note" }`: `ReviewLeave` menulis roster dan memicu job snapshot periode di antrean Laravel. Rust tidak bisa membuat job PHP dengan aman, jadi usulan: persetujuan cuti tetap dieksekusi Laravel. Pilihan: (a) aplikasi membuka halaman web persetujuan, atau (b) Laravel menyediakan endpoint internal yang dipanggil Rust. Perlu keputusan.

### Notifikasi

- Migration: `user_notifications (id, user_id, type, title, body, data json, read_at, created_at)`, ditulis oleh Laravel (cuti/punch disetujui atau ditolak, pengumuman) dan Rust (pengajuan baru untuk atasan).
- `GET /api/v2/notifications?page=`, `POST /api/v2/notifications/{id}/read`, `POST /api/v2/notifications/read-all`.
- Push ke ponsel (FCM) di luar rancangan ini: butuh akun Firebase dan tabel token perangkat.

## Keputusan yang dibutuhkan

1. Rekap bulanan: pilihan A (port kalkulator) atau B (Laravel menulis hasil harian)?
2. Persetujuan cuti dari aplikasi: (a) buka halaman web, atau (b) endpoint internal Laravel?
3. Urutan fase: setuju 1 → 2 → 3, atau ada fitur yang lebih mendesak?
4. Push notification (FCM) masuk rencana atau cukup kotak masuk di aplikasi?
