--
-- PostgreSQL database dump
--

\restrict 2BC2CXqJc6q7KbohQ4VOuTWgNg3zCSBwA0DuwiBPPOvGk9yxtAYyITWV8dmRJxp

-- Dumped from database version 18.6
-- Dumped by pg_dump version 18.6

SET statement_timeout = 0;
SET lock_timeout = 0;
SET idle_in_transaction_session_timeout = 0;
SET transaction_timeout = 0;
SET client_encoding = 'UTF8';
SET standard_conforming_strings = on;
SELECT pg_catalog.set_config('search_path', '', false);
SET check_function_bodies = false;
SET xmloption = content;
SET client_min_messages = warning;
SET row_security = off;

SET default_tablespace = '';

SET default_table_access_method = heap;

--
-- Name: attendance_daily; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.attendance_daily (
    id bigint NOT NULL,
    period_id bigint NOT NULL,
    employee_id bigint NOT NULL,
    department_id bigint NOT NULL,
    date date NOT NULL,
    status character varying(16) NOT NULL,
    first_in timestamp(0) with time zone,
    last_out timestamp(0) with time zone,
    late_minutes integer DEFAULT 0 NOT NULL,
    early_minutes integer DEFAULT 0 NOT NULL,
    worked_minutes integer DEFAULT 0 NOT NULL,
    shift_code character varying(32),
    status_code character varying(32),
    created_at timestamp(0) without time zone,
    updated_at timestamp(0) without time zone
);


--
-- Name: attendance_daily_id_seq; Type: SEQUENCE; Schema: public; Owner: -
--

CREATE SEQUENCE public.attendance_daily_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


--
-- Name: attendance_daily_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: -
--

ALTER SEQUENCE public.attendance_daily_id_seq OWNED BY public.attendance_daily.id;


--
-- Name: attendance_events; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.attendance_events (
    id bigint NOT NULL,
    name character varying(255) NOT NULL,
    event_date date NOT NULL,
    starts_at time(0) without time zone NOT NULL,
    ends_at time(0) without time zone NOT NULL,
    department_id bigint,
    is_active boolean DEFAULT true NOT NULL,
    created_by bigint,
    created_at timestamp(0) without time zone,
    updated_at timestamp(0) without time zone,
    location_mode character varying(16) DEFAULT 'none'::character varying NOT NULL,
    work_location_id bigint,
    CONSTRAINT attendance_events_custom_location_check CHECK ((((location_mode)::text <> 'custom'::text) OR (work_location_id IS NOT NULL))),
    CONSTRAINT attendance_events_location_mode_check CHECK (((location_mode)::text = ANY ((ARRAY['none'::character varying, 'employee'::character varying, 'custom'::character varying])::text[])))
);


--
-- Name: attendance_events_id_seq; Type: SEQUENCE; Schema: public; Owner: -
--

CREATE SEQUENCE public.attendance_events_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


--
-- Name: attendance_events_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: -
--

ALTER SEQUENCE public.attendance_events_id_seq OWNED BY public.attendance_events.id;


--
-- Name: attendance_location_checks; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.attendance_location_checks (
    id bigint NOT NULL,
    attendance_punch_id bigint,
    event_attendance_id bigint,
    work_location_id bigint,
    location_name character varying(100),
    radius_meters integer,
    distance_meters integer,
    flags jsonb DEFAULT '[]'::jsonb NOT NULL,
    is_flagged boolean DEFAULT false NOT NULL,
    created_at timestamp(0) without time zone,
    updated_at timestamp(0) without time zone,
    CONSTRAINT attendance_location_checks_target_check CHECK (((attendance_punch_id IS NOT NULL) <> (event_attendance_id IS NOT NULL)))
);


--
-- Name: attendance_location_checks_id_seq; Type: SEQUENCE; Schema: public; Owner: -
--

CREATE SEQUENCE public.attendance_location_checks_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


--
-- Name: attendance_location_checks_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: -
--

ALTER SEQUENCE public.attendance_location_checks_id_seq OWNED BY public.attendance_location_checks.id;


--
-- Name: attendance_punches; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.attendance_punches (
    id bigint NOT NULL,
    employee_id bigint NOT NULL,
    punched_at timestamp(0) with time zone NOT NULL,
    source character varying(16) NOT NULL,
    created_by bigint,
    note character varying(255),
    created_at timestamp(0) without time zone,
    updated_at timestamp(0) without time zone,
    status character varying(16) DEFAULT 'approved'::character varying NOT NULL,
    reviewed_by bigint,
    reviewed_at timestamp(0) with time zone,
    photo_path character varying(255),
    latitude numeric(9,6),
    longitude numeric(9,6),
    accuracy_meters smallint
);


--
-- Name: attendance_punches_id_seq; Type: SEQUENCE; Schema: public; Owner: -
--

CREATE SEQUENCE public.attendance_punches_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


--
-- Name: attendance_punches_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: -
--

ALTER SEQUENCE public.attendance_punches_id_seq OWNED BY public.attendance_punches.id;


--
-- Name: attendance_statuses; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.attendance_statuses (
    id bigint NOT NULL,
    code character varying(32) NOT NULL,
    name character varying(255) NOT NULL,
    counts_as_absent boolean DEFAULT false NOT NULL,
    is_active boolean DEFAULT true NOT NULL,
    created_at timestamp(0) without time zone,
    updated_at timestamp(0) without time zone
);


--
-- Name: attendance_statuses_id_seq; Type: SEQUENCE; Schema: public; Owner: -
--

CREATE SEQUENCE public.attendance_statuses_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


--
-- Name: attendance_statuses_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: -
--

ALTER SEQUENCE public.attendance_statuses_id_seq OWNED BY public.attendance_statuses.id;


--
-- Name: audit_logs; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.audit_logs (
    id bigint NOT NULL,
    user_id bigint,
    user_name character varying(255),
    action character varying(64) NOT NULL,
    description character varying(255) NOT NULL,
    context json,
    ip_address character varying(45),
    created_at timestamp(0) with time zone NOT NULL
);


--
-- Name: audit_logs_id_seq; Type: SEQUENCE; Schema: public; Owner: -
--

CREATE SEQUENCE public.audit_logs_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


--
-- Name: audit_logs_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: -
--

ALTER SEQUENCE public.audit_logs_id_seq OWNED BY public.audit_logs.id;


--
-- Name: biometric_templates; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.biometric_templates (
    id bigint NOT NULL,
    employee_id bigint NOT NULL,
    kind character varying(16) NOT NULL,
    finger_index smallint DEFAULT '0'::smallint NOT NULL,
    algorithm_version character varying(16) DEFAULT ''::character varying NOT NULL,
    is_valid boolean DEFAULT true NOT NULL,
    original_size integer NOT NULL,
    ciphertext bytea NOT NULL,
    nonce bytea NOT NULL,
    key_id character varying(16) NOT NULL,
    source_device_id bigint,
    created_at timestamp(0) without time zone,
    updated_at timestamp(0) without time zone
);


--
-- Name: biometric_templates_id_seq; Type: SEQUENCE; Schema: public; Owner: -
--

CREATE SEQUENCE public.biometric_templates_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


--
-- Name: biometric_templates_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: -
--

ALTER SEQUENCE public.biometric_templates_id_seq OWNED BY public.biometric_templates.id;


--
-- Name: cache; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.cache (
    key character varying(255) NOT NULL,
    value text NOT NULL,
    expiration bigint NOT NULL
);


--
-- Name: cache_locks; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.cache_locks (
    key character varying(255) NOT NULL,
    owner character varying(255) NOT NULL,
    expiration bigint NOT NULL
);


--
-- Name: departments; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.departments (
    id bigint NOT NULL,
    code character varying(32) NOT NULL,
    name character varying(255) NOT NULL,
    parent_id bigint,
    division_id bigint,
    region_id bigint,
    is_active boolean DEFAULT true NOT NULL,
    created_at timestamp(0) without time zone,
    updated_at timestamp(0) without time zone,
    work_location_id bigint
);


--
-- Name: departments_id_seq; Type: SEQUENCE; Schema: public; Owner: -
--

CREATE SEQUENCE public.departments_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


--
-- Name: departments_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: -
--

ALTER SEQUENCE public.departments_id_seq OWNED BY public.departments.id;


--
-- Name: device_commands; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.device_commands (
    id bigint NOT NULL,
    device_id bigint NOT NULL,
    command_type character varying(24) NOT NULL,
    priority smallint NOT NULL,
    status character varying(16) DEFAULT 'pending'::character varying NOT NULL,
    employee_id bigint,
    biometric_template_id bigint,
    attempts smallint DEFAULT '0'::smallint NOT NULL,
    requested_by bigint,
    available_at timestamp(0) with time zone DEFAULT CURRENT_TIMESTAMP NOT NULL,
    sent_at timestamp(0) with time zone,
    completed_at timestamp(0) with time zone,
    return_code integer,
    result_message character varying(255),
    created_at timestamp(0) without time zone,
    updated_at timestamp(0) without time zone
);


