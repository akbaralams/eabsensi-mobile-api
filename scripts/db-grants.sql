-- Role database khusus service Rust dengan hak minimum (rust-services-plan.md, "Operasional").
-- Jalankan sebagai pemilik skema/DBA pada database aplikasi:
--   psql "$ADMIN_DATABASE_URL" -v password='...' -f scripts/db-grants.sql
-- Skema tetap milik migration Laravel; role ini tidak bisa mengubah struktur tabel.
-- Bila migration baru menambah tabel/kolom yang dibaca service, perbarui file ini di PR yang sama.

\set ON_ERROR_STOP on
\if :{?role}
\else
\set role eabsensi_mobile
\endif

SELECT format('CREATE ROLE %I LOGIN PASSWORD %L', :'role', :'password')
WHERE NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = :'role') \gexec
SELECT format('ALTER ROLE %I PASSWORD %L', :'role', :'password') \gexec
SELECT format('GRANT CONNECT ON DATABASE %I TO %I', current_database(), :'role') \gexec

GRANT USAGE ON SCHEMA public TO :"role";

-- Baca: data referensi dan akun.
GRANT SELECT ON users, employees, departments, attendance_events, attendance_statuses, periods, work_locations TO :"role";

-- Tulis: punch, check-in event, hasil cek lokasi, cuti, audit (tanpa UPDATE/DELETE).
GRANT SELECT, INSERT ON attendance_punches, event_attendances, attendance_location_checks, leave_requests TO :"role";
GRANT INSERT ON audit_logs TO :"role";

-- Token: buat saat login, perbarui last_used_at, hapus saat logout/login ulang perangkat yang sama.
GRANT SELECT, INSERT, UPDATE, DELETE ON personal_access_tokens TO :"role";

GRANT USAGE ON SEQUENCE attendance_punches_id_seq, event_attendances_id_seq, attendance_location_checks_id_seq, leave_requests_id_seq,
    audit_logs_id_seq, personal_access_tokens_id_seq TO :"role";
