//! Article listing and rendering.

use std::sync::Arc;
use std::time::Duration;

use oximod::OxiModError;

use crate::models::Article;
use crate::repositories;
use crate::services::cache::TtlCache;
use crate::services::format;
use crate::services::portfolio::{ArticleCardView, article_card};

/// Article list cache TTL. Invalidated on every admin article mutation.
pub const ARTICLES_CACHE_TTL: Duration = Duration::from_secs(6 * 60 * 60);

/// Fully rendered article for the detail page.
#[derive(Debug, Clone)]
pub struct ArticlePageView {
    pub title: String,
    pub slug: String,
    pub banner: Option<String>,
    pub tags: Vec<String>,
    pub html: String,
    pub description: String,
    pub published: String,
    pub published_iso: String,
    pub updated: Option<String>,
    pub updated_iso: String,
}

/// Returns all article cards, newest first, cached.
pub async fn list(cache: &TtlCache<Vec<Article>>) -> Result<Arc<Vec<Article>>, OxiModError> {
    if let Some(cached) = cache.get() {
        return Ok(cached);
    }
    let articles = repositories::article::all_sorted().await?;
    Ok(cache.put(articles))
}

/// Case-insensitive title/tag filter used by the articles index search box.
/// Terms are OR-combined, matching the previous client-side behavior.
pub fn filter_cards(articles: &[Article], query: &str) -> Vec<ArticleCardView> {
    let raw = query.trim().to_lowercase();
    if raw.is_empty() {
        return articles.iter().map(article_card).collect();
    }
    let terms: Vec<&str> = raw.split([' ', ',']).filter(|t| !t.is_empty()).collect();

    articles
        .iter()
        .filter(|article| {
            let title = article.title.to_lowercase();
            let tags: Vec<String> = article.tags.iter().map(|t| t.to_lowercase()).collect();
            terms
                .iter()
                .any(|term| title.contains(term) || tags.iter().any(|tag| tag.contains(term)))
        })
        .map(article_card)
        .collect()
}

/// Loads one article by slug and renders it for the detail page.
pub async fn page_by_slug(slug: &str) -> Result<Option<ArticlePageView>, OxiModError> {
    let Some(article) = repositories::article::find_by_slug(slug).await? else {
        return Ok(None);
    };

    let published = format::long_date(article.created_at);
    let updated = format::long_date(article.updated_at);
    let was_updated = updated != published;

    Ok(Some(ArticlePageView {
        html: crate::markdown::render(&article.content),
        description: crate::markdown::excerpt(&article.content, 160),
        title: article.title,
        slug: article.slug,
        banner: article.banner.filter(|b| !b.trim().is_empty()),
        tags: article.tags,
        published,
        published_iso: format::iso(article.created_at),
        updated: was_updated.then_some(updated),
        updated_iso: format::iso(article.updated_at),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use mongodb::bson::DateTime;

    fn article(title: &str, tags: &[&str]) -> Article {
        Article {
            _id: None,
            title: title.to_string(),
            banner: None,
            content: "content".to_string(),
            created_at: DateTime::from_millis(1_700_000_000_000),
            updated_at: DateTime::from_millis(1_700_000_000_000),
            slug: "slug".to_string(),
            tags: tags.iter().map(|t| t.to_string()).collect(),
        }
    }

    #[test]
    fn filters_by_title_or_tag_case_insensitively() {
        let articles = vec![
            article("Rust ownership", &["rust"]),
            article("Cooking pasta", &["food"]),
        ];
        let hits = filter_cards(&articles, "RUST");
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].title, "Rust ownership");

        let by_tag = filter_cards(&articles, "food");
        assert_eq!(by_tag.len(), 1);
        assert_eq!(by_tag[0].title, "Cooking pasta");
    }

    #[test]
    fn empty_query_returns_everything() {
        let articles = vec![article("A", &[]), article("B", &[])];
        assert_eq!(filter_cards(&articles, "  ").len(), 2);
    }
}
