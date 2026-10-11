use crate::ai_settings_models::AiModelOverride;
use crate::models::LearningRecallRating;
use crate::services::agent_api;
use crate::state::AppState;
use axum::{
    body::Body,
    extract::{Path, Query, State},
    http::{header, Request, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use chrono::NaiveDate;
use serde::Deserialize;

#[derive(Deserialize)]
pub struct ResolveQuery {
    pub q: String,
    pub limit: Option<i64>,
}

#[derive(Default, Deserialize)]
pub struct LearningLogsQuery {
    pub date: Option<NaiveDate>,
    pub entry_id: Option<i64>,
    pub session_id: Option<String>,
    pub limit: Option<i64>,
}

#[derive(Default, Deserialize)]
pub struct InitializeRequest {
    #[serde(default)]
    pub model_override: Option<AiModelOverride>,
}

#[derive(Deserialize)]
pub struct CorrectFirstRatingRequest {
    pub rating: LearningRecallRating,
}

pub async fn authorize(
    State(state): State<AppState>,
    request: Request<Body>,
    next: Next,
) -> Response {
    let Some(expected) = state.config.agent_api_token.as_deref() else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            "agent API is not configured",
        )
            .into_response();
    };
    let supplied = request
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "));
    if supplied != Some(expected) {
        return (StatusCode::UNAUTHORIZED, "invalid agent API token").into_response();
    }
    next.run(request).await
}

pub async fn resolve_lexeme(
    State(state): State<AppState>,
    Query(query): Query<ResolveQuery>,
) -> impl IntoResponse {
    match agent_api::resolve_lexeme(&state, &query.q, query.limit.unwrap_or(8)).await {
        Ok(response) => Json(response).into_response(),
        Err(error) => api_error(error),
    }
}

pub async fn inspect_lexeme(
    State(state): State<AppState>,
    Path(lexeme_id): Path<i64>,
) -> impl IntoResponse {
    match agent_api::inspect_lexeme(&state, lexeme_id).await {
        Ok(response) => Json(response).into_response(),
        Err(error) => api_error(error),
    }
}

pub async fn initialize_lexeme(
    State(state): State<AppState>,
    Path(lexeme_id): Path<i64>,
    Json(request): Json<InitializeRequest>,
) -> impl IntoResponse {
    match agent_api::initialize_lexeme(&state, lexeme_id, request.model_override).await {
        Ok(response) => Json(response).into_response(),
        Err(error) => api_error(error),
    }
}

pub async fn list_learning(State(state): State<AppState>) -> impl IntoResponse {
    match agent_api::list_learning(&state).await {
        Ok(response) => Json(response).into_response(),
        Err(error) => api_error(error),
    }
}

pub async fn list_learning_logs(
    State(state): State<AppState>,
    Query(query): Query<LearningLogsQuery>,
) -> impl IntoResponse {
    match agent_api::list_learning_logs(
        &state,
        query.date,
        query.entry_id,
        query.session_id.as_deref(),
        query.limit.unwrap_or(100),
    )
    .await
    {
        Ok(response) => Json(response).into_response(),
        Err(error) => api_error(error),
    }
}

pub async fn get_learning_plan(State(state): State<AppState>) -> impl IntoResponse {
    match agent_api::get_learning_plan(&state).await {
        Ok(response) => Json(response).into_response(),
        Err(error) => api_error(error),
    }
}

pub async fn add_learning(
    State(state): State<AppState>,
    Path(entry_id): Path<i64>,
) -> impl IntoResponse {
    match agent_api::add_learning(&state, entry_id).await {
        Ok(response) => Json(response).into_response(),
        Err(error) => api_error(error),
    }
}

pub async fn remove_learning(
    State(state): State<AppState>,
    Path(entry_id): Path<i64>,
) -> impl IntoResponse {
    match agent_api::remove_learning(&state, entry_id).await {
        Ok(response) => Json(response).into_response(),
        Err(error) => api_error(error),
    }
}

pub async fn reset_learning(
    State(state): State<AppState>,
    Path(entry_id): Path<i64>,
) -> impl IntoResponse {
    match agent_api::reset_learning(&state, entry_id).await {
        Ok(response) => Json(response).into_response(),
        Err(error) => api_error(error),
    }
}

pub async fn reset_intraday(
    State(state): State<AppState>,
    Path(entry_id): Path<i64>,
) -> impl IntoResponse {
    match agent_api::reset_intraday(&state, entry_id).await {
        Ok(response) => Json(response).into_response(),
        Err(error) => api_error(error),
    }
}

pub async fn undo_last_intraday(
    State(state): State<AppState>,
    Path(entry_id): Path<i64>,
) -> impl IntoResponse {
    match agent_api::undo_last_intraday(&state, entry_id).await {
        Ok(response) => Json(response).into_response(),
        Err(error) => api_error(error),
    }
}

pub async fn correct_first_rating(
    State(state): State<AppState>,
    Path(entry_id): Path<i64>,
    Json(request): Json<CorrectFirstRatingRequest>,
) -> impl IntoResponse {
    let rating = match request.rating {
        LearningRecallRating::Known => "known",
        LearningRecallRating::Fuzzy => "fuzzy",
        LearningRecallRating::Forgotten => "forgotten",
    };
    match agent_api::correct_first_rating(&state, entry_id, rating).await {
        Ok(response) => Json(response).into_response(),
        Err(error) => api_error(error),
    }
}

fn api_error(error: anyhow::Error) -> Response {
    let message = format!("{error:#}");
    let status = if message.contains("not found") {
        StatusCode::NOT_FOUND
    } else if message.contains("cannot be empty")
        || message.contains("did not create")
        || message.contains("no active")
        || message.contains("no first feedback")
        || message.contains("no progress snapshot")
        || message.contains("invalid rating")
        || message.contains("unchanged")
        || message.contains("refusing automatic correction")
    {
        StatusCode::BAD_REQUEST
    } else {
        tracing::error!("agent API failed: {message}");
        StatusCode::INTERNAL_SERVER_ERROR
    };
    (status, message).into_response()
}
