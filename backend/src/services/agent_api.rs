use crate::ai_settings_models::AiModelOverride;
use crate::models::{AnalyzeRequest, AnalyzeResponse, LearningProgressView, LexemeCandidate};
use crate::repositories::{agent_api as repository, dictionary_lexemes, knowledge};
use crate::services::{analyze, learning};
use crate::state::AppState;
use anyhow::{anyhow, Result};
use chrono::{DateTime, Utc};
use serde::Serialize;
use serde_json::Value;

#[derive(Debug, Serialize)]
pub struct LexemeResolveResponse {
    pub surface: String,
    pub candidates: Vec<LexemeCandidate>,
}

#[derive(Debug, Serialize)]
pub struct LexemeStateResponse {
    pub lexeme: LexemeSummary,
    pub knowledge: KnowledgeState,
    pub learning: LearningState,
}

#[derive(Debug, Serialize)]
pub struct LexemeSummary {
    pub id: i64,
    pub headword: String,
    pub pos: Vec<String>,
    pub gloss: Value,
}

#[derive(Debug, Serialize)]
pub struct KnowledgeState {
    pub status: &'static str,
    pub entry_id: Option<i64>,
    pub query_text: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct LearningState {
    pub status: &'static str,
    pub due_date: Option<DateTime<Utc>>,
    pub review_count: Option<i32>,
    pub lapses: Option<i32>,
}

#[derive(Debug, Serialize)]
pub struct LearningListItem {
    pub entry_id: i64,
    pub lexeme_id: Option<i64>,
    pub query_text: String,
    pub status: &'static str,
    pub due_date: DateTime<Utc>,
    pub review_count: i32,
    pub lapses: i32,
}

#[derive(Debug, Serialize)]
pub struct LearningMutationResponse {
    pub entry_id: i64,
    pub status: &'static str,
}

pub async fn resolve_lexeme(
    state: &AppState,
    surface: &str,
    limit: i64,
) -> Result<LexemeResolveResponse> {
    let surface = surface.trim();
    anyhow::ensure!(!surface.is_empty(), "surface cannot be empty");
    let candidates = dictionary_lexemes::find_lexeme_candidates_by_surface(
        &state.pool,
        surface,
        limit.clamp(1, 20),
    )
    .await?;
    Ok(LexemeResolveResponse {
        surface: surface.to_string(),
        candidates,
    })
}

pub async fn inspect_lexeme(state: &AppState, lexeme_id: i64) -> Result<LexemeStateResponse> {
    let row = repository::find_lexeme_state(&state.pool, lexeme_id)
        .await?
        .ok_or_else(|| anyhow!("lexeme not found"))?;
    Ok(LexemeStateResponse {
        lexeme: LexemeSummary {
            id: row.lexeme_id,
            headword: row.surface,
            pos: row.pos_summary,
            gloss: row.gloss_preview,
        },
        knowledge: KnowledgeState {
            status: if row.entry_id.is_some() {
                "ready"
            } else {
                "missing"
            },
            entry_id: row.entry_id,
            query_text: row.query_text,
        },
        learning: LearningState {
            status: learning_status(row.learning_state),
            due_date: row.due_date,
            review_count: row.review_count,
            lapses: row.lapses,
        },
    })
}

pub async fn initialize_lexeme(
    state: &AppState,
    lexeme_id: i64,
    model_override: Option<AiModelOverride>,
) -> Result<AnalyzeResponse> {
    let current = inspect_lexeme(state, lexeme_id).await?;
    let result = analyze::analyze(
        state,
        AnalyzeRequest {
            query_text: current.lexeme.headword,
            entry_type: Some("WORD".to_string()),
            generation_hint: None,
            quality_mode: Default::default(),
            force_refresh: false,
            entry_id: None,
            model_override,
        },
    )
    .await?;
    anyhow::ensure!(
        result.entry_id > 0,
        "lexeme initialization did not create an entry"
    );
    Ok(result)
}

pub async fn list_learning(state: &AppState) -> Result<Vec<LearningListItem>> {
    Ok(repository::list_learning(&state.pool)
        .await?
        .into_iter()
        .map(|row| LearningListItem {
            entry_id: row.entry_id,
            lexeme_id: row.lexeme_id,
            query_text: row.query_text,
            status: learning_status(Some(row.learning_state)),
            due_date: row.due_date,
            review_count: row.review_count,
            lapses: row.lapses,
        })
        .collect())
}

pub async fn add_learning(state: &AppState, entry_id: i64) -> Result<LearningProgressView> {
    learning::add_word(state, entry_id).await
}

pub async fn remove_learning(state: &AppState, entry_id: i64) -> Result<LearningMutationResponse> {
    knowledge::find_by_id(&state.pool, entry_id)
        .await?
        .ok_or_else(|| anyhow!("knowledge entry not found"))?;
    repository::remove_learning(&state.pool, entry_id).await?;
    Ok(LearningMutationResponse {
        entry_id,
        status: "not_added",
    })
}

pub async fn reset_learning(state: &AppState, entry_id: i64) -> Result<LearningMutationResponse> {
    knowledge::find_by_id(&state.pool, entry_id)
        .await?
        .ok_or_else(|| anyhow!("knowledge entry not found"))?;
    repository::reset_learning(&state.pool, entry_id).await?;
    Ok(LearningMutationResponse {
        entry_id,
        status: "new",
    })
}

fn learning_status(state: Option<i32>) -> &'static str {
    match state {
        None => "not_added",
        Some(0) => "new",
        Some(1) => "learning",
        Some(2) => "review",
        Some(3) => "relearning",
        Some(_) => "unknown",
    }
}

#[cfg(test)]
mod tests {
    use super::learning_status;

    #[test]
    fn maps_learning_states() {
        assert_eq!(learning_status(None), "not_added");
        assert_eq!(learning_status(Some(0)), "new");
        assert_eq!(learning_status(Some(3)), "relearning");
        assert_eq!(learning_status(Some(99)), "unknown");
    }
}
