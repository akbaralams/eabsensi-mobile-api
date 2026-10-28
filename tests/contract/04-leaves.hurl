# Kontrak cuti (docs/mobile-api.md: Cuti). Tanggal di tahun 2099 agar tidak bergantung pada hari ini.

POST {{base_url}}/api/auth/login
Accept: application/json
{
    "login": "kontrak",
    "password": "{{password}}",
    "device_name": "kontrak-hurl-cuti"
}
HTTP 200
[Captures]
token: jsonpath "$.token"

GET {{base_url}}/api/leaves
Accept: application/json
Authorization: Bearer {{token}}
HTTP 200
[Captures]
type_id: jsonpath "$.types[0].id"
[Asserts]
jsonpath "$.data" count == 0
jsonpath "$.types" count > 0
jsonpath "$.types[0].id" isInteger
jsonpath "$.types[0].name" isString

# Validasi: tanggal akhir sebelum tanggal awal.
POST {{base_url}}/api/leaves
Accept: application/json
Authorization: Bearer {{token}}
{
    "attendance_status_id": {{type_id}},
    "start_date": "2099-01-10",
    "end_date": "2099-01-05"
}
HTTP 422
[Asserts]
jsonpath "$.errors.end_date" isCollection

# Rentang lebih dari 62 hari ditolak dengan pesan aturan.
POST {{base_url}}/api/leaves
Accept: application/json
Authorization: Bearer {{token}}
{
    "attendance_status_id": {{type_id}},
    "start_date": "2099-01-01",
    "end_date": "2099-03-15"
}
HTTP 422
[Asserts]
jsonpath "$.message" == "Rentang cuti maksimal 62 hari."

POST {{base_url}}/api/leaves
Accept: application/json
Authorization: Bearer {{token}}
{
    "attendance_status_id": {{type_id}},
    "start_date": "2099-01-05",
    "end_date": "2099-01-07",
    "reason": "Uji kontrak"
}
HTTP 201
[Asserts]
jsonpath "$.data.id" isInteger
jsonpath "$.data.attendance_status_id" == {{type_id}}
jsonpath "$.data.type" isString
jsonpath "$.data.start_date" == "2099-01-05"
jsonpath "$.data.end_date" == "2099-01-07"
jsonpath "$.data.reason" == "Uji kontrak"
jsonpath "$.data.status" == "pending"
jsonpath "$.data.status_label" isString

# Tumpang tindih dengan pengajuan yang masih menunggu.
POST {{base_url}}/api/leaves
Accept: application/json
Authorization: Bearer {{token}}
{
    "attendance_status_id": {{type_id}},
    "start_date": "2099-01-07",
    "end_date": "2099-01-09"
}
HTTP 422
[Asserts]
jsonpath "$.message" == "Pegawai sudah punya pengajuan yang menunggu atau disetujui pada tanggal tersebut."

GET {{base_url}}/api/leaves
Accept: application/json
Authorization: Bearer {{token}}
HTTP 200
[Asserts]
jsonpath "$.data" count == 1
jsonpath "$.data[0].start_date" == "2099-01-05"
