use std::sync::Arc;

use axum::extract::State;
use axum::http::HeaderMap;
use axum::Json;
use serde_json::{json, Value};

use crate::errors::ApiError;
use crate::AppState;

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

pub async fn get_content(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Value>, ApiError> {
    let row: Option<(String, String)> = sqlx::query_as(
        "SELECT content, updated_at FROM portfolio_content WHERE id = 1",
    )
    .fetch_optional(&state.db)
    .await?;

    if let Some((content, updated_at)) = row {
        let parsed: Value = serde_json::from_str(&content)
            .map_err(|e| ApiError::Internal(format!("Invalid stored portfolio content JSON: {}", e)))?;
        Ok(Json(json!({
            "content": parsed,
            "updatedAt": updated_at
        })))
    } else {
        Ok(Json(json!({
            "content": null,
            "updatedAt": null
        })))
    }
}

pub async fn update_content(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> Result<Json<Value>, ApiError> {
    check_admin(&headers, &state.config.admin_api_key)?;

    if !body.is_object() {
        return Err(ApiError::BadRequest("Portfolio content must be a JSON object".into()));
    }

    let content_text = serde_json::to_string(&body)
        .map_err(|e| ApiError::BadRequest(format!("Invalid JSON body: {}", e)))?;

    sqlx::query(
        r#"
        INSERT INTO portfolio_content (id, content, updated_at)
        VALUES (1, ?, datetime('now'))
        ON CONFLICT(id) DO UPDATE SET
            content = excluded.content,
            updated_at = datetime('now')
        "#,
    )
    .bind(content_text)
    .execute(&state.db)
    .await?;

    let updated: (String,) = sqlx::query_as(
        "SELECT updated_at FROM portfolio_content WHERE id = 1",
    )
    .fetch_one(&state.db)
    .await?;

    Ok(Json(json!({
        "ok": true,
        "updatedAt": updated.0
    })))
}
