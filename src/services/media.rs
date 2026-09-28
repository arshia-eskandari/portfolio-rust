//! Media management: validated S3 uploads and referential-integrity-safe
//! deletion.

use mongodb::bson::oid::ObjectId;

use crate::error::AppError;
use crate::models::MediaType;
use crate::repositories;

/// Maximum accepted upload size (enforced again by the request body limit).
pub const MAX_UPLOAD_BYTES: usize = 50 * 1024 * 1024;

/// Maps a lowercase file extension to its media category and content type.
/// The extension set matches the previous application.
fn classify_extension(extension: &str) -> Option<(MediaType, &'static str)> {
    let mapping = match extension {
        "jpg" | "jpeg" => (MediaType::Image, "image/jpeg"),
        "png" => (MediaType::Image, "image/png"),
        "gif" => (MediaType::Image, "image/gif"),
        "bmp" => (MediaType::Image, "image/bmp"),
        "svg" => (MediaType::Image, "image/svg+xml"),
        "mp4" => (MediaType::Video, "video/mp4"),
        "mov" => (MediaType::Video, "video/quicktime"),
        "avi" => (MediaType::Video, "video/x-msvideo"),
        "wmv" => (MediaType::Video, "video/x-ms-wmv"),
        "flv" => (MediaType::Video, "video/x-flv"),
        "webm" => (MediaType::Video, "video/webm"),
        "mkv" => (MediaType::Video, "video/x-matroska"),
        "pdf" => (MediaType::Pdf, "application/pdf"),
        _ => return None,
    };
    Some(mapping)
}

/// Validates a file by extension and, when the format is detectable,
/// magic bytes. Returns the media category and S3 content type or a 415.
pub fn validate_upload(
    file_name: &str,
    bytes: &[u8],
) -> Result<(MediaType, &'static str), AppError> {
    if bytes.is_empty() {
        return Err(AppError::BadRequest("No file was uploaded".to_string()));
    }
    if bytes.len() > MAX_UPLOAD_BYTES {
        return Err(AppError::BadRequest(
            "File exceeds the 50 MB upload limit".to_string(),
        ));
    }

    let extension = file_name
        .rsplit('.')
        .next()
        .unwrap_or("")
        .to_ascii_lowercase();
    let Some((media_type, content_type)) = classify_extension(&extension) else {
        return Err(AppError::UnsupportedMediaType);
    };

    // Don't trust the extension alone: when the content is sniffable,
    // require the detected kind to agree with the claimed category.
    if extension == "svg" {
        // SVG is XML text without a binary signature; require it to at
        // least look like SVG markup.
        let head = String::from_utf8_lossy(&bytes[..bytes.len().min(1024)]);
        if !head.contains("<svg") {
            return Err(AppError::UnsupportedMediaType);
        }
    } else if let Some(detected) = infer::get(bytes) {
        let detected_type = detected.mime_type();
        let agrees = match media_type {
            MediaType::Image => detected_type.starts_with("image/"),
            MediaType::Video => {
                detected_type.starts_with("video/") || detected_type == "audio/x-riff"
            }
            MediaType::Pdf => detected_type == "application/pdf",
        };
        if !agrees {
            return Err(AppError::UnsupportedMediaType);
        }
    }
    // Legacy formats without a magic-byte signature pass on extension only.

    Ok((media_type, content_type))
}

/// Builds a safe S3 object key: `media/{uuid}-{sanitized name}`.
/// Removes whitespace (like the previous app) and any path/URL-hostile
/// characters so uploads can never traverse directories.
pub fn object_key(file_name: &str) -> String {
    let sanitized: String = file_name
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_'))
        .collect();
    let sanitized = sanitized.trim_matches('.').to_string();
    let sanitized = if sanitized.is_empty() {
        "file".to_string()
    } else {
        sanitized
    };
    format!("media/{}-{}", uuid::Uuid::new_v4(), sanitized)
}

