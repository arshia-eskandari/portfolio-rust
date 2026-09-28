use mongodb::bson::oid::ObjectId;
use oximod::Model;
use serde::{Deserialize, Serialize};

/// The single hero/landing document.
#[derive(Debug, Clone, Serialize, Deserialize, Model)]
#[db("portfolio")]
#[collection("Hero")]
pub struct Hero {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub _id: Option<ObjectId>,
    pub title: Option<String>,
    pub text: Option<String>,
}
