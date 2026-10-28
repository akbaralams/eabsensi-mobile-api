//! Alamat klien. `X-Forwarded-For` hanya dipercaya bila koneksi datang dari `TRUSTED_PROXIES`
//! (nginx/load balancer), seperti `TrustProxies` Laravel.

use std::net::{IpAddr, Ipv4Addr, SocketAddr};

use axum::extract::{ConnectInfo, FromRequestParts};
use axum::http::HeaderMap;
use axum::http::request::Parts;

use crate::config::{IpNet, canonical};
use crate::error::AppError;
use crate::state::AppState;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ClientIp(pub IpAddr);

impl FromRequestParts<AppState> for ClientIp {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        Ok(Self(client_ip(parts, &state.config.trusted_proxies)))
    }
}

pub fn peer_ip(parts: &Parts) -> IpAddr {
    parts
        .extensions
        .get::<ConnectInfo<SocketAddr>>()
        .map_or(IpAddr::V4(Ipv4Addr::LOCALHOST), |c| canonical(c.0.ip()))
}

pub fn client_ip(parts: &Parts, trusted: &[IpNet]) -> IpAddr {
    resolve(peer_ip(parts), &parts.headers, trusted)
}

fn is_trusted(ip: IpAddr, trusted: &[IpNet]) -> bool {
    trusted.iter().any(|net| net.contains(ip))
}

/// Dari kanan ke kiri, lewati proxy tepercaya; alamat pertama yang tidak tepercaya adalah klien.
pub fn resolve(peer: IpAddr, headers: &HeaderMap, trusted: &[IpNet]) -> IpAddr {
    if !is_trusted(peer, trusted) {
        return peer;
    }
    let forwarded: Vec<IpAddr> = headers
        .get_all("x-forwarded-for")
        .iter()
        .filter_map(|v| v.to_str().ok())
        .flat_map(|v| v.split(','))
        .filter_map(|s| s.trim().parse::<IpAddr>().ok())
        .map(canonical)
        .collect();
    forwarded
        .iter()
        .rev()
        .find(|ip| !is_trusted(**ip, trusted))
        .or(forwarded.first())
        .copied()
        .unwrap_or(peer)
}

/// HTTPS di depan proxy tepercaya (untuk header HSTS).
pub fn is_secure(parts: &Parts, trusted: &[IpNet]) -> bool {
    is_trusted(peer_ip(parts), trusted)
        && parts
            .headers
            .get("x-forwarded-proto")
            .and_then(|v| v.to_str().ok())
            .is_some_and(|v| v.trim().eq_ignore_ascii_case("https"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::HeaderValue;

    fn nets(list: &[&str]) -> Vec<IpNet> {
        list.iter().map(|s| s.parse().unwrap()).collect()
    }

    fn xff(value: &str) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert("x-forwarded-for", HeaderValue::from_str(value).unwrap());
        headers
    }

    #[test]
    fn untrusted_peer_header_is_ignored() {
        let ip = resolve(
            "203.0.113.9".parse().unwrap(),
            &xff("1.2.3.4"),
            &nets(&["127.0.0.1"]),
        );
        assert_eq!(ip, "203.0.113.9".parse::<IpAddr>().unwrap());
    }

    #[test]
    fn trusted_proxy_chain() {
        let trusted = nets(&["127.0.0.1", "10.0.0.0/8"]);
        let ip = resolve(
            "127.0.0.1".parse().unwrap(),
            &xff("6.6.6.6, 198.51.100.7, 10.1.1.1"),
            &trusted,
        );
        assert_eq!(ip, "198.51.100.7".parse::<IpAddr>().unwrap());
    }

    #[test]
    fn trusted_proxy_without_header() {
        let ip = resolve(
            "127.0.0.1".parse().unwrap(),
            &HeaderMap::new(),
            &nets(&["127.0.0.1"]),
        );
        assert_eq!(ip, "127.0.0.1".parse::<IpAddr>().unwrap());
    }
}
