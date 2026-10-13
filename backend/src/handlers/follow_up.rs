use crate::models::{FollowUpEditRequest, FollowUpRequest, FollowUpWriteRequest};
use crate::services::follow_up;
use crate::services::follow_up_stream;
use crate::state::AppState;
use axum::{
    body::Body,
    extract::{Path, Query, State},
    http::{header, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use std::convert::Infallible;
use tokio::sync::mpsc;
use tokio_stream::{wrappers::UnboundedReceiverStream, StreamExt};

#[derive(serde::Deserialize)]
pub struct ListQuery {
    pub entry_id: i64,
}

pub async fn list(State(state): State<AppState>, Query(query): Query<ListQuery>) -> Response {
    match crate::repositories::knowledge::find_by_id(&state.pool, query.entry_id).await {
        Ok(Some(_)) => {}
        Ok(None) => return failure(anyhow::anyhow!("knowledge entry not found")),
        Err(error) => return failure(error.into()),
    }
    match crate::repositories::follow_up::list_by_entry_id(&state.pool, query.entry_id).await {
        Ok(items) => Json(serde_json::json!({"follow_ups":items})).into_response(),
        Err(error) => failure(error.into()),
    }
}

pub async fn get(State(state): State<AppState>, Path(id): Path<i64>) -> Response {
    match follow_up::get(&state, id).await {
        Ok(item) => Json(item).into_response(),
        Err(error) => failure(error),
    }
}

pub async fn save(
    State(state): State<AppState>,
    Json(request): Json<FollowUpWriteRequest>,
) -> Response {
    match follow_up::save(
        &state,
        request.entry_id,
        &request.question,
        &request.answer,
        &request.idempotency_key,
    )
    .await
    {
        Ok(item) => (StatusCode::CREATED, Json(item)).into_response(),
        Err(error) => failure(error),
    }
}

pub async fn edit(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(request): Json<FollowUpEditRequest>,
) -> Response {
    match follow_up::edit(&state, id, request).await {
        Ok(item) => Json(item).into_response(),
        Err(error) => failure(error),
    }
}

pub async fn delete(State(state): State<AppState>, Path(id): Path<i64>) -> Response {
    match crate::repositories::follow_up::delete(&state.pool, id).await {
        Ok(true) => Json(serde_json::json!({"deleted":true,"id":id})).into_response(),
        Ok(false) => failure(anyhow::anyhow!("follow-up not found")),
        Err(error) => failure(error.into()),
    }
}

fn failure(error: anyhow::Error) -> Response {
    let message = error.to_string();
    let status = if message.contains("not found") {
        StatusCode::NOT_FOUND
    } else if message.contains("changed or deleted") || message.contains("already belongs") {
        StatusCode::CONFLICT
    } else if message.contains("must contain")
        || message.contains("positive")
        || message.contains("another entry")
    {
        StatusCode::BAD_REQUEST
    } else {
        tracing::error!("follow-up failed: {error:#}");
        StatusCode::BAD_GATEWAY
    };
    (status, Json(serde_json::json!({"message":message}))).into_response()
}

pub async fn create_follow_up(
    State(state): State<AppState>,
    Json(request): Json<FollowUpRequest>,
) -> Response {
    match follow_up::create(&state, request).await {
        Ok(response) => Json(response).into_response(),
        Err(error) => failure(error),
    }
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
