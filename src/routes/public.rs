//! Public site pages: homepage, articles index, article detail, legacy
//! redirect and the 404 fallback.

use std::sync::Arc;

use askama::Template;
use askama_web::WebTemplate;
use axum::extract::{Path, Query, State};
use axum::http::{StatusCode, header};
use axum::response::{IntoResponse, Response};
use serde::Deserialize;

use crate::app::AppState;
use crate::error::{AppError, NotFoundTemplate};
use crate::seo::metadata::{PageMeta, absolute_url};
use crate::seo::structured_data;
use crate::services::articles::{self, ArticlePageView};
use crate::services::portfolio::{self, ArticleCardView, HomeData};

#[derive(Template, WebTemplate)]
#[template(path = "home.html")]
pub struct HomeTemplate {
    pub meta: PageMeta,
    pub data: Arc<HomeData>,
    pub flash: crate::routes::Flash,
    pub recaptcha_site_key: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct HomeQuery {
    #[serde(default)]
    pub ok: Option<String>,
    #[serde(default)]
    pub err: Option<String>,
}

pub async fn home(
    State(state): State<AppState>,
    Query(query): Query<HomeQuery>,
) -> Result<HomeTemplate, AppError> {
    let data = portfolio::home_data(&state.caches.home).await?;
    let base = &state.config.site_base_url;

    let meta = PageMeta::page(
        "Arshia Eskandari — Software Engineer",
        data.meta_description.clone(),
        absolute_url(base, "/"),
    )
    .with_image(data.about.image_url.clone())
    .with_json_ld(structured_data::person(
        base,
        data.about.image_url.as_deref(),
        &data.socials,
    ))
    .with_json_ld(structured_data::website(base));

    Ok(HomeTemplate {
        meta,
        data,
        flash: crate::routes::Flash {
            ok: query.ok,
            err: query.err,
        },
        recaptcha_site_key: state
            .config
            .enable_recaptcha
            .then(|| state.config.recaptcha_site_key.clone())
            .flatten(),
    })
}

#[derive(Template, WebTemplate)]
#[template(path = "articles.html")]
pub struct ArticlesTemplate {
    pub meta: PageMeta,
    pub cards: Vec<ArticleCardView>,
    pub query: String,
}

#[derive(Debug, Deserialize)]
pub struct ArticlesQuery {
    #[serde(default)]
    pub q: String,
}

pub async fn articles_index(
    State(state): State<AppState>,
    Query(query): Query<ArticlesQuery>,
) -> Result<ArticlesTemplate, AppError> {
    let articles = articles::list(&state.caches.articles).await?;
    let cards = articles::filter_cards(&articles, &query.q);

    let meta = PageMeta::page(
        "Articles — Arshia Eskandari",
        "Articles on Rust, backend engineering and software development by Arshia Eskandari.",
        absolute_url(&state.config.site_base_url, "/articles"),
    );

    Ok(ArticlesTemplate {
        meta,
        cards,
        query: query.q,
    })
}

#[derive(Template, WebTemplate)]
#[template(path = "article.html")]
pub struct ArticleTemplate {
    pub meta: PageMeta,
    pub article: ArticlePageView,
}

pub async fn article_page(
    State(state): State<AppState>,
    Path(slug): Path<String>,
) -> Result<ArticleTemplate, AppError> {
    let Some(article) = articles::page_by_slug(&slug).await? else {
        return Err(AppError::NotFound);
    };
    let base = &state.config.site_base_url;

    let mut meta = PageMeta::page(
        format!("{} — Arshia Eskandari", article.title),
        article.description.clone(),
        absolute_url(base, &format!("/articles/{}", article.slug)),
    )
    .with_image(article.banner.clone())
    .with_json_ld(structured_data::blog_posting(base, &article))
    .with_json_ld(structured_data::article_breadcrumbs(
        base,
        &article.title,
        &article.slug,
    ));
    meta.og_type = "article";
    meta.published_iso = Some(article.published_iso.clone());
    meta.modified_iso = Some(article.updated_iso.clone());
    meta.tags = article.tags.clone();

    Ok(ArticleTemplate { meta, article })
}

/// `/article/:slug` → `/articles/:slug`, preserved as a permanent 301
/// (the previous deployment issued this redirect through vercel.json).
pub async fn legacy_article_redirect(Path(slug): Path<String>) -> Response {
    let location = format!("/articles/{slug}");
    match header::HeaderValue::from_str(&location) {
        Ok(value) => (StatusCode::MOVED_PERMANENTLY, [(header::LOCATION, value)]).into_response(),
        Err(_) => StatusCode::BAD_REQUEST.into_response(),
    }
}

pub async fn not_found() -> Response {
    (StatusCode::NOT_FOUND, NotFoundTemplate).into_response()
}
