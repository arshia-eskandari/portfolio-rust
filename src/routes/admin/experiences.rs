use askama::Template;
use askama_web::WebTemplate;
use axum::Form;
use axum::extract::{Path, Query, State};
use axum::response::Redirect;
use serde::Deserialize;

use crate::app::AppState;
use crate::error::AppError;
use crate::repositories;
use crate::routes::Flash;
use crate::routes::admin::{
    SENTENCE_SEPARATOR, admin_meta, csv_list, date_input_value, parse_date, parse_object_id,
    redirect_err, redirect_ok, sentence_list,
};
use crate::seo::metadata::PageMeta;

pub struct ExperienceFormView {
    pub id: String,
    pub job_title: String,
    pub company: String,
    pub location: String,
    pub start_date: String,
    pub end_date: String,
    pub achievements: String,
    pub responsibilities: String,
    pub recommendation_letter_urls: String,
}

#[derive(Template, WebTemplate)]
#[template(path = "admin/experiences.html")]
pub struct ExperiencesAdminTemplate {
    pub meta: PageMeta,
    pub flash: Flash,
    pub experiences: Vec<ExperienceFormView>,
}

pub async fn page(Query(flash): Query<Flash>) -> Result<ExperiencesAdminTemplate, AppError> {
    let experiences = repositories::experience::all_sorted().await?;
    let views = experiences
        .into_iter()
        .map(|e| ExperienceFormView {
            id: e._id.map(|id| id.to_hex()).unwrap_or_default(),
            job_title: e.job_title,
            company: e.company,
            location: e.location,
            start_date: date_input_value(e.start_date),
            end_date: e.end_date.map(date_input_value).unwrap_or_default(),
            achievements: e.achievements.join(SENTENCE_SEPARATOR),
            responsibilities: e.responsibilities.join(SENTENCE_SEPARATOR),
            recommendation_letter_urls: e.recommendation_letter_urls.join(","),
        })
        .collect();

    Ok(ExperiencesAdminTemplate {
        meta: admin_meta("Experiences"),
        flash,
        experiences: views,
    })
}

pub async fn create(State(state): State<AppState>) -> Result<Redirect, AppError> {
    repositories::experience::create_default().await?;
    state.caches.invalidate_public();
    Ok(redirect_ok(
        "/admin/experiences",
        "Placeholder experience created.",
    ))
}

#[derive(Debug, Deserialize)]
pub struct ExperienceForm {
    #[serde(default)]
    pub job_title: String,
    #[serde(default)]
    pub company: String,
    #[serde(default)]
    pub location: String,
    #[serde(default)]
    pub start_date: String,
    #[serde(default)]
    pub end_date: String,
    #[serde(default)]
    pub achievements: String,
    #[serde(default)]
    pub responsibilities: String,
    #[serde(default)]
    pub recommendation_letter_urls: String,
}

pub async fn update(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Form(form): Form<ExperienceForm>,
) -> Result<Redirect, AppError> {
    let id = parse_object_id(&id)?;

    let job_title = form.job_title.trim();
    let company = form.company.trim();
    let location = form.location.trim();
    if job_title.is_empty() || job_title.chars().count() > 100 {
        return Ok(redirect_err(
            "/admin/experiences",
            "Job title must be 1 to 100 characters.",
        ));
    }
    if company.is_empty() || company.chars().count() > 100 {
        return Ok(redirect_err(
            "/admin/experiences",
            "Company must be 1 to 100 characters.",
        ));
    }
    if location.is_empty() || location.chars().count() > 100 {
        return Ok(redirect_err(
            "/admin/experiences",
            "Location must be 1 to 100 characters.",
        ));
    }

    let Some(start_date) = parse_date(&form.start_date) else {
        return Ok(redirect_err(
            "/admin/experiences",
            "Start date is required (YYYY-MM-DD).",
        ));
    };
    let end_date = if form.end_date.trim().is_empty() {
        None
    } else {
        match parse_date(&form.end_date) {
            Some(date) => Some(date),
            None => {
                return Ok(redirect_err(
                    "/admin/experiences",
                    "End date must be YYYY-MM-DD.",
                ));
            }
        }
    };

    let letters = csv_list(&form.recommendation_letter_urls);
    for url in &letters {
        if !repositories::media::exists_by_url(url).await? {
            return Ok(redirect_err(
                "/admin/experiences",
                "Recommendation letters must reference uploaded media files.",
            ));
        }
    }

    let found = repositories::experience::update(
        id,
        job_title,
        company,
        location,
        start_date,
        end_date,
        sentence_list(&form.achievements),
        sentence_list(&form.responsibilities),
        letters,
    )
    .await?;
    if !found {
        return Ok(redirect_err("/admin/experiences", "Experience not found."));
    }

    state.caches.invalidate_public();
    Ok(redirect_ok("/admin/experiences", "Experience updated."))
}

pub async fn delete(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Redirect, AppError> {
    let id = parse_object_id(&id)?;
    repositories::experience::delete(id).await?;
    state.caches.invalidate_public();
    Ok(redirect_ok("/admin/experiences", "Experience deleted."))
}
