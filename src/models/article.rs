use mongodb::bson::{DateTime, oid::ObjectId};
use oximod::Model;
use serde::{Deserialize, Serialize};

/// A Markdown article.
///
/// `slug` is unique (index created by the previous Prisma application) and
/// must stay compatible with published URLs; see [`crate::slug`] for the
/// preserved generation rules.
#[derive(Debug, Clone, Serialize, Deserialize, Model)]
#[db("portfolio")]
#[collection("Article")]
#[serde(rename_all = "camelCase")]
pub struct Article {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub _id: Option<ObjectId>,
    pub title: String,
    pub banner: Option<String>,
    pub content: String,
    #[default(DateTime::now())]
    pub created_at: DateTime,
    #[default(DateTime::now())]
    pub updated_at: DateTime,
    pub slug: String,
    pub tags: Vec<String>,
}
