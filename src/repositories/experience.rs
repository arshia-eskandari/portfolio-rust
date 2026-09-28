use mongodb::bson::{DateTime, oid::ObjectId};
use oximod::{Model, OxiModError, Queryable};

use crate::models::Experience;

/// Returns every experience, sorted for public display: current positions
/// (no end date) first, then by end date descending, then by start date
/// descending. This preserves the previous `sortExperiences` behavior.
pub async fn all_sorted() -> Result<Vec<Experience>, OxiModError> {
    let mut experiences = Experience::query().all().await?;
    experiences.sort_by(|a, b| match (a.end_date, b.end_date) {
        (None, Some(_)) => std::cmp::Ordering::Less,
        (Some(_), None) => std::cmp::Ordering::Greater,
        (Some(a_end), Some(b_end)) => b_end.cmp(&a_end),
        (None, None) => b.start_date.cmp(&a.start_date),
    });
    Ok(experiences)
}

pub async fn find_by_id(id: ObjectId) -> Result<Option<Experience>, OxiModError> {
    Experience::find_by_id(id).await
}

/// Creates a placeholder experience for the admin to fill in (same
/// behavior as the previous "add default" action).
pub async fn create_default() -> Result<ObjectId, OxiModError> {
    // Give the placeholder an end date (like the previous application did)
    // so it never shows up as a current position before being edited.
    let now = DateTime::now();
    Experience::new()
        .job_title("Title".to_string())
        .company("Company".to_string())
        .location("Location".to_string())
        .start_date(now)
        .end_date(now)
        .save()
        .await
}

#[allow(clippy::too_many_arguments)]
pub async fn update(
    id: ObjectId,
    job_title: &str,
    company: &str,
    location: &str,
    start_date: DateTime,
    end_date: Option<DateTime>,
    achievements: Vec<String>,
    responsibilities: Vec<String>,
    recommendation_letter_urls: Vec<String>,
) -> Result<bool, OxiModError> {
    let updated = Experience::query()
        .filter(|e| e._id.eq(id))
        .update_one(|e| {
            e.job_title.set(job_title.to_string())
                & e.company.set(company.to_string())
                & e.location.set(location.to_string())
                & e.start_date.set(start_date)
                & e.end_date.set(end_date)
                & e.achievements.set(achievements.clone())
                & e.responsibilities.set(responsibilities.clone())
                & e.recommendation_letter_urls
                    .set(recommendation_letter_urls.clone())
        })
        .await?;
    Ok(updated.is_some())
}

pub async fn delete(id: ObjectId) -> Result<(), OxiModError> {
    Experience::delete_by_id(id).await?;
    Ok(())
}

/// Removes `url` from every experience's recommendation letters (media
/// deletion cleanup, preserved from the previous application).
pub async fn pull_recommendation_url(url: &str) -> Result<(), OxiModError> {
    Experience::query()
        .filter(|e| e.recommendation_letter_urls.contains(url.to_string()))
        .update_all(|e| e.recommendation_letter_urls.pull(url.to_string()))
        .await?;
    Ok(())
}
