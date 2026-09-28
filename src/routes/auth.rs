//! Admin login and logout.

use std::net::SocketAddr;

use askama::Template;
use askama_web::WebTemplate;
use axum::Form;
use axum::extract::{ConnectInfo, Query, State};
use axum::http::HeaderMap;
use axum::response::{IntoResponse, Redirect, Response};
use axum_extra::extract::CookieJar;
use axum_extra::extract::cookie::{Cookie, SameSite};
use serde::Deserialize;

use crate::app::AppState;
use crate::error::AppError;
use crate::models::Role;
use crate::repositories;
use crate::routes::{client_ip, urlencode};
use crate::seo::metadata::{PageMeta, absolute_url};
use crate::services::auth::{AUTH_COOKIE, TOKEN_TTL_SECONDS, create_token, verify_password};

#[derive(Template, WebTemplate)]
#[template(path = "login.html")]
pub struct LoginTemplate {
    pub meta: PageMeta,
    pub error: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct LoginQuery {
    #[serde(default)]
    pub err: Option<String>,
}

pub async fn login_page(
    State(state): State<AppState>,
    Query(query): Query<LoginQuery>,
) -> LoginTemplate {
    LoginTemplate {
        meta: PageMeta::noindex(
            "Log in",
            absolute_url(&state.config.site_base_url, "/login"),
        ),
        error: query.err,
    }
}

#[derive(Debug, Deserialize)]
pub struct LoginForm {
    #[serde(default)]
    pub email: String,
    #[serde(default)]
    pub password: String,
}

fn login_failed(message: &str) -> Response {
    Redirect::to(&format!("/login?err={}", urlencode(message))).into_response()
}

pub async fn login_submit(
    State(state): State<AppState>,
    headers: HeaderMap,
    connect_info: ConnectInfo<SocketAddr>,
    jar: CookieJar,
    Form(form): Form<LoginForm>,
) -> Result<Response, AppError> {
    let ip = client_ip(&headers, &connect_info);
    if !state.limiters.login.check(ip) {
        tracing::warn!("login rate limit hit");
        return Ok(login_failed(
            "Too many attempts — please wait a few minutes.",
        ));
    }

    let email = form.email.trim();
    if email.is_empty() || form.password.is_empty() {
        return Ok(login_failed("Email and password are required."));
    }

    let Some(user) = repositories::user::find_by_email(email).await? else {
        tracing::info!("login failed: unknown email");
        return Ok(login_failed("Incorrect email or password."));
    };

    if !verify_password(&form.password, &user.password) {
        tracing::info!("login failed: wrong password");
        return Ok(login_failed("Incorrect email or password."));
    }

    if user.role != Role::Admin {
        tracing::info!("login failed: non-admin role");
        return Ok(login_failed("Unauthorized."));
    }

    let user_id = user._id.map(|id| id.to_hex()).unwrap_or_default();
    let token = create_token(
        &state.config.jwt_secret,
        &user_id,
        &user.email,
        user.role.as_str(),
    )
    .map_err(|e| AppError::internal(format!("token creation failed: {e}")))?;

    let cookie = Cookie::build((AUTH_COOKIE, token))
        .path("/")
        .http_only(true)
        .secure(state.config.production)
        .same_site(SameSite::Strict)
        .max_age(time_duration(TOKEN_TTL_SECONDS))
        .build();

    tracing::info!("admin login succeeded");
    Ok((jar.add(cookie), Redirect::to("/admin")).into_response())
}

pub async fn logout(State(state): State<AppState>, jar: CookieJar) -> Response {
    let cookie = Cookie::build((AUTH_COOKIE, ""))
        .path("/")
        .http_only(true)
        .secure(state.config.production)
        .same_site(SameSite::Strict)
        .max_age(time_duration(0))
        .build();
    (jar.add(cookie), Redirect::to("/login")).into_response()
}

fn time_duration(seconds: u64) -> time::Duration {
    time::Duration::seconds(seconds as i64)
}
