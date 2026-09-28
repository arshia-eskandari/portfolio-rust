//! Routing tests that run without a database: health, robots, the legacy
//! article redirect, admin auth protection, login page, security headers
//! and the 404 fallback.

use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use http_body_util::BodyExt;
use portfolio::app::{AppState, build_router};
use portfolio::config::Config;
use tower::ServiceExt;

fn test_config() -> Config {
    Config {
        database_url: "mongodb://localhost:27017/portfolio".to_string(),
        jwt_secret: "test-secret-test-secret".to_string(),
        s3_bucket: "test-bucket".to_string(),
        aws_region: "us-east-2".to_string(),
        site_base_url: "https://arshiaeskandari.com".to_string(),
        enable_recaptcha: false,
        recaptcha_secret: None,
        recaptcha_site_key: None,
        host: "127.0.0.1".to_string(),
        port: 0,
        production: false,
    }
}

fn test_router() -> Router {
    let s3_config = aws_sdk_s3::Config::builder()
        .behavior_version(aws_sdk_s3::config::BehaviorVersion::latest())
        .region(aws_sdk_s3::config::Region::new("us-east-2"))
        .build();
    let s3 = aws_sdk_s3::Client::from_conf(s3_config);
    build_router(AppState::new(test_config(), s3))
}

async fn body_string(response: axum::response::Response) -> String {
    let bytes = response
        .into_body()
        .collect()
        .await
        .expect("body")
        .to_bytes();
    String::from_utf8_lossy(&bytes).to_string()
}

#[tokio::test]
async fn health_returns_ok() {
    let response = test_router()
        .oneshot(
            Request::get("/health")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::OK);
    let body = body_string(response).await;
    assert!(body.contains("ok"), "unexpected body: {body}");
}

#[tokio::test]
async fn legacy_article_route_redirects_permanently() {
    let response = test_router()
        .oneshot(
            Request::get("/article/my-post-abc123")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(
        response.status(),
        StatusCode::MOVED_PERMANENTLY,
        "must be a 301"
    );
    assert_eq!(
        response
            .headers()
            .get(header::LOCATION)
            .and_then(|v| v.to_str().ok()),
        Some("/articles/my-post-abc123")
    );
}

#[tokio::test]
async fn robots_txt_blocks_admin_and_points_to_sitemap() {
    let response = test_router()
        .oneshot(
            Request::get("/robots.txt")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::OK);
    let body = body_string(response).await;
    assert!(body.contains("Disallow: /admin"));
    assert!(body.contains("Disallow: /login"));
    assert!(body.contains("Sitemap: https://arshiaeskandari.com/sitemap.xml"));
}

#[tokio::test]
async fn admin_without_token_redirects_to_login() {
    for path in ["/admin", "/admin/articles", "/admin/media"] {
        let response = test_router()
            .oneshot(Request::get(path).body(Body::empty()).expect("request"))
            .await
            .expect("response");
        assert!(
            response.status().is_redirection(),
            "{path} must redirect, got {}",
            response.status()
        );
        assert_eq!(
            response
                .headers()
                .get(header::LOCATION)
                .and_then(|v| v.to_str().ok()),
            Some("/login"),
            "{path} must redirect to /login"
        );
    }
}

#[tokio::test]
async fn admin_with_garbage_token_redirects_to_login() {
    let response = test_router()
        .oneshot(
            Request::get("/admin")
                .header(header::COOKIE, "token=not-a-jwt")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert!(response.status().is_redirection());
    assert_eq!(
        response
            .headers()
            .get(header::LOCATION)
            .and_then(|v| v.to_str().ok()),
        Some("/login")
    );
}

#[tokio::test]
async fn admin_with_non_admin_token_redirects_to_login() {
    let token = portfolio::services::auth::create_token(
        "test-secret-test-secret",
        "665f01234567890123abcdef",
        "user@example.com",
        "USER",
    )
    .expect("token");
    let response = test_router()
        .oneshot(
            Request::get("/admin")
                .header(header::COOKIE, format!("token={token}"))
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert!(
        response.status().is_redirection(),
        "non-admin token must be rejected"
    );
}

#[tokio::test]
async fn login_page_renders_and_is_noindex() {
    let response = test_router()
        .oneshot(Request::get("/login").body(Body::empty()).expect("request"))
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::OK);
    let body = body_string(response).await;
    assert!(
        body.contains("noindex"),
        "login page must carry a noindex robots meta"
    );
    assert!(
        body.contains("password"),
        "login page must contain the form"
    );
}

#[tokio::test]
async fn cross_origin_posts_are_rejected() {
    let response = test_router()
        .oneshot(
            Request::post("/login")
                .header(header::ORIGIN, "https://evil.example")
                .header(header::HOST, "arshiaeskandari.com")
                .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
                .body(Body::from("email=a@b.c&password=x"))
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn same_origin_posts_pass_the_origin_check() {
    // Reaches the handler (which then fails on the missing ConnectInfo
    // extension in this harness) — but it must NOT be a 403.
    let response = test_router()
        .oneshot(
            Request::post("/login")
                .header(header::ORIGIN, "https://arshiaeskandari.com")
                .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
                .body(Body::from("email=a@b.c&password=x"))
                .expect("request"),
        )
        .await
        .expect("response");
    assert_ne!(response.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn unknown_path_returns_custom_404() {
    let response = test_router()
        .oneshot(
            Request::get("/definitely-not-a-page")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    let body = body_string(response).await;
    assert!(body.contains("Page not found"), "custom 404 body expected");
}

#[tokio::test]
async fn pages_use_the_svg_brand_favicon() {
    for path in ["/login", "/definitely-not-a-page"] {
        let response = test_router()
            .oneshot(Request::get(path).body(Body::empty()).expect("request"))
            .await
            .expect("response");
        let body = body_string(response).await;
        assert!(
            body.contains(
                r#"<link rel="icon" href="/static/favicon.svg?v=2" type="image/svg+xml">"#
            ),
            "{path} must link the SVG favicon"
        );
    }

    let response = test_router()
        .oneshot(
            Request::get("/static/favicon.svg")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response
            .headers()
            .get(header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok()),
        Some("image/svg+xml")
    );
}

#[tokio::test]
async fn responses_carry_security_headers() {
    let response = test_router()
        .oneshot(
            Request::get("/health")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    let headers = response.headers();
    assert_eq!(
        headers
            .get("x-content-type-options")
            .and_then(|v| v.to_str().ok()),
        Some("nosniff")
    );
    assert_eq!(
        headers.get("x-frame-options").and_then(|v| v.to_str().ok()),
        Some("DENY")
    );
    let csp = headers
        .get("content-security-policy")
        .and_then(|v| v.to_str().ok())
        .expect("CSP present");
    assert!(csp.contains("default-src 'self'"));
    assert!(csp.contains("frame-ancestors 'none'"));
}
