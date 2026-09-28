//! Contact-form validation, abuse protection and persistence.

use serde::Deserialize;

use crate::error::AppError;

/// Raw form payload. `website` is a honeypot field rendered invisibly on
/// the public form; humans leave it empty, naive bots fill it in.
#[derive(Debug, Default, Deserialize)]
pub struct ContactForm {
    #[serde(default)]
    pub first_name: String,
    #[serde(default)]
    pub last_name: String,
    #[serde(default)]
    pub email: String,
    #[serde(default)]
    pub message: String,
    #[serde(default)]
    pub website: String,
    #[serde(default, rename = "g-recaptcha-response")]
    pub recaptcha_token: String,
}

/// Validates the submission.
///
/// The rules improve on the previous implementation's oddities (a
/// five-character minimum for every first name) without touching the data
/// model: names 1–50 chars, plausible email up to 100 chars, message
/// 10–2000 chars.
pub fn validate(form: &ContactForm) -> Result<(), String> {
    let first = form.first_name.trim();
    let last = form.last_name.trim();
    let email = form.email.trim();
    let message = form.message.trim();

    if first.is_empty() || first.chars().count() > 50 {
        return Err("First name must be 1 to 50 characters.".to_string());
    }
    if last.is_empty() || last.chars().count() > 50 {
        return Err("Last name must be 1 to 50 characters.".to_string());
    }
    if !is_plausible_email(email) || email.chars().count() > 100 {
        return Err("Please enter a valid email address.".to_string());
    }
    if message.chars().count() < 10 || message.chars().count() > 2000 {
        return Err("Message must be 10 to 2000 characters.".to_string());
    }
    Ok(())
}

fn is_plausible_email(email: &str) -> bool {
    let Some((local, domain)) = email.split_once('@') else {
        return false;
    };
    !local.is_empty()
        && !domain.is_empty()
        && domain.contains('.')
        && !domain.starts_with('.')
        && !domain.ends_with('.')
        && !email.contains(char::is_whitespace)
}

#[derive(Debug, Deserialize)]
struct RecaptchaResponse {
    success: bool,
    #[serde(default)]
    score: Option<f64>,
}

/// Verifies a reCAPTCHA v3 token with Google. Returns `Ok(())` when the
/// token is valid with an acceptable score (>= 0.5, like the previous
/// implementation).
pub async fn verify_recaptcha(
    http: &reqwest::Client,
    secret: &str,
    token: &str,
) -> Result<(), AppError> {
    let response = http
        .post("https://www.google.com/recaptcha/api/siteverify")
        .form(&[("secret", secret), ("response", token)])
        .send()
        .await
        .map_err(|e| AppError::internal(format!("recaptcha request failed: {e}")))?;

    let body: RecaptchaResponse = response
        .json()
        .await
        .map_err(|e| AppError::internal(format!("recaptcha response parse failed: {e}")))?;

    if body.success && body.score.unwrap_or(0.0) >= 0.5 {
        Ok(())
    } else {
        Err(AppError::BadRequest(
            "reCAPTCHA verification failed.".to_string(),
        ))
    }
}

/// Persists a validated submission.
pub async fn create(form: &ContactForm) -> Result<(), AppError> {
    crate::repositories::contact::create(
        form.first_name.trim(),
        form.last_name.trim(),
        form.email.trim(),
        form.message.trim(),
    )
    .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid_form() -> ContactForm {
        ContactForm {
            first_name: "Ada".to_string(),
            last_name: "Lovelace".to_string(),
            email: "ada@example.com".to_string(),
            message: "I would like to talk about a role.".to_string(),
            website: String::new(),
            recaptcha_token: String::new(),
        }
    }

    #[test]
    fn accepts_valid_submission() {
        assert!(validate(&valid_form()).is_ok());
    }

    #[test]
    fn accepts_short_names_unlike_the_old_rules() {
        let mut form = valid_form();
        form.first_name = "Al".to_string();
        assert!(validate(&form).is_ok(), "two-letter names must be allowed");
    }

    #[test]
    fn rejects_empty_names_and_bad_email() {
        let mut form = valid_form();
        form.first_name = "  ".to_string();
        assert!(validate(&form).is_err());

        let mut form = valid_form();
        form.email = "not-an-email".to_string();
        assert!(validate(&form).is_err());

        let mut form = valid_form();
        form.email = "a@b".to_string();
        assert!(validate(&form).is_err(), "domain without dot rejected");
    }

    #[test]
    fn rejects_too_short_or_too_long_message() {
        let mut form = valid_form();
        form.message = "hi".to_string();
        assert!(validate(&form).is_err());

        let mut form = valid_form();
        form.message = "x".repeat(2001);
        assert!(validate(&form).is_err());
    }
}
