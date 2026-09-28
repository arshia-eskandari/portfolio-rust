use mongodb::bson::oid::ObjectId;
use oximod::Model;
use serde::{Deserialize, Serialize};

/// The single skills document holding the flat list of skill names.
#[derive(Debug, Clone, Serialize, Deserialize, Model)]
#[db("portfolio")]
#[collection("Skills")]
pub struct Skills {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub _id: Option<ObjectId>,
    pub skills: Vec<String>,
}
