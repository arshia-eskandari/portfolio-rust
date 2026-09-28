use askama::Template;
use askama_web::WebTemplate;
use axum::Form;
use axum::extract::{Path, Query, State};
use axum::response::Redirect;
use serde::Deserialize;

use crate::app::AppState;
use crate::error::AppError;
use crate::models::MediaType;
use crate::repositories;
use crate::routes::Flash;
use crate::routes::admin::about::MediaOption;
use crate::routes::admin::{admin_meta, csv_list, parse_object_id, redirect_err, redirect_ok};
use crate::seo::metadata::PageMeta;
use crate::services::format;

pub struct ArticleRow {
    pub id: String,
    pub title: String,
    pub slug: String,
    pub created: String,
    pub updated: String,
}

#[derive(Template, WebTemplate)]
#[template(path = "admin/articles.html")]
pub struct ArticlesAdminTemplate {
    pub meta: PageMeta,
    pub flash: Flash,
    pub articles: Vec<ArticleRow>,
}

pub async fn page(Query(flash): Query<Flash>) -> Result<ArticlesAdminTemplate, AppError> {
    let articles = repositories::article::all_sorted().await?;
    let rows = articles
        .into_iter()
        .map(|a| ArticleRow {
            id: a._id.map(|id| id.to_hex()).unwrap_or_default(),
            title: a.title,
            slug: a.slug,
            created: format::iso_date(a.created_at),
            updated: format::iso_date(a.updated_at),
        })
        .collect();

    Ok(ArticlesAdminTemplate {
        meta: admin_meta("Articles"),
        flash,
        articles: rows,
    })
}

pub async fn create(State(state): State<AppState>) -> Result<Redirect, AppError> {
    let id = repositories::article::create_default().await?;
    state.caches.invalidate_public();
    Ok(redirect_ok(
        &format!("/admin/articles/{}", id.to_hex()),
        "Draft article created.",
    ))
}

#[derive(Template, WebTemplate)]
#[template(path = "admin/article_edit.html")]
pub struct ArticleEditTemplate {
    pub meta: PageMeta,
    pub flash: Flash,
    pub id: String,
    pub title: String,
    pub content: String,
    pub banner: String,
    pub tags: String,
    pub slug: String,
    pub image_options: Vec<MediaOption>,
}

pub async fn edit_page(
    Path(id): Path<String>,
    Query(flash): Query<Flash>,
) -> Result<ArticleEditTemplate, AppError> {
    let oid = parse_object_id(&id)?;
    let Some(article) = repositories::article::find_by_id(oid).await? else {
        return Err(AppError::NotFound);
    };
    let media = repositories::media::all().await?;

    Ok(ArticleEditTemplate {
        meta: admin_meta("Edit Article"),
        flash,
        id,
        title: article.title,
        content: article.content,
        banner: article.banner.unwrap_or_default(),
        tags: article.tags.join(", "),
        slug: article.slug,
        image_options: media
            .into_iter()
            .filter(|m| m.media_type == MediaType::Image)
            .map(|m| MediaOption {
                url: m.url,
                name: m.name,
            })
            .collect(),
    })
}

#[derive(Debug, Deserialize)]
pub struct ArticleForm {
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub content: String,
    #[serde(default)]
    pub banner: String,
    #[serde(default)]
    pub tags: String,
}

pub async fn update(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Form(form): Form<ArticleForm>,
) -> Result<Redirect, AppError> {
    let oid = parse_object_id(&id)?;
    let back = format!("/admin/articles/{id}");

    let title = form.title.trim();
    if title.is_empty() || title.chars().count() > 300 {
        return Ok(redirect_err(&back, "Title must be 1 to 300 characters."));
    }
    if form.content.chars().count() > 10_000_000 {
        return Ok(redirect_err(&back, "Content is too large."));
    }
    let banner = form.banner.trim();
    if !banner.is_empty() && !repositories::media::exists_by_url(banner).await? {
        return Ok(redirect_err(
            &back,
            "Banner must be an uploaded media file.",
        ));
    }

    let updated = repositories::article::update(
        oid,
        title,
        &form.content,
        (!banner.is_empty()).then(|| banner.to_string()),
        csv_list(&form.tags),
    )
    .await?;
    if updated.is_none() {
        return Ok(redirect_err("/admin/articles", "Article not found."));
    }

    state.caches.invalidate_public();
    Ok(redirect_ok(&back, "Article updated."))
}

pub async fn delete(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Redirect, AppError> {
    let oid = parse_object_id(&id)?;
    repositories::article::delete(oid).await?;
    state.caches.invalidate_public();
    Ok(redirect_ok("/admin/articles", "Article deleted."))
}
