use std::sync::Arc;

use axum::extract::State;
use axum::Json;
use serde::Serialize;

use crate::AppState;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CategoryCount {
    pub name: String,
    pub count: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TopPost {
    pub slug: String,
    pub title: String,
    pub views: i64,
    pub category: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DashboardStats {
    pub total_posts: i64,
    pub total_views: i64,
    pub categories: Vec<CategoryCount>,
    pub latest_post_date: Option<String>,
    pub top_posts: Vec<TopPost>,
}

pub async fn stats(
    State(state): State<Arc<AppState>>,
) -> Json<DashboardStats> {
    let total: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM blog_posts")
        .fetch_one(&state.db)
        .await
        .unwrap_or((0,));

    let total_views: (i64,) = sqlx::query_as("SELECT COALESCE(SUM(views), 0) FROM blog_posts")
        .fetch_one(&state.db)
        .await
        .unwrap_or((0,));

    let cat_rows: Vec<(String, i64)> = sqlx::query_as(
        "SELECT category, COUNT(*) as cnt FROM blog_posts GROUP BY category ORDER BY cnt DESC",
    )
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();

    let latest: Option<(String,)> = sqlx::query_as(
        "SELECT date FROM blog_posts ORDER BY date DESC, created_at DESC LIMIT 1",
    )
    .fetch_optional(&state.db)
    .await
    .unwrap_or(None);

    let top_posts_rows: Vec<(String, String, i64, String)> = sqlx::query_as(
        "SELECT slug, title, views, category FROM blog_posts ORDER BY views DESC LIMIT 10",
    )
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();

    Json(DashboardStats {
        total_posts: total.0,
        total_views: total_views.0,
        categories: cat_rows
            .into_iter()
            .map(|(name, count)| CategoryCount { name, count })
            .collect(),
        latest_post_date: latest.map(|r| r.0),
        top_posts: top_posts_rows
            .into_iter()
            .map(|(slug, title, views, category)| TopPost { slug, title, views, category })
            .collect(),
    })
}
