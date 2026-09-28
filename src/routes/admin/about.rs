use askama::Template;
use askama_web::WebTemplate;
use axum::Form;
use axum::extract::{Query, State};
use axum::response::Redirect;
use serde::Deserialize;

use crate::app::AppState;
use crate::error::AppError;
use crate::models::MediaType;
use crate::repositories;
use crate::routes::Flash;
use crate::routes::admin::{admin_meta, redirect_err, redirect_ok};
use crate::seo::metadata::PageMeta;

pub struct MediaOption {
    pub url: String,
    pub name: String,
}

#[derive(Template, WebTemplate)]
#[template(path = "admin/about.html")]
pub struct AboutAdminTemplate {
    pub meta: PageMeta,
    pub flash: Flash,
    pub title: String,
    pub text: String,
    pub resume_url: String,
    pub image_url: String,
    pub pdf_options: Vec<MediaOption>,
    pub image_options: Vec<MediaOption>,
}

pub async fn page(Query(flash): Query<Flash>) -> Result<AboutAdminTemplate, AppError> {
    let about = repositories::about::find().await?;
    let media = repositories::media::all().await?;

    let option = |m: &crate::models::Media| MediaOption {
        url: m.url.clone(),
        name: m.name.clone(),
    };
    let pdf_options = media
        .iter()
        .filter(|m| m.media_type == MediaType::Pdf)
        .map(option)
        .collect();
    let image_options = media
        .iter()
        .filter(|m| m.media_type == MediaType::Image)
        .map(option)
        .collect();

    Ok(AboutAdminTemplate {
        meta: admin_meta("About"),
        flash,
        title: about
            .as_ref()
            .and_then(|a| a.title.clone())
            .unwrap_or_default(),
        text: about.as_ref().map(|a| a.text.clone()).unwrap_or_default(),
        resume_url: about
            .as_ref()
            .and_then(|a| a.resume_url.clone())
            .unwrap_or_default(),
        image_url: about.and_then(|a| a.image_url).unwrap_or_default(),
        pdf_options,
        image_options,
    })
}

#[derive(Debug, Deserialize)]
pub struct AboutForm {
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub text: String,
    #[serde(default)]
    pub resume_url: String,
    #[serde(default)]
    pub image_url: String,
}

pub async fn update(
    State(state): State<AppState>,
    Form(form): Form<AboutForm>,
) -> Result<Redirect, AppError> {
    let title = form.title.trim();
    let text = form.text.trim();
    let resume_url = form.resume_url.trim();
    let image_url = form.image_url.trim();

    if title.is_empty() || title.chars().count() > 150 {
        return Ok(redirect_err(
            "/admin/about",
            "Title must be 1 to 150 characters.",
        ));
    }
    if text.is_empty() || text.chars().count() > 1500 {
        return Ok(redirect_err(
            "/admin/about",
            "Text must be 1 to 1500 characters.",
        ));
    }
    // References must point at managed media, like the previous app enforced
    // for the resume.
    if !resume_url.is_empty() && !repositories::media::exists_by_url(resume_url).await? {
        return Ok(redirect_err(
            "/admin/about",
            "Resume must be an uploaded media file.",
        ));
    }
    if !image_url.is_empty() && !repositories::media::exists_by_url(image_url).await? {
        return Ok(redirect_err(
            "/admin/about",
            "Image must be an uploaded media file.",
        ));
    }

    repositories::about::upsert(
        title,
        text,
        (!resume_url.is_empty()).then(|| resume_url.to_string()),
        (!image_url.is_empty()).then(|| image_url.to_string()),
    )
    .await?;
    state.caches.invalidate_public();
    Ok(redirect_ok("/admin/about", "About updated."))
}
