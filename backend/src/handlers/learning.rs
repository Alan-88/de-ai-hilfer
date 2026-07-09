use crate::models::{
    LearningProgressMapResponse, LearningProgressView, LearningReviewV3Request,
    LearningSessionResponse, LearningSessionV3Response, LearningStartSessionRequest,
    LearningStatsResponse,
};
use crate::services::learning;
use crate::state::AppState;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use serde::Deserialize;

#[derive(Deserialize)]
pub struct SessionQuery {
    pub limit_new_words: Option<i64>,
}

#[derive(Deserialize)]
pub struct ReviewBody {
    pub quality: i32,
}

pub async fn get_session(
    State(state): State<AppState>,
    Query(query): Query<SessionQuery>,
) -> Result<Json<LearningSessionResponse>, (StatusCode, String)> {
    learning::get_session(&state, query.limit_new_words.unwrap_or(5))
        .await
        .map(Json)
        .map_err(internal_error)
}

pub async fn add_word(
    State(state): State<AppState>,
    Path(entry_id): Path<i64>,
) -> Result<Json<LearningProgressView>, (StatusCode, String)> {
    learning::add_word(&state, entry_id)
        .await
        .map(Json)
        .map_err(internal_error)
}

pub async fn start_session_v3(
    State(state): State<AppState>,
    Json(body): Json<LearningStartSessionRequest>,
) -> Result<Json<LearningSessionV3Response>, (StatusCode, String)> {
    learning::start_session_v3(&state, body.limit_new_words)
        .await
        .map(Json)
        .map_err(internal_error)
}

pub async fn get_session_next_v3(
    State(state): State<AppState>,
    Path(session_id): Path<String>,
) -> Result<Json<LearningSessionV3Response>, (StatusCode, String)> {
    learning::get_session_next_v3(&state, &session_id)
        .await
        .map(Json)
        .map_err(internal_error)
}

pub async fn review_word(
    State(state): State<AppState>,
    Path(entry_id): Path<i64>,
    Json(body): Json<ReviewBody>,
) -> Result<Json<LearningProgressView>, (StatusCode, String)> {
    learning::submit_review(&state, entry_id, body.quality)
        .await
        .map(Json)
        .map_err(internal_error)
}

pub async fn review_word_v3(
    State(state): State<AppState>,
    Path((session_id, entry_id)): Path<(String, i64)>,
    Json(body): Json<LearningReviewV3Request>,
) -> Result<Json<LearningSessionV3Response>, (StatusCode, String)> {
    learning::submit_review_v3(&state, &session_id, entry_id, body.rating)
        .await
        .map(Json)
        .map_err(internal_error)
}

pub async fn get_progress(
    State(state): State<AppState>,
) -> Result<Json<LearningProgressMapResponse>, (StatusCode, String)> {
    learning::get_progress_map(&state)
        .await
        .map(Json)
        .map_err(internal_error)
}

pub async fn get_stats(
    State(state): State<AppState>,
) -> Result<Json<LearningStatsResponse>, (StatusCode, String)> {
    learning::get_stats(&state)
        .await
        .map(Json)
        .map_err(internal_error)
}

fn internal_error(err: anyhow::Error) -> (StatusCode, String) {
    tracing::error!("learning handler failed: {err:#}");
    (StatusCode::INTERNAL_SERVER_ERROR, err.to_string())
}
