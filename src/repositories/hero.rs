use oximod::{Model, OxiModError, Queryable};

use crate::models::Hero;

/// Returns the hero document (the collection holds at most one).
pub async fn find() -> Result<Option<Hero>, OxiModError> {
    Hero::query().first().await
}

/// Creates or updates the single hero document.
pub async fn upsert(title: &str, text: &str) -> Result<(), OxiModError> {
    match find().await? {
        Some(existing) => {
            let id = existing._id.expect("stored documents always have _id");
            Hero::query()
                .filter(|h| h._id.eq(id))
                .update_one(|h| {
                    h.title.set(Some(title.to_string())) & h.text.set(Some(text.to_string()))
                })
                .await?;
        }
        None => {
            Hero::new()
                .title(title.to_string())
                .text(text.to_string())
                .save()
                .await?;
        }
    }
    Ok(())
}
