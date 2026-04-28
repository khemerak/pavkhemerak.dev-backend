use serde::{Deserialize, Serialize};

// ── Database row ──────────────────────────────────────────────
/// Matches the `blog_posts` SQLite table exactly.
#[allow(dead_code)]
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct BlogPostRow {
    pub id: String,
    pub slug: String,
    pub title: String,
    pub excerpt: String,
    pub content: String,
    pub date: String,
    pub read_time: String,
    pub category: String,
    pub category_color: String,
    pub image_url: Option<String>,
    pub image_alt: Option<String>,
    pub tags: String, // JSON array stored as TEXT
    pub code_snippet: Option<String>,
    pub views: i64,
    pub created_at: String,
    pub updated_at: String,
}

// ── API responses ─────────────────────────────────────────────
/// Returned for list endpoints (no full content).
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BlogPostSummary {
    pub slug: String,
    pub title: String,
    pub excerpt: String,
    pub date: String,
    pub read_time: String,
    pub category: String,
    pub category_color: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_alt: Option<String>,
    pub tags: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code_snippet: Option<String>,
    pub views: i64,
}

/// Full post including content – returned for detail endpoint.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BlogPostDetail {
    #[serde(flatten)]
    pub summary: BlogPostSummary,
    pub content: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BlogListResponse {
    pub posts: Vec<BlogPostSummary>,
    pub total: i64,
    pub page: i64,
    pub per_page: i64,
    pub total_pages: i64,
}

// ── API request bodies ────────────────────────────────────────
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateBlogPost {
    pub slug: String,
    pub title: String,
    #[serde(default)]
    pub excerpt: String,
    #[serde(default)]
    pub content: String,
    pub date: String,
    #[serde(default = "default_read_time")]
    pub read_time: String,
    #[serde(default = "default_category")]
    pub category: String,
    #[serde(default = "default_category_color")]
    pub category_color: String,
    pub image_url: Option<String>,
    pub image_alt: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    pub code_snippet: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateBlogPost {
    pub title: Option<String>,
    pub excerpt: Option<String>,
    pub content: Option<String>,
    pub date: Option<String>,
    pub read_time: Option<String>,
    pub category: Option<String>,
    pub category_color: Option<String>,
    pub image_url: Option<String>,
    pub image_alt: Option<String>,
    pub tags: Option<Vec<String>>,
    pub code_snippet: Option<String>,
}

fn default_read_time() -> String { "5 min read".into() }
fn default_category() -> String { "GENERAL".into() }
fn default_category_color() -> String { "border-outline-variant text-on-surface-variant".into() }

// ── Conversions ───────────────────────────────────────────────
impl BlogPostRow {
    pub fn into_summary(self) -> BlogPostSummary {
        let tags: Vec<String> = serde_json::from_str(&self.tags).unwrap_or_default();
        BlogPostSummary {
            slug: self.slug,
            title: self.title,
            excerpt: self.excerpt,
            date: self.date,
            read_time: self.read_time,
            category: self.category,
            category_color: self.category_color,
            image_url: self.image_url,
            image_alt: self.image_alt,
            tags,
            code_snippet: self.code_snippet,
            views: self.views,
        }
    }

    pub fn into_detail(self) -> BlogPostDetail {
        let content = self.content.clone();
        BlogPostDetail {
            summary: self.into_summary(),
            content,
        }
    }
}
