use mongodb::bson::oid::ObjectId;
use oximod::Model;
use serde::{Deserialize, Serialize};

/// The single "about" document (title, biography, resume, portrait).
#[derive(Debug, Clone, Serialize, Deserialize, Model)]
#[db("portfolio")]
#[collection("About")]
#[serde(rename_all = "camelCase")]
pub struct About {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub _id: Option<ObjectId>,
    pub title: Option<String>,
    pub text: String,
    pub resume_url: Option<String>,
    pub image_url: Option<String>,
}
