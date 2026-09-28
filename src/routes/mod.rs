//! HTTP handlers.

pub mod admin;
pub mod auth;
pub mod contact;
pub mod health;
pub mod public;
pub mod seo_routes;

use std::net::{IpAddr, SocketAddr};

use axum::extract::ConnectInfo;
use axum::http::HeaderMap;
use percent_encoding::{NON_ALPHANUMERIC, utf8_percent_encode};

/// Flash message passed between redirects through query parameters
/// (the application is deliberately session-free).
#[derive(Debug, Default, Clone, serde::Deserialize)]
pub struct Flash {
    pub ok: Option<String>,
    pub err: Option<String>,
}

/// Percent-encodes a value for use inside a query string.
pub fn urlencode(value: &str) -> String {
    utf8_percent_encode(value, NON_ALPHANUMERIC).to_string()
}

/// Builds a redirect target carrying a flash message.
pub fn redirect_flash(path: &str, ok: Option<&str>, err: Option<&str>) -> String {
    match (ok, err) {
        (Some(ok), _) => format!("{path}?ok={}", urlencode(ok)),
        (_, Some(err)) => format!("{path}?err={}", urlencode(err)),
        _ => path.to_string(),
    }
}

/// Best-effort client IP for rate limiting: first `X-Forwarded-For` entry
/// when present (reverse-proxy deployments), otherwise the peer address.
pub fn client_ip(headers: &HeaderMap, ConnectInfo(addr): &ConnectInfo<SocketAddr>) -> IpAddr {
    if let Some(forwarded) = headers.get("x-forwarded-for").and_then(|v| v.to_str().ok())
        && let Some(first) = forwarded.split(',').next()
        && let Ok(ip) = first.trim().parse::<IpAddr>()
    {
        return ip;
    }
    addr.ip()
}
