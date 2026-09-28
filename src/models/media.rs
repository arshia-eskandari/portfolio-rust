use mongodb::bson::{Bson, DateTime, oid::ObjectId};
use oximod::Model;
use serde::{Deserialize, Serialize};

/// Kind of stored media object, stored as the Prisma enum strings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum MediaType {
    #[default]
    #[serde(rename = "IMAGE")]
    Image,
    #[serde(rename = "VIDEO")]
    Video,
    #[serde(rename = "PDF")]
    Pdf,
}

impl MediaType {
    pub fn as_str(&self) -> &'static str {
        match self {
            MediaType::Image => "IMAGE",
            MediaType::Video => "VIDEO",
            MediaType::Pdf => "PDF",
        }
    }
}

impl From<MediaType> for Bson {
    fn from(media_type: MediaType) -> Self {
        Bson::String(media_type.as_str().to_string())
    }
}

/// A media object stored in S3 and referenced by URL from other documents.
#[derive(Debug, Clone, Serialize, Deserialize, Model)]
#[db("portfolio")]
#[collection("Media")]
#[serde(rename_all = "camelCase")]
pub struct Media {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub _id: Option<ObjectId>,
    pub name: String,
    pub url: String,
    pub file_key: String,
    #[default(DateTime::now())]
    pub created_at: DateTime,
    pub media_type: MediaType,
}
