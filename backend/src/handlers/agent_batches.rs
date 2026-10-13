use crate::{batch_models::BatchRequest, services::agent_batches, state::AppState};
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Deserialize;

#[derive(Deserialize)]
pub struct ListQuery {
    limit: Option<i64>,
}

pub async fn submit(State(state): State<AppState>, Json(request): Json<BatchRequest>) -> Response {
    if let Err(error) = agent_batches::validate(&request) {
        return (StatusCode::BAD_REQUEST, error.to_string()).into_response();
    }
    match agent_batches::submit(&state, request).await {
        Ok(job) => (StatusCode::ACCEPTED, Json(job)).into_response(),
        Err(error) => failure(error),
    }
}

pub async fn get(State(state): State<AppState>, Path(id): Path<String>) -> Response {
    match crate::repositories::agent_batches::get(&state.pool, &id).await {
        Ok(job) => Json(job).into_response(),
        Err(error) => failure(error),
    }
}

pub async fn list(State(state): State<AppState>, Query(query): Query<ListQuery>) -> Response {
    match crate::repositories::agent_batches::list(&state.pool, query.limit.unwrap_or(20)).await {
        Ok(jobs) => Json(serde_json::json!({"jobs": jobs})).into_response(),
        Err(error) => failure(error.into()),
    }
}

pub async fn retry(State(state): State<AppState>, Path(id): Path<String>) -> Response {
    match agent_batches::retry(&state, &id).await {
        Ok(job) => (StatusCode::ACCEPTED, Json(job)).into_response(),
        Err(error) => failure(error),
    }
}

fn failure(error: anyhow::Error) -> Response {
    let message = format!("{error:#}");
    let status = if message.contains("not found") {
        StatusCode::NOT_FOUND
    } else if message.contains("already belongs") || message.contains("still active") {
        StatusCode::CONFLICT
    } else {
        tracing::error!("batch API failed: {message}");
        StatusCode::INTERNAL_SERVER_ERROR
    };
    (status, message).into_response()
}
