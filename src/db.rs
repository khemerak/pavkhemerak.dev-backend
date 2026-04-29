use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::SqlitePool;
use std::str::FromStr;

/// Initialise the SQLite database, create the file + tables if needed.
pub async fn init_db(database_url: &str) -> SqlitePool {
    // Ensure the data directory exists
    if let Some(path) = database_url.strip_prefix("sqlite:") {
        if let Some(parent) = std::path::Path::new(path).parent() {
            std::fs::create_dir_all(parent).expect("Failed to create database directory");
        }
    }

    let opts = SqliteConnectOptions::from_str(database_url)
        .expect("Invalid DATABASE_URL")
        .create_if_missing(true);

    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(opts)
        .await
        .expect("Failed to connect to SQLite");

    run_migrations(&pool).await;
    pool
}

/// Run table creation statements.
async fn run_migrations(pool: &SqlitePool) {
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS blog_posts (
            id              TEXT PRIMARY KEY,
            slug            TEXT NOT NULL UNIQUE,
            title           TEXT NOT NULL,
            excerpt         TEXT NOT NULL DEFAULT '',
            content         TEXT NOT NULL DEFAULT '',
            date            TEXT NOT NULL,
            read_time       TEXT NOT NULL DEFAULT '5 min read',
            category        TEXT NOT NULL DEFAULT 'GENERAL',
            category_color  TEXT NOT NULL DEFAULT 'border-outline-variant text-on-surface-variant',
            image_url       TEXT,
            image_alt       TEXT,
            tags            TEXT NOT NULL DEFAULT '[]',
            code_snippet    TEXT,
            created_at      TEXT NOT NULL DEFAULT (datetime('now')),
            updated_at      TEXT NOT NULL DEFAULT (datetime('now'))
        );
        "#,
    )
    .execute(pool)
    .await
    .expect("Failed to run migrations");

    // Add views column if it doesn't exist (safe to run multiple times)
    let _ = sqlx::query("ALTER TABLE blog_posts ADD COLUMN views INTEGER NOT NULL DEFAULT 0")
        .execute(pool)
        .await; // ignore error — column may already exist

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS portfolio_content (
            id          INTEGER PRIMARY KEY CHECK (id = 1),
            content     TEXT NOT NULL,
            updated_at  TEXT NOT NULL DEFAULT (datetime('now'))
        );
        "#,
    )
    .execute(pool)
    .await
    .expect("Failed to create portfolio_content table");

    tracing::info!("Database migrations applied");
}
