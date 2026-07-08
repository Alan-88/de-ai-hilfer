use crate::models::{
    AnalyzeRequest, AnalyzeResponse, FollowUpItem, NewKnowledgeEntry, QualityMode,
};
use crate::repositories::{dictionary, dictionary_lexemes, knowledge};
use crate::services::ai_model_resolver::{resolve_task_model_with_override, AiModelTask};
use crate::services::analysis_grounded_runtime::stream_grounded_analysis;
use crate::services::dictionary_facts::dictionary_pos;
use crate::services::dictionary_tags::build_tags;
use crate::services::query_inference::is_form_reference_entry;
use crate::services::query_resolution::{
    attached_phrase_modules_from_analysis, build_no_candidate_analysis,
};
use crate::services::stream_analyze_runtime::{cached_analysis_markdown, send_meta};
use crate::services::stream_response::sse_complete;
use crate::state::AppState;
use anyhow::Result;
use serde_json::Value;
use tokio::sync::mpsc::UnboundedSender;

pub async fn stream_analyze(
    state: AppState,
    request: AnalyzeRequest,
    tx: UnboundedSender<String>,
) -> Result<()> {
    let query = request.query_text.trim();
    let generation_hint = request
        .generation_hint
        .as_deref()
        .map(str::trim)
        .filter(|hint| !hint.is_empty());
    anyhow::ensure!(!query.is_empty(), "query_text cannot be empty");
    let quality_mode = request.quality_mode;

    if !request.force_refresh {
        if let Some(entry) = knowledge::find_by_query_text_exact(&state.pool, query).await? {
            send_meta(
                &tx,
                "analyze",
                entry
                    .analysis
                    .get("model")
                    .and_then(Value::as_str)
                    .unwrap_or(&state.config.ai_models.analyze),
                quality_mode,
                "知识库",
                false,
            );
            tx.send(sse_complete(&AnalyzeResponse {
                entry_id: entry.id,
                query_text: entry.query_text.clone(),
                analysis_markdown: cached_analysis_markdown(&entry.analysis),
                structured_analysis: crate::services::analysis_preview::structured_analysis(
                    &entry.analysis,
                ),
                attached_phrase_modules: attached_phrase_modules_from_analysis(&entry.analysis),
                source: "知识库".to_string(),
                model: entry
                    .analysis
                    .get("model")
                    .and_then(Value::as_str)
                    .map(ToString::to_string),
                quality_mode: entry
                    .analysis
                    .get("quality_mode")
                    .and_then(|value| serde_json::from_value(value.clone()).ok())
                    .or(Some(quality_mode)),
                follow_ups: Vec::new(),
            }))
            .ok();
            return Ok(());
        }
    }

    let exact_dictionary_hit = dictionary::find_by_headword(&state.pool, query).await?;
    let (prototype, dictionary_entry) = if let Some(entry) = exact_dictionary_hit {
        if is_form_reference_entry(&entry.raw_data) {
            if let Some(form_entry) = dictionary::find_by_form(&state.pool, query).await? {
                if form_entry.headword != entry.headword {
                    tracing::info!(
                        "stream analyze resolved exact form entry to prototype: query={query}, form_headword={}, prototype={}",
                        entry.headword,
                        form_entry.headword
                    );
                    (form_entry.headword.clone(), Some(form_entry))
                } else {
                    (entry.headword.clone(), Some(entry))
                }
            } else {
                (entry.headword.clone(), Some(entry))
            }
        } else {
            (entry.headword.clone(), Some(entry))
        }
    } else if let Some(form_entry) = dictionary::find_by_form(&state.pool, query).await? {
        tracing::info!(
            "stream analyze resolved surface form without AI: query={query}, prototype={}",
            form_entry.headword
        );
        (form_entry.headword.clone(), Some(form_entry))
    } else {
        (query.to_string(), None)
    };

    let existing_entry = match request.entry_id {
        Some(entry_id) => knowledge::find_by_id(&state.pool, entry_id).await?,
        None if request.force_refresh => {
            knowledge::find_by_query_text_exact(&state.pool, &prototype).await?
        }
        None => None,
    };

    if !request.force_refresh {
        if let Some(entry) = knowledge::find_by_query_text_exact(&state.pool, &prototype).await? {
            if prototype != query {
                knowledge::add_alias(&state.pool, entry.id, query).await?;
            }

            send_meta(
                &tx,
                "analyze",
                entry
                    .analysis
                    .get("model")
                    .and_then(Value::as_str)
                    .unwrap_or(&state.config.ai_models.analyze),
                quality_mode,
                "知识库",
                false,
            );
            tx.send(sse_complete(&AnalyzeResponse {
                entry_id: entry.id,
                query_text: entry.query_text.clone(),
                analysis_markdown: cached_analysis_markdown(&entry.analysis),
                structured_analysis: crate::services::analysis_preview::structured_analysis(
                    &entry.analysis,
                ),
                attached_phrase_modules: attached_phrase_modules_from_analysis(&entry.analysis),
                source: "知识库".to_string(),
                model: entry
                    .analysis
                    .get("model")
                    .and_then(Value::as_str)
                    .map(ToString::to_string),
                quality_mode: entry
                    .analysis
                    .get("quality_mode")
                    .and_then(|value| serde_json::from_value(value.clone()).ok())
                    .or(Some(quality_mode)),
                follow_ups: Vec::new(),
            }))
            .ok();
            return Ok(());
        }
    }

    if dictionary_entry.is_none() {
        let analysis = build_no_candidate_analysis(query);
        tx.send(sse_complete(&AnalyzeResponse {
            entry_id: 0,
            query_text: query.to_string(),
            analysis_markdown: analysis.markdown,
            structured_analysis: analysis.structured,
            attached_phrase_modules: analysis.attached_phrase_modules,
            source: "未找到可靠候选".to_string(),
            model: None,
            quality_mode: Some(quality_mode),
            follow_ups: Vec::new(),
        }))
        .ok();
        return Ok(());
    }

    let resolved_meta = resolve_task_model_with_override(
        &state,
        AiModelTask::Analyze,
        request.model_override.as_ref(),
    )
    .await?;
    send_meta(
        &tx,
        "analyze",
        &resolved_meta.model,
        quality_mode,
        if quality_mode == QualityMode::Pro {
            "Pro"
        } else {
            "Flash"
        },
        false,
    );
    let (mut analysis, used_ai_generation, source, model) = match stream_grounded_analysis(
        &state,
        &tx,
        &prototype,
        dictionary_entry.as_ref(),
        quality_mode,
        generation_hint,
        request.model_override.as_ref(),
    )
    .await
    {
        Ok(generated) => (
            generated.analysis,
            true,
            if quality_mode == QualityMode::Pro {
                "Pro".to_string()
            } else {
                "Flash".to_string()
            },
            Some(generated.model),
        ),
        Err(err) => {
            if is_stream_canceled(&err) {
                tracing::warn!(
                    "stream analyze canceled before persistence: query={query}, prototype={prototype}"
                );
                return Ok(());
            }
            return Err(err.context("grounded stream analysis generation failed"));
        }
    };

    if tx.is_closed() {
        tracing::info!(
            "stream analyze canceled after generation before persistence: query={query}, prototype={prototype}"
        );
        return Ok(());
    }
    if analysis.tags.is_empty() {
        if let Some(entry) = dictionary_entry.as_ref() {
            analysis.tags = build_tags(&entry.raw_data, dictionary_pos(&entry.raw_data));
        }
    }

    let final_response = if dictionary_entry.is_none() && !used_ai_generation {
        AnalyzeResponse {
            entry_id: 0,
            query_text: prototype.clone(),
            analysis_markdown: analysis.markdown.clone(),
            structured_analysis: analysis.structured.clone(),
            attached_phrase_modules: analysis.attached_phrase_modules.clone(),
            source,
            model,
            quality_mode: Some(quality_mode),
            follow_ups: Vec::new(),
        }
    } else {
        let lexeme_id =
            dictionary_lexemes::find_unique_lexeme_id_by_surface(&state.pool, &prototype).await?;

        let new_entry = NewKnowledgeEntry {
            query_text: prototype.clone(),
            lexeme_id,
            prototype: dictionary_entry
                .as_ref()
                .map(|entry| entry.headword.clone()),
            entry_type: request.entry_type.unwrap_or_else(|| "WORD".to_string()),
            analysis: serde_json::to_value(&analysis)?,
            tags: (!analysis.tags.is_empty()).then_some(analysis.tags.clone()),
            aliases: (!analysis.aliases.is_empty()).then_some(analysis.aliases.clone()),
        };

        let entry = match existing_entry {
            Some(existing) => {
                knowledge::update_analysis(
                    &state.pool,
                    existing.id,
                    new_entry.lexeme_id,
                    &new_entry.analysis,
                    &new_entry.tags,
                    &new_entry.aliases,
                )
                .await?
            }
            None => knowledge::insert(&state.pool, &new_entry).await?,
        };

        if prototype != query {
            knowledge::add_alias(&state.pool, entry.id, query).await?;
        }

        AnalyzeResponse {
            entry_id: entry.id,
            query_text: entry.query_text,
            analysis_markdown: analysis.markdown.clone(),
            structured_analysis: analysis.structured.clone(),
            attached_phrase_modules: analysis.attached_phrase_modules.clone(),
            source,
            model,
            quality_mode: Some(quality_mode),
            follow_ups: Vec::<FollowUpItem>::new(),
        }
    };

    tx.send(sse_complete(&final_response)).ok();
    Ok(())
}

fn is_stream_canceled(err: &anyhow::Error) -> bool {
    err.to_string()
        .contains("stream analyze canceled by client")
}
