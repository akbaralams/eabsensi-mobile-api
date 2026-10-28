#!/usr/bin/env bash
# Uji interop token dua arah: token dari login Laravel dipakai ke service Rust, dan sebaliknya.
# Kedua server harus memakai database yang sama (fixture kontrak sudah dimuat).
#
#   LARAVEL_URL=http://127.0.0.1:8000 RUST_URL=http://127.0.0.1:8080 bash scripts/token-interop.sh
set -euo pipefail

LARAVEL_URL="${LARAVEL_URL:-http://127.0.0.1:8000}"
RUST_URL="${RUST_URL:-http://127.0.0.1:8080}"
PASSWORD="${SEED_CONTRACT_PASSWORD:-kontrak-uji-2026}"
fail=0

login() { # $1 = base url, $2 = device
    curl -fsS -X POST "$1/api/auth/login" -H 'Accept: application/json' -H 'Content-Type: application/json' \
        -d "{\"login\":\"kontrak\",\"password\":\"$PASSWORD\",\"device_name\":\"$2\"}" \
        | grep -o '"token":"[^"]*"' | cut -d'"' -f4
}

status_me() { # $1 = base url, $2 = token
    curl -s -o /dev/null -w '%{http_code}' "$1/api/me" -H 'Accept: application/json' -H "Authorization: Bearer $2"
}

check() { # $1 = label, $2 = actual, $3 = expected
    if [ "$2" = "$3" ]; then echo "ok   $1 ($2)"; else echo "GAGAL $1: $2, harus $3"; fail=1; fi
}

laravel_token="$(login "$LARAVEL_URL" interop-laravel)"
rust_token="$(login "$RUST_URL" interop-rust)"

check "token Laravel diterima Rust" "$(status_me "$RUST_URL" "$laravel_token")" 200
check "token Rust diterima Laravel" "$(status_me "$LARAVEL_URL" "$rust_token")" 200

# Logout di satu sisi mencabut token di sisi lain.
curl -fsS -o /dev/null -X POST "$RUST_URL/api/auth/logout" -H 'Accept: application/json' -H "Authorization: Bearer $laravel_token"
check "token Laravel dicabut lewat Rust ditolak Laravel" "$(status_me "$LARAVEL_URL" "$laravel_token")" 401
curl -fsS -o /dev/null -X POST "$LARAVEL_URL/api/auth/logout" -H 'Accept: application/json' -H "Authorization: Bearer $rust_token"
check "token Rust dicabut lewat Laravel ditolak Rust" "$(status_me "$RUST_URL" "$rust_token")" 401

exit $fail
