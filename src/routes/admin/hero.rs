use askama::Template;
use askama_web::WebTemplate;
use axum::Form;
use axum::extract::{Query, State};
use axum::response::Redirect;
use serde::Deserialize;

use crate::app::AppState;
use crate::error::AppError;
use crate::repositories;
use crate::routes::Flash;
use crate::routes::admin::{admin_meta, redirect_err, redirect_ok};
use crate::seo::metadata::PageMeta;

#[derive(Template, WebTemplate)]
#[template(path = "admin/hero.html")]
pub struct HeroAdminTemplate {
    pub meta: PageMeta,
    pub flash: Flash,
    pub title: String,
    pub text: String,
}

pub async fn page(Query(flash): Query<Flash>) -> Result<HeroAdminTemplate, AppError> {
    let hero = repositories::hero::find().await?;
    Ok(HeroAdminTemplate {
        meta: admin_meta("Hero"),
        flash,
        title: hero
            .as_ref()
            .and_then(|h| h.title.clone())
            .unwrap_or_default(),
        text: hero.and_then(|h| h.text).unwrap_or_default(),
    })
}

#[derive(Debug, Deserialize)]
pub struct HeroForm {
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub text: String,
}

pub async fn update(
    State(state): State<AppState>,
    Form(form): Form<HeroForm>,
) -> Result<Redirect, AppError> {
    let title = form.title.trim();
    let text = form.text.trim();
    if title.is_empty() || title.chars().count() > 150 {
        return Ok(redirect_err(
            "/admin/hero",
            "Title must be 1 to 150 characters.",
        ));
    }
    if text.is_empty() || text.chars().count() > 1000 {
        return Ok(redirect_err(
            "/admin/hero",
            "Text must be 1 to 1000 characters.",
        ));
    }

    repositories::hero::upsert(title, text).await?;
    state.caches.invalidate_public();
    Ok(redirect_ok("/admin/hero", "Hero updated."))
}
