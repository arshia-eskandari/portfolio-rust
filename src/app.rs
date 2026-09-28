//! Application state and router assembly.

use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use axum::Router;
use axum::extract::DefaultBodyLimit;
use axum::routing::{get, post};
use tower_http::compression::CompressionLayer;
use tower_http::services::ServeDir;
use tower_http::set_header::SetResponseHeaderLayer;
use tower_http::trace::TraceLayer;

use crate::config::Config;
use crate::models::Article;
use crate::services::articles::ARTICLES_CACHE_TTL;
use crate::services::cache::TtlCache;
use crate::services::portfolio::{HomeData, PUBLIC_CACHE_TTL};

/// Caches for public read-mostly data, invalidated by admin mutations.
pub struct Caches {
    pub home: TtlCache<HomeData>,
    pub articles: TtlCache<Vec<Article>>,
}

impl Caches {
    pub fn new() -> Self {
        Self {
            home: TtlCache::new(PUBLIC_CACHE_TTL),
            articles: TtlCache::new(ARTICLES_CACHE_TTL),
        }
    }

    /// Drops all public caches; called after any admin write.
    pub fn invalidate_public(&self) {
        self.home.invalidate();
        self.articles.invalidate();
    }
}

impl Default for Caches {
    fn default() -> Self {
        Self::new()
    }
}

/// Small fixed-window rate limiter keyed by client IP.
pub struct RateLimiter {
    hits: Mutex<HashMap<IpAddr, Vec<Instant>>>,
    max: usize,
    window: Duration,
}

impl RateLimiter {
    pub fn new(max: usize, window: Duration) -> Self {
        Self {
            hits: Mutex::new(HashMap::new()),
            max,
            window,
        }
    }

    /// Records a hit and returns `true` when the client is within limits.
    pub fn check(&self, ip: IpAddr) -> bool {
        let now = Instant::now();
        let mut hits = self.hits.lock().expect("rate limiter lock poisoned");
        // Opportunistic global prune so the map cannot grow unbounded.
        if hits.len() > 4096 {
            hits.retain(|_, stamps| stamps.iter().any(|t| now.duration_since(*t) < self.window));
        }
        let stamps = hits.entry(ip).or_default();
        stamps.retain(|t| now.duration_since(*t) < self.window);
        if stamps.len() >= self.max {
            return false;
        }
        stamps.push(now);
        true
    }
}

pub struct Limiters {
    pub contact: RateLimiter,
    pub login: RateLimiter,
}

impl Limiters {
    pub fn new() -> Self {
        Self {
            contact: RateLimiter::new(5, Duration::from_secs(600)),
            login: RateLimiter::new(10, Duration::from_secs(600)),
        }
    }
}

impl Default for Limiters {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone)]
pub struct AppState {
    pub config: Arc<Config>,
    pub s3: aws_sdk_s3::Client,
    pub http: reqwest::Client,
    pub caches: Arc<Caches>,
    pub limiters: Arc<Limiters>,
}

impl AppState {
    pub fn new(config: Config, s3: aws_sdk_s3::Client) -> Self {
        Self {
            config: Arc::new(config),
            s3,
            http: reqwest::Client::new(),
            caches: Arc::new(Caches::new()),
            limiters: Arc::new(Limiters::new()),
        }
    }
}

/// Builds the complete application router.
pub fn build_router(state: AppState) -> Router {
    let admin_routes = Router::new()
        .route("/", get(crate::routes::admin::dashboard::page))
        .route(
            "/hero",
            get(crate::routes::admin::hero::page).post(crate::routes::admin::hero::update),
        )
        .route(
            "/about",
            get(crate::routes::admin::about::page).post(crate::routes::admin::about::update),
        )
        .route(
            "/skills",
            get(crate::routes::admin::skills::page).post(crate::routes::admin::skills::update),
        )
        .route(
            "/socials",
            get(crate::routes::admin::socials::page).post(crate::routes::admin::socials::update),
        )
        .route(
            "/contacts",
            get(crate::routes::admin::contacts::page).post(crate::routes::admin::contacts::update),
        )
        .route("/projects", get(crate::routes::admin::projects::page))
        .route(
            "/projects/create",
            post(crate::routes::admin::projects::create),
        )
        .route(
            "/projects/{id}",
            post(crate::routes::admin::projects::update),
        )
        .route(
            "/projects/{id}/delete",
            post(crate::routes::admin::projects::delete),
        )
        .route("/experiences", get(crate::routes::admin::experiences::page))
        .route(
            "/experiences/create",
            post(crate::routes::admin::experiences::create),
        )
        .route(
            "/experiences/{id}",
            post(crate::routes::admin::experiences::update),
        )
        .route(
            "/experiences/{id}/delete",
            post(crate::routes::admin::experiences::delete),
        )
        .route("/articles", get(crate::routes::admin::articles::page))
        .route(
            "/articles/create",
            post(crate::routes::admin::articles::create),
        )
        .route(
            "/articles/{id}",
            get(crate::routes::admin::articles::edit_page)
                .post(crate::routes::admin::articles::update),
        )
        .route(
            "/articles/{id}/delete",
            post(crate::routes::admin::articles::delete),
        )
        .layer(DefaultBodyLimit::max(12 * 1024 * 1024))
        .route("/media", get(crate::routes::admin::media::page))
        .route(
            "/media/upload",
            post(crate::routes::admin::media::upload)
                .layer(DefaultBodyLimit::max(60 * 1024 * 1024)),
        )
        .route("/media/delete", post(crate::routes::admin::media::delete))
        .route_layer(axum::middleware::from_fn_with_state(
            state.clone(),
            crate::middleware::auth::require_admin,
        ));

    Router::new()
        .route("/", get(crate::routes::public::home))
        .route("/articles", get(crate::routes::public::articles_index))
        .route("/articles/{slug}", get(crate::routes::public::article_page))
        .route(
            "/article/{slug}",
            get(crate::routes::public::legacy_article_redirect),
        )
        .route("/contact", post(crate::routes::contact::submit))
        .route("/robots.txt", get(crate::routes::seo_routes::robots))
        .route("/sitemap.xml", get(crate::routes::seo_routes::sitemap))
        .route("/feed.xml", get(crate::routes::seo_routes::feed))
        .route("/health", get(crate::routes::health::health))
        .route(
            "/login",
            get(crate::routes::auth::login_page).post(crate::routes::auth::login_submit),
        )
        .route("/logout", post(crate::routes::auth::logout))
        .nest("/admin", admin_routes)
        .nest_service(
            "/static",
            tower::ServiceBuilder::new()
                .layer(SetResponseHeaderLayer::if_not_present(
                    axum::http::header::CACHE_CONTROL,
                    axum::http::HeaderValue::from_static("public, max-age=86400"),
                ))
                .service(
                    ServeDir::new("static")
                        .precompressed_gzip()
                        .precompressed_br(),
                ),
        )
        .fallback(crate::routes::public::not_found)
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            crate::middleware::security::security_layer,
        ))
        .layer(CompressionLayer::new())
        .layer(TraceLayer::new_for_http())
        .layer(DefaultBodyLimit::max(2 * 1024 * 1024))
        .with_state(state)
}
