use crate::models::{EntryDeleteResponse, EntryDetailResponse, FollowUpItem};
use crate::repositories::{follow_up, knowledge};
use crate::services::analysis_preview::analysis_markdown;
use crate::services::query_resolution::phrase_usage_preview_from_analysis;
use crate::state::AppState;
use anyhow::{anyhow, Result};
use serde_json::Value;

pub async fn get_entry_detail(state: &AppState, entry_id: i64) -> Result<EntryDetailResponse> {
    let entry = knowledge::find_by_id(&state.pool, entry_id)
        .await?
        .ok_or_else(|| anyhow!("knowledge entry not found"))?;
    let follow_ups = follow_up::list_by_entry_id(&state.pool, entry_id)
        .await?
        .into_iter()
        .map(|item| FollowUpItem {
            id: item.id,
            question: item.question,
            answer: item.answer,
            created_at: item.created_at,
        })
        .collect::<Vec<_>>();

    Ok(EntryDetailResponse {
        entry_id: entry.id,
        query_text: entry.query_text,
        entry_type: entry.entry_type,
        prototype: entry.prototype,
        analysis_markdown: analysis_markdown(&entry.analysis),
        phrase_lookup: entry
            .analysis
            .get("phrase_lookup")
            .cloned()
            .and_then(|value| serde_json::from_value(value).ok()),
        phrase_usage_preview: phrase_usage_preview_from_analysis(&entry.analysis),
        attached_phrase_modules: entry
            .analysis
            .get("attached_phrase_modules")
            .cloned()
            .and_then(|value| serde_json::from_value(value).ok())
            .unwrap_or_default(),
        source: "知识库".to_string(),
        model: entry
            .analysis
            .get("model")
            .and_then(Value::as_str)
            .map(ToString::to_string),
        quality_mode: entry
            .analysis
            .get("quality_mode")
            .and_then(|value| serde_json::from_value(value.clone()).ok()),
        tags: entry.tags.unwrap_or_default(),
        aliases: entry.aliases.unwrap_or_default(),
        follow_ups,
        created_at: entry.created_at,
        updated_at: entry.updated_at,
    })
}

pub async fn delete_entry(state: &AppState, entry_id: i64) -> Result<EntryDeleteResponse> {
    let affected = knowledge::delete_by_id(&state.pool, entry_id).await?;
    if affected == 0 {
        return Err(anyhow!("knowledge entry not found"));
    }

    Ok(EntryDeleteResponse {
        message: format!("已删除词条 #{entry_id}"),
        deleted_entry_id: entry_id,
    })
}
