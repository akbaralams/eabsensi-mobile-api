# Kontrak /api/v2/auth (refresh token). Hanya service Rust; jalankan dengan INCLUDE_V2=1.

POST {{base_url}}/api/v2/auth/login
Accept: application/json
{
    "login": "kontrak@kontrak.test",
    "password": "{{password}}",
    "device_name": "kontrak-hurl-v2"
}
HTTP 200
[Captures]
access: jsonpath "$.token"
refresh: jsonpath "$.refresh_token"
[Asserts]
jsonpath "$.token" matches /^\d+\|[A-Za-z0-9]{40,}$/
jsonpath "$.refresh_token" matches /^\d+\|[A-Za-z0-9]{40,}$/
jsonpath "$.token_expires_at" matches /^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}\+07:00$/
jsonpath "$.refresh_expires_at" matches /^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}\+07:00$/
jsonpath "$.user.username" == "kontrak"

# Token akses dipakai ke endpoint v1.
GET {{base_url}}/api/me
Accept: application/json
Authorization: Bearer {{access}}
HTTP 200

# Refresh token bukan Bearer.
GET {{base_url}}/api/me
Accept: application/json
Authorization: Bearer {{refresh}}
HTTP 403
[Asserts]
jsonpath "$.message" == "Invalid ability provided."

POST {{base_url}}/api/v2/auth/refresh
Accept: application/json
{
}
HTTP 422
[Asserts]
jsonpath "$.errors.refresh_token" isCollection

# Rotasi: pasangan baru, yang lama tidak berlaku.
POST {{base_url}}/api/v2/auth/refresh
Accept: application/json
{
    "refresh_token": "{{refresh}}"
}
HTTP 200
[Captures]
access2: jsonpath "$.token"
[Asserts]
jsonpath "$.refresh_token" != "{{refresh}}"

POST {{base_url}}/api/v2/auth/refresh
Accept: application/json
{
    "refresh_token": "{{refresh}}"
}
HTTP 401
[Asserts]
jsonpath "$.message" == "Unauthenticated."

GET {{base_url}}/api/me
Accept: application/json
Authorization: Bearer {{access}}
HTTP 401

POST {{base_url}}/api/v2/auth/logout
Accept: application/json
Authorization: Bearer {{access2}}
HTTP 200
[Asserts]
jsonpath "$.message" == "Berhasil keluar."

GET {{base_url}}/api/me
Accept: application/json
Authorization: Bearer {{access2}}
HTTP 401
