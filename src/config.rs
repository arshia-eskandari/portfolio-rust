//! Environment-driven application configuration.
//!
//! The configuration keeps the deployment values used by the previous
//! Next.js application (`DATABASE_URL`, `JWT_SECRET`, `AWS_*`,
//! `S3_BUCKET_NAME`, `RECAPTCHA_SECRET_KEY`) and replaces the
//! Next.js-specific names with clearer equivalents:
//!
//! - `NEXT_PUBLIC_ENABLE_RECAPTCHA` -> `ENABLE_RECAPTCHA`
//!   (the old name is still honored as a fallback);
//! - `HOST_NAME` (used only for the Next.js image loader) is replaced by
//!   `SITE_BASE_URL`, the canonical public origin used for SEO output.
//!
//! New, purely additive values: `APP_HOST`, `APP_PORT`, `APP_ENV`.

use std::fmt;

/// Database name every OxiMod model is compiled against.
///
/// OxiMod resolves the database from the compile-time `#[db(...)]`
/// attribute, so this cannot be changed at runtime. [`Config::from_env`]
/// verifies at startup that the default database in `DATABASE_URL` matches
/// this name and refuses to boot on a mismatch, so the application can never
/// silently read or write a different database than the models declare.
pub const DB_NAME: &str = "portfolio";

#[derive(Clone)]
pub struct Config {
    /// MongoDB connection string (same value the Prisma app used).
    pub database_url: String,
    /// Secret used to sign/verify the admin JWT (HS256, compatible with the
    /// tokens issued by the previous `jose` implementation).
    pub jwt_secret: String,
    /// S3 bucket receiving media uploads.
    pub s3_bucket: String,
    /// AWS region of the bucket (also read by the AWS SDK itself).
    pub aws_region: String,
    /// Canonical public origin, e.g. `https://arshiaeskandari.com`.
    pub site_base_url: String,
    /// Whether contact submissions must pass reCAPTCHA v3 verification.
    pub enable_recaptcha: bool,
    /// Server-side reCAPTCHA secret (required only when enabled).
    pub recaptcha_secret: Option<String>,
    /// reCAPTCHA v3 site key rendered into the public page when enabled.
    pub recaptcha_site_key: Option<String>,
    /// Bind address host part.
    pub host: String,
    /// Bind address port.
    pub port: u16,
    /// `true` when `APP_ENV=production`: enables `Secure` cookies.
    pub production: bool,
}

impl fmt::Debug for Config {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Never expose secrets through Debug/logging.
        f.debug_struct("Config")
            .field("s3_bucket", &self.s3_bucket)
            .field("aws_region", &self.aws_region)
            .field("site_base_url", &self.site_base_url)
            .field("enable_recaptcha", &self.enable_recaptcha)
            .field("host", &self.host)
            .field("port", &self.port)
            .field("production", &self.production)
            .finish_non_exhaustive()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("missing required environment variable {0}")]
    Missing(&'static str),
    #[error("invalid value for {0}: {1}")]
    Invalid(&'static str, String),
    #[error(
        "DATABASE_URL selects database {actual:?} but the application models are compiled \
         against {expected:?}; point DATABASE_URL at {expected:?} or update the #[db(...)] \
         attribute on the models in src/models/"
    )]
    DatabaseNameMismatch {
        expected: &'static str,
        actual: String,
    },
}

fn required(name: &'static str) -> Result<String, ConfigError> {
    match std::env::var(name) {
        Ok(v) if !v.trim().is_empty() => Ok(v),
        _ => Err(ConfigError::Missing(name)),
    }
}

fn optional(name: &str) -> Option<String> {
    std::env::var(name).ok().filter(|v| !v.trim().is_empty())
}

/// Extracts the default database name from a MongoDB connection string.
///
/// The default database is the path segment between the host list and the
/// query string: `mongodb+srv://user:pass@host/dbname?options`.
pub fn database_name_from_url(url: &str) -> Option<String> {
    let after_scheme = url.split_once("://").map(|(_, rest)| rest)?;
    let after_host = after_scheme.split_once('/').map(|(_, rest)| rest)?;
    let db = after_host.split('?').next().unwrap_or("");
    let db = db.trim();
    if db.is_empty() {
        None
    } else {
        Some(db.to_string())
    }
}

impl Config {
    /// Loads configuration from the environment, validating required values
    /// and the compile-time database-name contract.
    pub fn from_env() -> Result<Self, ConfigError> {
        let database_url = required("DATABASE_URL")?;

        // OxiMod models carry the database name at compile time. Refuse to
        // start against a URI that selects a different default database
        // rather than silently splitting reads and writes across databases.
        if let Some(actual) = database_name_from_url(&database_url)
            && actual != DB_NAME
        {
            return Err(ConfigError::DatabaseNameMismatch {
                expected: DB_NAME,
                actual,
            });
        }

        let jwt_secret = required("JWT_SECRET")?;
        if jwt_secret.len() < 16 {
            return Err(ConfigError::Invalid(
                "JWT_SECRET",
                "must be at least 16 characters".to_string(),
            ));
        }

        let enable_recaptcha = optional("ENABLE_RECAPTCHA")
            .or_else(|| optional("NEXT_PUBLIC_ENABLE_RECAPTCHA"))
            .map(|v| v == "true" || v == "1")
            .unwrap_or(false);

        let recaptcha_secret = optional("RECAPTCHA_SECRET_KEY");
        let recaptcha_site_key = optional("RECAPTCHA_SITE_KEY");
        if enable_recaptcha && recaptcha_secret.is_none() {
            return Err(ConfigError::Missing("RECAPTCHA_SECRET_KEY"));
        }

        let port = match optional("APP_PORT").or_else(|| optional("PORT")) {
            Some(raw) => raw
                .parse::<u16>()
                .map_err(|e| ConfigError::Invalid("APP_PORT", e.to_string()))?,
            None => 3000,
        };

        let site_base_url =
            optional("SITE_BASE_URL").unwrap_or_else(|| "https://arshiaeskandari.com".to_string());
        let site_base_url = site_base_url.trim_end_matches('/').to_string();

        Ok(Self {
            database_url,
            jwt_secret,
            s3_bucket: required("S3_BUCKET_NAME")?,
            aws_region: optional("AWS_REGION").unwrap_or_else(|| "us-east-2".to_string()),
            site_base_url,
            enable_recaptcha,
            recaptcha_secret,
            recaptcha_site_key,
            host: optional("APP_HOST").unwrap_or_else(|| "0.0.0.0".to_string()),
            port,
            production: optional("APP_ENV").as_deref() == Some("production"),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_database_name_from_srv_url() {
        assert_eq!(
            database_name_from_url(
                "mongodb+srv://u:p@cluster0.x.mongodb.net/portfolio?retryWrites=true"
            ),
            Some("portfolio".to_string())
        );
    }

    #[test]
    fn extracts_database_name_from_plain_url() {
        assert_eq!(
            database_name_from_url("mongodb://localhost:27017/portfolio"),
            Some("portfolio".to_string())
        );
    }

    #[test]
    fn missing_database_name_is_none() {
        assert_eq!(database_name_from_url("mongodb://localhost:27017"), None);
        assert_eq!(database_name_from_url("mongodb://localhost:27017/"), None);
        assert_eq!(
            database_name_from_url("mongodb://localhost:27017/?w=majority"),
            None
        );
    }
}
