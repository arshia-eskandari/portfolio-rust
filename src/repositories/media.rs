use mongodb::bson::oid::ObjectId;
use oximod::{Model, OxiModError, Queryable};

use crate::models::{Media, MediaType};

/// Returns every media object, newest first.
pub async fn all() -> Result<Vec<Media>, OxiModError> {
    Media::query().sort_by(|m| m.created_at.desc()).all().await
}

pub async fn find_by_id(id: ObjectId) -> Result<Option<Media>, OxiModError> {
    Media::find_by_id(id).await
}

/// Records an uploaded S3 object.
pub async fn create(
    name: &str,
    url: &str,
    file_key: &str,
    media_type: MediaType,
) -> Result<ObjectId, OxiModError> {
    Media::new()
        .name(name.to_string())
        .url(url.to_string())
        .file_key(file_key.to_string())
        .media_type(media_type)
        .save()
        .await
}

/// Returns whether a media record with this URL exists (used to validate
/// admin form references before storing them).
pub async fn exists_by_url(url: &str) -> Result<bool, OxiModError> {
    Ok(Media::query()
        .filter(|m| m.url.eq(url.to_string()))
        .first()
        .await?
        .is_some())
}

pub async fn delete(id: ObjectId) -> Result<(), OxiModError> {
    Media::delete_by_id(id).await?;
    Ok(())
}
