//! Per-page metadata rendered into the document `<head>` by the base
//! template: title, description, canonical URL, Open Graph / Twitter
//! cards, robots directives and JSON-LD blocks.

pub const SITE_NAME: &str = "Arshia Eskandari";

#[derive(Debug, Clone)]
pub struct PageMeta {
    /// Full `<title>` text.
    pub title: String,
    pub description: String,
    /// Absolute canonical URL of the page.
    pub canonical: String,
    /// `website` or `article`.
    pub og_type: &'static str,
    /// Absolute URL of the social preview image, when one exists.
    pub og_image: Option<String>,
    /// Emits `noindex, nofollow` (auth/admin pages).
    pub noindex: bool,
    /// `article:published_time` (articles only).
    pub published_iso: Option<String>,
    /// `article:modified_time` (articles only).
    pub modified_iso: Option<String>,
    /// `article:tag` values (articles only).
    pub tags: Vec<String>,
    /// Pre-serialized JSON-LD documents, one `<script>` block each.
    pub json_ld: Vec<String>,
}

impl PageMeta {
    /// Metadata for a regular indexable page.
    pub fn page(
        title: impl Into<String>,
        description: impl Into<String>,
        canonical: String,
    ) -> Self {
        Self {
            title: title.into(),
            description: description.into(),
            canonical,
            og_type: "website",
            og_image: None,
            noindex: false,
            published_iso: None,
            modified_iso: None,
            tags: Vec::new(),
            json_ld: Vec::new(),
        }
    }

    /// Metadata for authentication/admin pages: never indexed.
    pub fn noindex(title: impl Into<String>, canonical: String) -> Self {
        let mut meta = Self::page(title, "", canonical);
        meta.noindex = true;
        meta
    }

    pub fn with_image(mut self, image: Option<String>) -> Self {
        self.og_image = image;
        self
    }

    pub fn with_json_ld(mut self, block: String) -> Self {
        self.json_ld.push(block);
        self
    }
}

/// Joins the configured base URL and a path into an absolute URL.
pub fn absolute_url(base: &str, path: &str) -> String {
    let base = base.trim_end_matches('/');
    if path.is_empty() || path == "/" {
        return format!("{base}/");
    }
    format!("{base}{path}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_absolute_urls() {
        assert_eq!(absolute_url("https://x.com/", "/"), "https://x.com/");
        assert_eq!(
            absolute_url("https://x.com", "/articles"),
            "https://x.com/articles"
        );
        assert_eq!(
            absolute_url("https://x.com", "/articles/a-b"),
            "https://x.com/articles/a-b"
        );
    }

    #[test]
    fn noindex_pages_are_marked() {
        let meta = PageMeta::noindex("Login", "https://x.com/login".to_string());
        assert!(meta.noindex);
    }
}
