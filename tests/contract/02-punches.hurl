# Kontrak punch mobile (docs/mobile-api.md: Punch). Mengandalkan default
# ATTENDANCE_MOBILE_REQUIRES_PHOTO=true, ATTENDANCE_MOBILE_REQUIRES_APPROVAL=true, ATTENDANCE_MOBILE_MIN_INTERVAL=60.

POST {{base_url}}/api/auth/login
Accept: application/json
{
    "login": "kontrak",
    "password": "{{password}}",
    "device_name": "kontrak-hurl-punch"
}
HTTP 200
[Captures]
token: jsonpath "$.token"

# Tanpa foto saat foto wajib: galat validasi pada field `photo`.
POST {{base_url}}/api/punches
Accept: application/json
Authorization: Bearer {{token}}
[MultipartFormData]
latitude: -6.2
longitude: 106.8
HTTP 422
[Asserts]
jsonpath "$.errors.photo" isCollection

# Berkas bukan gambar ditolak.
POST {{base_url}}/api/punches
Accept: application/json
Authorization: Bearer {{token}}
[MultipartFormData]
photo: file,fixtures/not-an-image.txt; text/plain
HTTP 422
[Asserts]
jsonpath "$.errors.photo" isCollection

# Latitude tanpa longitude ditolak.
POST {{base_url}}/api/punches
Accept: application/json
Authorization: Bearer {{token}}
[MultipartFormData]
photo: file,fixtures/photo.png; image/png
latitude: -6.2
HTTP 422
[Asserts]
jsonpath "$.errors.longitude" isCollection

# Punch berhasil: waktu dari server di zona bisnis, menunggu persetujuan.
POST {{base_url}}/api/punches
Accept: application/json
Authorization: Bearer {{token}}
[MultipartFormData]
photo: file,fixtures/photo.png; image/png
latitude: -6.2
longitude: 106.8
accuracy: 12
HTTP 201
[Asserts]
jsonpath "$.data.id" isInteger
jsonpath "$.data.punched_at" matches /^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}\+07:00$/
jsonpath "$.data.source" == "mobile"
jsonpath "$.data.status" == "pending"
jsonpath "$.data.status_label" isString
jsonpath "$.data.has_photo" == true

# Punch lagi sebelum interval minimum: ditolak dengan pesan aturan.
POST {{base_url}}/api/punches
Accept: application/json
Authorization: Bearer {{token}}
[MultipartFormData]
photo: file,fixtures/photo.png; image/png
HTTP 422
[Asserts]
jsonpath "$.message" == "Tunggu 60 detik sebelum melakukan punch lagi."
jsonpath "$.errors" not exists

# Riwayat hari ini: punch tadi ada, semua status.
GET {{base_url}}/api/punches
Accept: application/json
Authorization: Bearer {{token}}
HTTP 200
[Asserts]
jsonpath "$.data" count == 1
jsonpath "$.data[0].source" == "mobile"
jsonpath "$.data[0].status" == "pending"

# Tanggal lain tanpa punch: daftar kosong.
GET {{base_url}}/api/punches?date=2000-01-01
Accept: application/json
Authorization: Bearer {{token}}
HTTP 200
[Asserts]
jsonpath "$.data" count == 0

# Pegawai nonaktif tidak bisa punch.
POST {{base_url}}/api/auth/login
Accept: application/json
{
    "login": "kontrak.pegawai-nonaktif",
    "password": "{{password}}",
    "device_name": "kontrak-hurl-punch"
}
HTTP 200
[Captures]
inactive_token: jsonpath "$.token"

POST {{base_url}}/api/punches
Accept: application/json
Authorization: Bearer {{inactive_token}}
[MultipartFormData]
photo: file,fixtures/photo.png; image/png
HTTP 422
[Asserts]
jsonpath "$.message" == "Pegawai tidak aktif."
