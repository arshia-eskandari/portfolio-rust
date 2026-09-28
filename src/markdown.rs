//! Server-side Markdown rendering for articles.
//!
//! Uses comrak with GitHub-flavored extensions and syntect-based syntax
//! highlighting. Raw HTML in article content is never passed through
//! (`render.unsafe_` stays `false`), and comrak also suppresses dangerous
//! link protocols, so Markdown cannot introduce XSS. Highlighting is
//! emitted as CSS classes (`static/css/highlight.css`), keeping the
//! Content Security Policy free of inline-style exceptions.

use std::sync::OnceLock;

use comrak::options::Plugins;
use comrak::plugins::syntect::{SyntectAdapter, SyntectAdapterBuilder};
use comrak::{Options, markdown_to_html_with_plugins};

fn adapter() -> &'static SyntectAdapter {
    static ADAPTER: OnceLock<SyntectAdapter> = OnceLock::new();
    ADAPTER.get_or_init(|| SyntectAdapterBuilder::new().css().build())
}

fn options() -> Options<'static> {
    let mut options = Options::default();
    options.extension.table = true;
    options.extension.strikethrough = true;
    options.extension.autolink = true;
    options.extension.tasklist = true;
    options.extension.footnotes = true;
    // `render.unsafe_` stays false: raw HTML is escaped, dangerous URLs
    // (javascript:, data:) are suppressed.
    options
}

/// Renders article Markdown to safe HTML with syntax-highlighted code.
pub fn render(markdown: &str) -> String {
    let mut plugins = Plugins::default();
    plugins.render.codefence_syntax_highlighter = Some(adapter());
    markdown_to_html_with_plugins(markdown, &options(), &plugins)
}

/// Derives a plain-text excerpt from Markdown for meta descriptions.
///
/// Strips code fences, headings markers, emphasis, images and link targets,
/// then truncates to at most `max_chars` characters on a word boundary.
pub fn excerpt(markdown: &str, max_chars: usize) -> String {
    let mut text = String::new();
    let mut in_fence = false;

    for line in markdown.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
            in_fence = !in_fence;
            continue;
        }
        if in_fence || trimmed.is_empty() || trimmed.starts_with('|') {
            continue;
        }
        let stripped = strip_inline(trimmed);
        if stripped.is_empty() {
            continue;
        }
        if !text.is_empty() {
            text.push(' ');
        }
        text.push_str(&stripped);
        if text.chars().count() >= max_chars {
            break;
        }
    }

    truncate_on_word(&text, max_chars)
}

fn strip_inline(line: &str) -> String {
    // Drop leading heading/quote/list markers.
    let mut rest = line;
    while let Some(stripped) = rest
        .strip_prefix('#')
        .or_else(|| rest.strip_prefix('>'))
        .or_else(|| rest.strip_prefix('-'))
        .or_else(|| rest.strip_prefix('*'))
    {
        rest = stripped.trim_start();
    }

    let mut out = String::with_capacity(rest.len());
    let mut chars = rest.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '`' | '*' | '_' | '~' => {}
            '!' if chars.peek() == Some(&'[') => {
                // Skip the whole image: ![alt](url)
                skip_bracketed(&mut chars);
            }
            '[' => {
                // Keep link text, drop the (url) part.
                for inner in chars.by_ref() {
                    if inner == ']' {
                        break;
                    }
                    out.push(inner);
                }
                if chars.peek() == Some(&'(') {
                    for inner in chars.by_ref() {
                        if inner == ')' {
                            break;
                        }
                    }
                }
            }
            _ => out.push(c),
        }
    }
    out.trim().to_string()
}

fn skip_bracketed(chars: &mut std::iter::Peekable<std::str::Chars<'_>>) {
    // Consumes "[...](...)" from the iterator (the '!' was already taken).
    for c in chars.by_ref() {
        if c == ']' {
            break;
        }
    }
    if chars.peek() == Some(&'(') {
        for c in chars.by_ref() {
            if c == ')' {
                break;
            }
        }
    }
}

fn truncate_on_word(text: &str, max_chars: usize) -> String {
    if text.chars().count() <= max_chars {
        return text.to_string();
    }
    let truncated: String = text.chars().take(max_chars).collect();
    let cut = truncated.rfind(' ').unwrap_or(truncated.len());
    let mut out = truncated[..cut].trim_end().to_string();
    out.push('…');
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_gfm_tables_and_code() {
        let html = render("| a | b |\n|---|---|\n| 1 | 2 |\n\n```rust\nfn main() {}\n```\n");
        assert!(html.contains("<table>"), "tables should render: {html}");
        assert!(html.contains("<pre"), "code fence should render: {html}");
    }

    #[test]
    fn escapes_raw_html() {
        let html = render("hello <script>alert(1)</script> world");
        assert!(
            !html.contains("<script>"),
            "raw HTML must not pass through: {html}"
        );
    }

    #[test]
    fn suppresses_javascript_links() {
        let html = render("[click](javascript:alert(1))");
        assert!(
            !html.contains("javascript:"),
            "dangerous protocol must be stripped: {html}"
        );
    }

    #[test]
    fn excerpt_strips_markdown_syntax() {
        let md = "# Title\n\nSome **bold** text with a [link](https://example.com) and `code`.\n\n```rust\nfn hidden() {}\n```\n";
        let text = excerpt(md, 200);
        assert_eq!(text, "Title Some bold text with a link and code.");
    }

    #[test]
    fn excerpt_truncates_on_word_boundary() {
        let md = "one two three four five six seven";
        let text = excerpt(md, 12);
        assert_eq!(text, "one two…");
    }

    #[test]
    fn excerpt_skips_images() {
        let md = "![banner](https://cdn.example.com/x.png) Real text";
        assert_eq!(excerpt(md, 100), "Real text");
    }
}
