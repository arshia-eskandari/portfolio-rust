use askama::Template;
use askama_web::WebTemplate;
use axum::Extension;

use crate::error::AppError;
use crate::middleware::auth::AdminSession;
use crate::repositories;
use crate::routes::admin::{admin_meta, parse_object_id};
use crate::seo::metadata::PageMeta;

#[derive(Template, WebTemplate)]
#[template(path = "admin/dashboard.html")]
pub struct DashboardTemplate {
    pub meta: PageMeta,
    pub flash: crate::routes::Flash,
    pub name: String,
    pub pending: u64,
}

pub async fn page(
    Extension(session): Extension<AdminSession>,
) -> Result<DashboardTemplate, AppError> {
    let user = match parse_object_id(&session.user_id) {
        Ok(id) => repositories::user::find_by_id(id).await?,
        Err(_) => None,
    };
    let name = user
        .map(|u| format!("{} {}", u.first_name, u.last_name))
        .unwrap_or_else(|| session.email.clone());

    let pending = repositories::contact::count_pending().await?;

    Ok(DashboardTemplate {
        meta: admin_meta("Dashboard"),
        flash: crate::routes::Flash::default(),
        name,
        pending,
    })
}