/// Uploads a validated file to S3 and records it in the Media collection.
pub async fn upload(
    s3: &aws_sdk_s3::Client,
    bucket: &str,
    region: &str,
    file_name: &str,
    bytes: Vec<u8>,
) -> Result<(), AppError> {
    let (media_type, content_type) = validate_upload(file_name, &bytes)?;
    let key = object_key(file_name);

    s3.put_object()
        .bucket(bucket)
        .key(&key)
        .content_type(content_type)
        .body(aws_sdk_s3::primitives::ByteStream::from(bytes))
        .send()
        .await
        .map_err(|e| AppError::internal(format!("S3 upload failed: {e}")))?;

    let url = format!("https://{bucket}.s3.{region}.amazonaws.com/{key}");
    repositories::media::create(file_name, &url, &key, media_type).await?;
    Ok(())
}

/// Removes every reference to a media URL from the documents that can
/// hold one, then deletes the S3 object and the Media record.
///
/// Reference cleanup preserved from the previous application
/// (`deleteUsedMedia`): experience recommendation letters, project links
/// with their parallel titles, and the about resume. Additionally cleaned
/// here to avoid dangling references the old code missed: project media
/// arrays, the about image and article banners.
pub async fn delete_with_cleanup(
    s3: &aws_sdk_s3::Client,
    bucket: &str,
    id: ObjectId,
) -> Result<bool, AppError> {
    let Some(media) = repositories::media::find_by_id(id).await? else {
        return Ok(false);
    };
    let url = media.url.as_str();

    repositories::experience::pull_recommendation_url(url).await?;

    // Project links: parallel arrays, so remove the url and the title at
    // the same index (read-modify-write, like the previous implementation).
    for project in repositories::project::find_by_link_url(url).await? {
        let Some(project_id) = project._id else {
            continue;
        };
        let mut urls = project.urls.clone();
        let mut titles = project.url_titles.clone();
        if let Some(index) = urls.iter().position(|u| u == url) {
            urls.remove(index);
            if let Some(title) = project.url_titles.get(index)
                && let Some(title_index) = titles.iter().position(|t| t == title)
            {
                titles.remove(title_index);
            }
            repositories::project::set_links(project_id, urls, titles).await?;
        }
    }

    repositories::project::pull_media_url(url).await?;
    repositories::about::clear_resume_url(url).await?;
    repositories::about::clear_image_url(url).await?;
    repositories::article::clear_banner_url(url).await?;

    s3.delete_object()
        .bucket(bucket)
        .key(&media.file_key)
        .send()
        .await
        .map_err(|e| AppError::internal(format!("S3 delete failed: {e}")))?;

    repositories::media::delete(id).await?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    const PNG_MAGIC: &[u8] = &[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0, 0, 0, 0];

    #[test]
    fn accepts_png_with_matching_magic_bytes() {
        let (media_type, content_type) =
            validate_upload("photo.png", PNG_MAGIC).expect("valid png");
        assert_eq!(media_type, MediaType::Image);
        assert_eq!(content_type, "image/png");
    }

    #[test]
    fn rejects_extension_content_mismatch() {
        // PNG bytes claiming to be a PDF must be refused.
        let err = validate_upload("evil.pdf", PNG_MAGIC).expect_err("mismatch");
        assert!(matches!(err, AppError::UnsupportedMediaType));
    }

    #[test]
    fn rejects_unsupported_extension() {
        let err = validate_upload("script.exe", b"MZ\x90\x00").expect_err("unsupported");
        assert!(matches!(err, AppError::UnsupportedMediaType));
    }

    #[test]
    fn accepts_svg_by_content() {
        let svg = b"<?xml version=\"1.0\"?><svg xmlns=\"http://www.w3.org/2000/svg\"></svg>";
        let (media_type, _) = validate_upload("icon.svg", svg).expect("valid svg");
        assert_eq!(media_type, MediaType::Image);
    }

    #[test]
    fn rejects_fake_svg() {
        let err = validate_upload("icon.svg", b"#!/bin/sh\nrm -rf /").expect_err("not svg");
        assert!(matches!(err, AppError::UnsupportedMediaType));
    }

    #[test]
    fn object_keys_are_sanitized() {
        let key = object_key("../etc/pass wd.png");
        assert!(key.starts_with("media/"));
        assert!(!key.contains(".."), "no traversal: {key}");
        assert!(!key.contains(' '), "no spaces: {key}");
        assert!(
            !key.contains('/') || key.matches('/').count() == 1,
            "only the media/ prefix slash: {key}"
        );
        assert!(key.ends_with("etcpasswd.png"), "unexpected key: {key}");
    }
}
