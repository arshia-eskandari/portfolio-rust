use oximod::{Model, OxiModError, Queryable};

use crate::models::Skills;

/// Returns the skills document (the collection holds at most one).
pub async fn find() -> Result<Option<Skills>, OxiModError> {
    Skills::query().first().await
}

/// Creates or updates the single skills document.
pub async fn upsert(skills: Vec<String>) -> Result<(), OxiModError> {
    match find().await? {
        Some(existing) => {
            let id = existing._id.expect("stored documents always have _id");
            Skills::query()
                .filter(|s| s._id.eq(id))
                .update_one(|s| s.skills.set(skills.clone()))
                .await?;
        }
        None => {
            Skills::new().skills(skills).save().await?;
        }
    }
    Ok(())
}
