//! Public site data: view models plus a cached loader for the homepage.

use std::time::Duration;

use oximod::OxiModError;

use crate::models::{Experience, Project, Social, SocialName};
use crate::repositories;
use crate::services::cache::TtlCache;
use crate::services::format;

/// How long public data may be served from memory before it is reloaded.
/// Matches the six-hour revalidation the previous application used; admin
/// mutations invalidate the cache immediately.
pub const PUBLIC_CACHE_TTL: Duration = Duration::from_secs(6 * 60 * 60);

#[derive(Debug, Clone)]
pub struct HeroView {
    pub title: String,
    pub text: String,
}

#[derive(Debug, Clone, Default)]
pub struct AboutView {
    pub title: String,
    pub text: String,
    pub resume_url: Option<String>,
    pub image_url: Option<String>,
}

#[derive(Debug, Clone)]
pub struct LinkView {
    pub title: String,
    pub url: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MediaKind {
    Image,
    Video,
    Pdf,
    Other,
}

#[derive(Debug, Clone)]
pub struct MediaItemView {
    pub url: String,
    pub kind: MediaKind,
    pub name: String,
}

impl MediaItemView {
    pub fn is_image(&self) -> bool {
        self.kind == MediaKind::Image
    }
    pub fn is_video(&self) -> bool {
        self.kind == MediaKind::Video
    }
}

#[derive(Debug, Clone)]
pub struct ProjectView {
    pub anchor: String,
    pub title: String,
    pub objective: String,
    pub key_results: Vec<String>,
    pub technologies: Vec<String>,
    pub links: Vec<LinkView>,
    pub media: Vec<MediaItemView>,
    pub experience_anchor: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ExperienceView {
    pub anchor: String,
    pub job_title: String,
    pub company: String,
    pub location: String,
    pub date_range: String,
    pub achievements: Vec<String>,
    pub responsibilities: Vec<String>,
    pub recommendation_letter_urls: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct ArticleCardView {
    pub title: String,
    pub slug: String,
    pub published: String,
    pub published_iso: String,
    pub updated: Option<String>,
    pub updated_iso: Option<String>,
    pub tags: Vec<String>,
    pub excerpt: String,
}

#[derive(Debug, Clone, Default)]
pub struct SocialLinks {
    pub linkedin: Option<String>,
    pub github: Option<String>,
    pub telegram: Option<String>,
    pub email: Option<String>,
}

impl SocialLinks {
    pub fn from_socials(socials: &[Social]) -> Self {
        let mut links = SocialLinks::default();
        for social in socials {
            let url = social.url.trim();
            if url.is_empty() {
                continue;
            }
            match social.name {
                SocialName::Linkedin => links.linkedin = Some(url.to_string()),
                SocialName::Github => links.github = Some(url.to_string()),
                SocialName::Telegram => links.telegram = Some(url.to_string()),
                SocialName::Email => links.email = Some(url.to_string()),
            }
        }
        links
    }
}

/// Everything the homepage renders, assembled once and cached.
#[derive(Debug, Clone)]
pub struct HomeData {
    pub hero: HeroView,
    pub about: AboutView,
    pub skills: Vec<String>,
    pub projects: Vec<ProjectView>,
    pub experiences: Vec<ExperienceView>,
    pub recent_articles: Vec<ArticleCardView>,
    pub socials: SocialLinks,
    /// Skill names used for SEO keywords (deduplicated, capped).
    pub meta_description: String,
}

/// Classifies a media URL by extension — the same heuristic the previous
/// client used, since `Project.media` stores plain URLs.
pub fn media_kind_from_url(url: &str) -> MediaKind {
    let path = url.split(['?', '#']).next().unwrap_or(url);
    let ext = path.rsplit('.').next().unwrap_or("").to_ascii_lowercase();
    match ext.as_str() {
        "jpg" | "jpeg" | "png" | "gif" | "bmp" | "svg" | "webp" | "avif" => MediaKind::Image,
        "mp4" | "mov" | "avi" | "wmv" | "flv" | "webm" | "mkv" => MediaKind::Video,
        "pdf" => MediaKind::Pdf,
        _ => MediaKind::Other,
    }
}

/// Returns a human-readable file name for a media URL.
///
/// Uploaded objects are keyed `media/{uuid}-{original name}`, so the
/// UUID prefix is stripped when present to keep alt text and link labels
/// readable ("IMG_6549.jpg" rather than "31d20e53-…-IMG_6549.jpg").
pub fn file_name_from_url(url: &str) -> String {
    let path = url.split(['?', '#']).next().unwrap_or(url);
    let name = path.rsplit('/').next().unwrap_or(path);
    strip_uuid_prefix(name).to_string()
}

fn strip_uuid_prefix(name: &str) -> &str {
    const UUID_LEN: usize = 36;
    let Some((prefix, rest)) = name.split_at_checked(UUID_LEN + 1) else {
        return name;
    };
    let is_uuid = prefix.ends_with('-')
        && prefix[..UUID_LEN]
            .bytes()
            .enumerate()
            .all(|(i, b)| match i {
                8 | 13 | 18 | 23 => b == b'-',
                _ => b.is_ascii_hexdigit(),
            });
    if is_uuid && !rest.is_empty() {
        rest
    } else {
        name
    }
}

fn project_view(project: &Project) -> ProjectView {
    let links = project
        .urls
        .iter()
        .enumerate()
        .filter(|(_, url)| !url.trim().is_empty())
        .map(|(i, url)| LinkView {
            title: project
                .url_titles
                .get(i)
                .filter(|t| !t.trim().is_empty())
                .cloned()
                .unwrap_or_else(|| format!("Link {}", i + 1)),
            url: url.trim().to_string(),
        })
        .collect();

    let media = project
        .media
        .iter()
        .filter(|url| !url.trim().is_empty())
        .map(|url| MediaItemView {
            kind: media_kind_from_url(url),
            name: file_name_from_url(url),
            url: url.trim().to_string(),
        })
        .collect();

    ProjectView {
        anchor: project
            ._id
            .map(|id| format!("project-{}", id.to_hex()))
            .unwrap_or_else(|| "project".to_string()),
        title: project.project_title.clone(),
        objective: project.objective.clone(),
        key_results: project
            .key_results
            .iter()
            .filter(|k| !k.trim().is_empty())
            .cloned()
            .collect(),
        technologies: project
            .project_technologies
            .iter()
            .filter(|t| !t.trim().is_empty())
            .cloned()
            .collect(),
        links,
        media,
        experience_anchor: project
            .experience_id
            .as_ref()
            .filter(|id| !id.trim().is_empty())
            .map(|id| format!("experience-{id}")),
    }
}

fn experience_view(experience: &Experience) -> ExperienceView {
    let start = format::month_year(experience.start_date);
    let end = experience
        .end_date
        .map(format::month_year)
        .unwrap_or_else(|| "Present".to_string());

    ExperienceView {
        anchor: experience
            ._id
            .map(|id| format!("experience-{}", id.to_hex()))
            .unwrap_or_else(|| "experience".to_string()),
        job_title: experience.job_title.clone(),
        company: experience.company.clone(),
        location: experience.location.clone(),
        date_range: format!("{start} – {end}"),
        achievements: experience.achievements.clone(),
        responsibilities: experience.responsibilities.clone(),
        recommendation_letter_urls: experience.recommendation_letter_urls.clone(),
    }
}

pub fn article_card(article: &crate::models::Article) -> ArticleCardView {
    let published = format::long_date(article.created_at);
    let updated = format::long_date(article.updated_at);
    let was_updated = updated != published;
    ArticleCardView {
        title: article.title.clone(),
        slug: article.slug.clone(),
        published,
        published_iso: format::iso(article.created_at),
        updated: was_updated.then_some(updated),
        updated_iso: was_updated.then(|| format::iso(article.updated_at)),
        tags: article.tags.clone(),
        excerpt: crate::markdown::excerpt(&article.content, 180),
    }
}

/// Loads the full homepage data set from MongoDB.
pub async fn load_home_data() -> Result<HomeData, OxiModError> {
    let (hero, about, skills, projects, experiences, articles, socials) = tokio::try_join!(
        repositories::hero::find(),
        repositories::about::find(),
        repositories::skills::find(),
        repositories::project::all_ordered(),
        repositories::experience::all_sorted(),
        repositories::article::all_sorted(),
        repositories::social::all(),
    )?;

    let hero_view = HeroView {
        title: hero
            .as_ref()
            .and_then(|h| h.title.clone())
            .unwrap_or_else(|| "Arshia Eskandari".to_string()),
        text: hero
            .as_ref()
            .and_then(|h| h.text.clone())
            .unwrap_or_default(),
    };

    let about_view = about
        .map(|a| AboutView {
            title: a.title.unwrap_or_else(|| "About".to_string()),
            text: a.text,
            resume_url: a.resume_url.filter(|u| !u.trim().is_empty()),
            image_url: a.image_url.filter(|u| !u.trim().is_empty()),
        })
        .unwrap_or_default();

    let meta_description = build_meta_description(&hero_view.text, &about_view.text);

    Ok(HomeData {
        hero: hero_view,
        about: about_view,
        skills: skills.map(|s| s.skills).unwrap_or_default(),
        projects: projects.iter().map(project_view).collect(),
        experiences: experiences.iter().map(experience_view).collect(),
        recent_articles: articles.iter().take(3).map(article_card).collect(),
        socials: SocialLinks::from_socials(&socials),
        meta_description,
    })
}

/// Returns the homepage data, served from cache when fresh.
pub async fn home_data(
    cache: &TtlCache<HomeData>,
) -> Result<std::sync::Arc<HomeData>, OxiModError> {
    if let Some(cached) = cache.get() {
        return Ok(cached);
    }
    let data = load_home_data().await?;
    Ok(cache.put(data))
}

/// Builds an intentional meta description from hero/about copy instead of
/// concatenating whole documents: first sentence-ish fragment of each,
/// capped at a search-snippet-friendly length.
fn build_meta_description(hero_text: &str, about_text: &str) -> String {
    let combined = if hero_text.trim().is_empty() {
        about_text.trim().to_string()
    } else if about_text.trim().is_empty() {
        hero_text.trim().to_string()
    } else {
        format!("{} {}", hero_text.trim(), about_text.trim())
    };

    if combined.is_empty() {
        return "Portfolio of Arshia Eskandari, software engineer.".to_string();
    }

    let max = 160usize;
    if combined.chars().count() <= max {
        return combined;
    }
    let truncated: String = combined.chars().take(max).collect();
    let cut = truncated.rfind(' ').unwrap_or(truncated.len());
    format!("{}…", truncated[..cut].trim_end())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_media_urls() {
        assert_eq!(media_kind_from_url("https://x/y/z.PNG"), MediaKind::Image);
        assert_eq!(
            media_kind_from_url("https://x/y/z.mp4?sig=1"),
            MediaKind::Video
        );
        assert_eq!(
            media_kind_from_url("https://x/y/report.pdf"),
            MediaKind::Pdf
        );
        assert_eq!(media_kind_from_url("https://x/y/none"), MediaKind::Other);
    }

    #[test]
    fn file_name_strips_upload_uuid_prefix() {
        assert_eq!(
            file_name_from_url(
                "https://b.s3.us-east-2.amazonaws.com/media/31d20e53-2cbe-49f3-b506-5cd0ebe31a93-IMG_6549.jpg"
            ),
            "IMG_6549.jpg"
        );
        assert_eq!(
            file_name_from_url("https://example.com/files/report.pdf?x=1"),
            "report.pdf"
        );
        assert_eq!(
            file_name_from_url("https://example.com/not-a-uuid-prefix-name.png"),
            "not-a-uuid-prefix-name.png"
        );
        assert_eq!(file_name_from_url("short"), "short");
    }

    #[test]
    fn meta_description_is_capped() {
        let long = "word ".repeat(100);
        let out = build_meta_description(&long, &long);
        assert!(out.chars().count() <= 161);
        assert!(out.ends_with('…'));
    }
}