--
-- Name: device_commands_id_seq; Type: SEQUENCE; Schema: public; Owner: -
--

CREATE SEQUENCE public.device_commands_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


--
-- Name: device_commands_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: -
--

ALTER SEQUENCE public.device_commands_id_seq OWNED BY public.device_commands.id;


--
-- Name: device_logs; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.device_logs (
    id bigint NOT NULL,
    device_id bigint NOT NULL,
    pin character varying(32) NOT NULL,
    device_time timestamp(0) without time zone NOT NULL,
    punched_at timestamp(0) with time zone NOT NULL,
    io_status smallint,
    verify_mode smallint,
    received_at timestamp(0) with time zone DEFAULT CURRENT_TIMESTAMP NOT NULL,
    source_ip character varying(45),
    employee_id bigint,
    punch_id bigint,
    outcome character varying(24) NOT NULL,
    is_suspect boolean DEFAULT false NOT NULL,
    created_at timestamp(0) without time zone,
    updated_at timestamp(0) without time zone
);


--
-- Name: device_logs_id_seq; Type: SEQUENCE; Schema: public; Owner: -
--

CREATE SEQUENCE public.device_logs_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


--
-- Name: device_logs_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: -
--

ALTER SEQUENCE public.device_logs_id_seq OWNED BY public.device_logs.id;


--
-- Name: devices; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.devices (
    id bigint NOT NULL,
    serial_number character varying(64) NOT NULL,
    name character varying(255) NOT NULL,
    region_id bigint,
    location character varying(255),
    timezone character varying(64) DEFAULT 'Asia/Jakarta'::character varying NOT NULL,
    is_active boolean DEFAULT true NOT NULL,
    device_type character varying(16) DEFAULT 'unknown'::character varying NOT NULL,
    model character varying(64),
    platform character varying(64),
    firmware_version character varying(64),
    push_version character varying(16),
    ip_address character varying(45),
    registry_code character varying(32),
    poll_delay_seconds smallint DEFAULT '60'::smallint NOT NULL,
    error_delay_seconds smallint DEFAULT '60'::smallint NOT NULL,
    attlog_stamp character varying(32),
    operlog_stamp character varying(32),
    photo_stamp character varying(32),
    biodata_stamp character varying(32),
    user_count integer,
    fingerprint_count integer,
    face_count integer,
    log_count integer,
    reported_options jsonb,
    first_seen_at timestamp(0) with time zone,
    initialized_at timestamp(0) with time zone,
    last_seen_at timestamp(0) with time zone,
    created_at timestamp(0) without time zone,
    updated_at timestamp(0) without time zone
);


--
-- Name: devices_id_seq; Type: SEQUENCE; Schema: public; Owner: -
--

CREATE SEQUENCE public.devices_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


--
-- Name: devices_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: -
--

ALTER SEQUENCE public.devices_id_seq OWNED BY public.devices.id;


--
-- Name: divisions; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.divisions (
    id bigint NOT NULL,
    code character varying(32) NOT NULL,
    name character varying(255) NOT NULL,
    is_active boolean DEFAULT true NOT NULL,
    created_at timestamp(0) without time zone,
    updated_at timestamp(0) without time zone
);


--
-- Name: divisions_id_seq; Type: SEQUENCE; Schema: public; Owner: -
--

CREATE SEQUENCE public.divisions_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


--
-- Name: divisions_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: -
--

ALTER SEQUENCE public.divisions_id_seq OWNED BY public.divisions.id;


--
-- Name: documents; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.documents (
    id bigint NOT NULL,
    title character varying(255) NOT NULL,
    description character varying(500),
    file_path character varying(255) NOT NULL,
    original_name character varying(255) NOT NULL,
    mime_type character varying(128) NOT NULL,
    size_bytes bigint NOT NULL,
    uploaded_by bigint,
    created_at timestamp(0) without time zone,
    updated_at timestamp(0) without time zone
);


--
-- Name: documents_id_seq; Type: SEQUENCE; Schema: public; Owner: -
--

CREATE SEQUENCE public.documents_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


--
-- Name: documents_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: -
--

ALTER SEQUENCE public.documents_id_seq OWNED BY public.documents.id;


--
-- Name: employee_categories; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.employee_categories (
    id bigint NOT NULL,
    code character varying(32) NOT NULL,
    name character varying(255) NOT NULL,
    is_active boolean DEFAULT true NOT NULL,
    created_at timestamp(0) without time zone,
    updated_at timestamp(0) without time zone
);


--
-- Name: employee_categories_id_seq; Type: SEQUENCE; Schema: public; Owner: -
--

CREATE SEQUENCE public.employee_categories_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


--
-- Name: employee_categories_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: -
--

ALTER SEQUENCE public.employee_categories_id_seq OWNED BY public.employee_categories.id;


--
-- Name: employees; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.employees (
    id bigint NOT NULL,
    employee_number character varying(32) NOT NULL,
    national_id character varying(32),
    name character varying(255) NOT NULL,
    department_id bigint NOT NULL,
    employment_type_id bigint,
    employee_category_id bigint,
    "position" character varying(255),
    phone character varying(32),
    photo_path character varying(255),
    is_active boolean DEFAULT true NOT NULL,
    created_at timestamp(0) without time zone,
    updated_at timestamp(0) without time zone,
    work_location_id bigint,
    location_exempt boolean DEFAULT false NOT NULL
);


--
-- Name: employees_id_seq; Type: SEQUENCE; Schema: public; Owner: -
--

CREATE SEQUENCE public.employees_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


--
-- Name: employees_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: -
--

ALTER SEQUENCE public.employees_id_seq OWNED BY public.employees.id;


--
-- Name: employment_types; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.employment_types (
    id bigint NOT NULL,
    code character varying(32) NOT NULL,
    name character varying(255) NOT NULL,
    is_active boolean DEFAULT true NOT NULL,
    created_at timestamp(0) without time zone,
    updated_at timestamp(0) without time zone
);


--
-- Name: employment_types_id_seq; Type: SEQUENCE; Schema: public; Owner: -
--

CREATE SEQUENCE public.employment_types_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


--
-- Name: employment_types_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: -
--

ALTER SEQUENCE public.employment_types_id_seq OWNED BY public.employment_types.id;


--
-- Name: event_attendances; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.event_attendances (
    id bigint NOT NULL,
    event_id bigint NOT NULL,
    employee_id bigint NOT NULL,
    checked_in_at timestamp(0) with time zone NOT NULL,
    photo_path character varying(255),
    latitude numeric(9,6),
    longitude numeric(9,6),
    accuracy_meters smallint,
    created_at timestamp(0) without time zone,
    updated_at timestamp(0) without time zone
);


--
-- Name: event_attendances_id_seq; Type: SEQUENCE; Schema: public; Owner: -
--

CREATE SEQUENCE public.event_attendances_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


--
-- Name: event_attendances_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: -
--

ALTER SEQUENCE public.event_attendances_id_seq OWNED BY public.event_attendances.id;


--
-- Name: failed_jobs; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.failed_jobs (
    id bigint NOT NULL,
    uuid character varying(255) NOT NULL,
    connection character varying(255) NOT NULL,
    queue character varying(255) NOT NULL,
    payload text NOT NULL,
    exception text NOT NULL,
    failed_at timestamp(0) without time zone DEFAULT CURRENT_TIMESTAMP NOT NULL
);


--
-- Name: failed_jobs_id_seq; Type: SEQUENCE; Schema: public; Owner: -
--

CREATE SEQUENCE public.failed_jobs_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


--
-- Name: failed_jobs_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: -
--

ALTER SEQUENCE public.failed_jobs_id_seq OWNED BY public.failed_jobs.id;


--
-- Name: holidays; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.holidays (
    id bigint NOT NULL,
    date date NOT NULL,
    name character varying(255) NOT NULL,
    type character varying(16) NOT NULL,
    created_at timestamp(0) without time zone,
    updated_at timestamp(0) without time zone
);


--
-- Name: holidays_id_seq; Type: SEQUENCE; Schema: public; Owner: -
--

CREATE SEQUENCE public.holidays_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


--
-- Name: holidays_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: -
--

ALTER SEQUENCE public.holidays_id_seq OWNED BY public.holidays.id;


--
-- Name: job_batches; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.job_batches (
    id character varying(255) NOT NULL,
    name character varying(255) NOT NULL,
    total_jobs integer NOT NULL,
    pending_jobs integer NOT NULL,
    failed_jobs integer NOT NULL,
    failed_job_ids text NOT NULL,
    options text,
    cancelled_at integer,
    created_at integer NOT NULL,
    finished_at integer
);


--
-- Name: jobs; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.jobs (
    id bigint NOT NULL,
    queue character varying(255) NOT NULL,
    payload text NOT NULL,
    attempts smallint NOT NULL,
    reserved_at integer,
    available_at integer NOT NULL,
    created_at integer NOT NULL
);


