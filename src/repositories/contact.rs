use mongodb::bson::oid::ObjectId;
use oximod::{Model, OxiModError, Queryable};

use crate::models::{Contact, ContactStatus};

/// Returns every submission, newest first.
pub async fn all() -> Result<Vec<Contact>, OxiModError> {
    Contact::query()
        .sort_by(|c| c.created_at.desc())
        .all()
        .await
}

/// Counts submissions still awaiting a response.
pub async fn count_pending() -> Result<u64, OxiModError> {
    Contact::query()
        .filter(|c| c.status.eq(ContactStatus::Pending))
        .count()
        .await
}

/// Stores a new submission with status `PENDING` and the current time.
pub async fn create(
    first_name: &str,
    last_name: &str,
    email: &str,
    message: &str,
) -> Result<ObjectId, OxiModError> {
    Contact::new()
        .first_name(first_name.to_string())
        .last_name(last_name.to_string())
        .email(email.to_string())
        .message(message.to_string())
        .save()
        .await
}

/// Updates the review status of one submission.
pub async fn set_status(id: ObjectId, status: ContactStatus) -> Result<bool, OxiModError> {
    let updated = Contact::query()
        .filter(|c| c._id.eq(id))
        .update_one(|c| c.status.set(status))
        .await?;
    Ok(updated.is_some())
}
