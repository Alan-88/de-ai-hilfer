use crate::models::{
    AiModelTestRequest, AiModelTestResponse, AiSettingsResponse, AiSettingsUpdateRequest,
};
use crate::services::ai_settings;
use crate::state::AppState;
use axum::{extract::State, http::StatusCode, Json};

pub async fn get_ai_settings(
    State(state): State<AppState>,
) -> Result<Json<AiSettingsResponse>, (StatusCode, String)> {
    ai_settings::get_ai_settings(&state)
        .await
        .map(Json)
        .map_err(settings_error)
}

pub async fn update_ai_settings(
    State(state): State<AppState>,
    Json(request): Json<AiSettingsUpdateRequest>,
) -> Result<Json<AiSettingsResponse>, (StatusCode, String)> {
    ai_settings::update_ai_settings(&state, request)
        .await
        .map(Json)
        .map_err(settings_error)
}

pub async fn test_ai_model(
    State(state): State<AppState>,
    Json(request): Json<AiModelTestRequest>,
) -> Result<Json<AiModelTestResponse>, (StatusCode, String)> {
    ai_settings::test_ai_model(&state, request)
        .await
        .map(Json)
        .map_err(settings_error)
}

fn settings_error(err: anyhow::Error) -> (StatusCode, String) {
    let message = err.to_string();
    if message.contains("required")
        || message.contains("unsupported")
        || message.contains("duplicate")
        || message.contains("unknown provider")
        || message.contains("must be")
        || message.contains("cannot be empty")
    {
        return (StatusCode::BAD_REQUEST, message);
    }

    tracing::error!("ai settings handler failed: {err:#}");
    (StatusCode::INTERNAL_SERVER_ERROR, message)
}
