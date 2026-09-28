use oximod::{Model, OxiModError, Queryable};

use crate::models::About;

/// Returns the about document (the collection holds at most one).
pub async fn find() -> Result<Option<About>, OxiModError> {
    About::query().first().await
}

/// Creates or updates the single about document.
pub async fn upsert(
    title: &str,
    text: &str,
    resume_url: Option<String>,
    image_url: Option<String>,
) -> Result<(), OxiModError> {
    match find().await? {
        Some(existing) => {
            let id = existing._id.expect("stored documents always have _id");
            About::query()
                .filter(|a| a._id.eq(id))
                .update_one(|a| {
                    a.title.set(Some(title.to_string()))
                        & a.text.set(text.to_string())
                        & a.resume_url.set(resume_url.clone())
                        & a.image_url.set(image_url.clone())
                })
                .await?;
        }
        None => {
            let mut about = About::new().title(title.to_string()).text(text.to_string());
            about.resume_url = resume_url;
            about.image_url = image_url;
            about.save().await?;
        }
    }
    Ok(())
}

/// Clears `resumeUrl` on any about document referencing `url` (media
/// deletion cleanup, preserved from the previous application).
pub async fn clear_resume_url(url: &str) -> Result<(), OxiModError> {
    About::query()
        .filter(|a| a.resume_url.eq(url.to_string()))
        .update_all(|a| a.resume_url.set(None::<String>))
        .await?;
    Ok(())
}

/// Clears `imageUrl` on any about document referencing `url`.
pub async fn clear_image_url(url: &str) -> Result<(), OxiModError> {
    About::query()
        .filter(|a| a.image_url.eq(url.to_string()))
        .update_all(|a| a.image_url.set(None::<String>))
        .await?;
    Ok(())
}
