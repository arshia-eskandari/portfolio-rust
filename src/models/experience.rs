use mongodb::bson::{DateTime, oid::ObjectId};
use oximod::Model;
use serde::{Deserialize, Serialize};

/// An employment/experience entry.
#[derive(Debug, Clone, Serialize, Deserialize, Model)]
#[db("portfolio")]
#[collection("Experience")]
#[serde(rename_all = "camelCase")]
pub struct Experience {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub _id: Option<ObjectId>,
    pub job_title: String,
    pub company: String,
    #[default(DateTime::now())]
    pub start_date: DateTime,
    pub end_date: Option<DateTime>,
    pub achievements: Vec<String>,
    pub responsibilities: Vec<String>,
    pub recommendation_letter_urls: Vec<String>,
    pub location: String,
}
