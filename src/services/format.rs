//! Date formatting helpers for view models.
//!
//! Public article dates are rendered in the site's home timezone
//! (America/Toronto), matching the previous application's formatter.

use chrono::{DateTime as ChronoDateTime, Datelike, Utc};
use chrono_tz::America::Toronto;
use mongodb::bson::DateTime;

const MONTHS_LONG: [&str; 12] = [
    "January",
    "February",
    "March",
    "April",
    "May",
    "June",
    "July",
    "August",
    "September",
    "October",
    "November",
    "December",
];

const MONTHS_SHORT: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];

fn to_chrono(dt: DateTime) -> ChronoDateTime<Utc> {
    ChronoDateTime::<Utc>::from_timestamp_millis(dt.timestamp_millis())
        .unwrap_or(ChronoDateTime::<Utc>::UNIX_EPOCH)
}

/// `September 27, 2026` (long month, Toronto time) — article bylines.
pub fn long_date(dt: DateTime) -> String {
    let local = to_chrono(dt).with_timezone(&Toronto);
    format!(
        "{} {}, {}",
        MONTHS_LONG[local.month0() as usize],
        local.day(),
        local.year()
    )
}

/// `Sep 2026` (short month, UTC) — experience date ranges.
pub fn month_year(dt: DateTime) -> String {
    let utc = to_chrono(dt);
    format!("{} {}", MONTHS_SHORT[utc.month0() as usize], utc.year())
}

/// `2026-09-27T12:34:56.789Z` — `<time datetime>` attributes and JSON-LD.
pub fn iso(dt: DateTime) -> String {
    to_chrono(dt).to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
}

/// `2026-09-27` — sitemap `<lastmod>` and date inputs.
pub fn iso_date(dt: DateTime) -> String {
    let utc = to_chrono(dt);
    format!("{:04}-{:02}-{:02}", utc.year(), utc.month(), utc.day())
}

/// Full timestamp for the admin contact inbox, e.g. `Sep 27, 2026 14:03` (UTC).
pub fn admin_timestamp(dt: DateTime) -> String {
    let utc = to_chrono(dt);
    format!(
        "{} {}, {} {:02}:{:02}",
        MONTHS_SHORT[utc.month0() as usize],
        utc.day(),
        utc.year(),
        chrono::Timelike::hour(&utc),
        chrono::Timelike::minute(&utc),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_dates() {
        // 2024-01-15T12:00:00Z
        let dt = DateTime::from_millis(1_705_320_000_000);
        assert_eq!(long_date(dt), "January 15, 2024");
        assert_eq!(month_year(dt), "Jan 2024");
        assert_eq!(iso_date(dt), "2024-01-15");
        assert_eq!(iso(dt), "2024-01-15T12:00:00.000Z");
    }

    #[test]
    fn long_date_uses_toronto_timezone() {
        // 2024-01-15T03:00:00Z is Jan 14, 22:00 in Toronto (UTC-5).
        let dt = DateTime::from_millis(1_705_287_600_000);
        assert_eq!(long_date(dt), "January 14, 2024");
    }
}
