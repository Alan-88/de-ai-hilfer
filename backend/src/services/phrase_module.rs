use crate::ai::{is_hard_failure, AiChatOptions};
use crate::models::{
    AddPhraseModuleRequest, AnalysisDocument, AnalyzeResponse, AttachedPhraseModule,
    PhraseLookupConfidence, PhraseUsageModule, PhraseUsagePreview,
};
use crate::repositories::{follow_up, knowledge};
use crate::services::analysis_preview::{analysis_markdown, structured_analysis};
use crate::services::analyze_runtime::{fallback_model_for, primary_model_for};
use crate::services::analyze_support::{extract_json, render_phrase_preview_markdown};
use crate::services::query_resolution::{
    attached_phrase_modules_from_analysis, phrase_lookup_from_analysis,
    phrase_usage_preview_from_analysis,
};
use crate::state::AppState;
use anyhow::{anyhow, Context, Result};
use chrono::Utc;
use serde::Deserialize;
use serde_json::json;
use std::time::Duration;

#[derive(Debug, Deserialize)]
struct PhraseModuleModelOutput {
    #[serde(default)]
    related: bool,
    #[serde(default)]
    reason: String,
    #[serde(default)]
    usage_module: Option<PhraseUsageModule>,
}

pub async fn add_phrase_module_to_entry(
    state: &AppState,
    entry_id: i64,
    request: AddPhraseModuleRequest,
) -> Result<AnalyzeResponse> {
    let phrase = request.phrase.trim();
    anyhow::ensure!(!phrase.is_empty(), "phrase cannot be empty");

    let entry = knowledge::find_by_id(&state.pool, entry_id)
        .await?
        .ok_or_else(|| anyhow!("knowledge entry not found"))?;
    let mut host_analysis: AnalysisDocument = serde_json::from_value(entry.analysis.clone())
        .context("failed to parse host analysis document")?;

    let generated =
        generate_phrase_usage_module(state, &entry.query_text, &host_analysis, &request).await?;
    let attachment = build_attachment(&entry.query_text, phrase, generated.usage_module);

    let mut attachments = attached_phrase_modules_from_analysis(&entry.analysis);
    attachments.retain(|item| {
        item.phrase.trim().to_lowercase() != attachment.phrase.trim().to_lowercase()
    });
    attachments.push(attachment);
    attachments.sort_by(|left, right| {
        right
            .attached_at
            .cmp(&left.attached_at)
            .then(left.phrase.cmp(&right.phrase))
    });

    host_analysis.attached_phrase_modules = attachments;
    let updated_analysis = serde_json::to_value(&host_analysis)?;
    let updated_entry = knowledge::update_analysis(
        &state.pool,
        entry.id,
        entry.lexeme_id,
        &updated_analysis,
        &entry.tags,
        &entry.aliases,
    )
    .await?;

    build_response(state, updated_entry).await
}

async fn generate_phrase_usage_module(
    state: &AppState,
    host_headword: &str,
    host_analysis: &AnalysisDocument,
    request: &AddPhraseModuleRequest,
) -> Result<PhraseModuleModelOutput> {
    let prompt = state.prompts.phrase_module_prompt.trim();
    anyhow::ensure!(!prompt.is_empty(), "phrase module prompt is empty");

    let user_payload = build_phrase_module_user_payload(host_headword, host_analysis, request);
    let primary_model = primary_model_for(state, request.quality_mode);
    let fallback_model = fallback_model_for(state, request.quality_mode);
    let options = phrase_module_chat_options();

    let raw = match state
        .ai_client
        .chat_model_with_options(primary_model, prompt, &user_payload, options)
        .await
    {
        Ok(raw) => raw,
        Err(primary_err)
            if is_hard_failure(&primary_err)
                && !fallback_model.is_empty()
                && fallback_model != primary_model =>
        {
            tracing::warn!(
                "phrase module switching to fallback model: primary={primary_model}, fallback={fallback_model}, host={host_headword}, err={primary_err:#}"
            );
            state
                .ai_client
                .chat_model_with_options(fallback_model, prompt, &user_payload, options)
                .await?
        }
        Err(primary_err) => return Err(primary_err),
    };

    let mut parsed: PhraseModuleModelOutput = extract_json(&raw)?;
    if !parsed.related {
        let reason = parsed.reason.trim();
        return Err(anyhow!(
            "phrase is not related to host{}",
            if reason.is_empty() {
                String::new()
            } else {
                format!(": {reason}")
            }
        ));
    }

    let usage_module = normalize_usage_module(parsed.usage_module.take(), &request.phrase)?;
    parsed.usage_module = Some(usage_module);
    Ok(parsed)
}

