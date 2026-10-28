//! Format token Laravel Sanctum (v4), agar token buatan Laravel diterima di sini dan sebaliknya.
//!
//! - Token klien: `<id>|<rahasia>`, dengan `<rahasia>` = `SANCTUM_TOKEN_PREFIX` + 40 karakter alfanumerik
//!   acak + `crc32b` heksadesimal dari 40 karakter itu (`HasApiTokens::generateTokenString`).
//! - Kolom `personal_access_tokens.token` = SHA-256 heksadesimal dari `<rahasia>`.
//!   (Prefix ada di dalam rahasia, bukan sebelum id.)

use rand::distr::{Alphanumeric, SampleString};
use sha2::{Digest, Sha256};

pub const TOKENABLE_TYPE: &str = r"App\Models\User";
pub const MOBILE_ABILITY: &str = "mobile";

pub fn sha256_hex(value: &str) -> String {
    hex::encode(Sha256::digest(value.as_bytes()))
}

/// Rahasia baru: prefix + 40 alfanumerik + crc32b.
pub fn generate_secret(prefix: &str) -> String {
    let entropy = Alphanumeric.sample_string(&mut rand::rng(), 40);
    format!("{prefix}{entropy}{:08x}", crc32fast::hash(entropy.as_bytes()))
}

/// `Request::bearerToken()` Laravel: kemunculan terakhir `Bearer ` (tanpa membedakan huruf besar),
/// berhenti di koma pertama.
pub fn bearer_token(header: &str) -> Option<&str> {
    let lower = header.to_ascii_lowercase();
    let position = lower.rfind("bearer ")?;
    let rest = &header[position + 7..];
    let token = rest.split(',').next().unwrap_or(rest);
    (!token.is_empty()).then_some(token)
}

#[derive(Debug, PartialEq, Eq)]
pub enum ParsedToken<'a> {
    /// `<id>|<rahasia>`: cari baris berdasarkan id, lalu bandingkan hash.
    WithId { id: i64, secret: &'a str },
    /// Tanpa `|`: cari langsung berdasarkan hash.
    Plain(&'a str),
}

/// `Guard::isValidBearerToken` + `PersonalAccessToken::findToken`. `None` = pasti tidak sah.
pub fn parse(token: &str) -> Option<ParsedToken<'_>> {
    match token.split_once('|') {
        Some((id, secret)) => {
            if id.is_empty() || !id.bytes().all(|b| b.is_ascii_digit()) || secret.is_empty() {
                return None;
            }
            // Id melebihi bigint tidak mungkin ada di tabel.
            let id = id.parse::<i64>().ok()?;
            Some(ParsedToken::WithId { id, secret })
        }
        None => Some(ParsedToken::Plain(token)),
    }
}

/// Perbandingan waktu-konstan (`hash_equals`).
pub fn constant_time_eq(a: &str, b: &str) -> bool {
    let (a, b) = (a.as_bytes(), b.as_bytes());
    if a.len() != b.len() {
        return false;
    }
    a.iter().zip(b).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}

/// `tokenCan`: ability `*` atau yang diminta ada di kolom `abilities` (JSON array).
pub fn has_ability(abilities: Option<&str>, ability: &str) -> bool {
    abilities
        .and_then(|raw| serde_json::from_str::<Vec<String>>(raw).ok())
        .is_some_and(|list| list.iter().any(|a| a == "*" || a == ability))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_php_hash_functions() {
        // php -r 'echo hash("sha256", "abc"), " ", hash("crc32b", "abc");'
        assert_eq!(
            sha256_hex("abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        assert_eq!(format!("{:08x}", crc32fast::hash(b"abc")), "352441c2");
    }

    #[test]
    fn generated_secret_has_sanctum_shape() {
        let secret = generate_secret("");
        assert_eq!(secret.len(), 48);
        assert!(secret.chars().all(|c| c.is_ascii_alphanumeric()));
        let (entropy, crc) = secret.split_at(40);
        assert_eq!(format!("{:08x}", crc32fast::hash(entropy.as_bytes())), crc);

        let prefixed = generate_secret("eab_");
        assert!(prefixed.starts_with("eab_"));
        assert_eq!(prefixed.len(), 52);
    }

    #[test]
    fn bearer_parsing_like_laravel() {
        assert_eq!(bearer_token("Bearer 1|abc"), Some("1|abc"));
        assert_eq!(bearer_token("bearer 1|abc"), Some("1|abc"));
        assert_eq!(bearer_token("Basic x, Bearer 2|def"), Some("2|def"));
        assert_eq!(bearer_token("Bearer 1|abc,extra"), Some("1|abc"));
        assert_eq!(bearer_token("Basic x"), None);
        assert_eq!(bearer_token("Bearer "), None);
    }

    #[test]
    fn token_shapes() {
        assert_eq!(
            parse("12|secret"),
            Some(ParsedToken::WithId {
                id: 12,
                secret: "secret"
            })
        );
        assert_eq!(parse("plain"), Some(ParsedToken::Plain("plain")));
        assert_eq!(parse("x1|secret"), None);
        assert_eq!(parse("12|"), None);
        assert_eq!(parse("|secret"), None);
        assert_eq!(parse("99999999999999999999|s"), None);
    }

    #[test]
    fn abilities() {
        assert!(has_ability(Some(r#"["mobile"]"#), "mobile"));
        assert!(has_ability(Some(r#"["*"]"#), "mobile"));
        assert!(!has_ability(Some(r#"["web"]"#), "mobile"));
        assert!(!has_ability(None, "mobile"));
        assert!(!has_ability(Some("bukan json"), "mobile"));
    }

    #[test]
    fn constant_time_compare() {
        assert!(constant_time_eq("abc", "abc"));
        assert!(!constant_time_eq("abc", "abd"));
        assert!(!constant_time_eq("abc", "ab"));
    }
}
