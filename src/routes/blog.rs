use std::sync::Arc;

use axum::extract::{Path, Query, State};
use axum::http::HeaderMap;
use axum::Json;
use serde::Deserialize;

use crate::errors::ApiError;
use crate::models::blog::*;
use crate::AppState;

// ── Query params ──────────────────────────────────────────────
#[derive(Debug, Deserialize)]
pub struct ListParams {
    pub page: Option<i64>,
    pub per_page: Option<i64>,
    pub category: Option<String>,
}

// ── Helpers ───────────────────────────────────────────────────
fn check_admin(headers: &HeaderMap, key: &str) -> Result<(), ApiError> {
    let auth = headers
        .get("x-api-key")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    if auth != key {
        return Err(ApiError::Unauthorized);
    }
    Ok(())
}

// ── GET /api/blog/posts ───────────────────────────────────────
pub async fn list_posts(
    State(state): State<Arc<AppState>>,
    Query(params): Query<ListParams>,
) -> Result<Json<BlogListResponse>, ApiError> {
    let page = params.page.unwrap_or(1).max(1);
    let per_page = params.per_page.unwrap_or(10).clamp(1, 50);
    let offset = (page - 1) * per_page;

    let (rows, total): (Vec<BlogPostRow>, i64) = match &params.category {
        Some(cat) if cat.to_uppercase() != "ALL" => {
            let cat_upper = cat.to_uppercase();
            let rows = sqlx::query_as::<_, BlogPostRow>(
                "SELECT * FROM blog_posts WHERE UPPER(category) = ? ORDER BY date DESC, created_at DESC LIMIT ? OFFSET ?",
            )
            .bind(&cat_upper)
            .bind(per_page)
            .bind(offset)
            .fetch_all(&state.db)
            .await?;

            let count: (i64,) = sqlx::query_as(
                "SELECT COUNT(*) FROM blog_posts WHERE UPPER(category) = ?",
            )
            .bind(&cat_upper)
            .fetch_one(&state.db)
            .await?;

            (rows, count.0)
        }
        _ => {
            let rows = sqlx::query_as::<_, BlogPostRow>(
                "SELECT * FROM blog_posts ORDER BY date DESC, created_at DESC LIMIT ? OFFSET ?",
            )
            .bind(per_page)
            .bind(offset)
            .fetch_all(&state.db)
            .await?;

            let count: (i64,) =
                sqlx::query_as("SELECT COUNT(*) FROM blog_posts")
                    .fetch_one(&state.db)
                    .await?;

            (rows, count.0)
        }
    };

    let total_pages = (total as f64 / per_page as f64).ceil() as i64;

    Ok(Json(BlogListResponse {
        posts: rows.into_iter().map(|r| r.into_summary()).collect(),
        total,
        page,
        per_page,
        total_pages,
    }))
}

// ── GET /api/blog/posts/:slug ─────────────────────────────────
pub async fn get_post(
    State(state): State<Arc<AppState>>,
    Path(slug): Path<String>,
) -> Result<Json<BlogPostDetail>, ApiError> {
    let row = sqlx::query_as::<_, BlogPostRow>(
        "SELECT * FROM blog_posts WHERE slug = ?",
    )
    .bind(&slug)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| ApiError::NotFound(format!("Post '{}' not found", slug)))?;

    Ok(Json(row.into_detail()))
}