--
-- Name: jobs_id_seq; Type: SEQUENCE; Schema: public; Owner: -
--

CREATE SEQUENCE public.jobs_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


--
-- Name: jobs_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: -
--

ALTER SEQUENCE public.jobs_id_seq OWNED BY public.jobs.id;


--
-- Name: leave_requests; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.leave_requests (
    id bigint NOT NULL,
    employee_id bigint NOT NULL,
    attendance_status_id bigint NOT NULL,
    start_date date NOT NULL,
    end_date date NOT NULL,
    reason character varying(500),
    status character varying(16) NOT NULL,
    requested_by bigint,
    reviewed_by bigint,
    reviewed_at timestamp(0) with time zone,
    created_at timestamp(0) without time zone,
    updated_at timestamp(0) without time zone
);


--
-- Name: leave_requests_id_seq; Type: SEQUENCE; Schema: public; Owner: -
--

CREATE SEQUENCE public.leave_requests_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


--
-- Name: leave_requests_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: -
--

ALTER SEQUENCE public.leave_requests_id_seq OWNED BY public.leave_requests.id;


--
-- Name: migrations; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.migrations (
    id integer NOT NULL,
    migration character varying(255) NOT NULL,
    batch integer NOT NULL
);


--
-- Name: migrations_id_seq; Type: SEQUENCE; Schema: public; Owner: -
--

CREATE SEQUENCE public.migrations_id_seq
    AS integer
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


--
-- Name: migrations_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: -
--

ALTER SEQUENCE public.migrations_id_seq OWNED BY public.migrations.id;


--
-- Name: model_has_permissions; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.model_has_permissions (
    permission_id bigint NOT NULL,
    model_type character varying(255) NOT NULL,
    model_id bigint NOT NULL
);


--
-- Name: model_has_roles; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.model_has_roles (
    role_id bigint NOT NULL,
    model_type character varying(255) NOT NULL,
    model_id bigint NOT NULL
);


--
-- Name: password_reset_tokens; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.password_reset_tokens (
    email character varying(255) NOT NULL,
    token character varying(255) NOT NULL,
    created_at timestamp(0) without time zone
);


--
-- Name: periods; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.periods (
    id bigint NOT NULL,
    year smallint NOT NULL,
    month smallint NOT NULL,
    status character varying(16) NOT NULL,
    closed_at timestamp(0) with time zone,
    closed_by bigint,
    reopened_at timestamp(0) with time zone,
    reopened_by bigint,
    created_at timestamp(0) without time zone,
    updated_at timestamp(0) without time zone
);


--
-- Name: periods_id_seq; Type: SEQUENCE; Schema: public; Owner: -
--

CREATE SEQUENCE public.periods_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


--
-- Name: periods_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: -
--

ALTER SEQUENCE public.periods_id_seq OWNED BY public.periods.id;


--
-- Name: permissions; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.permissions (
    id bigint NOT NULL,
    name character varying(255) NOT NULL,
    guard_name character varying(255) NOT NULL,
    created_at timestamp(0) without time zone,
    updated_at timestamp(0) without time zone
);


--
-- Name: permissions_id_seq; Type: SEQUENCE; Schema: public; Owner: -
--

CREATE SEQUENCE public.permissions_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


--
-- Name: permissions_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: -
--

ALTER SEQUENCE public.permissions_id_seq OWNED BY public.permissions.id;


--
-- Name: personal_access_tokens; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.personal_access_tokens (
    id bigint NOT NULL,
    tokenable_type character varying(255) NOT NULL,
    tokenable_id bigint NOT NULL,
    name text NOT NULL,
    token character varying(64) NOT NULL,
    abilities text,
    last_used_at timestamp(0) without time zone,
    expires_at timestamp(0) without time zone,
    created_at timestamp(0) without time zone,
    updated_at timestamp(0) without time zone
);


--
-- Name: personal_access_tokens_id_seq; Type: SEQUENCE; Schema: public; Owner: -
--

CREATE SEQUENCE public.personal_access_tokens_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


--
-- Name: personal_access_tokens_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: -
--

ALTER SEQUENCE public.personal_access_tokens_id_seq OWNED BY public.personal_access_tokens.id;


--
-- Name: posts; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.posts (
    id bigint NOT NULL,
    title character varying(255) NOT NULL,
    body text NOT NULL,
    is_published boolean DEFAULT false NOT NULL,
    published_at timestamp(0) with time zone,
    created_by bigint,
    created_at timestamp(0) without time zone,
    updated_at timestamp(0) without time zone
);


--
-- Name: posts_id_seq; Type: SEQUENCE; Schema: public; Owner: -
--

CREATE SEQUENCE public.posts_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


--
-- Name: posts_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: -
--

ALTER SEQUENCE public.posts_id_seq OWNED BY public.posts.id;


--
-- Name: regions; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.regions (
    id bigint NOT NULL,
    code character varying(32) NOT NULL,
    name character varying(255) NOT NULL,
    is_active boolean DEFAULT true NOT NULL,
    created_at timestamp(0) without time zone,
    updated_at timestamp(0) without time zone
);


--
-- Name: regions_id_seq; Type: SEQUENCE; Schema: public; Owner: -
--

CREATE SEQUENCE public.regions_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


--
-- Name: regions_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: -
--

ALTER SEQUENCE public.regions_id_seq OWNED BY public.regions.id;


--
-- Name: role_has_permissions; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.role_has_permissions (
    permission_id bigint NOT NULL,
    role_id bigint NOT NULL
);


--
-- Name: roles; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.roles (
    id bigint NOT NULL,
    name character varying(255) NOT NULL,
    guard_name character varying(255) NOT NULL,
    created_at timestamp(0) without time zone,
    updated_at timestamp(0) without time zone
);


--
-- Name: roles_id_seq; Type: SEQUENCE; Schema: public; Owner: -
--

CREATE SEQUENCE public.roles_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


--
-- Name: roles_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: -
--

ALTER SEQUENCE public.roles_id_seq OWNED BY public.roles.id;


--
-- Name: rosters; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.rosters (
    id bigint NOT NULL,
    employee_id bigint NOT NULL,
    date date NOT NULL,
    shift_id bigint,
    attendance_status_id bigint,
    created_at timestamp(0) without time zone,
    updated_at timestamp(0) without time zone
);


--
-- Name: rosters_id_seq; Type: SEQUENCE; Schema: public; Owner: -
--

CREATE SEQUENCE public.rosters_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


--
-- Name: rosters_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: -
--

ALTER SEQUENCE public.rosters_id_seq OWNED BY public.rosters.id;


--
-- Name: sessions; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.sessions (
    id character varying(255) NOT NULL,
    user_id bigint,
    ip_address character varying(45),
    user_agent text,
    payload text NOT NULL,
    last_activity integer NOT NULL
);


--
-- Name: shift_department; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.shift_department (
    id bigint NOT NULL,
    shift_id bigint NOT NULL,
    department_id bigint NOT NULL,
    mode character varying(16) NOT NULL,
    created_at timestamp(0) without time zone,
    updated_at timestamp(0) without time zone
);


--
-- Name: shift_department_id_seq; Type: SEQUENCE; Schema: public; Owner: -
--

CREATE SEQUENCE public.shift_department_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


--
-- Name: shift_department_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: -
--

ALTER SEQUENCE public.shift_department_id_seq OWNED BY public.shift_department.id;


--
-- Name: shifts; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.shifts (
    id bigint NOT NULL,
    code character varying(32) NOT NULL,
    name character varying(255) NOT NULL,
    start_time time(0) without time zone NOT NULL,
    end_time time(0) without time zone NOT NULL,
    tolerance_minutes smallint DEFAULT '0'::smallint NOT NULL,
    is_flex boolean DEFAULT false NOT NULL,
    is_active boolean DEFAULT true NOT NULL,
    created_at timestamp(0) without time zone,
    updated_at timestamp(0) without time zone,
    checkin_open_minutes smallint DEFAULT '180'::smallint NOT NULL,
    checkout_close_minutes smallint DEFAULT '360'::smallint NOT NULL
);


--
-- Name: shifts_id_seq; Type: SEQUENCE; Schema: public; Owner: -
--

CREATE SEQUENCE public.shifts_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


--
-- Name: shifts_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: -
--

ALTER SEQUENCE public.shifts_id_seq OWNED BY public.shifts.id;


--
-- Name: users; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.users (
    id bigint NOT NULL,
    name character varying(255) NOT NULL,
    email character varying(255) NOT NULL,
    email_verified_at timestamp(0) without time zone,
    password character varying(255) NOT NULL,
    remember_token character varying(100),
    created_at timestamp(0) without time zone,
    updated_at timestamp(0) without time zone,
    username character varying(64),
    employee_id bigint,
    data_scope character varying(16) DEFAULT 'department'::character varying NOT NULL,
    department_id bigint,
    region_id bigint,
    is_active boolean DEFAULT true NOT NULL
);


--
-- Name: users_id_seq; Type: SEQUENCE; Schema: public; Owner: -
--

