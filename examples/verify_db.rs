//! Read-only compatibility check against a real MongoDB deployment.
//!
//! Connects with `DATABASE_URL` (loaded from `.env`), lists databases and
//! collections, inspects the BSON shape of every stored document, verifies
//! that each document deserializes into its OxiMod model, and finally runs
//! the application's own typed queries. It never writes, prints connection
//! strings, credentials, password hashes, or private contact messages.
//!
//! ```text
//! cargo run --example verify_db
//! ```

use std::collections::{BTreeMap, BTreeSet};

use mongodb::bson::{Bson, Document, doc};
use oximod::OxiClient;
use portfolio::config::{Config, DB_NAME};
use portfolio::models::{
    About, Article, Contact, Experience, Hero, Media, Project, Skills, Social, User,
};
use portfolio::repositories;
use portfolio::services::portfolio::load_home_data;
use portfolio::slug::generate_article_slug;
use serde::de::DeserializeOwned;

const EXPECTED_COLLECTIONS: [&str; 10] = [
    "About",
    "Contact",
    "Experience",
    "Hero",
    "Project",
    "Skills",
    "Social",
    "User",
    "Media",
    "Article",
];

/// Fields each model knows about (stored names). Anything else in a stored
/// document is reported as an extra field (ignored on deserialize).
fn known_fields(collection: &str) -> &'static [&'static str] {
    match collection {
        "About" => &["_id", "title", "text", "resumeUrl", "imageUrl"],
        "Contact" => &[
            "_id",
            "firstName",
            "lastName",
            "email",
            "message",
            "status",
            "createdAt",
        ],
        "Experience" => &[
            "_id",
            "jobTitle",
            "company",
            "startDate",
            "endDate",
            "achievements",
            "responsibilities",
            "recommendationLetterUrls",
            "location",
        ],
        "Hero" => &["_id", "title", "text"],
        "Project" => &[
            "_id",
            "urlTitles",
            "urls",
            "projectTechnologies",
            "projectTitle",
            "objective",
            "keyResults",
            "experienceId",
            "media",
            "order",
        ],
        "Skills" => &["_id", "skills"],
        "Social" => &["_id", "name", "url"],
        "User" => &[
            "_id",
            "firstName",
            "lastName",
            "email",
            "password",
            "role",
            "createdAt",
            "updatedAt",
        ],
        "Media" => &["_id", "name", "url", "fileKey", "createdAt", "mediaType"],
        "Article" => &[
            "_id",
            "title",
            "banner",
            "content",
            "createdAt",
            "updatedAt",
            "slug",
            "tags",
        ],
        _ => &[],
    }
}

fn bson_type_name(value: &Bson) -> &'static str {
    match value {
        Bson::Double(_) => "double",
        Bson::String(_) => "string",
        Bson::Array(_) => "array",
        Bson::Document(_) => "document",
        Bson::Boolean(_) => "bool",
        Bson::Null => "null",
        Bson::Int32(_) => "int32",
        Bson::Int64(_) => "int64",
        Bson::DateTime(_) => "date",
        Bson::ObjectId(_) => "objectId",
        Bson::Timestamp(_) => "timestamp",
        Bson::Binary(_) => "binary",
        Bson::Decimal128(_) => "decimal128",
        _ => "other",
    }
}

struct ShapeReport {
    count: usize,
    /// field -> (bson type -> occurrences)
    types: BTreeMap<String, BTreeMap<&'static str, usize>>,
    /// field -> number of documents missing it
    missing: BTreeMap<String, usize>,
    extra: BTreeSet<String>,
    failures: Vec<String>,
}

async fn inspect<T: DeserializeOwned>(
    db: &mongodb::Database,
    collection: &str,
) -> Result<ShapeReport, Box<dyn std::error::Error>> {
    let known = known_fields(collection);
    let mut report = ShapeReport {
        count: 0,
        types: BTreeMap::new(),
        missing: BTreeMap::new(),
        extra: BTreeSet::new(),
        failures: Vec::new(),
    };

    let mut cursor = db.collection::<Document>(collection).find(doc! {}).await?;
    while cursor.advance().await? {
        let document: Document = cursor.current().try_into()?;
        report.count += 1;

        for (key, value) in &document {
            *report
                .types
                .entry(key.clone())
                .or_default()
                .entry(bson_type_name(value))
                .or_insert(0) += 1;
            if !known.contains(&key.as_str()) {
                report.extra.insert(key.clone());
            }
        }
        for field in known {
            if !document.contains_key(field) {
                *report.missing.entry((*field).to_string()).or_insert(0) += 1;
            }
        }

        let id = document
            .get_object_id("_id")
            .map(|oid| oid.to_hex())
            .unwrap_or_else(|_| "<no ObjectId _id>".to_string());
        if let Err(err) = mongodb::bson::from_document::<T>(document) {
            report.failures.push(format!("{collection} {id}: {err}"));
        }
    }
    Ok(report)
}

