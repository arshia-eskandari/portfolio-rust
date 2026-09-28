use askama::Template;
use askama_web::WebTemplate;
use axum::Form;
use axum::extract::{Multipart, Query, State};
use axum::response::Redirect;

use crate::app::AppState;
use crate::error::AppError;
use crate::repositories;
use crate::routes::Flash;
use crate::routes::admin::{admin_meta, parse_object_id, redirect_err, redirect_ok};
use crate::seo::metadata::PageMeta;
use crate::services::format;
use crate::services::media as media_service;

pub struct MediaRow {
    pub id: String,
    pub name: String,
    pub url: String,
    pub media_type: &'static str,
    pub created: String,
}

#[derive(Template, WebTemplate)]
#[template(path = "admin/media.html")]
pub struct MediaAdminTemplate {
    pub meta: PageMeta,
    pub flash: Flash,
    pub media: Vec<MediaRow>,
}

pub async fn page(Query(flash): Query<Flash>) -> Result<MediaAdminTemplate, AppError> {
    let media = repositories::media::all().await?;
    let rows = media
        .into_iter()
        .map(|m| MediaRow {
            id: m._id.map(|id| id.to_hex()).unwrap_or_default(),
            name: m.name,
            url: m.url,
            media_type: m.media_type.as_str(),
            created: format::iso_date(m.created_at),
        })
        .collect();

    Ok(MediaAdminTemplate {
        meta: admin_meta("Media"),
        flash,
        media: rows,
    })
}

pub async fn upload(
    State(state): State<AppState>,
    mut multipart: Multipart,
) -> Result<Redirect, AppError> {
    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| AppError::BadRequest(format!("Invalid upload: {e}")))?
    {
        if field.name() != Some("file") {
            continue;
        }
        let file_name = field.file_name().unwrap_or("upload").to_string();
        let bytes = field
            .bytes()
            .await
            .map_err(|e| AppError::BadRequest(format!("Upload failed: {e}")))?;

        match media_service::upload(
            &state.s3,
            &state.config.s3_bucket,
            &state.config.aws_region,
            &file_name,
            bytes.to_vec(),
        )
        .await
        {
            Ok(()) => {
                state.caches.invalidate_public();
                return Ok(redirect_ok("/admin/media", "File uploaded."));
            }
            Err(AppError::UnsupportedMediaType) => {
                return Ok(redirect_err("/admin/media", "Unsupported file type."));
            }
            Err(AppError::BadRequest(message)) => {
                return Ok(redirect_err("/admin/media", &message));
            }
            Err(other) => return Err(other),
        }
    }
    Ok(redirect_err("/admin/media", "No file was uploaded."))
}

/// Bulk delete: one form field per selected media id (`ids`), each value a
/// hex ObjectId. Reference cleanup runs before anything is removed.
pub async fn delete(
    State(state): State<AppState>,
    Form(entries): Form<Vec<(String, String)>>,
) -> Result<Redirect, AppError> {
    let ids: Vec<String> = entries
        .into_iter()
        .filter(|(k, _)| k == "ids")
        .map(|(_, v)| v)
        .collect();
    if ids.is_empty() {
        return Ok(redirect_err(
            "/admin/media",
            "Select at least one file to delete.",
        ));
    }

    let mut deleted = 0usize;
    let attempted = ids.len();
    for id_hex in ids {
        let Ok(id) = parse_object_id(&id_hex) else {
            continue;
        };
        match media_service::delete_with_cleanup(&state.s3, &state.config.s3_bucket, id).await {
            Ok(true) => deleted += 1,
            Ok(false) => {}
            Err(err) => {
                tracing::error!(error = %err, "media deletion failed");
            }
        }
    }

    state.caches.invalidate_public();
    if deleted == attempted {
        Ok(redirect_ok("/admin/media", "Media deleted."))
    } else {
        Ok(redirect_ok(
            "/admin/media",
            &format!("{deleted} of {attempted} file(s) deleted; the rest failed."),
        ))
    }
}