CREATE SEQUENCE public.users_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


--
-- Name: users_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: -
--

ALTER SEQUENCE public.users_id_seq OWNED BY public.users.id;


--
-- Name: work_locations; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.work_locations (
    id bigint NOT NULL,
    name character varying(100) NOT NULL,
    latitude numeric(9,6) NOT NULL,
    longitude numeric(9,6) NOT NULL,
    radius_meters integer NOT NULL,
    is_active boolean DEFAULT true NOT NULL,
    created_at timestamp(0) without time zone,
    updated_at timestamp(0) without time zone,
    CONSTRAINT work_locations_radius_meters_check CHECK (((radius_meters >= 10) AND (radius_meters <= 5000)))
);


--
-- Name: work_locations_id_seq; Type: SEQUENCE; Schema: public; Owner: -
--

CREATE SEQUENCE public.work_locations_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


--
-- Name: work_locations_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: -
--

ALTER SEQUENCE public.work_locations_id_seq OWNED BY public.work_locations.id;


--
-- Name: attendance_daily id; Type: DEFAULT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.attendance_daily ALTER COLUMN id SET DEFAULT nextval('public.attendance_daily_id_seq'::regclass);


--
-- Name: attendance_events id; Type: DEFAULT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.attendance_events ALTER COLUMN id SET DEFAULT nextval('public.attendance_events_id_seq'::regclass);


--
-- Name: attendance_location_checks id; Type: DEFAULT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.attendance_location_checks ALTER COLUMN id SET DEFAULT nextval('public.attendance_location_checks_id_seq'::regclass);


--
-- Name: attendance_punches id; Type: DEFAULT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.attendance_punches ALTER COLUMN id SET DEFAULT nextval('public.attendance_punches_id_seq'::regclass);


--
-- Name: attendance_statuses id; Type: DEFAULT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.attendance_statuses ALTER COLUMN id SET DEFAULT nextval('public.attendance_statuses_id_seq'::regclass);


--
-- Name: audit_logs id; Type: DEFAULT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.audit_logs ALTER COLUMN id SET DEFAULT nextval('public.audit_logs_id_seq'::regclass);


--
-- Name: biometric_templates id; Type: DEFAULT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.biometric_templates ALTER COLUMN id SET DEFAULT nextval('public.biometric_templates_id_seq'::regclass);


--
-- Name: departments id; Type: DEFAULT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.departments ALTER COLUMN id SET DEFAULT nextval('public.departments_id_seq'::regclass);


--
-- Name: device_commands id; Type: DEFAULT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.device_commands ALTER COLUMN id SET DEFAULT nextval('public.device_commands_id_seq'::regclass);


--
-- Name: device_logs id; Type: DEFAULT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.device_logs ALTER COLUMN id SET DEFAULT nextval('public.device_logs_id_seq'::regclass);


--
-- Name: devices id; Type: DEFAULT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.devices ALTER COLUMN id SET DEFAULT nextval('public.devices_id_seq'::regclass);


--
-- Name: divisions id; Type: DEFAULT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.divisions ALTER COLUMN id SET DEFAULT nextval('public.divisions_id_seq'::regclass);


--
-- Name: documents id; Type: DEFAULT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.documents ALTER COLUMN id SET DEFAULT nextval('public.documents_id_seq'::regclass);


--
-- Name: employee_categories id; Type: DEFAULT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.employee_categories ALTER COLUMN id SET DEFAULT nextval('public.employee_categories_id_seq'::regclass);


--
-- Name: employees id; Type: DEFAULT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.employees ALTER COLUMN id SET DEFAULT nextval('public.employees_id_seq'::regclass);


--
-- Name: employment_types id; Type: DEFAULT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.employment_types ALTER COLUMN id SET DEFAULT nextval('public.employment_types_id_seq'::regclass);


--
-- Name: event_attendances id; Type: DEFAULT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.event_attendances ALTER COLUMN id SET DEFAULT nextval('public.event_attendances_id_seq'::regclass);


--
-- Name: failed_jobs id; Type: DEFAULT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.failed_jobs ALTER COLUMN id SET DEFAULT nextval('public.failed_jobs_id_seq'::regclass);


--
-- Name: holidays id; Type: DEFAULT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.holidays ALTER COLUMN id SET DEFAULT nextval('public.holidays_id_seq'::regclass);


--
-- Name: jobs id; Type: DEFAULT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.jobs ALTER COLUMN id SET DEFAULT nextval('public.jobs_id_seq'::regclass);


--
-- Name: leave_requests id; Type: DEFAULT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.leave_requests ALTER COLUMN id SET DEFAULT nextval('public.leave_requests_id_seq'::regclass);


--
-- Name: migrations id; Type: DEFAULT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.migrations ALTER COLUMN id SET DEFAULT nextval('public.migrations_id_seq'::regclass);


--
-- Name: periods id; Type: DEFAULT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.periods ALTER COLUMN id SET DEFAULT nextval('public.periods_id_seq'::regclass);


--
-- Name: permissions id; Type: DEFAULT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.permissions ALTER COLUMN id SET DEFAULT nextval('public.permissions_id_seq'::regclass);


--
-- Name: personal_access_tokens id; Type: DEFAULT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.personal_access_tokens ALTER COLUMN id SET DEFAULT nextval('public.personal_access_tokens_id_seq'::regclass);


--
-- Name: posts id; Type: DEFAULT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.posts ALTER COLUMN id SET DEFAULT nextval('public.posts_id_seq'::regclass);


--
-- Name: regions id; Type: DEFAULT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.regions ALTER COLUMN id SET DEFAULT nextval('public.regions_id_seq'::regclass);


--
-- Name: roles id; Type: DEFAULT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.roles ALTER COLUMN id SET DEFAULT nextval('public.roles_id_seq'::regclass);


--
-- Name: rosters id; Type: DEFAULT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.rosters ALTER COLUMN id SET DEFAULT nextval('public.rosters_id_seq'::regclass);


--
-- Name: shift_department id; Type: DEFAULT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.shift_department ALTER COLUMN id SET DEFAULT nextval('public.shift_department_id_seq'::regclass);


--
-- Name: shifts id; Type: DEFAULT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.shifts ALTER COLUMN id SET DEFAULT nextval('public.shifts_id_seq'::regclass);


--
-- Name: users id; Type: DEFAULT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.users ALTER COLUMN id SET DEFAULT nextval('public.users_id_seq'::regclass);


--
-- Name: work_locations id; Type: DEFAULT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.work_locations ALTER COLUMN id SET DEFAULT nextval('public.work_locations_id_seq'::regclass);


--
-- Name: attendance_daily attendance_daily_employee_id_date_unique; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.attendance_daily
    ADD CONSTRAINT attendance_daily_employee_id_date_unique UNIQUE (employee_id, date);


--
-- Name: attendance_daily attendance_daily_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.attendance_daily
    ADD CONSTRAINT attendance_daily_pkey PRIMARY KEY (id);


--
-- Name: attendance_events attendance_events_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.attendance_events
    ADD CONSTRAINT attendance_events_pkey PRIMARY KEY (id);


--
-- Name: attendance_location_checks attendance_location_checks_attendance_punch_id_unique; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.attendance_location_checks
    ADD CONSTRAINT attendance_location_checks_attendance_punch_id_unique UNIQUE (attendance_punch_id);


--
-- Name: attendance_location_checks attendance_location_checks_event_attendance_id_unique; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.attendance_location_checks
    ADD CONSTRAINT attendance_location_checks_event_attendance_id_unique UNIQUE (event_attendance_id);


--
-- Name: attendance_location_checks attendance_location_checks_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.attendance_location_checks
    ADD CONSTRAINT attendance_location_checks_pkey PRIMARY KEY (id);


--
-- Name: attendance_punches attendance_punches_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.attendance_punches
    ADD CONSTRAINT attendance_punches_pkey PRIMARY KEY (id);


--
-- Name: attendance_statuses attendance_statuses_code_unique; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.attendance_statuses
    ADD CONSTRAINT attendance_statuses_code_unique UNIQUE (code);


--
-- Name: attendance_statuses attendance_statuses_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.attendance_statuses
    ADD CONSTRAINT attendance_statuses_pkey PRIMARY KEY (id);


--
-- Name: audit_logs audit_logs_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.audit_logs
    ADD CONSTRAINT audit_logs_pkey PRIMARY KEY (id);


--
-- Name: biometric_templates biometric_templates_identity_unique; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.biometric_templates
    ADD CONSTRAINT biometric_templates_identity_unique UNIQUE (employee_id, kind, finger_index, algorithm_version);


--
-- Name: biometric_templates biometric_templates_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.biometric_templates
    ADD CONSTRAINT biometric_templates_pkey PRIMARY KEY (id);


--
-- Name: cache_locks cache_locks_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.cache_locks
    ADD CONSTRAINT cache_locks_pkey PRIMARY KEY (key);


--
-- Name: cache cache_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.cache
    ADD CONSTRAINT cache_pkey PRIMARY KEY (key);


