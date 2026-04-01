use crate::models::{AnalyzeRequest, AnalyzeResponse, AttachPhraseRequest, DetachPhraseRequest};
use crate::services::analyze;
use crate::services::analyze_stream;
use crate::services::phrase_attach;
use crate::state::AppState;
use axum::{
    body::Body,
    extract::State,
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
            (StatusCode::INTERNAL_SERVER_ERROR, err.to_string())
        })
}

pub async fn stream_analyze_word(
    State(state): State<AppState>,
    Json(request): Json<AnalyzeRequest>,
) -> Result<Response, (StatusCode, String)> {
    let (tx, rx) = mpsc::unbounded_channel::<String>();
    tokio::spawn(async move {
        if let Err(err) = analyze_stream::stream_analyze(state, request, tx.clone()).await {
            let _ = tx.send(crate::services::stream_response::sse_error(err.to_string()));
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

pub async fn attach_phrase_to_host(
    State(state): State<AppState>,
    Json(request): Json<AttachPhraseRequest>,
) -> Result<Json<AnalyzeResponse>, (StatusCode, String)> {
    phrase_attach::attach_phrase_to_host(&state, request)
        .await
        .map(Json)
        .map_err(|err| {
            tracing::error!("attach phrase failed: {err:#}");
            (StatusCode::INTERNAL_SERVER_ERROR, err.to_string())
        })
}

pub async fn detach_phrase_from_host(
    State(state): State<AppState>,
    Json(request): Json<DetachPhraseRequest>,
) -> Result<Json<AnalyzeResponse>, (StatusCode, String)> {
    phrase_attach::detach_phrase_from_host(&state, request)
        .await
        .map(Json)
        .map_err(|err| {
            tracing::error!("detach phrase failed: {err:#}");
            (StatusCode::INTERNAL_SERVER_ERROR, err.to_string())
        })
}
