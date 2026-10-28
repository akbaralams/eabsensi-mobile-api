# Kontrak event dan check-in (docs/mobile-api.md: Event). Seeder membuat "Apel Kontrak" hari ini 00:00-23:59
# untuk semua pegawai, dan satu event untuk department lain yang tidak boleh terlihat.

POST {{base_url}}/api/auth/login
Accept: application/json
{
    "login": "kontrak",
    "password": "{{password}}",
    "device_name": "kontrak-hurl-event"
}
HTTP 200
[Captures]
token: jsonpath "$.token"

GET {{base_url}}/api/events
Accept: application/json
Authorization: Bearer {{token}}
HTTP 200
[Captures]
event_id: jsonpath "$.data[?(@.name == 'Apel Kontrak')].id"
[Asserts]
jsonpath "$.data[?(@.name == 'Apel Kontrak')]" exists
jsonpath "$.data[?(@.name == 'Apel Kontrak Department Lain')]" not exists
jsonpath "$.data[?(@.name == 'Apel Kontrak')].event_date" matches /^\d{4}-\d{2}-\d{2}$/
jsonpath "$.data[?(@.name == 'Apel Kontrak')].starts_at" == "00:00"
jsonpath "$.data[?(@.name == 'Apel Kontrak')].ends_at" == "23:59"
jsonpath "$.data[?(@.name == 'Apel Kontrak')].checked_in" == false

# Check-in tanpa foto saat foto wajib.
POST {{base_url}}/api/events/{{event_id}}/check-in
Accept: application/json
Authorization: Bearer {{token}}
[MultipartFormData]
latitude: -6.2
longitude: 106.8
HTTP 422
[Asserts]
jsonpath "$.errors.photo" isCollection

POST {{base_url}}/api/events/{{event_id}}/check-in
Accept: application/json
Authorization: Bearer {{token}}
[MultipartFormData]
photo: file,fixtures/photo.png; image/png
latitude: -6.2
longitude: 106.8
HTTP 201
[Asserts]
jsonpath "$.data.event_id" == {{event_id}}
jsonpath "$.data.checked_in_at" matches /^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}\+07:00$/

# Satu pegawai satu check-in per event.
POST {{base_url}}/api/events/{{event_id}}/check-in
Accept: application/json
Authorization: Bearer {{token}}
[MultipartFormData]
photo: file,fixtures/photo.png; image/png
HTTP 422
[Asserts]
jsonpath "$.message" == "Anda sudah check-in di event ini."

GET {{base_url}}/api/events
Accept: application/json
Authorization: Bearer {{token}}
HTTP 200
[Asserts]
jsonpath "$.data[?(@.name == 'Apel Kontrak')].checked_in" == true

# Event yang tidak ada: 404.
POST {{base_url}}/api/events/999999999/check-in
Accept: application/json
Authorization: Bearer {{token}}
[MultipartFormData]
photo: file,fixtures/photo.png; image/png
HTTP 404