--
-- Name: departments departments_code_unique; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.departments
    ADD CONSTRAINT departments_code_unique UNIQUE (code);


--
-- Name: departments departments_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.departments
    ADD CONSTRAINT departments_pkey PRIMARY KEY (id);


--
-- Name: device_commands device_commands_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.device_commands
    ADD CONSTRAINT device_commands_pkey PRIMARY KEY (id);


--
-- Name: device_logs device_logs_device_pin_time_unique; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.device_logs
    ADD CONSTRAINT device_logs_device_pin_time_unique UNIQUE (device_id, pin, device_time);


--
-- Name: device_logs device_logs_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.device_logs
    ADD CONSTRAINT device_logs_pkey PRIMARY KEY (id);


--
-- Name: devices devices_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.devices
    ADD CONSTRAINT devices_pkey PRIMARY KEY (id);


--
-- Name: devices devices_serial_number_unique; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.devices
    ADD CONSTRAINT devices_serial_number_unique UNIQUE (serial_number);


--
-- Name: divisions divisions_code_unique; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.divisions
    ADD CONSTRAINT divisions_code_unique UNIQUE (code);


--
-- Name: divisions divisions_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.divisions
    ADD CONSTRAINT divisions_pkey PRIMARY KEY (id);


--
-- Name: documents documents_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.documents
    ADD CONSTRAINT documents_pkey PRIMARY KEY (id);


--
-- Name: employee_categories employee_categories_code_unique; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.employee_categories
    ADD CONSTRAINT employee_categories_code_unique UNIQUE (code);


--
-- Name: employee_categories employee_categories_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.employee_categories
    ADD CONSTRAINT employee_categories_pkey PRIMARY KEY (id);


--
-- Name: employees employees_employee_number_unique; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.employees
    ADD CONSTRAINT employees_employee_number_unique UNIQUE (employee_number);


--
-- Name: employees employees_national_id_unique; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.employees
    ADD CONSTRAINT employees_national_id_unique UNIQUE (national_id);


--
-- Name: employees employees_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.employees
    ADD CONSTRAINT employees_pkey PRIMARY KEY (id);


--
-- Name: employment_types employment_types_code_unique; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.employment_types
    ADD CONSTRAINT employment_types_code_unique UNIQUE (code);


--
-- Name: employment_types employment_types_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.employment_types
    ADD CONSTRAINT employment_types_pkey PRIMARY KEY (id);


--
-- Name: event_attendances event_attendances_event_id_employee_id_unique; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.event_attendances
    ADD CONSTRAINT event_attendances_event_id_employee_id_unique UNIQUE (event_id, employee_id);


--
-- Name: event_attendances event_attendances_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.event_attendances
    ADD CONSTRAINT event_attendances_pkey PRIMARY KEY (id);


--
-- Name: failed_jobs failed_jobs_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.failed_jobs
    ADD CONSTRAINT failed_jobs_pkey PRIMARY KEY (id);


--
-- Name: failed_jobs failed_jobs_uuid_unique; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.failed_jobs
    ADD CONSTRAINT failed_jobs_uuid_unique UNIQUE (uuid);


--
-- Name: holidays holidays_date_unique; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.holidays
    ADD CONSTRAINT holidays_date_unique UNIQUE (date);


--
-- Name: holidays holidays_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.holidays
    ADD CONSTRAINT holidays_pkey PRIMARY KEY (id);


--
-- Name: job_batches job_batches_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.job_batches
    ADD CONSTRAINT job_batches_pkey PRIMARY KEY (id);


--
-- Name: jobs jobs_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.jobs
    ADD CONSTRAINT jobs_pkey PRIMARY KEY (id);


--
-- Name: leave_requests leave_requests_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.leave_requests
    ADD CONSTRAINT leave_requests_pkey PRIMARY KEY (id);


--
-- Name: migrations migrations_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.migrations
    ADD CONSTRAINT migrations_pkey PRIMARY KEY (id);


--
-- Name: model_has_permissions model_has_permissions_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.model_has_permissions
    ADD CONSTRAINT model_has_permissions_pkey PRIMARY KEY (permission_id, model_id, model_type);


--
-- Name: model_has_roles model_has_roles_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.model_has_roles
    ADD CONSTRAINT model_has_roles_pkey PRIMARY KEY (role_id, model_id, model_type);


--
-- Name: password_reset_tokens password_reset_tokens_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.password_reset_tokens
    ADD CONSTRAINT password_reset_tokens_pkey PRIMARY KEY (email);


--
-- Name: periods periods_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.periods
    ADD CONSTRAINT periods_pkey PRIMARY KEY (id);


--
-- Name: periods periods_year_month_unique; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.periods
    ADD CONSTRAINT periods_year_month_unique UNIQUE (year, month);


--
-- Name: permissions permissions_name_guard_name_unique; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.permissions
    ADD CONSTRAINT permissions_name_guard_name_unique UNIQUE (name, guard_name);


--
-- Name: permissions permissions_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.permissions
    ADD CONSTRAINT permissions_pkey PRIMARY KEY (id);


--
-- Name: personal_access_tokens personal_access_tokens_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.personal_access_tokens
    ADD CONSTRAINT personal_access_tokens_pkey PRIMARY KEY (id);


--
-- Name: personal_access_tokens personal_access_tokens_token_unique; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.personal_access_tokens
    ADD CONSTRAINT personal_access_tokens_token_unique UNIQUE (token);


--
-- Name: posts posts_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.posts
    ADD CONSTRAINT posts_pkey PRIMARY KEY (id);


--
-- Name: regions regions_code_unique; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.regions
    ADD CONSTRAINT regions_code_unique UNIQUE (code);


--
-- Name: regions regions_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.regions
    ADD CONSTRAINT regions_pkey PRIMARY KEY (id);


--
-- Name: role_has_permissions role_has_permissions_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.role_has_permissions
    ADD CONSTRAINT role_has_permissions_pkey PRIMARY KEY (permission_id, role_id);


--
-- Name: roles roles_name_guard_name_unique; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.roles
    ADD CONSTRAINT roles_name_guard_name_unique UNIQUE (name, guard_name);


--
-- Name: roles roles_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.roles
    ADD CONSTRAINT roles_pkey PRIMARY KEY (id);


--
-- Name: rosters rosters_employee_id_date_unique; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.rosters
    ADD CONSTRAINT rosters_employee_id_date_unique UNIQUE (employee_id, date);


--
-- Name: rosters rosters_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.rosters
    ADD CONSTRAINT rosters_pkey PRIMARY KEY (id);


--
-- Name: sessions sessions_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.sessions
    ADD CONSTRAINT sessions_pkey PRIMARY KEY (id);


--
-- Name: shift_department shift_department_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.shift_department
    ADD CONSTRAINT shift_department_pkey PRIMARY KEY (id);


--
-- Name: shift_department shift_department_shift_id_department_id_unique; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.shift_department
    ADD CONSTRAINT shift_department_shift_id_department_id_unique UNIQUE (shift_id, department_id);


--
-- Name: shifts shifts_code_unique; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.shifts
    ADD CONSTRAINT shifts_code_unique UNIQUE (code);


--
-- Name: shifts shifts_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.shifts
    ADD CONSTRAINT shifts_pkey PRIMARY KEY (id);


--
-- Name: users users_email_unique; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.users
    ADD CONSTRAINT users_email_unique UNIQUE (email);


--
-- Name: users users_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.users
    ADD CONSTRAINT users_pkey PRIMARY KEY (id);


--
-- Name: users users_username_unique; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.users
    ADD CONSTRAINT users_username_unique UNIQUE (username);


--
-- Name: work_locations work_locations_name_unique; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.work_locations
    ADD CONSTRAINT work_locations_name_unique UNIQUE (name);


--
-- Name: work_locations work_locations_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.work_locations
    ADD CONSTRAINT work_locations_pkey PRIMARY KEY (id);


--
-- Name: attendance_daily_date_index; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX attendance_daily_date_index ON public.attendance_daily USING btree (date);


--
-- Name: attendance_daily_department_id_date_index; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX attendance_daily_department_id_date_index ON public.attendance_daily USING btree (department_id, date);


--
-- Name: attendance_daily_department_id_index; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX attendance_daily_department_id_index ON public.attendance_daily USING btree (department_id);


--
-- Name: attendance_daily_employee_id_index; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX attendance_daily_employee_id_index ON public.attendance_daily USING btree (employee_id);


--
-- Name: attendance_daily_period_id_index; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX attendance_daily_period_id_index ON public.attendance_daily USING btree (period_id);


--
-- Name: attendance_daily_status_index; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX attendance_daily_status_index ON public.attendance_daily USING btree (status);


--
-- Name: attendance_events_created_by_index; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX attendance_events_created_by_index ON public.attendance_events USING btree (created_by);


--
-- Name: attendance_events_department_id_index; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX attendance_events_department_id_index ON public.attendance_events USING btree (department_id);