fn print_report(collection: &str, report: &ShapeReport) {
    println!("\n[{collection}] documents: {}", report.count);
    for (field, types) in &report.types {
        let summary: Vec<String> = types.iter().map(|(t, n)| format!("{t}x{n}")).collect();
        let missing = report
            .missing
            .get(field)
            .map(|n| format!(", missing in {n}"))
            .unwrap_or_default();
        println!("  {field}: {}{missing}", summary.join(" "));
    }
    for (field, n) in &report.missing {
        if !report.types.contains_key(field) {
            println!("  {field}: missing in {n} (never present)");
        }
    }
    if !report.extra.is_empty() {
        println!("  extra fields (ignored by model): {:?}", report.extra);
    }
    if report.failures.is_empty() {
        println!(
            "  model deserialization: OK ({}/{})",
            report.count, report.count
        );
    } else {
        println!("  model deserialization: {} FAILED", report.failures.len());
        for failure in &report.failures {
            println!("    - {failure}");
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();
    let config = Config::from_env()?;
    println!("config loaded: {config:?}");
    println!("database name from DATABASE_URL matches models: {DB_NAME}");

    OxiClient::init_global(config.database_url.clone()).await?;
    let client = OxiClient::global()?;

    let databases = client.list_database_names().await?;
    println!("\ndatabases visible to this user: {databases:?}");
    if !databases.iter().any(|d| d == DB_NAME) {
        println!("!! database {DB_NAME:?} not found on the server");
    }

    let db = client.database(DB_NAME);
    let mut collections = db.list_collection_names().await?;
    collections.sort();
    println!("collections in {DB_NAME}: {collections:?}");
    for expected in EXPECTED_COLLECTIONS {
        if !collections.iter().any(|c| c == expected) {
            println!("!! expected collection {expected:?} is missing");
        }
    }
    for found in &collections {
        if !EXPECTED_COLLECTIONS.contains(&found.as_str()) {
            println!("note: unexpected collection {found:?} (unused by the app)");
        }
    }

    println!("\n== raw BSON shape inspection + per-document model deserialization ==");
    let mut total_failures = 0usize;
    macro_rules! check {
        ($name:literal, $ty:ty) => {{
            let report = inspect::<$ty>(&db, $name).await?;
            print_report($name, &report);
            total_failures += report.failures.len();
        }};
    }
    check!("About", About);
    check!("Contact", Contact);
    check!("Experience", Experience);
    check!("Hero", Hero);
    check!("Project", Project);
    check!("Skills", Skills);
    check!("Social", Social);
    check!("User", User);
    check!("Media", Media);
    check!("Article", Article);

    println!("\n== indexes (names only) ==");
    for name in ["User", "Article"] {
        let indexes = db.collection::<Document>(name).list_index_names().await?;
        println!("  {name}: {indexes:?}");
    }

    println!("\n== OxiMod typed queries used by the application ==");
    let hero = repositories::hero::find().await?;
    println!(
        "hero: present={}, title_set={}, text_len={}",
        hero.is_some(),
        hero.as_ref().is_some_and(|h| h.title.is_some()),
        hero.as_ref()
            .and_then(|h| h.text.as_ref())
            .map(|t| t.chars().count())
            .unwrap_or(0)
    );
    let about = repositories::about::find().await?;
    println!(
        "about: present={}, title_set={}, text_len={}, resume_url={:?}, image_url={:?}",
        about.is_some(),
        about.as_ref().is_some_and(|a| a.title.is_some()),
        about.as_ref().map(|a| a.text.chars().count()).unwrap_or(0),
        about.as_ref().and_then(|a| a.resume_url.clone()),
        about.as_ref().and_then(|a| a.image_url.clone()),
    );
    let skills = repositories::skills::find().await?;
    println!(
        "skills: present={}, count={}, values={:?}",
        skills.is_some(),
        skills.as_ref().map(|s| s.skills.len()).unwrap_or(0),
        skills
            .as_ref()
            .map(|s| s.skills.clone())
            .unwrap_or_default()
    );

    let projects = repositories::project::all_ordered().await?;
    println!("projects (order asc): {}", projects.len());
    for p in &projects {
        println!(
            "  order={} title={:?} urls={} urlTitles={} techs={} keyResults={} media={} experienceId={:?}",
            p.order,
            p.project_title,
            p.urls.len(),
            p.url_titles.len(),
            p.project_technologies.len(),
            p.key_results.len(),
            p.media.len(),
            p.experience_id
        );
        if p.urls.len() != p.url_titles.len() {
            println!("    !! urls/urlTitles length mismatch");
        }
        for url in &p.media {
            println!("    media: {url}");
        }
        for (url, title) in p.urls.iter().zip(&p.url_titles) {
            println!("    link: {title:?} -> {url}");
        }
    }

    let experiences = repositories::experience::all_sorted().await?;
    println!("experiences (sorted): {}", experiences.len());
    let experience_ids: BTreeSet<String> = experiences
        .iter()
        .filter_map(|e| e._id.map(|id| id.to_hex()))
        .collect();
    for e in &experiences {
        println!(
            "  id={} company={:?} title={:?} location={:?} start={} end={:?} achievements={} responsibilities={} letters={}",
            e._id.map(|id| id.to_hex()).unwrap_or_default(),
            e.company,
            e.job_title,
            e.location,
            e.start_date,
            e.end_date,
            e.achievements.len(),
            e.responsibilities.len(),
            e.recommendation_letter_urls.len()
        );
        for url in &e.recommendation_letter_urls {
            println!("    letter: {url}");
        }
    }
    for p in &projects {
        if let Some(exp) = &p.experience_id
            && !exp.is_empty()
            && !experience_ids.contains(exp)
        {
            println!(
                "  !! project {:?} references unknown experienceId {exp}",
                p.project_title
            );
        }
    }

    let articles = repositories::article::all_sorted().await?;
    println!("articles (createdAt desc): {}", articles.len());
    for a in &articles {
        let regenerated = generate_article_slug(&a.title, a._id.as_ref(), Some(a.created_at));
        let stable = regenerated == a.slug;
        println!(
            "  slug={:?} title={:?} tags={:?} banner={:?} created={} updated={} content_len={} slug_stable_on_resave={}",
            a.slug,
            a.title,
            a.tags,
            a.banner,
            a.created_at,
            a.updated_at,
            a.content.chars().count(),
            stable
        );
        if !stable {
            println!("    !! re-saving in admin would change slug to {regenerated:?}");
        }
    }

    let socials = repositories::social::all().await?;
    println!("socials: {}", socials.len());
    for s in &socials {
        println!("  {} -> {}", s.name.as_str(), s.url);
    }

    let media = repositories::media::all().await?;
    println!("media: {}", media.len());
    let mut url_scheme_mismatches = 0usize;
    for m in &media {
        let expected = format!(
            "https://{}.s3.{}.amazonaws.com/{}",
            config.s3_bucket, config.aws_region, m.file_key
        );
        let matches = m.url == expected;
        if !matches {
            url_scheme_mismatches += 1;
        }
        println!(
            "  {} name={:?} key={:?} created={} url_matches_bucket_scheme={}",
            m.media_type.as_str(),
            m.name,
            m.file_key,
            m.created_at,
            matches
        );
        if !matches {
            println!("    stored url: {}", m.url);
        }
    }
    println!(
        "media URL scheme mismatches vs https://{{bucket}}.s3.{{region}}.amazonaws.com/{{fileKey}}: {url_scheme_mismatches}"
    );
    let media_urls: BTreeSet<&str> = media.iter().map(|m| m.url.as_str()).collect();
    let mut referenced: Vec<(&str, String)> = Vec::new();
    if let Some(a) = &about {
        if let Some(u) = &a.resume_url {
            referenced.push(("about.resumeUrl", u.clone()));
        }
        if let Some(u) = &a.image_url {
            referenced.push(("about.imageUrl", u.clone()));
        }
    }
    for p in &projects {
        for u in &p.media {
            referenced.push(("project.media", u.clone()));
        }
    }
    for e in &experiences {
        for u in &e.recommendation_letter_urls {
            referenced.push(("experience.recommendationLetterUrls", u.clone()));
        }
    }
    for a in &articles {
        if let Some(u) = &a.banner {
            referenced.push(("article.banner", u.clone()));
        }
    }
    for (field, url) in &referenced {
        if !url.is_empty() && !media_urls.contains(url.as_str()) {
            println!("  note: {field} references a URL with no Media record: {url}");
        }
    }

    let contacts = repositories::contact::all().await?;
    let pending = repositories::contact::count_pending().await?;
    let mut by_status: BTreeMap<&str, usize> = BTreeMap::new();
    for c in &contacts {
        *by_status.entry(c.status.as_str()).or_insert(0) += 1;
    }
    println!(
        "contacts: {} (pending via count query: {pending}) by status: {by_status:?}",
        contacts.len()
    );

    let users = db.collection::<Document>("User").find(doc! {}).await?;
    let mut users = users;
    let mut user_summaries = Vec::new();
    while users.advance().await? {
        let document: Document = users.current().try_into()?;
        let user: User = mongodb::bson::from_document(document)?;
        let pw = &user.password;
        let looks_sha512_hex = pw.len() == 128 && pw.chars().all(|c| c.is_ascii_hexdigit());
        user_summaries.push(format!(
            "role={} email_domain={:?} password_len={} sha512_hex_shape={}",
            user.role.as_str(),
            user.email.rsplit('@').next().unwrap_or(""),
            pw.len(),
            looks_sha512_hex
        ));
    }
    println!("users: {}", user_summaries.len());
    for s in &user_summaries {
        println!("  {s}");
    }

    println!("\n== full homepage load through services::portfolio::load_home_data ==");
    let home = load_home_data().await?;
    println!(
        "home: hero_title={:?} projects={} experiences={} recent_articles={} skills={} meta_description_len={}",
        home.hero.title,
        home.projects.len(),
        home.experiences.len(),
        home.recent_articles.len(),
        home.skills.len(),
        home.meta_description.chars().count()
    );

    println!("\n== summary ==");
    if total_failures == 0 {
        println!("all stored documents deserialize into the OxiMod models: OK");
    } else {
        println!("!! {total_failures} document(s) failed to deserialize");
        std::process::exit(2);
    }
    Ok(())
}
