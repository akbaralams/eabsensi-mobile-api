# Kontrak lokasi kerja (docs/mobile-api.md: Lokasi kerja). Akun `kontrak.lokasi` memakai lokasi di -6.2, 106.8
# dengan radius 100 m. Mengandalkan default ATTENDANCE_LOCATION_MAX_ACCURACY=100 dan ATTENDANCE_LOCATION_MAX_FIX_AGE=120.

POST {{base_url}}/api/auth/login
Accept: application/json
{
    "login": "kontrak.lokasi",
    "password": "{{password}}",
    "device_name": "kontrak-hurl-lokasi"
}
HTTP 200
[Captures]
token: jsonpath "$.token"
[Asserts]
jsonpath "$.user.work_location.name" == "Lokasi Kontrak"

# Profil memuat lokasi yang berlaku.
GET {{base_url}}/api/me
Accept: application/json
Authorization: Bearer {{token}}
HTTP 200
[Asserts]
jsonpath "$.data.work_location.name" == "Lokasi Kontrak"
jsonpath "$.data.work_location.latitude" == -6.2
jsonpath "$.data.work_location.longitude" == 106.8
jsonpath "$.data.work_location.radius_meters" == 100

# Nilai sinyal integritas di luar kontrak: galat validasi per field.
POST {{base_url}}/api/punches
Accept: application/json
Authorization: Bearer {{token}}
[MultipartFormData]
photo: file,fixtures/photo.png; image/png
is_mocked: 2
location_age_seconds: -1
device_rooted: 2
HTTP 422
[Asserts]
jsonpath "$.errors.is_mocked" isCollection
jsonpath "$.errors.location_age_seconds" isCollection
jsonpath "$.errors.device_rooted" isCollection

# Tanpa koordinat.
POST {{base_url}}/api/punches
Accept: application/json
Authorization: Bearer {{token}}
[MultipartFormData]
photo: file,fixtures/photo.png; image/png
HTTP 422
[Asserts]
jsonpath "$.message" == "Lokasi wajib diaktifkan untuk melakukan absen."
jsonpath "$.errors" not exists

# Aplikasi lama: koordinat ada, sinyal integritas tidak.
POST {{base_url}}/api/punches
Accept: application/json
Authorization: Bearer {{token}}
[MultipartFormData]
photo: file,fixtures/photo.png; image/png
latitude: -6.2
longitude: 106.8
accuracy: 10
HTTP 422
[Asserts]
jsonpath "$.message" == "Perbarui aplikasi ke versi terbaru untuk melakukan absen."

# Lokasi palsu.
POST {{base_url}}/api/punches
Accept: application/json
Authorization: Bearer {{token}}
[MultipartFormData]
photo: file,fixtures/photo.png; image/png
latitude: -6.2
longitude: 106.8
accuracy: 10
is_mocked: 1
location_age_seconds: 5
device_rooted: 0
HTTP 422
[Asserts]
jsonpath "$.message" == "Lokasi palsu terdeteksi. Matikan aplikasi pengubah lokasi lalu coba lagi."

# GPS kedaluwarsa.
POST {{base_url}}/api/punches
Accept: application/json
Authorization: Bearer {{token}}
[MultipartFormData]
photo: file,fixtures/photo.png; image/png
latitude: -6.2
longitude: 106.8
accuracy: 10
is_mocked: 0
location_age_seconds: 121
device_rooted: 0
HTTP 422
[Asserts]
jsonpath "$.message" == "Lokasi sudah kedaluwarsa. Silakan coba lagi."

# Akurasi buruk.
POST {{base_url}}/api/punches
Accept: application/json
Authorization: Bearer {{token}}
[MultipartFormData]
photo: file,fixtures/photo.png; image/png
latitude: -6.2
longitude: 106.8
accuracy: 101
is_mocked: 0
location_age_seconds: 5
device_rooted: 0
HTTP 422
[Asserts]
jsonpath "$.message" == "Sinyal GPS lemah. Pindah ke area terbuka lalu coba lagi."

# Di luar radius: tetap tercatat dan diberi tanda.
POST {{base_url}}/api/punches
Accept: application/json
Authorization: Bearer {{token}}
[MultipartFormData]
photo: file,fixtures/photo.png; image/png
latitude: -6.21
longitude: 106.8
accuracy: 10
is_mocked: 0
location_age_seconds: 5
device_rooted: 1
HTTP 201
[Asserts]
jsonpath "$.data.status" == "pending"
jsonpath "$.data.location_flags" count == 2
jsonpath "$.data.location_flags[0]" == "outside_radius"
jsonpath "$.data.location_flags[1]" == "rooted_device"

# Riwayat memuat tanda yang sama.
GET {{base_url}}/api/punches
Accept: application/json
Authorization: Bearer {{token}}
HTTP 200
[Asserts]
jsonpath "$.data[0].location_flags[0]" == "outside_radius"
