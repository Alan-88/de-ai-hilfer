use crate::models::DatabaseImportResponse;
use crate::services::management;
use crate::state::AppState;
use axum::{
    extract::{Multipart, State},
    http::{
        header::{CONTENT_DISPOSITION, CONTENT_TYPE},
        HeaderMap, HeaderValue, StatusCode,
    },
    response::IntoResponse,
    Json,
};
use chrono::Utc;

pub async fn export_database(
    State(state): State<AppState>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let body = management::export_database(&state)
        .await
        .map_err(internal_error)?;
    let filename = format!(
        "de_ai_hilfer_backup_{}.json",
        Utc::now().format("%Y%m%d_%H%M%S")
    );

    let mut headers = HeaderMap::new();
    headers.insert(
        CONTENT_TYPE,
        HeaderValue::from_static("application/json; charset=utf-8"),
    );
    headers.insert(
        CONTENT_DISPOSITION,
        HeaderValue::from_str(&format!("attachment; filename=\"{filename}\""))
            .map_err(|err| (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()))?,
    );

    Ok((headers, body))
}

pub async fn import_database(
    State(state): State<AppState>,
    mut multipart: Multipart,
) -> Result<Json<DatabaseImportResponse>, (StatusCode, String)> {
    let mut backup_bytes = None;

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|err| (StatusCode::BAD_REQUEST, err.to_string()))?
    {
        if field.name() == Some("backup_file") || backup_bytes.is_none() {
            let filename = field.file_name().unwrap_or_default().to_string();
            if !filename.is_empty() && !filename.ends_with(".json") {
                return Err((StatusCode::BAD_REQUEST, "仅支持 .json 备份文件".to_string()));
            }

            backup_bytes = Some(
                field
                    .bytes()
                    .await
                    .map_err(|err| (StatusCode::BAD_REQUEST, err.to_string()))?
                    .to_vec(),
            );
        }
    }

    let backup_bytes = backup_bytes.ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            "未接收到 backup_file 文件字段".to_string(),
        )
    })?;

    management::import_database(&state, &backup_bytes)
        .await
        .map(Json)
        .map_err(internal_error)
}

fn internal_error(err: anyhow::Error) -> (StatusCode, String) {
    tracing::error!("management handler failed: {err:#}");
    (StatusCode::INTERNAL_SERVER_ERROR, err.to_string())
}
