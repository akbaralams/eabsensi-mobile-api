#!/usr/bin/env bash
# Jalankan suite kontrak API mobile (tests/contract/*.hurl) terhadap server yang sedang berjalan:
# service Rust di repo ini, atau Laravel (eabsensi-v2) sebagai pembanding.
#
# Pemakaian (dari root repo; database uji/dev, BUKAN produksi):
#   DATABASE_URL=postgres://user:pass@127.0.0.1:5433/eabsensi_test BASE_URL=http://127.0.0.1:8080 bash scripts/contract-test.sh
#
# Variabel:
#   BASE_URL                 alamat server yang diuji (default http://127.0.0.1:8080)
#   DATABASE_URL             database yang dipakai server itu; fixture dimuat ke sini lewat psql
#   SEED_CONTRACT_PASSWORD   password akun kontrak* (default kontrak-uji-2026, sama dengan fixture)
#   ATTENDANCE_TIMEZONE      zona bisnis untuk tanggal event fixture (default Asia/Jakarta)
#   SKIP_SEED=1              lewati fixture (database sudah disiapkan)
#   INCLUDE_V2=1             ikut jalankan tests/contract/v2 (hanya service Rust; Laravel tidak punya /api/v2)
#   PSQL, HURL               path biner bila tidak ada di PATH
# Argumen tambahan diteruskan ke hurl (mis. tests/contract/01-auth.hurl untuk satu file saja).
set -euo pipefail

BASE_URL="${BASE_URL:-http://127.0.0.1:8080}"
PASSWORD="${SEED_CONTRACT_PASSWORD:-kontrak-uji-2026}"
TZ_BUSINESS="${ATTENDANCE_TIMEZONE:-Asia/Jakarta}"
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
PSQL="${PSQL:-psql}"
HURL="${HURL:-hurl}"

if ! command -v "$HURL" > /dev/null; then
    echo "hurl tidak ditemukan. Pasang dari https://hurl.dev atau set HURL=/path/ke/hurl." >&2
    exit 2
fi

if [ "${SKIP_SEED:-0}" != "1" ]; then
    if [ -z "${DATABASE_URL:-}" ]; then
        echo "DATABASE_URL wajib diisi untuk memuat fixture (atau SKIP_SEED=1)." >&2
        exit 2
    fi
    if ! command -v "$PSQL" > /dev/null; then
        echo "psql tidak ditemukan. Set PSQL=/path/ke/psql." >&2
        exit 2
    fi
    # Parameter khusus sqlx (mis. ?sslmode atau opsi pool) tidak dikenal psql: buang query string.
    "$PSQL" "${DATABASE_URL%%\?*}" -q -X -v ON_ERROR_STOP=1 -v tz="$TZ_BUSINESS" -f "$ROOT/tests/contract/fixtures.sql"
fi

if [ "$#" -gt 0 ]; then
    files=("$@")
else
    files=("$ROOT"/tests/contract/*.hurl)
    if [ "${INCLUDE_V2:-0}" = "1" ]; then
        files+=("$ROOT"/tests/contract/v2/*.hurl)
    fi
fi

# --jobs 1: berurutan, karena punch punya interval minimum dan file saling bergantung pada fixture yang sama.
"$HURL" --test --jobs 1 \
    --variable "base_url=$BASE_URL" \
    --variable "password=$PASSWORD" \
    "${files[@]}"
