//! `robots.txt`, `sitemap.xml` and the Atom feed.

use axum::extract::State;
use axum::http::header;
use axum::response::IntoResponse;

use crate::app::AppState;
use crate::error::AppError;
use crate::services::articles;

pub async fn robots(State(state): State<AppState>) -> impl IntoResponse {
    let base = state.config.site_base_url.trim_end_matches('/');
    let body = format!(
        "User-agent: *\nAllow: /\nDisallow: /admin\nDisallow: /login\n\nSitemap: {base}/sitemap.xml\n"
    );
    ([(header::CONTENT_TYPE, "text/plain; charset=utf-8")], body)
}

pub async fn sitemap(State(state): State<AppState>) -> Result<impl IntoResponse, AppError> {
    let articles = articles::list(&state.caches.articles).await?;
    let xml = crate::seo::sitemap::build(&state.config.site_base_url, &articles);
    Ok((
        [(header::CONTENT_TYPE, "application/xml; charset=utf-8")],
        xml,
    ))
}

pub async fn feed(State(state): State<AppState>) -> Result<impl IntoResponse, AppError> {
    let articles = articles::list(&state.caches.articles).await?;
    let xml = crate::seo::feed::build(&state.config.site_base_url, &articles);
    Ok((
        [(header::CONTENT_TYPE, "application/atom+xml; charset=utf-8")],
        xml,
    ))
}
