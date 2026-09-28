//! SEO infrastructure: per-page metadata, JSON-LD structured data,
//! sitemap, robots directives and the Atom feed.

pub mod feed;
pub mod metadata;
pub mod sitemap;
pub mod structured_data;

/// Minimal XML text escaping for hand-assembled XML documents.
pub fn xml_escape(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for c in input.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            _ => out.push(c),
        }
    }
    out
}
