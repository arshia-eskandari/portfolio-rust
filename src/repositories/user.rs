use mongodb::bson::oid::ObjectId;
use oximod::{Model, OxiModError, Queryable};

use crate::models::User;

/// Looks a user up by unique email.
pub async fn find_by_email(email: &str) -> Result<Option<User>, OxiModError> {
    User::query()
        .filter(|u| u.email.eq(email.to_string()))
        .first()
        .await
}

/// Looks a user up by id.
pub async fn find_by_id(id: ObjectId) -> Result<Option<User>, OxiModError> {
    User::find_by_id(id).await
}