--
-- Name: attendance_events_event_date_index; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX attendance_events_event_date_index ON public.attendance_events USING btree (event_date);


--
-- Name: attendance_events_is_active_index; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX attendance_events_is_active_index ON public.attendance_events USING btree (is_active);


--
-- Name: attendance_events_work_location_id_index; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX attendance_events_work_location_id_index ON public.attendance_events USING btree (work_location_id);


--
-- Name: attendance_location_checks_is_flagged_created_at_index; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX attendance_location_checks_is_flagged_created_at_index ON public.attendance_location_checks USING btree (is_flagged, created_at);


--
-- Name: attendance_location_checks_work_location_id_index; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX attendance_location_checks_work_location_id_index ON public.attendance_location_checks USING btree (work_location_id);


--
-- Name: attendance_punches_created_by_index; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX attendance_punches_created_by_index ON public.attendance_punches USING btree (created_by);


--
-- Name: attendance_punches_employee_id_index; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX attendance_punches_employee_id_index ON public.attendance_punches USING btree (employee_id);


--
-- Name: attendance_punches_employee_id_punched_at_index; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX attendance_punches_employee_id_punched_at_index ON public.attendance_punches USING btree (employee_id, punched_at);


--
-- Name: attendance_punches_punched_at_index; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX attendance_punches_punched_at_index ON public.attendance_punches USING btree (punched_at);


--
-- Name: attendance_punches_reviewed_by_index; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX attendance_punches_reviewed_by_index ON public.attendance_punches USING btree (reviewed_by);


--
-- Name: attendance_punches_status_index; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX attendance_punches_status_index ON public.attendance_punches USING btree (status);


--
-- Name: attendance_statuses_is_active_index; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX attendance_statuses_is_active_index ON public.attendance_statuses USING btree (is_active);


--
-- Name: audit_logs_action_index; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX audit_logs_action_index ON public.audit_logs USING btree (action);


--
-- Name: audit_logs_created_at_index; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX audit_logs_created_at_index ON public.audit_logs USING btree (created_at);


--
-- Name: audit_logs_user_id_index; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX audit_logs_user_id_index ON public.audit_logs USING btree (user_id);


--
-- Name: biometric_templates_employee_id_index; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX biometric_templates_employee_id_index ON public.biometric_templates USING btree (employee_id);


--
-- Name: biometric_templates_source_device_id_index; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX biometric_templates_source_device_id_index ON public.biometric_templates USING btree (source_device_id);


--
-- Name: cache_expiration_index; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX cache_expiration_index ON public.cache USING btree (expiration);


--
-- Name: cache_locks_expiration_index; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX cache_locks_expiration_index ON public.cache_locks USING btree (expiration);


--
-- Name: departments_division_id_index; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX departments_division_id_index ON public.departments USING btree (division_id);


--
-- Name: departments_parent_id_index; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX departments_parent_id_index ON public.departments USING btree (parent_id);


--
-- Name: departments_region_id_index; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX departments_region_id_index ON public.departments USING btree (region_id);


--
-- Name: departments_work_location_id_index; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX departments_work_location_id_index ON public.departments USING btree (work_location_id);


--
-- Name: device_commands_active_maintenance_unique; Type: INDEX; Schema: public; Owner: -
--

CREATE UNIQUE INDEX device_commands_active_maintenance_unique ON public.device_commands USING btree (device_id, command_type) WHERE (((status)::text = ANY ((ARRAY['pending'::character varying, 'sent'::character varying])::text[])) AND ((command_type)::text = ANY ((ARRAY['info'::character varying, 'check'::character varying, 'set_time'::character varying])::text[])));


--
-- Name: device_commands_biometric_template_id_index; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX device_commands_biometric_template_id_index ON public.device_commands USING btree (biometric_template_id);


--
-- Name: device_commands_device_id_index; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX device_commands_device_id_index ON public.device_commands USING btree (device_id);


--
-- Name: device_commands_employee_id_index; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX device_commands_employee_id_index ON public.device_commands USING btree (employee_id);


--
-- Name: device_commands_poll_index; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX device_commands_poll_index ON public.device_commands USING btree (device_id, status, priority, id);


--
-- Name: device_commands_requested_by_index; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX device_commands_requested_by_index ON public.device_commands USING btree (requested_by);


--
-- Name: device_logs_device_id_index; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX device_logs_device_id_index ON public.device_logs USING btree (device_id);


--
-- Name: device_logs_employee_id_index; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX device_logs_employee_id_index ON public.device_logs USING btree (employee_id);


--
-- Name: device_logs_punch_id_index; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX device_logs_punch_id_index ON public.device_logs USING btree (punch_id);


--
-- Name: device_logs_punch_id_unique; Type: INDEX; Schema: public; Owner: -
--

CREATE UNIQUE INDEX device_logs_punch_id_unique ON public.device_logs USING btree (punch_id) WHERE (punch_id IS NOT NULL);


--
-- Name: device_logs_punched_at_index; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX device_logs_punched_at_index ON public.device_logs USING btree (punched_at);


--
-- Name: device_logs_review_index; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX device_logs_review_index ON public.device_logs USING btree (outcome, device_id) WHERE ((outcome)::text <> 'punched'::text);


--
-- Name: devices_is_active_last_seen_at_index; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX devices_is_active_last_seen_at_index ON public.devices USING btree (is_active, last_seen_at);


--
-- Name: devices_region_id_index; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX devices_region_id_index ON public.devices USING btree (region_id);


--
-- Name: documents_created_at_index; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX documents_created_at_index ON public.documents USING btree (created_at);


--
-- Name: documents_uploaded_by_index; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX documents_uploaded_by_index ON public.documents USING btree (uploaded_by);


--
-- Name: employees_department_id_index; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX employees_department_id_index ON public.employees USING btree (department_id);


--
-- Name: employees_employee_category_id_index; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX employees_employee_category_id_index ON public.employees USING btree (employee_category_id);


--
-- Name: employees_employment_type_id_index; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX employees_employment_type_id_index ON public.employees USING btree (employment_type_id);


--
-- Name: employees_name_index; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX employees_name_index ON public.employees USING btree (name);


--
-- Name: employees_work_location_id_index; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX employees_work_location_id_index ON public.employees USING btree (work_location_id);


--
-- Name: event_attendances_employee_id_index; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX event_attendances_employee_id_index ON public.event_attendances USING btree (employee_id);


--
-- Name: event_attendances_event_id_index; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX event_attendances_event_id_index ON public.event_attendances USING btree (event_id);


--
-- Name: failed_jobs_connection_queue_failed_at_index; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX failed_jobs_connection_queue_failed_at_index ON public.failed_jobs USING btree (connection, queue, failed_at);


--
-- Name: jobs_queue_index; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX jobs_queue_index ON public.jobs USING btree (queue);


--
-- Name: leave_requests_attendance_status_id_index; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX leave_requests_attendance_status_id_index ON public.leave_requests USING btree (attendance_status_id);


--
-- Name: leave_requests_employee_id_index; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX leave_requests_employee_id_index ON public.leave_requests USING btree (employee_id);


--
-- Name: leave_requests_employee_id_start_date_end_date_index; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX leave_requests_employee_id_start_date_end_date_index ON public.leave_requests USING btree (employee_id, start_date, end_date);


--
-- Name: leave_requests_end_date_index; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX leave_requests_end_date_index ON public.leave_requests USING btree (end_date);


--
-- Name: leave_requests_requested_by_index; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX leave_requests_requested_by_index ON public.leave_requests USING btree (requested_by);


--
-- Name: leave_requests_reviewed_by_index; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX leave_requests_reviewed_by_index ON public.leave_requests USING btree (reviewed_by);


--
-- Name: leave_requests_start_date_index; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX leave_requests_start_date_index ON public.leave_requests USING btree (start_date);


--
-- Name: leave_requests_status_index; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX leave_requests_status_index ON public.leave_requests USING btree (status);


--
-- Name: model_has_permissions_model_id_model_type_index; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX model_has_permissions_model_id_model_type_index ON public.model_has_permissions USING btree (model_id, model_type);


--
-- Name: model_has_roles_model_id_model_type_index; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX model_has_roles_model_id_model_type_index ON public.model_has_roles USING btree (model_id, model_type);


--
-- Name: periods_closed_by_index; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX periods_closed_by_index ON public.periods USING btree (closed_by);


--
-- Name: periods_reopened_by_index; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX periods_reopened_by_index ON public.periods USING btree (reopened_by);


--
-- Name: periods_status_index; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX periods_status_index ON public.periods USING btree (status);


--
-- Name: personal_access_tokens_expires_at_index; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX personal_access_tokens_expires_at_index ON public.personal_access_tokens USING btree (expires_at);


--
-- Name: personal_access_tokens_tokenable_type_tokenable_id_index; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX personal_access_tokens_tokenable_type_tokenable_id_index ON public.personal_access_tokens USING btree (tokenable_type, tokenable_id);


--
-- Name: posts_created_by_index; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX posts_created_by_index ON public.posts USING btree (created_by);


