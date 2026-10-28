//! Konfigurasi dari environment. Nama variabel aturan bisnis sama dengan Laravel
//! (`ATTENDANCE_*`, `SANCTUM_*`) agar satu file env bisa dipakai bersama.

use std::net::{IpAddr, SocketAddr};
use std::path::PathBuf;
use std::str::FromStr;
use std::time::Duration;

use chrono_tz::Tz;

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("{0} wajib diisi")]
    Missing(&'static str),
    #[error("{name} tidak valid: {value}")]
    Invalid { name: &'static str, value: String },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LogFormat {
    Json,
    Pretty,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RateLimitStoreKind {
    Memory,
    Redis(String),
}

/// Batas per menit; 0 berarti nonaktif.
#[derive(Clone, Debug)]
pub struct RateLimits {
    pub login_per_ip: u32,
    pub login_per_account: u32,
    pub api_per_token: u32,
    pub actions_per_user: u32,
}

#[derive(Clone, Debug)]
pub struct Config {
    pub listen_addr: SocketAddr,
    pub max_concurrent_requests: usize,
    pub request_timeout: Duration,
    pub log_format: LogFormat,
    pub trusted_proxies: Vec<IpNet>,

    pub database_url: String,
    pub db_max_connections: u32,
    pub db_min_connections: u32,
    pub db_acquire_timeout: Duration,

    pub timezone: Tz,
    pub requires_approval: bool,
    pub requires_photo: bool,
    pub min_interval_seconds: i64,

    /// Menit; 0 berarti token tidak kedaluwarsa berdasarkan umur (perilaku Sanctum).
    pub token_expiration_minutes: i64,
    pub token_prefix: String,
    pub token_last_used_debounce: Duration,

    pub photo_root: PathBuf,
    pub photo_file_mode: u32,
    pub photo_dir_mode: u32,

    pub hash_concurrency: usize,
    pub hash_queue_timeout: Duration,

    pub rate_limits: RateLimits,
    pub rate_limit_store: RateLimitStoreKind,
}

impl Config {
    pub fn from_env() -> Result<Self, ConfigError> {
        Self::from_lookup(|key| std::env::var(key).ok())
    }

    /// Dipisah dari `from_env` agar bisa diuji tanpa mengubah environment proses.
    pub fn from_lookup(get: impl Fn(&str) -> Option<String>) -> Result<Self, ConfigError> {
        // `raw` dipakai untuk variabel Laravel: `X=` (kosong) di Laravel berarti string kosong, bukan default.
        let raw = &get;
        let get = |key: &str| raw(key).filter(|v| !v.trim().is_empty());

        let database_url = get("DATABASE_URL").ok_or(ConfigError::Missing("DATABASE_URL"))?;

        let timezone_name = get("ATTENDANCE_TIMEZONE").unwrap_or_else(|| "Asia/Jakarta".into());
        let timezone = Tz::from_str(timezone_name.trim()).map_err(|_| ConfigError::Invalid {
            name: "ATTENDANCE_TIMEZONE",
            value: timezone_name.clone(),
        })?;

        let rate_limit_store = match get("RATE_LIMIT_STORE").as_deref().map(str::trim) {
            None | Some("memory") => RateLimitStoreKind::Memory,
            Some("redis") => RateLimitStoreKind::Redis(
                get("REDIS_URL").ok_or(ConfigError::Missing("REDIS_URL"))?,
            ),
            Some(other) => {
                return Err(ConfigError::Invalid {
                    name: "RATE_LIMIT_STORE",
                    value: other.into(),
                });
            }
        };

        let log_format = match get("LOG_FORMAT").as_deref().map(str::trim) {
            None | Some("json") => LogFormat::Json,
            Some("pretty") => LogFormat::Pretty,
            Some(other) => {
                return Err(ConfigError::Invalid {
                    name: "LOG_FORMAT",
                    value: other.into(),
                });
            }
        };

        let trusted_proxies = get("TRUSTED_PROXIES")
            .unwrap_or_else(|| "127.0.0.1,::1".into())
            .split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(|s| {
                s.parse::<IpNet>().map_err(|_| ConfigError::Invalid {
                    name: "TRUSTED_PROXIES",
                    value: s.into(),
                })
            })
            .collect::<Result<Vec<_>, _>>()?;

        let cpus = std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(4);

        Ok(Self {
            listen_addr: parse(&get, "LISTEN_ADDR", "127.0.0.1:8080")?,
            max_concurrent_requests: parse(&get, "MAX_CONCURRENT_REQUESTS", "2048")?,
            request_timeout: Duration::from_secs(parse(&get, "REQUEST_TIMEOUT_SECS", "60")?),
            log_format,
            trusted_proxies,

            database_url,
            db_max_connections: parse(&get, "DB_MAX_CONNECTIONS", "20")?,
            db_min_connections: parse(&get, "DB_MIN_CONNECTIONS", "2")?,
            db_acquire_timeout: Duration::from_secs(parse(&get, "DB_ACQUIRE_TIMEOUT_SECS", "5")?),

            timezone,
            requires_approval: laravel_bool(raw("ATTENDANCE_MOBILE_REQUIRES_APPROVAL"), true),
            requires_photo: laravel_bool(raw("ATTENDANCE_MOBILE_REQUIRES_PHOTO"), true),
            min_interval_seconds: laravel_int(raw("ATTENDANCE_MOBILE_MIN_INTERVAL"), 60),

            token_expiration_minutes: laravel_int(raw("SANCTUM_TOKEN_EXPIRATION"), 43200),
            // Prefix tidak di-trim: Laravel memakai nilainya apa adanya.
            token_prefix: raw("SANCTUM_TOKEN_PREFIX").unwrap_or_default(),
            token_last_used_debounce: Duration::from_secs(parse(
                &get,
                "TOKEN_LAST_USED_DEBOUNCE_SECS",
                "60",
            )?),

            photo_root: PathBuf::from(
                get("PHOTO_ROOT").unwrap_or_else(|| "storage/app/private".into()),
            ),
            photo_file_mode: parse_octal(&get, "PHOTO_FILE_MODE", 0o644)?,
            photo_dir_mode: parse_octal(&get, "PHOTO_DIR_MODE", 0o755)?,

            hash_concurrency: parse(&get, "LOGIN_HASH_CONCURRENCY", &cpus.to_string())?,
            hash_queue_timeout: Duration::from_secs(parse(&get, "LOGIN_HASH_QUEUE_SECS", "20")?),

            rate_limits: RateLimits {
                login_per_ip: parse(&get, "RATE_LIMIT_LOGIN_PER_IP", "60")?,
                login_per_account: parse(&get, "RATE_LIMIT_LOGIN_PER_ACCOUNT", "5")?,
                api_per_token: parse(&get, "RATE_LIMIT_API_PER_TOKEN", "120")?,
                actions_per_user: parse(&get, "RATE_LIMIT_ACTIONS_PER_USER", "20")?,
            },
            rate_limit_store,
        })
    }
}

fn parse<T: FromStr>(
    get: &impl Fn(&str) -> Option<String>,
    name: &'static str,
    default: &str,
) -> Result<T, ConfigError> {
    let value = get(name).unwrap_or_else(|| default.to_string());
    value
        .trim()
        .parse()
        .map_err(|_| ConfigError::Invalid { name, value })
}

fn parse_octal(
    get: &impl Fn(&str) -> Option<String>,
    name: &'static str,
    default: u32,
) -> Result<u32, ConfigError> {
    match get(name) {
        None => Ok(default),
        Some(value) => u32::from_str_radix(value.trim().trim_start_matches("0o"), 8)
            .map_err(|_| ConfigError::Invalid { name, value }),
    }
}

/// `(bool) env('X', default)` di Laravel: `true`/`(true)` benar; `false`/`(false)`/`null`/`(null)`/`empty`/
/// string kosong/`"0"` salah; string lain benar.
pub fn laravel_bool(value: Option<String>, default: bool) -> bool {
    let Some(value) = value else {
        return default;
    };
    let v = value.trim();
    !matches!(
        v.to_ascii_lowercase().as_str(),
        "false" | "(false)" | "null" | "(null)" | "empty" | "(empty)" | "" | "0"
    )
}

/// `(int) env('X', default)` di Laravel: awalan angka diambil, selain itu 0.
pub fn laravel_int(value: Option<String>, default: i64) -> i64 {
    let Some(value) = value else {
        return default;
    };
    let v = value.trim();
    let (sign, digits) = match v.strip_prefix('-') {
        Some(rest) => (-1, rest),
        None => (1, v.strip_prefix('+').unwrap_or(v)),
    };
    let digits: String = digits.chars().take_while(char::is_ascii_digit).collect();
    digits.parse::<i64>().map(|n| sign * n).unwrap_or(0)
}

/// Alamat IP atau jaringan CIDR (untuk `TRUSTED_PROXIES`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct IpNet {
    addr: IpAddr,
    prefix: u8,
}

impl IpNet {
    pub fn contains(&self, ip: IpAddr) -> bool {
        let ip = canonical(ip);
        match (self.addr, ip) {
            (IpAddr::V4(net), IpAddr::V4(ip)) => {
                let mask = u32::MAX
                    .checked_shl(32 - u32::from(self.prefix))
                    .unwrap_or(0);
                u32::from(net) & mask == u32::from(ip) & mask
            }
            (IpAddr::V6(net), IpAddr::V6(ip)) => {
                let mask = u128::MAX
                    .checked_shl(128 - u32::from(self.prefix))
                    .unwrap_or(0);
                u128::from(net) & mask == u128::from(ip) & mask
            }
            _ => false,
        }
    }
}

/// IPv4 yang dibungkus IPv6 (`::ffff:a.b.c.d`) diperlakukan sebagai IPv4.
pub fn canonical(ip: IpAddr) -> IpAddr {
    match ip {
        IpAddr::V6(v6) => v6.to_ipv4_mapped().map_or(ip, IpAddr::V4),
        v4 => v4,
    }
}

impl FromStr for IpNet {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (addr, prefix) = match s.split_once('/') {
            Some((a, p)) => (a, Some(p)),
            None => (s, None),
        };
        let addr = canonical(addr.parse::<IpAddr>().map_err(|_| ())?);
        let max = if addr.is_ipv4() { 32 } else { 128 };
        let prefix = match prefix {
            Some(p) => p.parse::<u8>().map_err(|_| ())?,
            None => max,
        };
        if prefix > max {
            return Err(());
        }
        Ok(Self { addr, prefix })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lookup(pairs: &[(&str, &str)]) -> impl Fn(&str) -> Option<String> {
        let pairs: Vec<(String, String)> = pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
        move |key| pairs.iter().find(|(k, _)| k == key).map(|(_, v)| v.clone())
    }

    #[test]
    fn defaults_match_laravel() {
        let config = Config::from_lookup(lookup(&[("DATABASE_URL", "postgres://x")])).unwrap();
        assert_eq!(config.timezone, chrono_tz::Asia::Jakarta);
        assert!(config.requires_approval);
        assert!(config.requires_photo);
        assert_eq!(config.min_interval_seconds, 60);
        assert_eq!(config.token_expiration_minutes, 43200);
        assert_eq!(config.token_prefix, "");
        assert_eq!(config.rate_limits.login_per_ip, 60);
        assert_eq!(config.rate_limits.login_per_account, 5);
        assert_eq!(config.rate_limits.api_per_token, 120);
        assert_eq!(config.rate_limits.actions_per_user, 20);
        assert_eq!(config.rate_limit_store, RateLimitStoreKind::Memory);
    }

    #[test]
    fn database_url_is_required() {
        assert!(matches!(
            Config::from_lookup(lookup(&[])),
            Err(ConfigError::Missing("DATABASE_URL"))
        ));
    }

    #[test]
    fn redis_store_needs_url() {
        let result = Config::from_lookup(lookup(&[
            ("DATABASE_URL", "postgres://x"),
            ("RATE_LIMIT_STORE", "redis"),
        ]));
        assert!(matches!(result, Err(ConfigError::Missing("REDIS_URL"))));
    }

    #[test]
    fn laravel_bool_casting() {
        for falsy in ["false", "(false)", "FALSE", "0", "null", "empty", " ", ""] {
            assert!(!laravel_bool(Some(falsy.into()), true), "{falsy}");
        }
        for truthy in ["true", "(true)", "1", "yes", "off"] {
            assert!(laravel_bool(Some(truthy.into()), false), "{truthy}");
        }
        assert!(laravel_bool(None, true));
    }

    #[test]
    fn laravel_int_casting() {
        assert_eq!(laravel_int(Some("90".into()), 60), 90);
        assert_eq!(laravel_int(Some("90abc".into()), 60), 90);
        assert_eq!(laravel_int(Some("abc".into()), 60), 0);
        assert_eq!(laravel_int(Some("-5".into()), 60), -5);
        assert_eq!(laravel_int(None, 60), 60);
    }

    #[test]
    fn ip_networks() {
        let net: IpNet = "10.0.0.0/8".parse().unwrap();
        assert!(net.contains("10.1.2.3".parse().unwrap()));
        assert!(net.contains("::ffff:10.1.2.3".parse().unwrap()));
        assert!(!net.contains("11.0.0.1".parse().unwrap()));
        let single: IpNet = "::1".parse().unwrap();
        assert!(single.contains("::1".parse().unwrap()));
        assert!("10.0.0.0/33".parse::<IpNet>().is_err());
    }
}
