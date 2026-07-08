use crate::models::{
    AddPhraseModuleRequest, AnalyzeRequest, AnalyzeResponse, DeletePhraseModuleRequest,
};
use crate::services::analyze;
use crate::services::analyze_stream;
use crate::services::phrase_module;
use crate::state::AppState;
use axum::{
    body::Body,
    extract::{Path, State},
    http::{header, HeaderValue, StatusCode},
    response::Response,
    Json,
};
use std::convert::Infallible;
use tokio::sync::mpsc;
use tokio_stream::{wrappers::UnboundedReceiverStream, StreamExt};

pub async fn analyze_word(
    State(state): State<AppState>,
    Json(request): Json<AnalyzeRequest>,
) -> Result<Json<AnalyzeResponse>, (StatusCode, String)> {
    analyze::analyze(&state, request)
        .await
        .map(Json)
        .map_err(|err| {
            tracing::error!("analyze failed: {err:#}");
            (StatusCode::INTERNAL_SERVER_ERROR, format!("{err:#}"))
        })
}

pub async fn stream_analyze_word(
    State(state): State<AppState>,
    Json(request): Json<AnalyzeRequest>,
) -> Result<Response, (StatusCode, String)> {
    let (tx, rx) = mpsc::unbounded_channel::<String>();
    tokio::spawn(async move {
        if let Err(err) = analyze_stream::stream_analyze(state, request, tx.clone()).await {
            tracing::error!("stream analyze failed: {err:#}");
            let _ = tx.send(crate::services::stream_response::sse_error(format!(
                "{err:#}"
            )));
        }
    });

    let stream = UnboundedReceiverStream::new(rx).map(|chunk| Ok::<_, Infallible>(chunk));
    let mut response = Response::new(Body::from_stream(stream));
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("text/event-stream; charset=utf-8"),
    );
    response
        .headers_mut()
        .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-cache"));
    Ok(response)
}

pub async fn add_phrase_module_to_entry(
    State(state): State<AppState>,
    Path(entry_id): Path<i64>,
    Json(request): Json<AddPhraseModuleRequest>,
) -> Result<Json<AnalyzeResponse>, (StatusCode, String)> {
    phrase_module::add_phrase_module_to_entry(&state, entry_id, request)
        .await
        .map(Json)
        .map_err(|err| {
            tracing::error!("add phrase module failed: {err:#}");
            (StatusCode::INTERNAL_SERVER_ERROR, err.to_string())
        })
}

pub async fn delete_phrase_module_from_entry(
    State(state): State<AppState>,
    Path(entry_id): Path<i64>,
    Json(request): Json<DeletePhraseModuleRequest>,
) -> Result<Json<AnalyzeResponse>, (StatusCode, String)> {
    phrase_module::delete_phrase_module_from_entry(&state, entry_id, request)
        .await
        .map(Json)
        .map_err(|err| {
            tracing::error!("delete phrase module failed: {err:#}");
            (StatusCode::INTERNAL_SERVER_ERROR, err.to_string())
        })
}
