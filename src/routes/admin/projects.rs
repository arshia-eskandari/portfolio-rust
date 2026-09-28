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
    SENTENCE_SEPARATOR, admin_meta, csv_list, parse_object_id, redirect_err, redirect_ok,
    sentence_list,
};
use crate::seo::metadata::PageMeta;

pub struct ProjectFormView {
    pub id: String,
    pub project_title: String,
    pub objective: String,
    pub urls: String,
    pub url_titles: String,
    pub technologies: String,
    pub key_results: String,
    pub media: String,
    pub experience_id: String,
    pub order: i64,
}

pub struct ExperienceOption {
    pub id: String,
    pub label: String,
}

#[derive(Template, WebTemplate)]
#[template(path = "admin/projects.html")]
pub struct ProjectsAdminTemplate {
    pub meta: PageMeta,
    pub flash: Flash,
    pub projects: Vec<ProjectFormView>,
    pub experience_options: Vec<ExperienceOption>,
    pub media_urls: Vec<String>,
}

pub async fn page(Query(flash): Query<Flash>) -> Result<ProjectsAdminTemplate, AppError> {
    let projects = repositories::project::all_ordered().await?;
    let experiences = repositories::experience::all_sorted().await?;
    let media = repositories::media::all().await?;

    let views = projects
        .into_iter()
        .map(|p| ProjectFormView {
            id: p._id.map(|id| id.to_hex()).unwrap_or_default(),
            project_title: p.project_title,
            objective: p.objective,
            urls: p.urls.join(","),
            url_titles: p.url_titles.join(","),
            technologies: p.project_technologies.join(","),
            key_results: p.key_results.join(SENTENCE_SEPARATOR),
            media: p.media.join(","),
            experience_id: p.experience_id.unwrap_or_default(),
            order: p.order,
        })
        .collect();

    let experience_options = experiences
        .into_iter()
        .filter_map(|e| {
            e._id.map(|id| ExperienceOption {
                id: id.to_hex(),
                label: format!("{} — {}", e.job_title, e.company),
            })
        })
        .collect();

    Ok(ProjectsAdminTemplate {
        meta: admin_meta("Projects"),
        flash,
        projects: views,
        experience_options,
        media_urls: media.into_iter().map(|m| m.url).collect(),
    })
}

pub async fn create(State(state): State<AppState>) -> Result<Redirect, AppError> {
    repositories::project::create_default().await?;
    state.caches.invalidate_public();
    Ok(redirect_ok(
        "/admin/projects",
        "Placeholder project created.",
    ))
}

#[derive(Debug, Deserialize)]
pub struct ProjectForm {
    #[serde(default)]
    pub project_title: String,
    #[serde(default)]
    pub objective: String,
    #[serde(default)]
    pub urls: String,
    #[serde(default)]
    pub url_titles: String,
    #[serde(default)]
    pub technologies: String,
    #[serde(default)]
    pub key_results: String,
    #[serde(default)]
    pub media: String,
    #[serde(default)]
    pub experience_id: String,
    #[serde(default)]
    pub order: String,
}

pub async fn update(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Form(form): Form<ProjectForm>,
) -> Result<Redirect, AppError> {
    let id = parse_object_id(&id)?;

    let title = form.project_title.trim();
    if title.is_empty() || title.chars().count() > 100 {
        return Ok(redirect_err(
            "/admin/projects",
            "Title must be 1 to 100 characters.",
        ));
    }
    let objective = form.objective.trim();
    if objective.chars().count() > 1000 {
        return Ok(redirect_err(
            "/admin/projects",
            "Objective must be at most 1000 characters.",
        ));
    }

    let urls = csv_list(&form.urls);
    let raw_titles = csv_list(&form.url_titles);
    // Parallel arrays: pad or trim titles to match the url count, exactly
    // like the previous implementation did.
    let url_titles: Vec<String> = urls
        .iter()
        .enumerate()
        .map(|(i, _)| {
            raw_titles
                .get(i)
                .cloned()
                .unwrap_or_else(|| format!("Title {i}"))
        })
        .collect();

    let media = csv_list(&form.media);
    for url in &media {
        if !repositories::media::exists_by_url(url).await? {
            return Ok(redirect_err(
                "/admin/projects",
                "Project media must reference uploaded media files.",
            ));
        }
    }

    let experience_id = form.experience_id.trim();
    let experience_id = if experience_id.is_empty() || experience_id == "null" {
        None
    } else {
        match parse_object_id(experience_id) {
            Ok(oid) => {
                if repositories::experience::find_by_id(oid).await?.is_none() {
                    return Ok(redirect_err(
                        "/admin/projects",
                        "Linked experience not found.",
                    ));
                }
                Some(experience_id.to_string())
            }
            Err(_) => {
                return Ok(redirect_err(
                    "/admin/projects",
                    "Invalid linked experience.",
                ));
            }
        }
    };

    let order: i64 = form.order.trim().parse().unwrap_or(0);

    let found = repositories::project::update(
        id,
        title,
        objective,
        url_titles,
        urls,
        csv_list(&form.technologies),
        sentence_list(&form.key_results),
        media,
        experience_id,
        order,
    )
    .await?;
    if !found {
        return Ok(redirect_err("/admin/projects", "Project not found."));
    }

    state.caches.invalidate_public();
    Ok(redirect_ok("/admin/projects", "Project updated."))
}

pub async fn delete(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Redirect, AppError> {
    let id = parse_object_id(&id)?;
    repositories::project::delete(id).await?;
    state.caches.invalidate_public();
    Ok(redirect_ok("/admin/projects", "Project deleted."))
}
