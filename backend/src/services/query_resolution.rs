use crate::ai::{AiChatOptions, AiScene};
use crate::models::{AnalysisDocument, AttachedPhraseModule, PhraseLookupInfo, PhraseUsagePreview};
use crate::services::analyze_support::extract_json;
use crate::state::AppState;
use anyhow::Result;
use serde::Deserialize;
use serde_json::Value;
use std::time::Duration;

pub fn phrase_lookup_from_analysis(analysis: &Value) -> Option<PhraseLookupInfo> {
    analysis
        .get("phrase_lookup")
        .cloned()
        .and_then(|value| serde_json::from_value(value).ok())
}

pub fn attached_phrase_modules_from_analysis(analysis: &Value) -> Vec<AttachedPhraseModule> {
    analysis
        .get("attached_phrase_modules")
        .cloned()
        .and_then(|value| serde_json::from_value(value).ok())
        .unwrap_or_default()
}

pub fn phrase_usage_preview_from_analysis(analysis: &Value) -> Option<PhraseUsagePreview> {
    analysis
        .get("phrase_usage_preview")
        .cloned()
        .and_then(|value| serde_json::from_value(value).ok())
}

pub fn build_phrase_unavailable_analysis(
    phrase: &str,
    phrase_lookup: Option<&PhraseLookupInfo>,
) -> AnalysisDocument {
    let host_line = phrase_lookup
        .and_then(|lookup| lookup.best_host_headword.as_deref())
        .map(|host| format!("- 当前已锁定的候选主词：{host}"))
        .unwrap_or_else(|| "- 当前尚未锁定可靠主词。".to_string());

    AnalysisDocument {
        markdown: format!(
            "## {phrase}\n\n- 这是一次短语/搭配查询。\n{host_line}\n- 但本轮模型暂时不可用，无法稳定生成该短语的场景化解释。\n- 建议：稍后重试，或先切换查看候选主词。"
        ),
        tags: vec!["短语待确认".to_string()],
        aliases: Vec::new(),
        prototype: phrase_lookup.and_then(|lookup| lookup.best_host_headword.clone()),
        phrase_lookup: phrase_lookup.cloned(),
        phrase_usage_preview: None,
        attached_phrase_modules: Vec::new(),
        dictionary_excerpt: None,
        model: None,
        quality_mode: None,
    }
}

pub async fn maybe_correct_spelling(state: &AppState, query: &str) -> Result<String> {
    #[derive(Deserialize)]
    struct SpellCheck {
        is_correct: bool,
        suggestion: Option<String>,
    }

    let response = state
        .ai_client
        .chat_with_options(
            AiScene::SpellCheck,
            &state.prompts.spell_checker_prompt,
            query,
            AiChatOptions {
                temperature: 0.0,
                max_tokens: 80,
                timeout: Duration::from_secs(8),
            },
        )
        .await?;
    let parsed: SpellCheck = extract_json(&response)?;

    Ok(if parsed.is_correct {
        query.to_string()
    } else {
        parsed.suggestion.unwrap_or_else(|| query.to_string())
    })
}

pub async fn identify_prototype(state: &AppState, query: &str) -> Result<String> {
    #[derive(Deserialize)]
    struct PrototypeResponse {
        prototype: String,
    }

    let response = state
        .ai_client
        .chat_with_options(
            AiScene::Prototype,
            &state.prompts.prototype_identification_prompt,
            query,
            AiChatOptions {
                temperature: 0.0,
                max_tokens: 80,
                timeout: Duration::from_secs(8),
            },
        )
        .await?;
    let parsed: PrototypeResponse = extract_json(&response)?;
    Ok(parsed.prototype)
}
