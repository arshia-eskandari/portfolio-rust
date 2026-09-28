//! BSON compatibility tests.
//!
//! These tests prove that the Rust models serialize to and deserialize
//! from exactly the document shapes the previous Prisma application
//! stored in MongoDB: camelCase field names, `_id` ObjectIds, BSON dates,
//! SCREAMING_CASE enum strings, string arrays, and nullable/missing
//! optionals. No database connection is required.

use mongodb::bson::{Bson, DateTime, doc, from_document, oid::ObjectId, to_document};
use portfolio::models::{
    About, Article, Contact, ContactStatus, Experience, Hero, Media, MediaType, Project, Role,
    Skills, Social, SocialName, User,
};

fn oid() -> ObjectId {
    ObjectId::parse_str("665f01234567890123abcdef").expect("valid hex")
}

fn date() -> DateTime {
    DateTime::from_millis(1_705_320_000_000) // 2024-01-15T12:00:00Z
}

#[test]
fn about_roundtrip_matches_prisma_shape() {
    let stored = doc! {
        "_id": oid(),
        "title": "About Me",
        "text": "I build backend systems.",
        "resumeUrl": "https://bucket.s3.us-east-2.amazonaws.com/media/resume.pdf",
        "imageUrl": Bson::Null,
    };

    let about: About = from_document(stored).expect("deserializes Prisma document");
    assert_eq!(about.title.as_deref(), Some("About Me"));
    assert_eq!(about.text, "I build backend systems.");
    assert!(about.resume_url.is_some());
    assert_eq!(about.image_url, None, "BSON null maps to None");

    let out = to_document(&about).expect("serializes");
    assert!(
        out.contains_key("resumeUrl"),
        "field must stay camelCase: {out:?}"
    );
    assert!(out.contains_key("imageUrl"));
    assert!(
        !out.contains_key("resume_url"),
        "snake_case must never be written"
    );
    assert_eq!(out.get_object_id("_id").expect("_id is ObjectId"), oid());
}

#[test]
fn about_tolerates_missing_optional_fields() {
    // Documents created before a field existed simply lack the key.
    let stored = doc! { "_id": oid(), "text": "hello" };
    let about: About = from_document(stored).expect("missing optionals deserialize");
    assert_eq!(about.title, None);
    assert_eq!(about.resume_url, None);
}

#[test]
fn contact_roundtrip_with_enum_and_date() {
    let stored = doc! {
        "_id": oid(),
        "firstName": "Ada",
        "lastName": "Lovelace",
        "email": "ada@example.com",
        "message": "Hello there",
        "status": "PENDING",
        "createdAt": date(),
    };

    let contact: Contact = from_document(stored).expect("deserializes");
    assert_eq!(contact.first_name, "Ada");
    assert_eq!(contact.status, ContactStatus::Pending);
    assert_eq!(contact.created_at, date());

    let out = to_document(&contact).expect("serializes");
    assert_eq!(out.get_str("status").expect("status"), "PENDING");
    assert_eq!(out.get_str("firstName").expect("firstName"), "Ada");
    assert!(
        matches!(out.get("createdAt"), Some(Bson::DateTime(_))),
        "dates stay BSON dates"
    );
}

#[test]
fn contact_status_responded_roundtrips() {
    let stored = doc! {
        "_id": oid(),
        "firstName": "A",
        "lastName": "B",
        "email": "a@b.co",
        "message": "m",
        "status": "RESPONDED",
        "createdAt": date(),
    };
    let contact: Contact = from_document(stored).expect("deserializes");
    assert_eq!(contact.status, ContactStatus::Responded);
    let out = to_document(&contact).expect("serializes");
    assert_eq!(out.get_str("status").expect("status"), "RESPONDED");
}

#[test]
fn experience_roundtrip_with_arrays_and_optional_end_date() {
    let stored = doc! {
        "_id": oid(),
        "jobTitle": "Software Engineer",
        "company": "Acme",
        "startDate": date(),
        "endDate": Bson::Null,
        "achievements": ["Shipped X", "Improved Y"],
        "responsibilities": ["Own the API"],
        "recommendationLetterUrls": [],
        "location": "Toronto, ON",
    };

    let experience: Experience = from_document(stored).expect("deserializes");
    assert_eq!(experience.job_title, "Software Engineer");
    assert_eq!(experience.end_date, None);
    assert_eq!(experience.achievements.len(), 2);
    assert!(experience.recommendation_letter_urls.is_empty());

    let out = to_document(&experience).expect("serializes");
    assert!(out.contains_key("jobTitle"));
    assert!(out.contains_key("recommendationLetterUrls"));
    assert!(matches!(out.get("startDate"), Some(Bson::DateTime(_))));
}

#[test]
fn hero_roundtrip() {
    let stored = doc! { "_id": oid(), "title": "Hi, I'm Arshia", "text": Bson::Null };
    let hero: Hero = from_document(stored).expect("deserializes");
    assert_eq!(hero.title.as_deref(), Some("Hi, I'm Arshia"));
    assert_eq!(hero.text, None);
}

