use crate::models::{DatabaseBackupPayload, DatabaseImportResponse};
use crate::repositories::{follow_up, knowledge, learning, management};
use crate::state::AppState;
use anyhow::{Context, Result};
use chrono::Utc;

pub async fn export_database(state: &AppState) -> Result<String> {
    let payload = DatabaseBackupPayload {
        format: "de_ai_hilfer_backup_v1".to_string(),
        exported_at: Utc::now(),
        knowledge_entries: knowledge::list_all(&state.pool).await?,
        follow_ups: follow_up::list_all(&state.pool).await?,
        learning_progress: learning::list_all_progress(&state.pool).await?,
    };

    serde_json::to_string_pretty(&payload).context("failed to serialize database backup")
}

pub async fn import_database(
    state: &AppState,
    file_bytes: &[u8],
) -> Result<DatabaseImportResponse> {
    let payload: DatabaseBackupPayload =
        serde_json::from_slice(file_bytes).context("failed to parse backup json")?;

    if payload.format != "de_ai_hilfer_backup_v1" {
        anyhow::bail!("unsupported backup format: {}", payload.format);
    }

    management::replace_database_snapshot(&state.pool, &payload).await?;

    Ok(DatabaseImportResponse {
        message: format!(
            "已导入 {} 个词条、{} 条追问和 {} 条学习进度",
            payload.knowledge_entries.len(),
            payload.follow_ups.len(),
            payload.learning_progress.len()
        ),
    })
}
