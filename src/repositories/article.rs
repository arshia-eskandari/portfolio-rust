use mongodb::bson::{DateTime, oid::ObjectId};
use oximod::{Model, OxiModError, Queryable};

use crate::models::Article;
use crate::slug::{generate_article_slug, generate_article_slug_with};

/// Returns every article, newest first.
pub async fn all_sorted() -> Result<Vec<Article>, OxiModError> {
    Article::query()
        .sort_by(|a| a.created_at.desc())
        .all()
        .await
}

pub async fn find_by_slug(slug: &str) -> Result<Option<Article>, OxiModError> {
    Article::query()
        .filter(|a| a.slug.eq(slug.to_string()))
        .first()
        .await
}

pub async fn find_by_id(id: ObjectId) -> Result<Option<Article>, OxiModError> {
    Article::find_by_id(id).await
}

/// Creates a placeholder article, then rewrites its provisional random
/// slug with the canonical id-based slug — exactly like the previous
/// application's "add default article" action.
pub async fn create_default() -> Result<ObjectId, OxiModError> {
    let provisional_slug = generate_article_slug_with("Title", None, None, 64, 12);
    let id = Article::new()
        .title("Title".to_string())
        .content("Content".to_string())
        .slug(provisional_slug.clone())
        .save()
        .await?;

    let created = Article::find_by_id(id).await?;
    let created_at = created.as_ref().map(|a| a.created_at);
    let final_slug = generate_article_slug("Title", Some(&id), created_at);
    if final_slug != provisional_slug {
        Article::query()
            .filter(|a| a._id.eq(id))
            .update_one(|a| a.slug.set(final_slug.clone()))
            .await?;
    }
    Ok(id)
}

/// Updates an article. The slug is regenerated from the (possibly new)
/// title with the stable id-derived suffix, matching the previous
/// behavior: an unchanged title keeps its stored slug.
pub async fn update(
    id: ObjectId,
    title: &str,
    content: &str,
    banner: Option<String>,
    tags: Vec<String>,
) -> Result<Option<String>, OxiModError> {
    let Some(existing) = Article::find_by_id(id).await? else {
        return Ok(None);
    };

    let new_slug = generate_article_slug(title, Some(&id), Some(existing.created_at));
    Article::query()
        .filter(|a| a._id.eq(id))
        .update_one(|a| {
            a.title.set(title.to_string())
                & a.content.set(content.to_string())
                & a.banner.set(banner.clone())
                & a.tags.set(tags.clone())
                & a.slug.set(new_slug.clone())
                & a.updated_at.set(DateTime::now())
        })
        .await?;
    Ok(Some(new_slug))
}

pub async fn delete(id: ObjectId) -> Result<(), OxiModError> {
    Article::delete_by_id(id).await?;
    Ok(())
}

/// Clears the banner on any article referencing `url` (media deletion
/// cleanup so article pages never point at deleted S3 objects).
pub async fn clear_banner_url(url: &str) -> Result<(), OxiModError> {
    Article::query()
        .filter(|a| a.banner.eq(url.to_string()))
        .update_all(|a| a.banner.set(None::<String>))
        .await?;
    Ok(())
}
