use crate::models::{
    EntryDeleteResponse, EntryDetailResponse, IntelligentSearchRequest, LibraryEntriesPageResponse,
    LibraryQueryTab, StatusResponse, SuggestionResponse,
};
use crate::services::{library, query};
use crate::state::AppState;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use serde::Deserialize;

#[derive(Deserialize)]
pub struct SuggestionsQuery {
    pub q: String,
}

#[derive(Deserialize)]
pub struct LibraryEntriesQuery {
    pub q: Option<String>,
    pub tab: Option<LibraryQueryTab>,
    pub cursor: Option<String>,
    pub limit: Option<i64>,
}

pub async fn get_recent_entries(
    State(state): State<AppState>,
) -> Result<Json<Vec<crate::models::RecentItem>>, (StatusCode, String)> {
    query::get_recent_entries(&state)
        .await
        .map(Json)
        .map_err(internal_error)
}

pub async fn get_all_entries(
    State(state): State<AppState>,
) -> Result<Json<Vec<crate::models::RecentItem>>, (StatusCode, String)> {
    query::get_all_entries(&state)
        .await
        .map(Json)
        .map_err(internal_error)
}

pub async fn get_library_entries_page(
    State(state): State<AppState>,
    Query(params): Query<LibraryEntriesQuery>,
) -> Result<Json<LibraryEntriesPageResponse>, (StatusCode, String)> {
    query::get_library_entries_page(
        &state,
        params.q.as_deref().unwrap_or_default(),
        params.tab.unwrap_or_default(),
        params.limit.unwrap_or(24),
        params.cursor.as_deref(),
    )
    .await
    .map(Json)
    .map_err(library_page_error)
}

pub async fn get_entry_detail(
    State(state): State<AppState>,
    Path(entry_id): Path<i64>,
) -> Result<Json<EntryDetailResponse>, (StatusCode, String)> {
    library::get_entry_detail(&state, entry_id)
        .await
        .map(Json)
        .map_err(library_error)
}

pub async fn delete_entry(
    State(state): State<AppState>,
    Path(entry_id): Path<i64>,
) -> Result<Json<EntryDeleteResponse>, (StatusCode, String)> {
    library::delete_entry(&state, entry_id)
        .await
        .map(Json)
        .map_err(library_error)
}

pub async fn get_suggestions(
    State(state): State<AppState>,
    Query(params): Query<SuggestionsQuery>,
) -> Result<Json<SuggestionResponse>, (StatusCode, String)> {
    query::get_suggestions(&state, &params.q)
        .await
        .map(Json)
        .map_err(internal_error)
}

pub async fn intelligent_search(
    State(state): State<AppState>,
    Json(request): Json<IntelligentSearchRequest>,
) -> Result<Json<crate::models::AnalyzeResponse>, (StatusCode, String)> {
    query::intelligent_search(&state, request)
        .await
        .map(Json)
        .map_err(internal_error)
}

pub async fn get_status(
    State(state): State<AppState>,
) -> Result<Json<StatusResponse>, (StatusCode, String)> {
    query::status(&state)
        .await
        .map(Json)
        .map_err(internal_error)
}

fn internal_error(err: anyhow::Error) -> (StatusCode, String) {
    tracing::error!("query handler failed: {err:#}");
    (StatusCode::INTERNAL_SERVER_ERROR, err.to_string())
}

fn library_error(err: anyhow::Error) -> (StatusCode, String) {
    let message = err.to_string();
    if message.contains("knowledge entry not found") {
        return (StatusCode::NOT_FOUND, message);
    }
    internal_error(err)
}

fn library_page_error(err: anyhow::Error) -> (StatusCode, String) {
    let message = err.to_string();
    if message.contains("invalid library cursor") {
        return (StatusCode::BAD_REQUEST, message);
    }
    internal_error(err)
}
