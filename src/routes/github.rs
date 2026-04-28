use std::sync::Arc;

use axum::extract::State;
use axum::Json;
use serde::{Deserialize, Serialize};

use crate::errors::ApiError;
use crate::AppState;

#[derive(Debug, Serialize, Deserialize)]
pub struct GitHubEvent {
    pub id: String,
    #[serde(rename = "type")]
    pub event_type: String,
    pub repo: GitHubRepo,
    pub created_at: String,
    pub payload: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GitHubRepo {
    pub name: String,
    pub url: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ActivityResponse {
    pub username: String,
    pub events: Vec<ActivityEvent>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ActivityEvent {
    pub id: String,
    pub event_type: String,
    pub repo: String,
    pub created_at: String,
    pub description: String,
}

/// GET /api/github/activity
pub async fn activity(
    State(state): State<Arc<AppState>>,
) -> Result<Json<ActivityResponse>, ApiError> {
    let username = &state.config.github_username;
    let url = format!("https://api.github.com/users/{}/events?per_page=15", username);

    let events: Vec<GitHubEvent> = state
        .http_client
        .get(&url)
        .header("User-Agent", "pavkhemerak-api")
        .send()
        .await?
        .json()
        .await?;

    let activity_events: Vec<ActivityEvent> = events
        .into_iter()
        .map(|e| {
            let description = describe_event(&e.event_type, &e.payload);
            ActivityEvent {
                id: e.id,
                event_type: e.event_type,
                repo: e.repo.name,
                created_at: e.created_at,
                description,
            }
        })
        .collect();

    Ok(Json(ActivityResponse {
        username: username.clone(),
        events: activity_events,
    }))
}

fn describe_event(event_type: &str, payload: &serde_json::Value) -> String {
    match event_type {
        "PushEvent" => {
            let count = payload["size"].as_i64().unwrap_or(0);
            let branch = payload["ref"]
                .as_str()
                .unwrap_or("unknown")
                .replace("refs/heads/", "");
            format!("Pushed {} commit(s) to {}", count, branch)
        }
        "CreateEvent" => {
            let ref_type = payload["ref_type"].as_str().unwrap_or("unknown");
            format!("Created {}", ref_type)
        }
        "DeleteEvent" => {
            let ref_type = payload["ref_type"].as_str().unwrap_or("unknown");
            format!("Deleted {}", ref_type)
        }
        "WatchEvent" => "Starred repository".into(),
        "ForkEvent" => "Forked repository".into(),
        "IssuesEvent" => {
            let action = payload["action"].as_str().unwrap_or("unknown");
            format!("Issue {}", action)
        }
        "PullRequestEvent" => {
            let action = payload["action"].as_str().unwrap_or("unknown");
            format!("Pull request {}", action)
        }
        "IssueCommentEvent" => "Commented on an issue".into(),
        _ => event_type.replace("Event", "").to_string(),
    }
}
