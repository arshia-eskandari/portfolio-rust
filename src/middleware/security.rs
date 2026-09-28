//! Security middleware: response headers and cross-origin POST rejection.

use axum::extract::{Request, State};
use axum::http::{HeaderValue, Method, StatusCode, header};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};

use crate::app::AppState;

/// Builds the Content Security Policy.
///
/// All first-party: scripts, styles and fonts are served from `/static`.
/// Images and media may come from the S3 bucket (any https host, since
/// media URLs are stored absolute). When reCAPTCHA is enabled the Google
/// script/frame hosts are added — only then.
fn content_security_policy(recaptcha: bool) -> String {
    let script_src = if recaptcha {
        "script-src 'self' https://www.google.com https://www.gstatic.com"
    } else {
        "script-src 'self'"
    };
    let frame_src = if recaptcha {
        "frame-src https://www.google.com"
    } else {
        "frame-src 'none'"
    };
    format!(
        "default-src 'self'; {script_src}; style-src 'self'; img-src 'self' https: data:; \
         media-src 'self' https:; object-src 'none'; base-uri 'self'; form-action 'self'; \
         frame-ancestors 'none'; {frame_src}; connect-src 'self'"
    )
}

/// Checks that a state-changing request does not originate from another
/// site. Browsers attach an `Origin` header to every cross-origin (and
/// same-origin `fetch`/form) POST; if one is present it must match the
/// request's own host. Combined with `SameSite=Strict` auth cookies this
/// blocks CSRF without per-form tokens.
fn origin_allowed(request: &Request, site_base_url: &str) -> bool {
    let Some(origin) = request.headers().get(header::ORIGIN) else {
        return true; // non-browser clients (no CSRF vector without cookies)
    };
    let Ok(origin) = origin.to_str() else {
        return false;
    };
    if origin == "null" {
        return false;
    }

    let origin_host = origin
        .split_once("://")
        .map(|(_, rest)| rest)
        .unwrap_or(origin);
    let origin_host = origin_host.split('/').next().unwrap_or(origin_host);

    // Accept the configured public origin…
    let base_host = site_base_url
        .split_once("://")
        .map(|(_, rest)| rest)
        .unwrap_or(site_base_url)
        .split('/')
        .next()
        .unwrap_or("");
    if origin_host.eq_ignore_ascii_case(base_host) {
        return true;
    }

    // …or the host the request itself was addressed to (local development,
    // deployments behind another hostname).
    if let Some(request_host) = request
        .headers()
        .get(header::HOST)
        .and_then(|h| h.to_str().ok())
    {
        return origin_host.eq_ignore_ascii_case(request_host);
    }
    false
}

pub async fn security_layer(
    State(state): State<AppState>,
    request: Request,
    next: Next,
) -> Response {
    let mutating = matches!(
        *request.method(),
        Method::POST | Method::PUT | Method::PATCH | Method::DELETE
    );
    if mutating && !origin_allowed(&request, &state.config.site_base_url) {
        tracing::warn!("rejected cross-origin mutating request");
        return (StatusCode::FORBIDDEN, "Cross-origin request rejected").into_response();
    }

    let mut response = next.run(request).await;

    let headers = response.headers_mut();
    headers.insert(
        header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    );
    headers.insert(
        header::REFERRER_POLICY,
        HeaderValue::from_static("strict-origin-when-cross-origin"),
    );
    headers.insert(header::X_FRAME_OPTIONS, HeaderValue::from_static("DENY"));
    if let Ok(csp) = HeaderValue::from_str(&content_security_policy(state.config.enable_recaptcha))
    {
        headers.insert(header::CONTENT_SECURITY_POLICY, csp);
    }
    if state.config.production {
        headers.insert(
            header::STRICT_TRANSPORT_SECURITY,
            HeaderValue::from_static("max-age=31536000; includeSubDomains"),
        );
    }
    response
}