fn build_phrase_module_user_payload(
    host_headword: &str,
    host_analysis: &AnalysisDocument,
    request: &AddPhraseModuleRequest,
) -> String {
    let structured = host_analysis.structured.as_ref();
    let existing_attachments: Vec<_> = host_analysis
        .attached_phrase_modules
        .iter()
        .map(|item| {
            json!({
                "phrase": item.phrase,
                "title": item.usage_module.as_ref().map(|module| module.title.clone()).unwrap_or_default(),
            })
        })
        .collect();

    json!({
        "host_headword": host_headword,
        "phrase": request.phrase.trim(),
        "instruction": request.instruction.as_deref().unwrap_or("").trim(),
        "host_context": {
            "meanings": structured.map(|item| item.meanings.clone()).unwrap_or_default(),
            "grammar_branches": structured.map(|item| item.grammar_branches.clone()).unwrap_or_default(),
            "usage_modules": structured.map(|item| item.usage_modules.clone()).unwrap_or_default(),
            "attached_phrases": existing_attachments,
        }
    })
    .to_string()
}

fn normalize_usage_module(
    usage_module: Option<PhraseUsageModule>,
    fallback_title: &str,
) -> Result<PhraseUsageModule> {
    let module = usage_module.ok_or_else(|| anyhow!("phrase module missing usage_module"))?;
    let normalized = PhraseUsageModule {
        title: first_non_empty(&module.title, fallback_title),
        explanation: module.explanation.trim().to_string(),
        example_de: module.example_de.trim().to_string(),
        example_zh: module.example_zh.trim().to_string(),
    };

    anyhow::ensure!(
        !normalized.explanation.is_empty(),
        "phrase module missing explanation"
    );
    anyhow::ensure!(
        !normalized.example_de.is_empty(),
        "phrase module missing German example"
    );
    anyhow::ensure!(
        !normalized.example_zh.is_empty(),
        "phrase module missing Chinese translation"
    );

    Ok(normalized)
}

fn build_attachment(
    host_headword: &str,
    phrase: &str,
    usage_module: Option<PhraseUsageModule>,
) -> AttachedPhraseModule {
    let usage_module = usage_module.expect("usage module validated before attachment");
    let preview = PhraseUsagePreview {
        meaning_zh: usage_module.title.clone(),
        meaning_en: String::new(),
        usage_module: usage_module.clone(),
    };

    AttachedPhraseModule {
        phrase: phrase.trim().to_string(),
        host_headword: host_headword.to_string(),
        source_phrase_entry_id: transient_attachment_id(),
        usage_module: Some(usage_module),
        analysis_markdown: render_phrase_preview_markdown(phrase, &preview),
        confidence: PhraseLookupConfidence::High,
        attached_at: Utc::now(),
    }
}

async fn build_response(
    state: &AppState,
    entry: crate::models::KnowledgeEntry,
) -> Result<AnalyzeResponse> {
    let follow_ups = follow_up::list_by_entry_id(&state.pool, entry.id)
        .await?
        .into_iter()
        .map(|item| crate::models::FollowUpItem {
            id: item.id,
            question: item.question,
            answer: item.answer,
            created_at: item.created_at,
        })
        .collect();

    Ok(AnalyzeResponse {
        entry_id: entry.id,
        query_text: entry.query_text,
        analysis_markdown: analysis_markdown(&entry.analysis),
        structured_analysis: structured_analysis(&entry.analysis),
        phrase_lookup: phrase_lookup_from_analysis(&entry.analysis),
        phrase_usage_preview: phrase_usage_preview_from_analysis(&entry.analysis),
        attached_phrase_modules: attached_phrase_modules_from_analysis(&entry.analysis),
        source: "知识库".to_string(),
        model: entry
            .analysis
            .get("model")
            .and_then(serde_json::Value::as_str)
            .map(ToString::to_string),
        quality_mode: entry
            .analysis
            .get("quality_mode")
            .and_then(|value| serde_json::from_value(value.clone()).ok()),
        follow_ups,
    })
}

fn first_non_empty(value: &str, fallback: &str) -> String {
    let value = value.trim();
    if value.is_empty() {
        fallback.trim().to_string()
    } else {
        value.to_string()
    }
}

fn transient_attachment_id() -> i64 {
    -Utc::now().timestamp_micros()
}

fn phrase_module_chat_options() -> AiChatOptions {
    AiChatOptions {
        temperature: 0.1,
        max_tokens: Some(900),
        timeout: Duration::from_secs(60),
    }
}
