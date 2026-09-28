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
use crate::routes::admin::{admin_meta, csv_list, redirect_err, redirect_ok};
use crate::seo::metadata::PageMeta;

#[derive(Template, WebTemplate)]
#[template(path = "admin/skills.html")]
pub struct SkillsAdminTemplate {
    pub meta: PageMeta,
    pub flash: Flash,
    pub skills_csv: String,
}

pub async fn page(Query(flash): Query<Flash>) -> Result<SkillsAdminTemplate, AppError> {
    let skills = repositories::skills::find().await?;
    Ok(SkillsAdminTemplate {
        meta: admin_meta("Skills"),
        flash,
        skills_csv: skills.map(|s| s.skills.join(", ")).unwrap_or_default(),
    })
}

#[derive(Debug, Deserialize)]
pub struct SkillsForm {
    #[serde(default)]
    pub skills: String,
}

pub async fn update(
    State(state): State<AppState>,
    Form(form): Form<SkillsForm>,
) -> Result<Redirect, AppError> {
    let skills = csv_list(&form.skills);
    if skills.is_empty() {
        return Ok(redirect_err("/admin/skills", "Enter at least one skill."));
    }
    if form.skills.chars().count() > 2000 {
        return Ok(redirect_err("/admin/skills", "Skills list is too long."));
    }

    repositories::skills::upsert(skills).await?;
    state.caches.invalidate_public();
    Ok(redirect_ok("/admin/skills", "Skills updated."))
}
