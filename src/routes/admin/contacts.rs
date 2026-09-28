use askama::Template;
use askama_web::WebTemplate;
use axum::Form;
use axum::extract::Query;
use axum::response::Redirect;

use crate::error::AppError;
use crate::models::ContactStatus;
use crate::repositories;
use crate::routes::Flash;
use crate::routes::admin::{admin_meta, parse_object_id, redirect_err, redirect_ok};
use crate::seo::metadata::PageMeta;
use crate::services::format;

pub struct ContactRow {
    pub id: String,
    pub name: String,
    pub email: String,
    pub message: String,
    pub status: &'static str,
    pub created: String,
}

#[derive(Template, WebTemplate)]
#[template(path = "admin/contacts.html")]
pub struct ContactsAdminTemplate {
    pub meta: PageMeta,
    pub flash: Flash,
    pub contacts: Vec<ContactRow>,
}

pub async fn page(Query(flash): Query<Flash>) -> Result<ContactsAdminTemplate, AppError> {
    let contacts = repositories::contact::all().await?;
    let rows = contacts
        .into_iter()
        .map(|c| ContactRow {
            id: c._id.map(|id| id.to_hex()).unwrap_or_default(),
            name: format!("{} {}", c.first_name, c.last_name),
            email: c.email,
            message: c.message,
            status: c.status.as_str(),
            created: format::admin_timestamp(c.created_at),
        })
        .collect();

    Ok(ContactsAdminTemplate {
        meta: admin_meta("Contact Inbox"),
        flash,
        contacts: rows,
    })
}

/// Status update: one field per submission, named by its id (matching the
/// previous admin's semantics). Unknown ids and invalid statuses are
/// skipped, valid rows are applied.
pub async fn update(Form(entries): Form<Vec<(String, String)>>) -> Result<Redirect, AppError> {
    let mut updated = 0usize;
    let mut attempted = 0usize;

    for (id_hex, status_raw) in entries {
        attempted += 1;
        let Ok(id) = parse_object_id(&id_hex) else {
            continue;
        };
        let Some(status) = ContactStatus::parse(&status_raw) else {
            continue;
        };
        if repositories::contact::set_status(id, status).await? {
            updated += 1;
        }
    }

    if attempted == 0 {
        return Ok(redirect_err("/admin/contacts", "Nothing to update."));
    }
    if updated == attempted {
        Ok(redirect_ok("/admin/contacts", "Contacts updated."))
    } else {
        Ok(redirect_ok(
            "/admin/contacts",
            &format!("{updated} of {attempted} contacts updated."),
        ))
    }
}
