//! JSON-LD builders. Only truthful values derived from stored content and
//! configuration are emitted — no fabricated credentials or ratings.

use serde_json::{Value, json};

use crate::services::articles::ArticlePageView;
use crate::services::portfolio::SocialLinks;

/// Serializes a JSON-LD document for embedding inside a `<script>` block.
/// `<` is escaped so content can never terminate the script element.
fn finish(value: Value) -> String {
    value.to_string().replace('<', "\\u003c")
}

/// `Person` for the homepage, using the about image and social profiles.
pub fn person(base_url: &str, image: Option<&str>, socials: &SocialLinks) -> String {
    let mut same_as: Vec<&String> = Vec::new();
    if let Some(url) = &socials.linkedin {
        same_as.push(url);
    }
    if let Some(url) = &socials.github {
        same_as.push(url);
    }
    if let Some(url) = &socials.telegram {
        same_as.push(url);
    }

    let mut value = json!({
        "@context": "https://schema.org",
        "@type": "Person",
        "name": super::metadata::SITE_NAME,
        "url": format!("{base_url}/"),
    });
    if let Some(image) = image {
        value["image"] = Value::String(image.to_string());
    }
    if !same_as.is_empty() {
        value["sameAs"] = json!(same_as);
    }
    finish(value)
}

/// `WebSite` for the homepage.
pub fn website(base_url: &str) -> String {
    finish(json!({
        "@context": "https://schema.org",
        "@type": "WebSite",
        "name": super::metadata::SITE_NAME,
        "url": format!("{base_url}/"),
    }))
}

/// `BlogPosting` for an article page.
pub fn blog_posting(base_url: &str, article: &ArticlePageView) -> String {
    let canonical = format!("{base_url}/articles/{}", article.slug);
    let mut value = json!({
        "@context": "https://schema.org",
        "@type": "BlogPosting",
        "headline": article.title,
        "description": article.description,
        "author": {
            "@type": "Person",
            "name": super::metadata::SITE_NAME,
            "url": format!("{base_url}/"),
        },
        "datePublished": article.published_iso,
        "dateModified": article.updated_iso,
        "mainEntityOfPage": canonical,
        "url": canonical,
    });
    if let Some(banner) = &article.banner {
        value["image"] = Value::String(banner.clone());
    }
    if !article.tags.is_empty() {
        value["keywords"] = json!(article.tags.join(", "));
    }
    finish(value)
}

/// `BreadcrumbList` matching the visible article-page navigation
/// (Home → Articles → current article).
pub fn article_breadcrumbs(base_url: &str, article_title: &str, slug: &str) -> String {
    finish(json!({
        "@context": "https://schema.org",
        "@type": "BreadcrumbList",
        "itemListElement": [
            {
                "@type": "ListItem",
                "position": 1,
                "name": "Home",
                "item": format!("{base_url}/"),
            },
            {
                "@type": "ListItem",
                "position": 2,
                "name": "Articles",
                "item": format!("{base_url}/articles"),
            },
            {
                "@type": "ListItem",
                "position": 3,
                "name": article_title,
                "item": format!("{base_url}/articles/{slug}"),
            },
        ],
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn person_includes_social_profiles() {
        let socials = SocialLinks {
            github: Some("https://github.com/arshia-eskandari".to_string()),
            ..Default::default()
        };
        let block = person("https://example.com", None, &socials);
        let parsed: Value = serde_json::from_str(&block).expect("valid JSON");
        assert_eq!(parsed["@type"], "Person");
        assert_eq!(parsed["sameAs"][0], "https://github.com/arshia-eskandari");
        assert!(parsed.get("image").is_none(), "no fabricated image");
    }

    #[test]
    fn breadcrumbs_have_three_levels() {
        let block = article_breadcrumbs("https://example.com", "Post", "post-abc123");
        let parsed: Value = serde_json::from_str(&block).expect("valid JSON");
        assert_eq!(
            parsed["itemListElement"].as_array().map(|a| a.len()),
            Some(3)
        );
    }
}