// ── GET /api/blog/categories ──────────────────────────────────
pub async fn list_categories(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<String>>, ApiError> {
    let rows: Vec<(String,)> = sqlx::query_as(
        "SELECT DISTINCT category FROM blog_posts ORDER BY category",
    )
    .fetch_all(&state.db)
    .await?;

    Ok(Json(rows.into_iter().map(|r| r.0).collect()))
}

// ── POST /api/blog/posts ──────────────────────────────────────
pub async fn create_post(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(body): Json<CreateBlogPost>,
) -> Result<Json<BlogPostDetail>, ApiError> {
    check_admin(&headers, &state.config.admin_api_key)?;

    let id = uuid::Uuid::new_v4().to_string();
    let tags_json = serde_json::to_string(&body.tags).unwrap_or_else(|_| "[]".into());

    sqlx::query(
        r#"INSERT INTO blog_posts
           (id, slug, title, excerpt, content, date, read_time, category, category_color,
            image_url, image_alt, tags, code_snippet)
           VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"#,
    )
    .bind(&id)
    .bind(&body.slug)
    .bind(&body.title)
    .bind(&body.excerpt)
    .bind(&body.content)
    .bind(&body.date)
    .bind(&body.read_time)
    .bind(&body.category)
    .bind(&body.category_color)
    .bind(&body.image_url)
    .bind(&body.image_alt)
    .bind(&tags_json)
    .bind(&body.code_snippet)
    .execute(&state.db)
    .await?;

    let row = sqlx::query_as::<_, BlogPostRow>("SELECT * FROM blog_posts WHERE id = ?")
        .bind(&id)
        .fetch_one(&state.db)
        .await?;

    Ok(Json(row.into_detail()))
}

// ── PUT /api/blog/posts/:slug ─────────────────────────────────
pub async fn update_post(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(slug): Path<String>,
    Json(body): Json<UpdateBlogPost>,
) -> Result<Json<BlogPostDetail>, ApiError> {
    check_admin(&headers, &state.config.admin_api_key)?;

    // Verify post exists
    let existing = sqlx::query_as::<_, BlogPostRow>("SELECT * FROM blog_posts WHERE slug = ?")
        .bind(&slug)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("Post '{}' not found", slug)))?;

    let title = body.title.unwrap_or(existing.title);
    let excerpt = body.excerpt.unwrap_or(existing.excerpt);
    let content = body.content.unwrap_or(existing.content);
    let date = body.date.unwrap_or(existing.date);
    let read_time = body.read_time.unwrap_or(existing.read_time);
    let category = body.category.unwrap_or(existing.category);
    let category_color = body.category_color.unwrap_or(existing.category_color);
    let image_url = body.image_url.or(existing.image_url);
    let image_alt = body.image_alt.or(existing.image_alt);
    let code_snippet = body.code_snippet.or(existing.code_snippet);
    let tags_json = match body.tags {
        Some(t) => serde_json::to_string(&t).unwrap_or_else(|_| "[]".into()),
        None => existing.tags,
    };

    sqlx::query(
        r#"UPDATE blog_posts SET
           title = ?, excerpt = ?, content = ?, date = ?, read_time = ?,
           category = ?, category_color = ?, image_url = ?, image_alt = ?,
           tags = ?, code_snippet = ?, updated_at = datetime('now')
           WHERE slug = ?"#,
    )
    .bind(&title)
    .bind(&excerpt)
    .bind(&content)
    .bind(&date)
    .bind(&read_time)
    .bind(&category)
    .bind(&category_color)
    .bind(&image_url)
    .bind(&image_alt)
    .bind(&tags_json)
    .bind(&code_snippet)
    .bind(&slug)
    .execute(&state.db)
    .await?;

    let row = sqlx::query_as::<_, BlogPostRow>("SELECT * FROM blog_posts WHERE slug = ?")
        .bind(&slug)
        .fetch_one(&state.db)
        .await?;

    Ok(Json(row.into_detail()))
}

// ── DELETE /api/blog/posts/:slug ──────────────────────────────
pub async fn delete_post(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(slug): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    check_admin(&headers, &state.config.admin_api_key)?;

    let result = sqlx::query("DELETE FROM blog_posts WHERE slug = ?")
        .bind(&slug)
        .execute(&state.db)
        .await?;

    if result.rows_affected() == 0 {
        return Err(ApiError::NotFound(format!("Post '{}' not found", slug)));
    }

    Ok(Json(serde_json::json!({ "deleted": slug })))
}

// ── POST /api/blog/posts/:slug/view ──────────────────────────
pub async fn view_post(
    State(state): State<Arc<AppState>>,
    Path(slug): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let result = sqlx::query("UPDATE blog_posts SET views = views + 1 WHERE slug = ?")
        .bind(&slug)
        .execute(&state.db)
        .await?;

    if result.rows_affected() == 0 {
        return Err(ApiError::NotFound(format!("Post '{}' not found", slug)));
    }

    Ok(Json(serde_json::json!({ "ok": true })))
}
