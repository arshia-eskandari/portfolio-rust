//! Atom feed for articles (`/feed.xml`).

use crate::models::Article;
use crate::seo::xml_escape;
use crate::services::format;

pub fn build(base_url: &str, articles: &[Article]) -> String {
    let base = base_url.trim_end_matches('/');
    let updated = articles
        .iter()
        .map(|a| a.updated_at.max(a.created_at))
        .max()
        .map(format::iso)
        .unwrap_or_else(|| "1970-01-01T00:00:00.000Z".to_string());

    let mut xml = String::with_capacity(1024 + articles.len() * 400);
    xml.push_str(r#"<?xml version="1.0" encoding="UTF-8"?>"#);
    xml.push('\n');
    xml.push_str(r#"<feed xmlns="http://www.w3.org/2005/Atom">"#);
    xml.push('\n');
    xml.push_str(&format!(
        "  <title>{} — Articles</title>\n",
        xml_escape(super::metadata::SITE_NAME)
    ));
    xml.push_str(&format!("  <id>{base}/articles</id>\n"));
    xml.push_str(&format!(r#"  <link href="{base}/articles"/>"#));
    xml.push('\n');
    xml.push_str(&format!(r#"  <link rel="self" href="{base}/feed.xml"/>"#));
    xml.push('\n');
    xml.push_str(&format!("  <updated>{updated}</updated>\n"));
    xml.push_str(&format!(
        "  <author><name>{}</name></author>\n",
        xml_escape(super::metadata::SITE_NAME)
    ));

    for article in articles {
        let url = format!("{base}/articles/{}", article.slug);
        let summary = crate::markdown::excerpt(&article.content, 300);
        xml.push_str("  <entry>\n");
        xml.push_str(&format!(
            "    <title>{}</title>\n",
            xml_escape(&article.title)
        ));
        xml.push_str(&format!("    <id>{}</id>\n", xml_escape(&url)));
        xml.push_str(&format!(r#"    <link href="{}"/>"#, xml_escape(&url)));
        xml.push('\n');
        xml.push_str(&format!(
            "    <published>{}</published>\n",
            format::iso(article.created_at)
        ));
        xml.push_str(&format!(
            "    <updated>{}</updated>\n",
            format::iso(article.updated_at)
        ));
        if !summary.is_empty() {
            xml.push_str(&format!(
                "    <summary>{}</summary>\n",
                xml_escape(&summary)
            ));
        }
        for tag in &article.tags {
            xml.push_str(&format!(r#"    <category term="{}"/>"#, xml_escape(tag)));
            xml.push('\n');
        }
        xml.push_str("  </entry>\n");
    }

    xml.push_str("</feed>\n");
    xml
}

#[cfg(test)]
mod tests {
    use super::*;
    use mongodb::bson::DateTime;

    #[test]
    fn builds_valid_feed_with_entries() {
        let articles = vec![Article {
            _id: None,
            title: "Rust & Mongo".to_string(),
            banner: None,
            content: "Some **content** here for the summary.".to_string(),
            created_at: DateTime::from_millis(1_705_320_000_000),
            updated_at: DateTime::from_millis(1_705_320_000_000),
            slug: "rust-mongo-abc123".to_string(),
            tags: vec!["rust".to_string()],
        }];
        let xml = build("https://example.com", &articles);
        assert!(xml.contains("<title>Rust &amp; Mongo</title>"));
        assert!(xml.contains("https://example.com/articles/rust-mongo-abc123"));
        assert!(xml.contains(r#"<category term="rust"/>"#));
        assert!(xml.contains("<updated>2024-01-15T12:00:00.000Z</updated>"));
    }
}
