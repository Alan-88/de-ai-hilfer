use crate::models::{FollowUpRequest, FollowUpResponse};
use crate::services::follow_up;
use crate::services::follow_up_stream;
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

pub async fn create_follow_up(
    State(state): State<AppState>,
    Json(request): Json<FollowUpRequest>,
) -> Result<Json<FollowUpResponse>, (StatusCode, String)> {
    follow_up::create(&state, request)
        .await
        .map(Json)
        .map_err(|err| {
            tracing::error!("follow-up failed: {err:#}");
            (StatusCode::INTERNAL_SERVER_ERROR, err.to_string())
        })
}

pub async fn stream_follow_up(
    State(state): State<AppState>,
    Json(request): Json<FollowUpRequest>,
) -> Result<Response, (StatusCode, String)> {
    let (tx, rx) = mpsc::unbounded_channel::<String>();
    tokio::spawn(async move {
        if let Err(err) = follow_up_stream::stream_follow_up(state, request, tx.clone()).await {
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
