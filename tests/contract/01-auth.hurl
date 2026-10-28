# Kontrak autentikasi API mobile (docs/mobile-api.md: Login, Profil dan keluar).
# Variabel: base_url, password. Data: ContractTestSeeder.

# Sandi salah: 422 dengan pesan yang sama untuk semua kegagalan akun.
POST {{base_url}}/api/auth/login
Accept: application/json
{
    "login": "kontrak",
    "password": "salah-sekali",
    "device_name": "kontrak-hurl"
}
HTTP 422
[Asserts]
jsonpath "$.message" == "Username atau kata sandi salah."
jsonpath "$.token" not exists

# Akun nonaktif: pesan sama dengan sandi salah (tidak membocorkan akun).
POST {{base_url}}/api/auth/login
Accept: application/json
{
    "login": "kontrak.akun-nonaktif",
    "password": "{{password}}",
    "device_name": "kontrak-hurl"
}
HTTP 422
[Asserts]
jsonpath "$.message" == "Username atau kata sandi salah."

# Akun belum terhubung ke pegawai: 403.
POST {{base_url}}/api/auth/login
Accept: application/json
{
    "login": "kontrak.tanpa-pegawai",
    "password": "{{password}}",
    "device_name": "kontrak-hurl"
}
HTTP 403
[Asserts]
jsonpath "$.message" == "Akun ini belum terhubung ke data pegawai."

# Field wajib kosong: 422 dengan format galat validasi Laravel.
POST {{base_url}}/api/auth/login
Accept: application/json
{
}
HTTP 422
[Asserts]
jsonpath "$.message" isString
jsonpath "$.errors.login" isCollection
jsonpath "$.errors.password" isCollection
jsonpath "$.errors.device_name" isCollection

# Login berhasil dengan email: token dan profil.
POST {{base_url}}/api/auth/login
Accept: application/json
{
    "login": "kontrak@kontrak.test",
    "password": "{{password}}",
    "device_name": "kontrak-hurl"
}
HTTP 200
[Captures]
token: jsonpath "$.token"
[Asserts]
jsonpath "$.token" matches /^\d+\|[A-Za-z0-9]{40,}$/
jsonpath "$.user.username" == "kontrak"
jsonpath "$.user.employee.employee_number" == "KTR-001"
jsonpath "$.user.employee.name" == "Pegawai Kontrak"
jsonpath "$.user.employee.department" == "Department Kontrak"
jsonpath "$.user.timezone" == "Asia/Jakarta"
jsonpath "$.user.requires_photo" isBoolean
jsonpath "$.user.requires_approval" isBoolean

# Login ulang dengan device_name yang sama menggantikan token lama.
POST {{base_url}}/api/auth/login
Accept: application/json
{
    "login": "kontrak",
    "password": "{{password}}",
    "device_name": "kontrak-hurl"
}
HTTP 200
[Captures]
token2: jsonpath "$.token"

GET {{base_url}}/api/me
Accept: application/json
Authorization: Bearer {{token}}
HTTP 401
[Asserts]
jsonpath "$.message" == "Unauthenticated."

# Profil dibungkus `data`.
GET {{base_url}}/api/me
Accept: application/json
Authorization: Bearer {{token2}}
HTTP 200
[Asserts]
jsonpath "$.data.username" == "kontrak"
jsonpath "$.data.employee.employee_number" == "KTR-001"
jsonpath "$.data.timezone" == "Asia/Jakarta"

# Tanpa token: 401.
GET {{base_url}}/api/me
Accept: application/json
HTTP 401
[Asserts]
jsonpath "$.message" == "Unauthenticated."

# Keluar mencabut token yang dipakai.
POST {{base_url}}/api/auth/logout
Accept: application/json
Authorization: Bearer {{token2}}
HTTP 200
[Asserts]
jsonpath "$.message" == "Berhasil keluar."

GET {{base_url}}/api/me
Accept: application/json
Authorization: Bearer {{token2}}
HTTP 401
