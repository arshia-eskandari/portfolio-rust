use mongodb::bson::oid::ObjectId;
use oximod::{Model, OxiModError, Queryable};

use crate::models::Project;

/// Returns every project in public display order (`order` ascending).
pub async fn all_ordered() -> Result<Vec<Project>, OxiModError> {
    Project::query().sort_by(|p| p.order.asc()).all().await
}

pub async fn find_by_id(id: ObjectId) -> Result<Option<Project>, OxiModError> {
    Project::find_by_id(id).await
}

/// Creates a placeholder project for the admin to fill in.
pub async fn create_default() -> Result<ObjectId, OxiModError> {
    Project::new()
        .project_title("Title".to_string())
        .save()
        .await
}

#[allow(clippy::too_many_arguments)]
pub async fn update(
    id: ObjectId,
    project_title: &str,
    objective: &str,
    url_titles: Vec<String>,
    urls: Vec<String>,
    project_technologies: Vec<String>,
    key_results: Vec<String>,
    media: Vec<String>,
    experience_id: Option<String>,
    order: i64,
) -> Result<bool, OxiModError> {
    let updated = Project::query()
        .filter(|p| p._id.eq(id))
        .update_one(|p| {
            p.project_title.set(project_title.to_string())
                & p.objective.set(objective.to_string())
                & p.url_titles.set(url_titles.clone())
                & p.urls.set(urls.clone())
                & p.project_technologies.set(project_technologies.clone())
                & p.key_results.set(key_results.clone())
                & p.media.set(media.clone())
                & p.experience_id.set(experience_id.clone())
                & p.order.set(order)
        })
        .await?;
    Ok(updated.is_some())
}

pub async fn delete(id: ObjectId) -> Result<(), OxiModError> {
    Project::delete_by_id(id).await?;
    Ok(())
}

/// Returns projects whose parallel `urls`/`urlTitles` arrays reference
/// `url` (used for media deletion cleanup, which must drop the matching
/// title by index and therefore needs a read-modify-write).
pub async fn find_by_link_url(url: &str) -> Result<Vec<Project>, OxiModError> {
    Project::query()
        .filter(|p| p.urls.contains(url.to_string()))
        .all()
        .await
}

/// Rewrites one project's parallel link arrays.
pub async fn set_links(
    id: ObjectId,
    urls: Vec<String>,
    url_titles: Vec<String>,
) -> Result<(), OxiModError> {
    Project::query()
        .filter(|p| p._id.eq(id))
        .update_one(|p| p.urls.set(urls.clone()) & p.url_titles.set(url_titles.clone()))
        .await?;
    Ok(())
}

/// Removes `url` from every project's `media` array (media deletion
/// cleanup; the previous application left these references dangling —
/// cleaning them here avoids broken carousels without changing the model).
pub async fn pull_media_url(url: &str) -> Result<(), OxiModError> {
    Project::query()
        .filter(|p| p.media.contains(url.to_string()))
        .update_all(|p| p.media.pull(url.to_string()))
        .await?;
    Ok(())
}
