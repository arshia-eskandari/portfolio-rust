use mongodb::bson::{Bson, DateTime, oid::ObjectId};
use oximod::Model;
use serde::{Deserialize, Serialize};

/// Review status of a contact-form submission.
///
/// Stored as the exact strings the Prisma enum wrote: `PENDING`,
/// `RESPONDED`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum ContactStatus {
    #[default]
    #[serde(rename = "PENDING")]
    Pending,
    #[serde(rename = "RESPONDED")]
    Responded,
}

impl ContactStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            ContactStatus::Pending => "PENDING",
            ContactStatus::Responded => "RESPONDED",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "PENDING" => Some(ContactStatus::Pending),
            "RESPONDED" => Some(ContactStatus::Responded),
            _ => None,
        }
    }
}

// Required so typed OxiMod queries/updates (`eq`, `set`) work on the field.
impl From<ContactStatus> for Bson {
    fn from(status: ContactStatus) -> Self {
        Bson::String(status.as_str().to_string())
    }
}

/// A contact-form submission.
#[derive(Debug, Clone, Serialize, Deserialize, Model)]
#[db("portfolio")]
#[collection("Contact")]
#[serde(rename_all = "camelCase")]
pub struct Contact {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub _id: Option<ObjectId>,
    pub first_name: String,
    pub last_name: String,
    pub email: String,
    pub message: String,
    pub status: ContactStatus,
    #[default(DateTime::now())]
    pub created_at: DateTime,
}