--
-- Name: posts_is_published_published_at_index; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX posts_is_published_published_at_index ON public.posts USING btree (is_published, published_at);


--
-- Name: rosters_attendance_status_id_index; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX rosters_attendance_status_id_index ON public.rosters USING btree (attendance_status_id);


--
-- Name: rosters_date_index; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX rosters_date_index ON public.rosters USING btree (date);


--
-- Name: rosters_employee_id_index; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX rosters_employee_id_index ON public.rosters USING btree (employee_id);


--
-- Name: rosters_shift_id_index; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX rosters_shift_id_index ON public.rosters USING btree (shift_id);


--
-- Name: sessions_last_activity_index; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX sessions_last_activity_index ON public.sessions USING btree (last_activity);


--
-- Name: sessions_user_id_index; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX sessions_user_id_index ON public.sessions USING btree (user_id);


--
-- Name: shift_department_department_id_index; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX shift_department_department_id_index ON public.shift_department USING btree (department_id);


--
-- Name: shift_department_shift_id_index; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX shift_department_shift_id_index ON public.shift_department USING btree (shift_id);


--
-- Name: shifts_is_active_index; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX shifts_is_active_index ON public.shifts USING btree (is_active);


--
-- Name: users_department_id_index; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX users_department_id_index ON public.users USING btree (department_id);


--
-- Name: users_employee_id_index; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX users_employee_id_index ON public.users USING btree (employee_id);


--
-- Name: users_region_id_index; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX users_region_id_index ON public.users USING btree (region_id);


--
-- Name: work_locations_is_active_index; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX work_locations_is_active_index ON public.work_locations USING btree (is_active);


--
-- Name: attendance_daily attendance_daily_department_id_foreign; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.attendance_daily
    ADD CONSTRAINT attendance_daily_department_id_foreign FOREIGN KEY (department_id) REFERENCES public.departments(id) ON DELETE RESTRICT;


--
-- Name: attendance_daily attendance_daily_employee_id_foreign; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.attendance_daily
    ADD CONSTRAINT attendance_daily_employee_id_foreign FOREIGN KEY (employee_id) REFERENCES public.employees(id) ON DELETE CASCADE;


--
-- Name: attendance_daily attendance_daily_period_id_foreign; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.attendance_daily
    ADD CONSTRAINT attendance_daily_period_id_foreign FOREIGN KEY (period_id) REFERENCES public.periods(id) ON DELETE CASCADE;


--
-- Name: attendance_events attendance_events_created_by_foreign; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.attendance_events
    ADD CONSTRAINT attendance_events_created_by_foreign FOREIGN KEY (created_by) REFERENCES public.users(id) ON DELETE SET NULL;


--
-- Name: attendance_events attendance_events_department_id_foreign; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.attendance_events
    ADD CONSTRAINT attendance_events_department_id_foreign FOREIGN KEY (department_id) REFERENCES public.departments(id) ON DELETE RESTRICT;


--
-- Name: attendance_events attendance_events_work_location_id_foreign; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.attendance_events
    ADD CONSTRAINT attendance_events_work_location_id_foreign FOREIGN KEY (work_location_id) REFERENCES public.work_locations(id) ON DELETE RESTRICT;


--
-- Name: attendance_location_checks attendance_location_checks_attendance_punch_id_foreign; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.attendance_location_checks
    ADD CONSTRAINT attendance_location_checks_attendance_punch_id_foreign FOREIGN KEY (attendance_punch_id) REFERENCES public.attendance_punches(id) ON DELETE CASCADE;


--
-- Name: attendance_location_checks attendance_location_checks_event_attendance_id_foreign; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.attendance_location_checks
    ADD CONSTRAINT attendance_location_checks_event_attendance_id_foreign FOREIGN KEY (event_attendance_id) REFERENCES public.event_attendances(id) ON DELETE CASCADE;


--
-- Name: attendance_location_checks attendance_location_checks_work_location_id_foreign; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.attendance_location_checks
    ADD CONSTRAINT attendance_location_checks_work_location_id_foreign FOREIGN KEY (work_location_id) REFERENCES public.work_locations(id) ON DELETE SET NULL;


--
-- Name: attendance_punches attendance_punches_created_by_foreign; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.attendance_punches
    ADD CONSTRAINT attendance_punches_created_by_foreign FOREIGN KEY (created_by) REFERENCES public.users(id) ON DELETE SET NULL;


--
-- Name: attendance_punches attendance_punches_employee_id_foreign; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.attendance_punches
    ADD CONSTRAINT attendance_punches_employee_id_foreign FOREIGN KEY (employee_id) REFERENCES public.employees(id) ON DELETE CASCADE;


--
-- Name: attendance_punches attendance_punches_reviewed_by_foreign; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.attendance_punches
    ADD CONSTRAINT attendance_punches_reviewed_by_foreign FOREIGN KEY (reviewed_by) REFERENCES public.users(id) ON DELETE SET NULL;


--
-- Name: audit_logs audit_logs_user_id_foreign; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.audit_logs
    ADD CONSTRAINT audit_logs_user_id_foreign FOREIGN KEY (user_id) REFERENCES public.users(id) ON DELETE SET NULL;


--
-- Name: biometric_templates biometric_templates_employee_id_foreign; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.biometric_templates
    ADD CONSTRAINT biometric_templates_employee_id_foreign FOREIGN KEY (employee_id) REFERENCES public.employees(id) ON DELETE CASCADE;


--
-- Name: biometric_templates biometric_templates_source_device_id_foreign; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.biometric_templates
    ADD CONSTRAINT biometric_templates_source_device_id_foreign FOREIGN KEY (source_device_id) REFERENCES public.devices(id) ON DELETE SET NULL;


--
-- Name: departments departments_division_id_foreign; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.departments
    ADD CONSTRAINT departments_division_id_foreign FOREIGN KEY (division_id) REFERENCES public.divisions(id) ON DELETE SET NULL;


--
-- Name: departments departments_parent_id_foreign; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.departments
    ADD CONSTRAINT departments_parent_id_foreign FOREIGN KEY (parent_id) REFERENCES public.departments(id) ON DELETE SET NULL;


--
-- Name: departments departments_region_id_foreign; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.departments
    ADD CONSTRAINT departments_region_id_foreign FOREIGN KEY (region_id) REFERENCES public.regions(id) ON DELETE SET NULL;


--
-- Name: departments departments_work_location_id_foreign; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.departments
    ADD CONSTRAINT departments_work_location_id_foreign FOREIGN KEY (work_location_id) REFERENCES public.work_locations(id) ON DELETE RESTRICT;


--
-- Name: device_commands device_commands_biometric_template_id_foreign; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.device_commands
    ADD CONSTRAINT device_commands_biometric_template_id_foreign FOREIGN KEY (biometric_template_id) REFERENCES public.biometric_templates(id) ON DELETE CASCADE;


--
-- Name: device_commands device_commands_device_id_foreign; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.device_commands
    ADD CONSTRAINT device_commands_device_id_foreign FOREIGN KEY (device_id) REFERENCES public.devices(id) ON DELETE CASCADE;


--
-- Name: device_commands device_commands_employee_id_foreign; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.device_commands
    ADD CONSTRAINT device_commands_employee_id_foreign FOREIGN KEY (employee_id) REFERENCES public.employees(id) ON DELETE CASCADE;


--
-- Name: device_commands device_commands_requested_by_foreign; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.device_commands
    ADD CONSTRAINT device_commands_requested_by_foreign FOREIGN KEY (requested_by) REFERENCES public.users(id) ON DELETE SET NULL;


--
-- Name: device_logs device_logs_device_id_foreign; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.device_logs
    ADD CONSTRAINT device_logs_device_id_foreign FOREIGN KEY (device_id) REFERENCES public.devices(id) ON DELETE RESTRICT;


--
-- Name: device_logs device_logs_employee_id_foreign; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.device_logs
    ADD CONSTRAINT device_logs_employee_id_foreign FOREIGN KEY (employee_id) REFERENCES public.employees(id) ON DELETE SET NULL;


--
-- Name: device_logs device_logs_punch_id_foreign; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.device_logs
    ADD CONSTRAINT device_logs_punch_id_foreign FOREIGN KEY (punch_id) REFERENCES public.attendance_punches(id) ON DELETE SET NULL;


--
-- Name: devices devices_region_id_foreign; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.devices
    ADD CONSTRAINT devices_region_id_foreign FOREIGN KEY (region_id) REFERENCES public.regions(id) ON DELETE SET NULL;


--
-- Name: documents documents_uploaded_by_foreign; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.documents
    ADD CONSTRAINT documents_uploaded_by_foreign FOREIGN KEY (uploaded_by) REFERENCES public.users(id) ON DELETE SET NULL;


--
-- Name: employees employees_department_id_foreign; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.employees
    ADD CONSTRAINT employees_department_id_foreign FOREIGN KEY (department_id) REFERENCES public.departments(id) ON DELETE RESTRICT;


