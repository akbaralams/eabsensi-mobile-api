-- Data tetap untuk suite kontrak (tests/contract/*.hurl), padanan SQL dari ContractTestSeeder di eabsensi-v2.
-- Idempoten: bisa dijalankan berulang; jejak uji sebelumnya milik akun kontrak* dihapus dulu.
-- HANYA untuk database uji/dev. Dimuat oleh scripts/contract-test.sh lewat psql.
--
-- Variabel psql opsional: tz (zona waktu bisnis, default Asia/Jakarta; samakan dengan ATTENDANCE_TIMEZONE).
-- Password semua akun kontrak* = kontrak-uji-2026 (bcrypt $2y$ cost 12, sama dengan config/seeding.php Laravel).

\set ON_ERROR_STOP on
\if :{?tz}
\else
\set tz 'Asia/Jakarta'
\endif

BEGIN;

SELECT set_config('kontrak.tz', :'tz', true);

-- Suite butuh bulan berjalan yang terbuka (seeder Laravel menolak dengan cara yang sama).
DO $$
DECLARE
    today date := (now() AT TIME ZONE current_setting('kontrak.tz'))::date;
BEGIN
    IF EXISTS (
        SELECT 1 FROM periods
        WHERE year = extract(year FROM today) AND month = extract(month FROM today)
          AND status IN ('closing', 'closed')
    ) THEN
        RAISE EXCEPTION 'Periode bulan % terkunci di database ini; suite kontrak butuh bulan berjalan yang terbuka.', to_char(today, 'YYYY-MM');
    END IF;
END $$;

-- Status kehadiran awal (AttendanceStatusSeeder): firstOrCreate, tidak menimpa perubahan admin.
INSERT INTO attendance_statuses (code, name, counts_as_absent, is_active, created_at, updated_at)
VALUES ('SAKIT', 'Sakit', false, true, now() AT TIME ZONE 'UTC', now() AT TIME ZONE 'UTC'),
       ('IZIN', 'Izin', false, true, now() AT TIME ZONE 'UTC', now() AT TIME ZONE 'UTC'),
       ('CUTI', 'Cuti', false, true, now() AT TIME ZONE 'UTC', now() AT TIME ZONE 'UTC'),
       ('DINAS', 'Dinas luar', false, true, now() AT TIME ZONE 'UTC', now() AT TIME ZONE 'UTC')
ON CONFLICT (code) DO NOTHING;

-- Department (updateOrCreate by code).
INSERT INTO departments (code, name, is_active, created_at, updated_at)
VALUES ('KONTRAK', 'Department Kontrak', true, now() AT TIME ZONE 'UTC', now() AT TIME ZONE 'UTC'),
       ('KONTRAK-LAIN', 'Department Kontrak Lain', true, now() AT TIME ZONE 'UTC', now() AT TIME ZONE 'UTC')
ON CONFLICT (code) DO UPDATE SET name = EXCLUDED.name, is_active = EXCLUDED.is_active, updated_at = EXCLUDED.updated_at;

-- Pegawai (updateOrCreate by employee_number).
INSERT INTO employees (employee_number, name, department_id, is_active, created_at, updated_at)
SELECT v.number, v.name, d.id, v.active, now() AT TIME ZONE 'UTC', now() AT TIME ZONE 'UTC'
FROM (VALUES ('KTR-001', 'Pegawai Kontrak', true), ('KTR-002', 'Pegawai Kontrak Nonaktif', false)) AS v(number, name, active)
JOIN departments d ON d.code = 'KONTRAK'
ON CONFLICT (employee_number) DO UPDATE
    SET name = EXCLUDED.name, department_id = EXCLUDED.department_id, is_active = EXCLUDED.is_active, updated_at = EXCLUDED.updated_at;

-- Empat akun kontrak* (updateOrCreate by username).
INSERT INTO users (username, name, email, password, employee_id, is_active, created_at, updated_at)
SELECT v.username, 'Akun ' || v.username, v.username || '@kontrak.test',
       '$2y$12$gh2nUuLCjjiBZHMlpGaWv.g8DrcA0zyebGRYvulZV7Nd9QZmcRRfq',
       (SELECT id FROM employees WHERE employee_number = v.employee_number), v.active,
       now() AT TIME ZONE 'UTC', now() AT TIME ZONE 'UTC'
FROM (VALUES ('kontrak', 'KTR-001', true),
             ('kontrak.tanpa-pegawai', NULL, true),
             ('kontrak.akun-nonaktif', 'KTR-001', false),
             ('kontrak.pegawai-nonaktif', 'KTR-002', true)) AS v(username, employee_number, active)
ON CONFLICT (username) DO UPDATE
    SET name = EXCLUDED.name, email = EXCLUDED.email, password = EXCLUDED.password,
        employee_id = EXCLUDED.employee_id, is_active = EXCLUDED.is_active, updated_at = EXCLUDED.updated_at;

-- Hapus jejak uji lama: token akun kontrak*, punch, check-in, dan cuti pegawai KTR-*.
DELETE FROM personal_access_tokens
WHERE tokenable_type = 'App\Models\User'
  AND tokenable_id IN (SELECT id FROM users WHERE username IN ('kontrak', 'kontrak.tanpa-pegawai', 'kontrak.akun-nonaktif', 'kontrak.pegawai-nonaktif'));

DELETE FROM attendance_punches WHERE employee_id IN (SELECT id FROM employees WHERE employee_number IN ('KTR-001', 'KTR-002'));
DELETE FROM event_attendances WHERE employee_id IN (SELECT id FROM employees WHERE employee_number IN ('KTR-001', 'KTR-002'));
DELETE FROM leave_requests WHERE employee_id IN (SELECT id FROM employees WHERE employee_number IN ('KTR-001', 'KTR-002'));

-- Dua event hari ini (tanggal zona bisnis): satu untuk semua pegawai, satu untuk department lain (tidak terlihat).
-- updateOrCreate by name: perbarui bila ada, buat bila belum.
WITH v(name, department_code) AS (
    VALUES ('Apel Kontrak', NULL), ('Apel Kontrak Department Lain', 'KONTRAK-LAIN')
), updated AS (
    UPDATE attendance_events e
    SET event_date = (now() AT TIME ZONE current_setting('kontrak.tz'))::date,
        starts_at = '00:00', ends_at = '23:59', is_active = true,
        department_id = (SELECT id FROM departments WHERE code = v.department_code),
        updated_at = now() AT TIME ZONE 'UTC'
    FROM v
    WHERE e.name = v.name
    RETURNING e.name
)
INSERT INTO attendance_events (name, event_date, starts_at, ends_at, department_id, is_active, created_at, updated_at)
SELECT v.name, (now() AT TIME ZONE current_setting('kontrak.tz'))::date, '00:00', '23:59',
       (SELECT id FROM departments WHERE code = v.department_code), true,
       now() AT TIME ZONE 'UTC', now() AT TIME ZONE 'UTC'
FROM v
WHERE NOT EXISTS (SELECT 1 FROM attendance_events e WHERE e.name = v.name);

-- Batas percobaan login Laravel disimpan di tabel cache (CACHE_STORE=database); runner lama memanggil cache:clear.
-- Service Rust menyimpan batas laju di memori/Redis, jadi ini hanya berpengaruh saat suite diarahkan ke Laravel.
DELETE FROM cache;
DELETE FROM cache_locks;

COMMIT;
