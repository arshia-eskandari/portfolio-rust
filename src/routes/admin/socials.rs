use askama::Template;
use askama_web::WebTemplate;
use axum::Form;
use axum::extract::{Query, State};
use axum::response::Redirect;
use serde::Deserialize;

use crate::app::AppState;
use crate::error::AppError;
use crate::models::SocialName;
use crate::repositories;
use crate::routes::Flash;
use crate::routes::admin::{admin_meta, redirect_err, redirect_ok};
use crate::seo::metadata::PageMeta;
use crate::services::portfolio::SocialLinks;

#[derive(Template, WebTemplate)]
#[template(path = "admin/socials.html")]
pub struct SocialsAdminTemplate {
    pub meta: PageMeta,
    pub flash: Flash,
    pub linkedin: String,
    pub github: String,
    pub telegram: String,
    pub email: String,
}

pub async fn page(Query(flash): Query<Flash>) -> Result<SocialsAdminTemplate, AppError> {
    let socials = repositories::social::all().await?;
    let links = SocialLinks::from_socials(&socials);
    Ok(SocialsAdminTemplate {
        meta: admin_meta("Social Links"),
        flash,
        linkedin: links.linkedin.unwrap_or_default(),
        github: links.github.unwrap_or_default(),
        telegram: links.telegram.unwrap_or_default(),
        email: links.email.unwrap_or_default(),
    })
}

#[derive(Debug, Deserialize)]
pub struct SocialsForm {
    #[serde(default)]
    pub linkedin: String,
    #[serde(default)]
    pub github: String,
    #[serde(default)]
    pub telegram: String,
    #[serde(default)]
    pub email: String,
}

pub async fn update(
    State(state): State<AppState>,
    Form(form): Form<SocialsForm>,
) -> Result<Redirect, AppError> {
    let entries = [
        (SocialName::Linkedin, form.linkedin.trim()),
        (SocialName::Github, form.github.trim()),
        (SocialName::Telegram, form.telegram.trim()),
        (SocialName::Email, form.email.trim()),
    ];

    for (_, value) in &entries {
        if value.chars().count() > 200 {
            return Ok(redirect_err(
                "/admin/socials",
                "Links must be at most 200 characters.",
            ));
        }
    }

    for (name, value) in entries {
        // Empty inputs leave the stored value untouched, matching the
        // previous admin behavior (it only wrote submitted keys).
        if !value.is_empty() {
            repositories::social::upsert(name, value).await?;
        }
    }

    state.caches.invalidate_public();
    Ok(redirect_ok("/admin/socials", "Social links updated."))
}
