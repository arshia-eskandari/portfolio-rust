//! Application error type and its mapping onto HTTP responses.

use askama::Template;
use askama_web::WebTemplate;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Redirect, Response};

/// Application-wide error type.
///
/// Handlers return `Result<_, AppError>`; the [`IntoResponse`] impl maps
/// each variant to an intentional HTTP response without leaking internal
/// details (connection strings, secrets, driver messages) to clients.
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    /// The requested resource does not exist (rendered as the custom 404).
    #[error("not found")]
    NotFound,

    /// The client sent an invalid request or form payload.
    #[error("bad request: {0}")]
    BadRequest(String),

    /// The client is not authenticated for the requested admin resource.
    /// Browsers are redirected to the login page.
    #[error("unauthenticated")]
    Unauthenticated,

    /// Upload with an unsupported or spoofed media type.
    #[error("unsupported media type")]
    UnsupportedMediaType,

    /// Database failure. The internal error is logged, never surfaced.
    #[error("database error: {0}")]
    Database(#[from] oximod::OxiModError),

    /// Any other internal failure (S3, rendering, IO, ...).
    #[error("internal error: {0}")]
    Internal(String),
}

impl AppError {
    /// Convenience constructor for internal errors from arbitrary sources.
    pub fn internal(err: impl std::fmt::Display) -> Self {
        Self::Internal(err.to_string())
    }
}

#[derive(Template, WebTemplate)]
#[template(path = "404.html")]
pub struct NotFoundTemplate;

#[derive(Template, WebTemplate)]
#[template(path = "error.html")]
pub struct ServerErrorTemplate;

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        match self {
            AppError::NotFound => (StatusCode::NOT_FOUND, NotFoundTemplate).into_response(),
            AppError::BadRequest(message) => (StatusCode::BAD_REQUEST, message).into_response(),
            AppError::Unauthenticated => Redirect::to("/login").into_response(),
            AppError::UnsupportedMediaType => {
                (StatusCode::UNSUPPORTED_MEDIA_TYPE, "Unsupported file type").into_response()
            }
            AppError::Database(err) => {
                tracing::error!(error = %err, "database operation failed");
                (StatusCode::INTERNAL_SERVER_ERROR, ServerErrorTemplate).into_response()
            }
            AppError::Internal(err) => {
                tracing::error!(error = %err, "internal error");
                (StatusCode::INTERNAL_SERVER_ERROR, ServerErrorTemplate).into_response()
            }
        }
    }
}
