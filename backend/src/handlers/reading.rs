use crate::reading_models::{
    ReadingArticleFolderRequest, ReadingArticleSaveRequest, ReadingArticleStatusRequest,
    ReadingArticleTitleRequest, ReadingErrorResponse, ReadingFolderRequest,
    ReadingMutationResponse,
};
use crate::services::reading::{self, ReadingError};
use crate::state::AppState;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};

type ApiError = (StatusCode, Json<ReadingErrorResponse>);

pub async fn index(
    State(state): State<AppState>,
) -> Result<Json<crate::reading_models::ReadingIndexResponse>, ApiError> {
    reading::index(&state).await.map(Json).map_err(map_error)
}

pub async fn get_article(
    State(state): State<AppState>,
    Path(article_id): Path<i64>,
) -> Result<Json<crate::reading_models::ReadingArticleDetail>, ApiError> {
    reading::article_detail(&state, article_id)
        .await
        .map(Json)
        .map_err(map_error)
}

pub async fn create_article(
    State(state): State<AppState>,
    Json(body): Json<ReadingArticleSaveRequest>,
) -> Result<
    (
        StatusCode,
        Json<crate::reading_models::ReadingArticleDetail>,
    ),
    ApiError,
> {
    reading::create_article(&state, &body.title, &body.body, body.folder_id)
        .await
        .map(|article| (StatusCode::CREATED, Json(article)))
        .map_err(map_error)
}

pub async fn save_article(
    State(state): State<AppState>,
    Path(article_id): Path<i64>,
    Json(body): Json<ReadingArticleSaveRequest>,
) -> Result<Json<crate::reading_models::ReadingArticleDetail>, ApiError> {
    reading::save_article(&state, article_id, &body.title, &body.body, body.folder_id)
        .await
        .map(Json)
        .map_err(map_error)
}

pub async fn rename_article(
    State(state): State<AppState>,
    Path(article_id): Path<i64>,
    Json(body): Json<ReadingArticleTitleRequest>,
) -> Result<Json<ReadingMutationResponse>, ApiError> {
    reading::rename_article(&state, article_id, &body.title)
        .await
        .map(|_| Json(ReadingMutationResponse { id: article_id }))
        .map_err(map_error)
}

pub async fn move_article(
    State(state): State<AppState>,
    Path(article_id): Path<i64>,
    Json(body): Json<ReadingArticleFolderRequest>,
) -> Result<Json<ReadingMutationResponse>, ApiError> {
    reading::move_article(&state, article_id, body.folder_id)
        .await
        .map(|_| Json(ReadingMutationResponse { id: article_id }))
        .map_err(map_error)
}

pub async fn set_article_status(
    State(state): State<AppState>,
    Path(article_id): Path<i64>,
    Json(body): Json<ReadingArticleStatusRequest>,
) -> Result<Json<ReadingMutationResponse>, ApiError> {
    reading::set_article_status(&state, article_id, body.completed)
        .await
        .map(|_| Json(ReadingMutationResponse { id: article_id }))
        .map_err(map_error)
}

pub async fn mark_article_opened(
    State(state): State<AppState>,
    Path(article_id): Path<i64>,
) -> Result<Json<ReadingMutationResponse>, ApiError> {
    reading::mark_article_opened(&state, article_id)
        .await
        .map(|_| Json(ReadingMutationResponse { id: article_id }))
        .map_err(map_error)
}

pub async fn delete_article(
    State(state): State<AppState>,
    Path(article_id): Path<i64>,
) -> Result<Json<ReadingMutationResponse>, ApiError> {
    reading::delete_article(&state, article_id)
        .await
        .map(|_| Json(ReadingMutationResponse { id: article_id }))
        .map_err(map_error)
}

pub async fn create_folder(
    State(state): State<AppState>,
    Json(body): Json<ReadingFolderRequest>,
) -> Result<(StatusCode, Json<crate::reading_models::ReadingFolder>), ApiError> {
    reading::create_folder(&state, &body.name, body.parent_id)
        .await
        .map(|folder| (StatusCode::CREATED, Json(folder)))
        .map_err(map_error)
}

pub async fn update_folder(
    State(state): State<AppState>,
    Path(folder_id): Path<i64>,
    Json(body): Json<ReadingFolderRequest>,
) -> Result<Json<crate::reading_models::ReadingFolder>, ApiError> {
    reading::update_folder(&state, folder_id, &body.name, body.parent_id)
        .await
        .map(Json)
        .map_err(map_error)
}

pub async fn delete_folder(
    State(state): State<AppState>,
    Path(folder_id): Path<i64>,
) -> Result<Json<ReadingMutationResponse>, ApiError> {
    reading::delete_folder(&state, folder_id)
        .await
        .map(|_| Json(ReadingMutationResponse { id: folder_id }))
        .map_err(map_error)
}

fn map_error(error: ReadingError) -> ApiError {
    let (status, message, bookmarks) = match error {
        ReadingError::BadRequest(message) => (StatusCode::BAD_REQUEST, message, Vec::new()),
        ReadingError::NotFound(message) => (StatusCode::NOT_FOUND, message, Vec::new()),
        ReadingError::Conflict(message) => (StatusCode::CONFLICT, message, Vec::new()),
        ReadingError::BookmarkValidation(bookmarks) => (
            StatusCode::UNPROCESSABLE_ENTITY,
            "书签校验失败，文章未保存".to_string(),
            bookmarks,
        ),
        ReadingError::Database(error) => {
            tracing::error!("reading handler failed: {error:#}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "文章服务暂时不可用".to_string(),
                Vec::new(),
            )
        }
    };
    (status, Json(ReadingErrorResponse { message, bookmarks }))
}
