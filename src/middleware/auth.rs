//! Admin authentication guard.
//!
//! Every request under `/admin` must carry a valid, unexpired admin JWT in
//! the `token` cookie (verified server-side, exactly like the previous
//! middleware). Unauthenticated browsers are redirected to `/login`.

use axum::extract::{Request, State};
use axum::middleware::Next;
use axum::response::{IntoResponse, Redirect, Response};
use axum_extra::extract::CookieJar;

use crate::app::AppState;
use crate::services::auth::{AUTH_COOKIE, verify_admin_token};

/// Authenticated admin identity, inserted as a request extension for
/// handlers that need to know who is acting.
#[derive(Debug, Clone)]
pub struct AdminSession {
    pub user_id: String,
    pub email: String,
}

pub async fn require_admin(
    State(state): State<AppState>,
    jar: CookieJar,
    mut request: Request,
    next: Next,
) -> Response {
    let Some(token) = jar.get(AUTH_COOKIE).map(|c| c.value().to_string()) else {
        return Redirect::to("/login").into_response();
    };

    let Some(claims) = verify_admin_token(&state.config.jwt_secret, &token) else {
        tracing::info!("rejected admin request with invalid or expired token");
        return Redirect::to("/login").into_response();
    };

    request.extensions_mut().insert(AdminSession {
        user_id: claims.user_id,
        email: claims.email,
    });
    next.run(request).await
}
