//! Article slug generation.
//!
//! Faithful port of the previous implementation (`src/lib/slug.ts` in the
//! Next.js app) so that editing an article's title through the new admin
//! regenerates exactly the slug the old application would have produced,
//! and unchanged titles keep their existing stored slugs.

use mongodb::bson::DateTime;
use mongodb::bson::oid::ObjectId;
use unicode_normalization::UnicodeNormalization;
use unicode_normalization::char::is_combining_mark;

const DEFAULT_MAX_LENGTH: usize = 64;
const DEFAULT_SUFFIX_LENGTH: usize = 6;

/// Normalizes arbitrary text into a lowercase dash-separated slug body.
///
/// Mirrors the JS `slugifyText`: NFKD-normalize, strip combining marks,
/// lowercase, collapse every non-`[a-z0-9]` run into a single `-`, and trim
/// leading/trailing dashes.
fn slugify_text(input: &str) -> String {
    let normalized: String = input.nfkd().filter(|c| !is_combining_mark(*c)).collect();
    let lower = normalized.to_lowercase();

    let mut slug = String::with_capacity(lower.len());
    let mut pending_dash = false;
    for c in lower.chars() {
        if c.is_ascii_lowercase() || c.is_ascii_digit() {
            if pending_dash && !slug.is_empty() {
                slug.push('-');
            }
            pending_dash = false;
            slug.push(c);
        } else {
            pending_dash = true;
        }
    }
    slug
}

/// Builds the short stable suffix appended to every slug.
///
/// Mirrors the JS `makeShortSuffix`: prefer the hex tail of the document id,
/// then a base-36 encoding of the creation timestamp, then randomness (only
/// used for provisional slugs before an id exists).
fn make_short_suffix(id: Option<&ObjectId>, created_at: Option<DateTime>, length: usize) -> String {
    if let Some(id) = id {
        let clean: String = id
            .to_hex()
            .chars()
            .filter(|c| c.is_ascii_hexdigit())
            .collect();
        let tail: String = clean
            .chars()
            .rev()
            .take(length)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect();
        return if tail.is_empty() { clean } else { tail };
    }

    if let Some(created_at) = created_at {
        let seconds = created_at.timestamp_millis().div_euclid(1000);
        let encoded = to_base36(seconds.unsigned_abs());
        let tail: String = encoded
            .chars()
            .rev()
            .take(length)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect();
        return tail;
    }

    // Random fallback: base-36 alphabet, like Math.random().toString(36).
    let alphabet = b"0123456789abcdefghijklmnopqrstuvwxyz";
    let mut out = String::with_capacity(length);
    while out.len() < length {
        let byte = uuid::Uuid::new_v4().as_bytes()[out.len() % 16] as usize;
        out.push(alphabet[byte % alphabet.len()] as char);
    }
    out
}

fn to_base36(mut n: u64) -> String {
    if n == 0 {
        return "0".to_string();
    }
    let alphabet = b"0123456789abcdefghijklmnopqrstuvwxyz";
    let mut digits = Vec::new();
    while n > 0 {
        digits.push(alphabet[(n % 36) as usize]);
        n /= 36;
    }
    digits.reverse();
    String::from_utf8(digits).expect("base36 output is ASCII")
}

/// Generates the canonical slug for an article.
///
/// Equivalent to the JS `generateArticleSlug` with default options:
/// `{base truncated to fit}-{6-char suffix}`, total length <= 64.
pub fn generate_article_slug(
    title: &str,
    id: Option<&ObjectId>,
    created_at: Option<DateTime>,
) -> String {
    generate_article_slug_with(
        title,
        id,
        created_at,
        DEFAULT_MAX_LENGTH,
        DEFAULT_SUFFIX_LENGTH,
    )
}

/// Slug generation with explicit limits (used for provisional slugs, which
/// the old application created with a 12-character random suffix).
pub fn generate_article_slug_with(
    title: &str,
    id: Option<&ObjectId>,
    created_at: Option<DateTime>,
    max_length: usize,
    suffix_length: usize,
) -> String {
    let title = if title.is_empty() { "untitled" } else { title };
    let base = slugify_text(title);
    let base = if base.is_empty() {
        slugify_text("untitled")
    } else {
        base
    };

    let suffix = make_short_suffix(id, created_at, suffix_length);

    let reserved = suffix.len() + 1;
    let available = max_length.saturating_sub(reserved).max(1);
    let head: String = base.chars().take(available).collect();
    let head = head.trim_end_matches('-');

    format!("{head}-{suffix}").to_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn oid(hex: &str) -> ObjectId {
        ObjectId::parse_str(hex).expect("valid ObjectId hex")
    }

    #[test]
    fn slugifies_basic_titles() {
        assert_eq!(slugify_text("Hello, World!"), "hello-world");
        assert_eq!(slugify_text("  Rust & MongoDB  "), "rust-mongodb");
        assert_eq!(slugify_text("---"), "");
    }

    #[test]
    fn strips_diacritics_like_nfkd() {
        assert_eq!(slugify_text("Éléphant à côté"), "elephant-a-cote");
    }

    #[test]
    fn suffix_uses_last_six_hex_chars_of_id() {
        let id = oid("665f01234567890123abcdef");
        assert_eq!(make_short_suffix(Some(&id), None, 6), "abcdef");
    }

    #[test]
    fn generates_slug_with_id_suffix() {
        let id = oid("665f01234567890123abcdef");
        assert_eq!(
            generate_article_slug("Building OxiMod", Some(&id), None),
            "building-oximod-abcdef"
        );
    }

    #[test]
    fn identical_title_and_id_produce_stable_slug() {
        let id = oid("665f01234567890123abcdef");
        let a = generate_article_slug("Some Title", Some(&id), None);
        let b = generate_article_slug("Some Title", Some(&id), None);
        assert_eq!(a, b);
    }

    #[test]
    fn truncates_long_titles_to_max_length() {
        let id = oid("665f01234567890123abcdef");
        let title = "a".repeat(200);
        let slug = generate_article_slug(&title, Some(&id), None);
        assert!(slug.len() <= 64, "slug too long: {} chars", slug.len());
        assert!(slug.ends_with("-abcdef"));
    }

    #[test]
    fn truncation_does_not_leave_double_dash() {
        let id = oid("665f01234567890123abcdef");
        // 57 chars of head budget; engineer the boundary to land on a dash.
        let title = format!("{} {}", "a".repeat(56), "b".repeat(20));
        let slug = generate_article_slug(&title, Some(&id), None);
        assert!(!slug.contains("--"), "unexpected double dash in {slug}");
    }

    #[test]
    fn created_at_fallback_uses_base36_seconds() {
        let created = DateTime::from_millis(1_700_000_000_000);
        let suffix = make_short_suffix(None, Some(created), 6);
        // 1700000000 seconds in base36 == "s44we8" (matches JS toString(36)).
        assert_eq!(suffix, "s44we8");
    }

    #[test]
    fn empty_title_falls_back_to_untitled() {
        let id = oid("665f01234567890123abcdef");
        assert_eq!(
            generate_article_slug("", Some(&id), None),
            "untitled-abcdef"
        );
    }

    #[test]
    fn provisional_slug_uses_random_12_char_suffix() {
        let slug = generate_article_slug_with("Title", None, None, 64, 12);
        let (head, suffix) = slug.rsplit_once('-').expect("slug has suffix");
        assert_eq!(head, "title");
        assert_eq!(suffix.len(), 12);
    }
}
