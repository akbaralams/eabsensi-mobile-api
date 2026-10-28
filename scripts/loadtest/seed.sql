-- Data uji beban "puncak pagi": 2.000 pegawai di 20 department (100 per department) di bawah satu induk,
-- seperti data uji docs/performance.md eabsensi-v2. HANYA untuk database uji. Idempoten.
--
-- Akun: loadtest-0001 .. loadtest-2000, sandi kontrak-uji-2026 (bcrypt cost 12, sama dengan fixture kontrak).
-- Token siap pakai (meniru pegawai yang sudah login dalam 30 hari terakhir):
--   <900000000 + n>|lt<n dipad 46 digit>   mis. 900000001|lt0000...0001
-- Event "Apel Beban" hari ini 00:00-23:59 untuk department induk (semua pegawai uji jadi peserta).

\set ON_ERROR_STOP on
\if :{?users}
\else
\set users 2000
\endif

BEGIN;

INSERT INTO departments (code, name, is_active, created_at, updated_at)
VALUES ('BEBAN', 'Induk Uji Beban', true, now() AT TIME ZONE 'UTC', now() AT TIME ZONE 'UTC')
ON CONFLICT (code) DO NOTHING;

INSERT INTO departments (code, name, parent_id, is_active, created_at, updated_at)
SELECT 'BEBAN-' || lpad(d::text, 2, '0'), 'Department Uji Beban ' || d,
       (SELECT id FROM departments WHERE code = 'BEBAN'), true, now() AT TIME ZONE 'UTC', now() AT TIME ZONE 'UTC'
FROM generate_series(1, 20) AS d
ON CONFLICT (code) DO UPDATE SET parent_id = EXCLUDED.parent_id;

INSERT INTO employees (employee_number, name, department_id, is_active, created_at, updated_at)
SELECT 'LT-' || lpad(n::text, greatest(4, length(n::text)), '0'), 'Pegawai Uji ' || n,
       (SELECT id FROM departments WHERE code = 'BEBAN-' || lpad((((n - 1) % 20) + 1)::text, 2, '0')),
       true, now() AT TIME ZONE 'UTC', now() AT TIME ZONE 'UTC'
FROM generate_series(1, :users) AS n
ON CONFLICT (employee_number) DO UPDATE SET is_active = true;

INSERT INTO users (username, name, email, password, employee_id, is_active, created_at, updated_at)
SELECT 'loadtest-' || lpad(n::text, greatest(4, length(n::text)), '0'), 'Akun Uji ' || n, 'loadtest-' || lpad(n::text, greatest(4, length(n::text)), '0') || '@beban.test',
       '$2y$12$gh2nUuLCjjiBZHMlpGaWv.g8DrcA0zyebGRYvulZV7Nd9QZmcRRfq',
       (SELECT id FROM employees WHERE employee_number = 'LT-' || lpad(n::text, greatest(4, length(n::text)), '0')),
       true, now() AT TIME ZONE 'UTC', now() AT TIME ZONE 'UTC'
FROM generate_series(1, :users) AS n
ON CONFLICT (username) DO UPDATE SET employee_id = EXCLUDED.employee_id, is_active = true;

-- Jejak uji sebelumnya.
DELETE FROM personal_access_tokens
WHERE tokenable_type = 'App\Models\User'
  AND tokenable_id IN (SELECT id FROM users WHERE username LIKE 'loadtest-%');
DELETE FROM attendance_punches WHERE employee_id IN (SELECT id FROM employees WHERE employee_number LIKE 'LT-%');
DELETE FROM event_attendances WHERE employee_id IN (SELECT id FROM employees WHERE employee_number LIKE 'LT-%');

-- Token siap pakai, ditulis persis seperti Sanctum (kolom token = SHA-256 rahasia).
INSERT INTO personal_access_tokens (id, tokenable_type, tokenable_id, name, token, abilities, created_at, updated_at)
SELECT 900000000 + n, 'App\Models\User', u.id, 'loadtest',
       encode(sha256(convert_to('lt' || lpad(n::text, 46, '0'), 'UTF8')), 'hex'),
       '["mobile"]', now() AT TIME ZONE 'UTC', now() AT TIME ZONE 'UTC'
FROM generate_series(1, :users) AS n
JOIN users u ON u.username = 'loadtest-' || lpad(n::text, greatest(4, length(n::text)), '0');

DELETE FROM attendance_events WHERE name = 'Apel Beban';
INSERT INTO attendance_events (name, event_date, starts_at, ends_at, department_id, is_active, created_at, updated_at)
SELECT 'Apel Beban', (now() AT TIME ZONE 'Asia/Jakarta')::date, '00:00', '23:59', id, true,
       now() AT TIME ZONE 'UTC', now() AT TIME ZONE 'UTC'
FROM departments WHERE code = 'BEBAN';

COMMIT;

ANALYZE departments, employees, users, personal_access_tokens, attendance_events;
