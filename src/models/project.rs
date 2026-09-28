use mongodb::bson::oid::ObjectId;
use oximod::Model;
use serde::{Deserialize, Serialize};

/// A portfolio project.
///
/// `urls`/`url_titles` are parallel arrays (link + its label), `media`
/// holds URLs referencing the `Media` collection, `experience_id`
/// optionally links to an `Experience` document by hex id, and `order`
/// drives the public display ordering (ascending).
///
/// `order` is `i64` because the existing documents store it as BSON Int64
/// (verified against the production database); an `i32` model would read
/// them fine but silently rewrite the field as Int32 on every admin save.
/// Int32 values still deserialize.
#[derive(Debug, Clone, Serialize, Deserialize, Model)]
#[db("portfolio")]
#[collection("Project")]
#[serde(rename_all = "camelCase")]
pub struct Project {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub _id: Option<ObjectId>,
    pub url_titles: Vec<String>,
    pub urls: Vec<String>,
    pub project_technologies: Vec<String>,
    pub project_title: String,
    pub objective: String,
    pub key_results: Vec<String>,
    pub experience_id: Option<String>,
    pub media: Vec<String>,
    pub order: i64,
}
