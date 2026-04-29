use std::sync::Arc;

use axum::routing::get;
use axum::Router;

use crate::AppState;

pub mod blog;
pub mod dashboard;
pub mod github;
pub mod health;
pub mod portfolio;
pub mod tools;

/// Assemble every `/api/*` route.
pub fn api_routes() -> Router<Arc<AppState>> {
    Router::new()
        // Health
        .route("/api/health", get(health::health_check))
        // Blog
        .route("/api/blog/posts", get(blog::list_posts).post(blog::create_post))
        .route("/api/blog/posts/{slug}", get(blog::get_post).put(blog::update_post).delete(blog::delete_post))
        .route("/api/blog/posts/{slug}/view", axum::routing::post(blog::view_post))
        .route("/api/blog/categories", get(blog::list_categories))
        // Dashboard
        .route("/api/dashboard/stats", get(dashboard::stats))
        // Portfolio CMS
        .route("/api/portfolio/content", get(portfolio::get_content).put(portfolio::update_content))
        // GitHub
        .route("/api/github/activity", get(github::activity))
        // Tools
        .route("/api/tools/ping", get(tools::ping))
        .route("/api/tools/etherscan", get(tools::etherscan_analyze))
}
