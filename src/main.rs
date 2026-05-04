use std::sync::Arc;

use axum::Router;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;

mod config;
mod db;
mod errors;
mod models;
mod routes;

/// Shared application state available to all route handlers.
pub struct AppState {
    pub db: sqlx::SqlitePool,
    pub config: config::Config,
    pub start_time: std::time::Instant,
    pub http_client: reqwest::Client,
}

#[tokio::main]
async fn main() {
    // Initialise tracing / logging
    tracing_subscriber::fmt::init();
    dotenvy::dotenv().ok();

    let config = config::Config::from_env();
    let port = config.port;

    // Database
    let pool = db::init_db(&config.database_url).await;
    // seed::seed_if_empty(&pool).await; // Disabled: user requested dynamic data only

    // Shared state
    let state = Arc::new(AppState {
        db: pool,
        config,
        start_time: std::time::Instant::now(),
        http_client: reqwest::Client::new(),
    });

    // CORS – allow the Next.js dev server
    let cors = CorsLayer::new()
        .allow_origin([
            "https://pavkhemerak.is-a.dev".parse::<axum::http::HeaderValue>().unwrap(),
            "https://dash-pavkhemerak.vercel.app".parse::<axum::http::HeaderValue>().unwrap(),
        ])
        .allow_methods(Any)
        .allow_headers(Any);

    let app = Router::new()
        .merge(routes::api_routes())
        .with_state(state)
        .layer(cors)
        .layer(TraceLayer::new_for_http());

    let addr = format!("0.0.0.0:{}", port);
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    tracing::info!("pavkhemerak-api listening on {}", addr);
    axum::serve(listener, app).await.unwrap();
}
