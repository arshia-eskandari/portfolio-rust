//! `sitemap.xml` generation: static public pages plus every article,
//! with `lastmod` derived from article timestamps.

use crate::models::Article;
use crate::seo::xml_escape;
use crate::services::format;

pub fn build(base_url: &str, articles: &[Article]) -> String {
    let base = base_url.trim_end_matches('/');
    let mut xml = String::with_capacity(1024 + articles.len() * 200);
    xml.push_str(r#"<?xml version="1.0" encoding="UTF-8"?>"#);
    xml.push('\n');
    xml.push_str(r#"<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">"#);
    xml.push('\n');

    // Homepage: lastmod is the newest article change if any exist.
    let newest = articles
        .iter()
        .map(|a| a.updated_at.max(a.created_at))
        .max();
    push_url(
        &mut xml,
        &format!("{base}/"),
        newest.map(format::iso_date).as_deref(),
    );
    push_url(
        &mut xml,
        &format!("{base}/articles"),
        newest.map(format::iso_date).as_deref(),
    );

    for article in articles {
        let lastmod = format::iso_date(article.updated_at.max(article.created_at));
        push_url(
            &mut xml,
            &format!("{base}/articles/{}", article.slug),
            Some(&lastmod),
        );
    }

    xml.push_str("</urlset>\n");
    xml
}

fn push_url(xml: &mut String, loc: &str, lastmod: Option<&str>) {
    xml.push_str("  <url>\n");
    xml.push_str(&format!("    <loc>{}</loc>\n", xml_escape(loc)));
    if let Some(lastmod) = lastmod {
        xml.push_str(&format!("    <lastmod>{}</lastmod>\n", xml_escape(lastmod)));
    }
    xml.push_str("  </url>\n");
}

#[cfg(test)]
mod tests {
    use super::*;
    use mongodb::bson::DateTime;

    fn article(slug: &str, millis: i64) -> Article {
        Article {
            _id: None,
            title: slug.to_string(),
            banner: None,
            content: String::new(),
            created_at: DateTime::from_millis(millis),
            updated_at: DateTime::from_millis(millis),
            slug: slug.to_string(),
            tags: Vec::new(),
        }
    }

    #[test]
    fn includes_static_pages_and_articles_with_lastmod() {
        let articles = vec![article("first-post-abc123", 1_705_320_000_000)];
        let xml = build("https://example.com/", &articles);
        assert!(xml.contains("<loc>https://example.com/</loc>"));
        assert!(xml.contains("<loc>https://example.com/articles</loc>"));
        assert!(xml.contains("<loc>https://example.com/articles/first-post-abc123</loc>"));
        assert!(xml.contains("<lastmod>2024-01-15</lastmod>"));
    }

    #[test]
    fn escapes_xml_entities_in_slugs() {
        let articles = vec![article("a&b", 0)];
        let xml = build("https://example.com", &articles);
        assert!(xml.contains("a&amp;b"));
        assert!(!xml.contains("a&b<"));
    }
}
