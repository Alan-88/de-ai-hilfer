use crate::models::{
    AnalysisDocument, AnalyzeRequest, AnalyzeResponse, AttachPhraseRequest, AttachedPhraseModule,
    DetachPhraseRequest, PhraseLookupConfidence, QualityMode,
};
use crate::repositories::{dictionary, knowledge};
use crate::services::analysis_preview::analysis_markdown;
use crate::services::analyze;
use crate::services::analyze_support::render_phrase_preview_markdown;
use crate::services::query_resolution::{
    attached_phrase_modules_from_analysis, phrase_lookup_from_analysis,
    phrase_usage_preview_from_analysis,
};
use crate::state::AppState;
use anyhow::{anyhow, Context, Result};
use chrono::Utc;

fn build_response(entry: crate::models::KnowledgeEntry) -> AnalyzeResponse {
    AnalyzeResponse {
        entry_id: entry.id,
        query_text: entry.query_text,
        analysis_markdown: analysis_markdown(&entry.analysis),
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
        follow_ups: Vec::new(),
    }
}

fn transient_attachment_id() -> i64 {
    -Utc::now()
        .timestamp_nanos_opt()
        .unwrap_or_else(|| Utc::now().timestamp_micros())
}

pub async fn attach_phrase_to_host(
    state: &AppState,
    request: AttachPhraseRequest,
) -> Result<AnalyzeResponse> {
    let host_headword = request.host_headword.trim();
    anyhow::ensure!(!host_headword.is_empty(), "host_headword cannot be empty");
    let persisted_phrase_id = request.phrase_entry_id.filter(|id| *id > 0);

    let (
        phrase_text,
        phrase_lookup,
        phrase_usage_preview,
        phrase_analysis_markdown,
        source_phrase_entry_id,
        phrase_entry_to_delete,
    ) = if let Some(phrase_entry_id) = persisted_phrase_id {
        let phrase_entry = knowledge::find_by_id(&state.pool, phrase_entry_id)
            .await?
            .ok_or_else(|| anyhow!("phrase entry not found"))?;

        let phrase_lookup = phrase_lookup_from_analysis(&phrase_entry.analysis);
        let is_phrase_entry = phrase_entry.entry_type == "PHRASE" || phrase_lookup.is_some();
        anyhow::ensure!(is_phrase_entry, "selected entry is not a phrase analysis");

        (
            phrase_entry.query_text.clone(),
            phrase_lookup,
            phrase_usage_preview_from_analysis(&phrase_entry.analysis),
            analysis_markdown(&phrase_entry.analysis),
            phrase_entry.id,
            Some(phrase_entry.id),
        )
    } else {
        let phrase_text = request
            .phrase
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| anyhow!("phrase preview payload missing phrase"))?
            .to_string();
        let phrase_usage_preview = request
            .phrase_usage_preview
            .clone()
            .ok_or_else(|| anyhow!("phrase preview payload missing usage preview"))?;
        let phrase_analysis_markdown = request
            .analysis_markdown
            .clone()
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| render_phrase_preview_markdown(&phrase_text, &phrase_usage_preview));

        (
            phrase_text,
            request.phrase_lookup.clone(),
            Some(phrase_usage_preview),
            phrase_analysis_markdown,
            transient_attachment_id(),
            None,
        )
    };

    let Some(host_dictionary_entry) =
        dictionary::find_by_headword(&state.pool, host_headword).await?
    else {
        return Err(anyhow!("host headword not found in dictionary"));
    };

    let host_entry =
        match knowledge::find_by_query(&state.pool, &host_dictionary_entry.headword).await? {
            Some(entry) => entry,
            None => {
                let generated = analyze::analyze(
                    state,
                    AnalyzeRequest {
                        query_text: host_dictionary_entry.headword.clone(),
                        entry_type: Some("WORD".to_string()),
                        generation_hint: None,
                        quality_mode: QualityMode::Default,
                        force_refresh: false,
                        entry_id: None,
                    },
                )
                .await
                .with_context(|| {
                    format!(
                        "failed to prepare host entry for {}",
                        host_dictionary_entry.headword
                    )
                })?;

                anyhow::ensure!(generated.entry_id > 0, "host entry could not be persisted");
                knowledge::find_by_id(&state.pool, generated.entry_id)
                    .await?
                    .ok_or_else(|| anyhow!("host entry missing after generation"))?
            }
        };

    let mut host_analysis: AnalysisDocument = serde_json::from_value(host_entry.analysis.clone())
        .context("failed to parse host analysis document")?;

    let attachment = AttachedPhraseModule {
        phrase: phrase_text,
        host_headword: host_dictionary_entry.headword.clone(),
        source_phrase_entry_id,
        usage_module: phrase_usage_preview.map(|preview| preview.usage_module),
        analysis_markdown: phrase_analysis_markdown,
        confidence: phrase_lookup
            .as_ref()
            .map(|lookup| lookup.confidence)
            .unwrap_or(PhraseLookupConfidence::Low),
        attached_at: Utc::now(),
    };

    let mut attachments = attached_phrase_modules_from_analysis(&host_entry.analysis);
    let host_markdown = analysis_markdown(&host_entry.analysis).to_lowercase();
    let phrase_already_covered = host_markdown.contains(&attachment.phrase.to_lowercase());
    attachments.retain(|item| {
        !(item.source_phrase_entry_id == attachment.source_phrase_entry_id
            || (item.host_headword == attachment.host_headword && item.phrase == attachment.phrase))
    });
    if !phrase_already_covered {
        attachments.push(attachment);
    }
    attachments.sort_by(|left, right| {
        right
            .attached_at
            .cmp(&left.attached_at)
            .then(left.phrase.cmp(&right.phrase))
    });

    host_analysis.attached_phrase_modules = attachments.clone();
    let updated_analysis = serde_json::to_value(&host_analysis)?;
    let updated_entry = knowledge::update_analysis(
        &state.pool,
        host_entry.id,
        &updated_analysis,
        &host_entry.tags,
        &host_entry.aliases,
    )
    .await?;

    if let Some(phrase_entry_id) = phrase_entry_to_delete {
        let deleted = knowledge::delete_by_id(&state.pool, phrase_entry_id).await?;
        tracing::info!(
            "phrase preview removed after attach: phrase_entry_id={}, deleted={deleted}",
            phrase_entry_id
        );
    }

    Ok(build_response(updated_entry))
}

pub async fn detach_phrase_from_host(
    state: &AppState,
    request: DetachPhraseRequest,
) -> Result<AnalyzeResponse> {
    let host_entry = knowledge::find_by_id(&state.pool, request.host_entry_id)
        .await?
        .ok_or_else(|| anyhow!("host entry not found"))?;

    let mut host_analysis: AnalysisDocument = serde_json::from_value(host_entry.analysis.clone())
        .context("failed to parse host analysis document")?;

    let original_len = host_analysis.attached_phrase_modules.len();
    host_analysis
        .attached_phrase_modules
        .retain(|item| item.source_phrase_entry_id != request.source_phrase_entry_id);

    anyhow::ensure!(
        host_analysis.attached_phrase_modules.len() != original_len,
        "attached phrase not found on host entry"
    );

    let updated_analysis = serde_json::to_value(&host_analysis)?;
    let updated_entry = knowledge::update_analysis(
        &state.pool,
        host_entry.id,
        &updated_analysis,
        &host_entry.tags,
        &host_entry.aliases,
    )
    .await?;

    Ok(build_response(updated_entry))
}
