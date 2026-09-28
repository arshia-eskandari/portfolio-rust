use mongodb::bson::{Bson, oid::ObjectId};
use oximod::Model;
use serde::{Deserialize, Serialize};

/// Social-link kind, stored as the Prisma enum strings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum SocialName {
    #[default]
    #[serde(rename = "LINKEDIN")]
    Linkedin,
    #[serde(rename = "GITHUB")]
    Github,
    #[serde(rename = "TELEGRAM")]
    Telegram,
    #[serde(rename = "EMAIL")]
    Email,
}

impl SocialName {
    pub fn as_str(&self) -> &'static str {
        match self {
            SocialName::Linkedin => "LINKEDIN",
            SocialName::Github => "GITHUB",
            SocialName::Telegram => "TELEGRAM",
            SocialName::Email => "EMAIL",
        }
    }

    pub const ALL: [SocialName; 4] = [
        SocialName::Linkedin,
        SocialName::Github,
        SocialName::Telegram,
        SocialName::Email,
    ];
}

impl From<SocialName> for Bson {
    fn from(name: SocialName) -> Self {
        Bson::String(name.as_str().to_string())
    }
}

/// A social link (LinkedIn, GitHub, Telegram or a plain email address).
#[derive(Debug, Clone, Serialize, Deserialize, Model)]
#[db("portfolio")]
#[collection("Social")]
pub struct Social {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub _id: Option<ObjectId>,
    pub name: SocialName,
    pub url: String,
}
