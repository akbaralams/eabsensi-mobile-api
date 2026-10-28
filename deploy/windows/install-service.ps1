<#
.SYNOPSIS
  Pasang eabsensi-mobile-api sebagai Windows service memakai NSSM (https://nssm.cc).

.DESCRIPTION
  Binary Rust adalah program konsol biasa; NSSM membungkusnya menjadi service, meneruskan
  environment, mengalihkan log ke berkas, dan menyalakan ulang bila proses berhenti.
  Jalankan di PowerShell sebagai Administrator.

.EXAMPLE
  .\install-service.ps1 -Binary C:\eabsensi\eabsensi-mobile-api.exe -EnvFile C:\eabsensi\mobile-api.env -Port 8080
#>
param(
    [Parameter(Mandatory)] [string] $Binary,
    [Parameter(Mandatory)] [string] $EnvFile,
    [int] $Port = 8080,
    [string] $Nssm = 'nssm.exe',
    [string] $LogDir = 'C:\eabsensi\logs'
)

$ErrorActionPreference = 'Stop'
$name = "eabsensi-mobile-api-$Port"

if (-not (Get-Command $Nssm -ErrorAction SilentlyContinue)) {
    throw "NSSM tidak ditemukan. Unduh dari https://nssm.cc lalu tambahkan ke PATH atau isi -Nssm."
}
if (-not (Test-Path $Binary)) { throw "Binary tidak ada: $Binary" }
if (-not (Test-Path $EnvFile)) { throw "Berkas env tidak ada: $EnvFile" }
New-Item -ItemType Directory -Force $LogDir | Out-Null

# Baca KEY=VALUE dari berkas env (abaikan komentar dan baris kosong).
$vars = Get-Content $EnvFile |
    Where-Object { $_ -match '^\s*[A-Z_][A-Z0-9_]*=' } |
    ForEach-Object { $_.Trim() }
$vars += "LISTEN_ADDR=127.0.0.1:$Port"
$vars += 'LOG_FORMAT=json'

& $Nssm install $name $Binary
& $Nssm set $name AppDirectory (Split-Path $Binary)
& $Nssm set $name AppEnvironmentExtra $vars
& $Nssm set $name AppStdout (Join-Path $LogDir "$name.log")
& $Nssm set $name AppStderr (Join-Path $LogDir "$name.log")
& $Nssm set $name AppRotateFiles 1
& $Nssm set $name AppRotateBytes 52428800
& $Nssm set $name AppExit Default Restart
& $Nssm set $name AppRestartDelay 2000
# Windows tidak punya SIGTERM: NSSM mengirim Ctrl+C dulu (service berhenti dengan rapi), lalu menunggu.
& $Nssm set $name AppStopMethodConsole 30000
& $Nssm set $name Start SERVICE_AUTO_START
& $Nssm start $name

Write-Host "Service $name terpasang. Cek: Invoke-WebRequest http://127.0.0.1:$Port/healthz"
