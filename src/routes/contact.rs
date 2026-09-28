//! Public contact-form submission (PRG pattern: POST, then redirect back
//! to the contact section with a flash message in the query string).

use std::net::SocketAddr;

use axum::Form;
use axum::extract::{ConnectInfo, State};
use axum::http::HeaderMap;
use axum::response::Redirect;

use crate::app::AppState;
use crate::error::AppError;
use crate::routes::{client_ip, urlencode};
use crate::services::contact::{self, ContactForm};

fn back_ok(message: &str) -> Redirect {
    Redirect::to(&format!("/?ok={}#contact", urlencode(message)))
}

fn back_err(message: &str) -> Redirect {
    Redirect::to(&format!("/?err={}#contact", urlencode(message)))
}

pub async fn submit(
    State(state): State<AppState>,
    headers: HeaderMap,
    connect_info: ConnectInfo<SocketAddr>,
    Form(form): Form<ContactForm>,
) -> Result<Redirect, AppError> {
    // Honeypot: bots that fill the invisible field get a fake success and
    // nothing is stored.
    if !form.website.trim().is_empty() {
        tracing::info!("contact honeypot triggered; submission dropped");
        return Ok(back_ok("Thanks — your message has been sent."));
    }

    let ip = client_ip(&headers, &connect_info);
    if !state.limiters.contact.check(ip) {
        tracing::warn!("contact form rate limit hit");
        return Ok(back_err("Too many messages — please try again later."));
    }

    if let Err(message) = contact::validate(&form) {
        return Ok(back_err(&message));
    }

    if state.config.enable_recaptcha {
        let secret = state
            .config
            .recaptcha_secret
            .as_deref()
            .ok_or_else(|| AppError::internal("reCAPTCHA enabled without secret"))?;
        if let Err(err) =
            contact::verify_recaptcha(&state.http, secret, &form.recaptcha_token).await
        {
            tracing::info!(error = %err, "reCAPTCHA verification failed");
            return Ok(back_err("Verification failed — please try again."));
        }
    }

    contact::create(&form).await?;
    Ok(back_ok("Thanks — your message has been sent."))
}
