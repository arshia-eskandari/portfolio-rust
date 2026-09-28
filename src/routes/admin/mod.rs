//! Admin area handlers.
//!
//! Every route here sits behind [`crate::middleware::auth::require_admin`].
//! Pages are server-rendered forms; mutations are plain POSTs answered
//! with a redirect carrying a flash message (`?ok=` / `?err=`), so the
//! admin works without any client-side framework.

pub mod about;
pub mod articles;
pub mod contacts;
pub mod dashboard;
pub mod experiences;
pub mod hero;
pub mod media;
pub mod projects;
pub mod skills;
pub mod socials;

use axum::response::Redirect;
use chrono::{Datelike, NaiveDate};
use mongodb::bson::DateTime;
use mongodb::bson::oid::ObjectId;

use crate::error::AppError;
use crate::routes::redirect_flash;
use crate::seo::metadata::PageMeta;

/// Multi-sentence list separator used by the previous admin UI for
/// achievements, responsibilities and key results. Kept identical so the
/// editing workflow (and the stored data it produces) does not change.
pub const SENTENCE_SEPARATOR: &str = " | ";

pub fn admin_meta(title: &str) -> PageMeta {
    // Canonical is irrelevant for noindex admin pages; keep it relative-free.
    PageMeta::noindex(format!("{title} — Admin"), String::new())
}

pub fn parse_object_id(hex: &str) -> Result<ObjectId, AppError> {
    ObjectId::parse_str(hex).map_err(|_| AppError::BadRequest("Invalid id".to_string()))
}

/// Splits a comma-separated input into trimmed, non-empty items.
pub fn csv_list(input: &str) -> Vec<String> {
    input
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(String::from)
        .collect()
}

/// Splits a `" | "`-separated input into trimmed, non-empty items.
pub fn sentence_list(input: &str) -> Vec<String> {
    input
        .split('|')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(String::from)
        .collect()
}

/// Parses an `<input type="date">` value (`YYYY-MM-DD`) into a BSON date
/// at midnight UTC.
pub fn parse_date(input: &str) -> Option<DateTime> {
    let date = NaiveDate::parse_from_str(input.trim(), "%Y-%m-%d").ok()?;
    let millis = date.and_hms_opt(0, 0, 0)?.and_utc().timestamp_millis();
    Some(DateTime::from_millis(millis))
}

/// Formats a BSON date back into an `<input type="date">` value.
pub fn date_input_value(dt: DateTime) -> String {
    let utc = chrono::DateTime::<chrono::Utc>::from_timestamp_millis(dt.timestamp_millis())
        .unwrap_or(chrono::DateTime::<chrono::Utc>::UNIX_EPOCH);
    format!("{:04}-{:02}-{:02}", utc.year(), utc.month(), utc.day())
}

pub fn redirect_ok(path: &str, message: &str) -> Redirect {
    Redirect::to(&redirect_flash(path, Some(message), None))
}

pub fn redirect_err(path: &str, message: &str) -> Redirect {
    Redirect::to(&redirect_flash(path, None, Some(message)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_and_formats_dates() {
        let dt = parse_date("2024-01-15").expect("valid date");
        assert_eq!(date_input_value(dt), "2024-01-15");
        assert!(parse_date("not-a-date").is_none());
        assert!(parse_date("").is_none());
    }

    #[test]
    fn splits_lists() {
        assert_eq!(csv_list(" a, b ,, c "), vec!["a", "b", "c"]);
        assert_eq!(
            sentence_list("one | two |  | three"),
            vec!["one", "two", "three"]
        );
        assert!(csv_list("").is_empty());
    }
}