#[test]
fn project_roundtrip_with_parallel_arrays_and_order() {
    let stored = doc! {
        "_id": oid(),
        "urlTitles": ["GitHub", "Docs"],
        "urls": ["https://github.com/x", "https://docs.x"],
        "projectTechnologies": ["Rust", "MongoDB"],
        "projectTitle": "OxiMod",
        "objective": "A MongoDB ODM for Rust",
        "keyResults": ["Published on crates.io"],
        "experienceId": "665f01234567890123abcdef",
        "media": ["https://bucket/img.png"],
        "order": 1i64,
    };

    let project: Project = from_document(stored).expect("deserializes");
    assert_eq!(project.project_title, "OxiMod");
    assert_eq!(project.url_titles, vec!["GitHub", "Docs"]);
    assert_eq!(project.order, 1);
    assert_eq!(
        project.experience_id.as_deref(),
        Some("665f01234567890123abcdef")
    );

    let out = to_document(&project).expect("serializes");
    assert!(out.contains_key("projectTitle"));
    assert!(out.contains_key("urlTitles"));
    assert!(out.contains_key("keyResults"));
    assert!(out.contains_key("experienceId"));
    assert!(
        matches!(out.get("order"), Some(Bson::Int64(1))),
        "order is written back as Int64, the type the existing documents use"
    );
}

#[test]
fn project_order_stored_as_int32_still_deserializes() {
    let stored = doc! {
        "_id": oid(),
        "urlTitles": [],
        "urls": [],
        "projectTechnologies": [],
        "projectTitle": "Legacy",
        "objective": "",
        "keyResults": [],
        "experienceId": Bson::Null,
        "media": [],
        "order": 7i32,
    };
    let project: Project = from_document(stored).expect("deserializes Int32 order");
    assert_eq!(project.order, 7);
}

#[test]
fn skills_roundtrip() {
    let stored = doc! { "_id": oid(), "skills": ["Rust", "TypeScript"] };
    let skills: Skills = from_document(stored).expect("deserializes");
    assert_eq!(skills.skills, vec!["Rust", "TypeScript"]);
}

#[test]
fn social_roundtrip_with_all_enum_values() {
    for (name, expected) in [
        ("LINKEDIN", SocialName::Linkedin),
        ("GITHUB", SocialName::Github),
        ("TELEGRAM", SocialName::Telegram),
        ("EMAIL", SocialName::Email),
    ] {
        let stored = doc! { "_id": oid(), "name": name, "url": "https://example.com" };
        let social: Social = from_document(stored).expect("deserializes");
        assert_eq!(social.name, expected);
        let out = to_document(&social).expect("serializes");
        assert_eq!(out.get_str("name").expect("name"), name);
    }
}

#[test]
fn user_roundtrip_with_role_and_timestamps() {
    let stored = doc! {
        "_id": oid(),
        "firstName": "Arshia",
        "lastName": "Eskandari",
        "email": "admin@example.com",
        "password": "bd2b1aaf7ef4f09be9f52ce2d8d599674d81aa9d6a4421696dc4d93dd0619d682ce56b4d64a9ef097761ced99e0f67265b5f76085e5b0ee7ca4696b2ad6fe2b2",
        "role": "ADMIN",
        "createdAt": date(),
        "updatedAt": date(),
    };

    let user: User = from_document(stored).expect("deserializes");
    assert_eq!(user.role, Role::Admin);
    assert_eq!(user.email, "admin@example.com");

    let out = to_document(&user).expect("serializes");
    assert_eq!(out.get_str("role").expect("role"), "ADMIN");
    assert!(out.contains_key("createdAt"));
    assert!(out.contains_key("updatedAt"));
}

#[test]
fn media_roundtrip_with_all_media_types() {
    for (value, expected) in [
        ("IMAGE", MediaType::Image),
        ("VIDEO", MediaType::Video),
        ("PDF", MediaType::Pdf),
    ] {
        let stored = doc! {
            "_id": oid(),
            "name": "file.bin",
            "url": "https://bucket/file.bin",
            "fileKey": "media/uuid-file.bin",
            "createdAt": date(),
            "mediaType": value,
        };
        let media: Media = from_document(stored).expect("deserializes");
        assert_eq!(media.media_type, expected);
        let out = to_document(&media).expect("serializes");
        assert_eq!(out.get_str("mediaType").expect("mediaType"), value);
        assert!(out.contains_key("fileKey"));
    }
}

#[test]
fn article_roundtrip() {
    let stored = doc! {
        "_id": oid(),
        "title": "Building OxiMod",
        "banner": Bson::Null,
        "content": "# Heading\n\nBody text.",
        "createdAt": date(),
        "updatedAt": date(),
        "slug": "building-oximod-abcdef",
        "tags": ["rust", "mongodb"],
    };

    let article: Article = from_document(stored).expect("deserializes");
    assert_eq!(article.slug, "building-oximod-abcdef");
    assert_eq!(article.banner, None);
    assert_eq!(article.tags, vec!["rust", "mongodb"]);

    let out = to_document(&article).expect("serializes");
    assert!(out.contains_key("createdAt"));
    assert!(out.contains_key("updatedAt"));
    assert_eq!(out.get_str("slug").expect("slug"), "building-oximod-abcdef");
}

#[test]
fn new_documents_omit_id_until_mongodb_assigns_one() {
    // The insert path must not write `_id: null` — MongoDB assigns the id.
    let contact = Contact {
        _id: None,
        first_name: "A".into(),
        last_name: "B".into(),
        email: "a@b.co".into(),
        message: "hello".into(),
        status: ContactStatus::Pending,
        created_at: date(),
    };
    let out = to_document(&contact).expect("serializes");
    assert!(
        !out.contains_key("_id"),
        "_id must be skipped when None: {out:?}"
    );
}

#[test]
fn unknown_enum_value_fails_loudly_rather_than_corrupting() {
    let stored = doc! {
        "_id": oid(),
        "firstName": "A",
        "lastName": "B",
        "email": "a@b.co",
        "message": "m",
        "status": "SOMETHING_ELSE",
        "createdAt": date(),
    };
    assert!(from_document::<Contact>(stored).is_err());
}