--
-- Name: employees employees_employee_category_id_foreign; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.employees
    ADD CONSTRAINT employees_employee_category_id_foreign FOREIGN KEY (employee_category_id) REFERENCES public.employee_categories(id) ON DELETE SET NULL;


--
-- Name: employees employees_employment_type_id_foreign; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.employees
    ADD CONSTRAINT employees_employment_type_id_foreign FOREIGN KEY (employment_type_id) REFERENCES public.employment_types(id) ON DELETE SET NULL;


--
-- Name: employees employees_work_location_id_foreign; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.employees
    ADD CONSTRAINT employees_work_location_id_foreign FOREIGN KEY (work_location_id) REFERENCES public.work_locations(id) ON DELETE RESTRICT;


--
-- Name: event_attendances event_attendances_employee_id_foreign; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.event_attendances
    ADD CONSTRAINT event_attendances_employee_id_foreign FOREIGN KEY (employee_id) REFERENCES public.employees(id) ON DELETE CASCADE;


--
-- Name: event_attendances event_attendances_event_id_foreign; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.event_attendances
    ADD CONSTRAINT event_attendances_event_id_foreign FOREIGN KEY (event_id) REFERENCES public.attendance_events(id) ON DELETE CASCADE;


--
-- Name: leave_requests leave_requests_attendance_status_id_foreign; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.leave_requests
    ADD CONSTRAINT leave_requests_attendance_status_id_foreign FOREIGN KEY (attendance_status_id) REFERENCES public.attendance_statuses(id) ON DELETE RESTRICT;


--
-- Name: leave_requests leave_requests_employee_id_foreign; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.leave_requests
    ADD CONSTRAINT leave_requests_employee_id_foreign FOREIGN KEY (employee_id) REFERENCES public.employees(id) ON DELETE CASCADE;


--
-- Name: leave_requests leave_requests_requested_by_foreign; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.leave_requests
    ADD CONSTRAINT leave_requests_requested_by_foreign FOREIGN KEY (requested_by) REFERENCES public.users(id) ON DELETE SET NULL;


--
-- Name: leave_requests leave_requests_reviewed_by_foreign; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.leave_requests
    ADD CONSTRAINT leave_requests_reviewed_by_foreign FOREIGN KEY (reviewed_by) REFERENCES public.users(id) ON DELETE SET NULL;


--
-- Name: model_has_permissions model_has_permissions_permission_id_foreign; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.model_has_permissions
    ADD CONSTRAINT model_has_permissions_permission_id_foreign FOREIGN KEY (permission_id) REFERENCES public.permissions(id) ON DELETE CASCADE;


--
-- Name: model_has_roles model_has_roles_role_id_foreign; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.model_has_roles
    ADD CONSTRAINT model_has_roles_role_id_foreign FOREIGN KEY (role_id) REFERENCES public.roles(id) ON DELETE CASCADE;


--
-- Name: periods periods_closed_by_foreign; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.periods
    ADD CONSTRAINT periods_closed_by_foreign FOREIGN KEY (closed_by) REFERENCES public.users(id) ON DELETE SET NULL;


--
-- Name: periods periods_reopened_by_foreign; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.periods
    ADD CONSTRAINT periods_reopened_by_foreign FOREIGN KEY (reopened_by) REFERENCES public.users(id) ON DELETE SET NULL;


--
-- Name: posts posts_created_by_foreign; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.posts
    ADD CONSTRAINT posts_created_by_foreign FOREIGN KEY (created_by) REFERENCES public.users(id) ON DELETE SET NULL;


--
-- Name: role_has_permissions role_has_permissions_permission_id_foreign; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.role_has_permissions
    ADD CONSTRAINT role_has_permissions_permission_id_foreign FOREIGN KEY (permission_id) REFERENCES public.permissions(id) ON DELETE CASCADE;


--
-- Name: role_has_permissions role_has_permissions_role_id_foreign; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.role_has_permissions
    ADD CONSTRAINT role_has_permissions_role_id_foreign FOREIGN KEY (role_id) REFERENCES public.roles(id) ON DELETE CASCADE;


--
-- Name: rosters rosters_attendance_status_id_foreign; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.rosters
    ADD CONSTRAINT rosters_attendance_status_id_foreign FOREIGN KEY (attendance_status_id) REFERENCES public.attendance_statuses(id) ON DELETE RESTRICT;


--
-- Name: rosters rosters_employee_id_foreign; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.rosters
    ADD CONSTRAINT rosters_employee_id_foreign FOREIGN KEY (employee_id) REFERENCES public.employees(id) ON DELETE CASCADE;


--
-- Name: rosters rosters_shift_id_foreign; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.rosters
    ADD CONSTRAINT rosters_shift_id_foreign FOREIGN KEY (shift_id) REFERENCES public.shifts(id) ON DELETE RESTRICT;


--
-- Name: shift_department shift_department_department_id_foreign; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.shift_department
    ADD CONSTRAINT shift_department_department_id_foreign FOREIGN KEY (department_id) REFERENCES public.departments(id) ON DELETE CASCADE;


--
-- Name: shift_department shift_department_shift_id_foreign; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.shift_department
    ADD CONSTRAINT shift_department_shift_id_foreign FOREIGN KEY (shift_id) REFERENCES public.shifts(id) ON DELETE CASCADE;


--
-- Name: users users_department_id_foreign; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.users
    ADD CONSTRAINT users_department_id_foreign FOREIGN KEY (department_id) REFERENCES public.departments(id) ON DELETE SET NULL;


--
-- Name: users users_employee_id_foreign; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.users
    ADD CONSTRAINT users_employee_id_foreign FOREIGN KEY (employee_id) REFERENCES public.employees(id) ON DELETE SET NULL;


--
-- Name: users users_region_id_foreign; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.users
    ADD CONSTRAINT users_region_id_foreign FOREIGN KEY (region_id) REFERENCES public.regions(id) ON DELETE SET NULL;


--
-- PostgreSQL database dump complete
--

\unrestrict 2BC2CXqJc6q7KbohQ4VOuTWgNg3zCSBwA0DuwiBPPOvGk9yxtAYyITWV8dmRJxp

--
-- PostgreSQL database dump
--

\restrict frVRCtg7lrBPMaA9otdre4ikiXw8ZE66j52o9Dk0EFM38wzp2taJuaQFWwbYjTm

-- Dumped from database version 18.6
-- Dumped by pg_dump version 18.6

SET statement_timeout = 0;
SET lock_timeout = 0;
SET idle_in_transaction_session_timeout = 0;
SET transaction_timeout = 0;
SET client_encoding = 'UTF8';
SET standard_conforming_strings = on;
SELECT pg_catalog.set_config('search_path', '', false);
SET check_function_bodies = false;
SET xmloption = content;
SET client_min_messages = warning;
SET row_security = off;

--
-- Data for Name: migrations; Type: TABLE DATA; Schema: public; Owner: -
--

COPY public.migrations (id, migration, batch) FROM stdin;
1	0001_01_01_000000_create_users_table	1
2	0001_01_01_000001_create_cache_table	1
3	0001_01_01_000002_create_jobs_table	1
4	2026_10_20_145421_create_permission_tables	1
5	2026_10_20_150000_create_organization_tables	1
6	2026_10_20_150100_add_scope_columns_to_users_table	1
7	2026_10_20_160000_make_employee_national_id_unique	1
8	2026_10_21_032859_create_personal_access_tokens_table	1
9	2026_10_21_100000_create_shifts_tables	1
10	2026_10_21_110000_create_attendance_statuses_table	1
11	2026_10_21_120000_create_holidays_table	1
12	2026_10_21_130000_create_rosters_table	1
13	2026_10_21_140000_add_punch_windows_to_shifts_table	1
14	2026_10_21_150000_create_attendance_punches_table	1
15	2026_10_21_160000_create_periods_and_attendance_daily_tables	1
16	2026_10_21_170000_add_review_and_evidence_to_attendance_punches	1
17	2026_10_21_180000_create_attendance_events_tables	1
18	2026_10_21_190000_create_audit_logs_table	1
19	2026_10_21_200000_create_leave_requests_table	1
20	2026_10_21_210000_create_content_tables	1
21	2026_10_25_100000_create_devices_table	1
22	2026_10_25_110000_create_biometric_templates_table	1
23	2026_10_25_120000_create_device_commands_table	1
24	2026_10_25_130000_create_device_logs_table	1
25	2026_11_18_100000_create_work_locations_table	1
26	2026_11_18_100100_add_location_mode_to_attendance_events_table	1
27	2026_11_18_100200_create_attendance_location_checks_table	1
\.


--
-- Name: migrations_id_seq; Type: SEQUENCE SET; Schema: public; Owner: -
--

SELECT pg_catalog.setval('public.migrations_id_seq', 27, true);


--
-- PostgreSQL database dump complete
--

\unrestrict frVRCtg7lrBPMaA9otdre4ikiXw8ZE66j52o9Dk0EFM38wzp2taJuaQFWwbYjTm

