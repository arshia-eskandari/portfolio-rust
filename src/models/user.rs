use mongodb::bson::{Bson, DateTime, oid::ObjectId};
use oximod::Model;
use serde::{Deserialize, Serialize};

/// User role, stored as the Prisma enum strings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum Role {
    #[serde(rename = "ADMIN")]
    Admin,
    #[default]
    #[serde(rename = "USER")]
    User,
}

impl Role {
    pub fn as_str(&self) -> &'static str {
        match self {
            Role::Admin => "ADMIN",
            Role::User => "USER",
        }
    }
}

impl From<Role> for Bson {
    fn from(role: Role) -> Self {
        Bson::String(role.as_str().to_string())
    }
}

/// An application user.
///
/// The `password` field keeps the existing stored representation: an
/// unsalted hex-encoded SHA-512 digest written by the previous
/// application. See `services::auth` for the compatible verification.
#[derive(Debug, Clone, Serialize, Deserialize, Model)]
#[db("portfolio")]
#[collection("User")]
#[serde(rename_all = "camelCase")]
pub struct User {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub _id: Option<ObjectId>,
    pub first_name: String,
    pub last_name: String,
    pub email: String,
    pub password: String,
    pub role: Role,
    #[default(DateTime::now())]
    pub created_at: DateTime,
    #[default(DateTime::now())]
    pub updated_at: DateTime,
}
