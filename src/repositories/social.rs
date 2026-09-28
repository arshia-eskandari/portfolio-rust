use oximod::{Model, OxiModError, Queryable};

use crate::models::{Social, SocialName};

/// Returns every social link.
pub async fn all() -> Result<Vec<Social>, OxiModError> {
    Social::query().all().await
}

/// Creates or updates the link for one social kind (there is at most one
/// document per kind, matching the previous application's behavior).
pub async fn upsert(name: SocialName, url: &str) -> Result<(), OxiModError> {
    let existing = Social::query().filter(|s| s.name.eq(name)).first().await?;
    match existing {
        Some(social) => {
            let id = social._id.expect("stored documents always have _id");
            Social::query()
                .filter(|s| s._id.eq(id))
                .update_one(|s| s.url.set(url.to_string()))
                .await?;
        }
        None => {
            Social::new().name(name).url(url.to_string()).save().await?;
        }
    }
    Ok(())
}
