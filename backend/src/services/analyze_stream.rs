use crate::models::{
    AnalyzeRequest, AnalyzeResponse, FollowUpItem, NewKnowledgeEntry, QualityMode,
};
use crate::repositories::{dictionary, knowledge};
use crate::services::analyze_runtime::{fallback_model_for, primary_model_for};
use crate::services::analyze_support::build_stream_analysis_prompt;
use crate::services::dictionary_render::{
    build_full_analysis_from_dictionary, build_unavailable_analysis,
};
use crate::services::query_inference::{
    infer_phrase_lookup, is_form_reference_entry, is_phrase_like_query,
};
use crate::services::query_resolution::{
    attached_phrase_modules_from_analysis, build_phrase_unavailable_analysis, identify_prototype,
    maybe_correct_spelling, phrase_lookup_from_analysis, phrase_usage_preview_from_analysis,
};
use crate::services::stream_analyze_runtime::{
    build_stream_analysis_document, cached_analysis_markdown, generate_phrase_preview_with_model,
    send_meta, stream_model_markdown,
};
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
            if entry.entry_type == "PHRASE" {
                tracing::info!(
                    "stream analyze skip phrase cache preview: query={query}, entry_id={}",
                    entry.id
                );
            } else {
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
                    phrase_lookup: phrase_lookup_from_analysis(&entry.analysis),
                    phrase_usage_preview: phrase_usage_preview_from_analysis(&entry.analysis),
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
    }

    let exact_dictionary_hit = dictionary::find_by_headword(&state.pool, query).await?;
    let phrase_lookup = if exact_dictionary_hit.is_none() && is_phrase_like_query(query) {
        infer_phrase_lookup(&state, query).await?
    } else {
        None
    };
    let is_phrase_query = phrase_lookup.is_some();

    let (prototype, dictionary_entry) = if let Some(lookup) = &phrase_lookup {
        let host_entry = match lookup.best_host_headword.as_deref() {
            Some(host) => dictionary::find_by_headword(&state.pool, host).await?,
            None => None,
        };
        (
            lookup
                .best_host_headword
                .clone()
                .unwrap_or_else(|| query.to_string()),
            host_entry,
        )
    } else if let Some(entry) = exact_dictionary_hit {
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
    } else {
        let corrected_query = maybe_correct_spelling(&state, query)
            .await
            .unwrap_or_else(|_| query.to_string());
        let prototype = identify_prototype(&state, &corrected_query)
            .await
            .unwrap_or_else(|_| corrected_query.clone());
        let dictionary_entry = dictionary::find_by_headword(&state.pool, &prototype).await?;
        (prototype, dictionary_entry)
    };

    let existing_entry = match request.entry_id {
        Some(entry_id) => knowledge::find_by_id(&state.pool, entry_id).await?,
        None if request.force_refresh && is_phrase_query => None,
        None if request.force_refresh => {
            knowledge::find_by_query_text_exact(&state.pool, &prototype).await?
        }
        None => None,
    };

    if !request.force_refresh && !is_phrase_query {
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
                phrase_lookup: phrase_lookup_from_analysis(&entry.analysis),
                phrase_usage_preview: phrase_usage_preview_from_analysis(&entry.analysis),
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

    let (analysis, used_ai_generation, source, model) = if is_phrase_query {
        match generate_phrase_preview_with_model(
            &state,
            query,
            dictionary_entry.as_ref(),
            quality_mode,
            generation_hint,
            phrase_lookup.as_ref(),
            &tx,
        )
        .await
        {
            Ok(analysis) => {
                let model = analysis.model.clone();
                (
                    analysis,
                    true,
                    if quality_mode == QualityMode::Pro {
                        "Pro".to_string()
                    } else {
                        "Flash".to_string()
                    },
                    model,
                )
            }
            Err(err) => {
                tracing::warn!(
                    "stream analyze phrase fallback to deterministic rendering: query={query}, prototype={prototype}, err={err:#}"
                );
                (
                    build_phrase_unavailable_analysis(query, phrase_lookup.as_ref()),
                    false,
                    "短语待确认".to_string(),
                    None,
                )
            }
        }
    } else {
        let primary_model = primary_model_for(&state, quality_mode);
        let fallback_model = fallback_model_for(&state, quality_mode);
        let system_prompt = build_stream_analysis_prompt(
            &state.prompts,
            dictionary_entry.as_ref(),
            generation_hint,
            phrase_lookup.as_ref(),
        );

        let generated = match stream_model_markdown(
            &state.ai_client,
            &tx,
            "analyze",
            primary_model,
            fallback_model,
            quality_mode,
            &system_prompt,
            &prototype,
            if quality_mode == QualityMode::Pro {
                "Pro"
            } else {
                "Flash"
            },
        )
        .await
        {
            Ok(result) => Some(result),
            Err(err) => {
                tracing::warn!(
                    "stream analyze fallback to deterministic rendering: query={query}, prototype={prototype}, err={err:#}"
                );
                None
            }
        };

        if let Some(markdown) = generated {
            (
                build_stream_analysis_document(
                    markdown.markdown,
                    dictionary_entry.as_ref(),
                    quality_mode,
                    &markdown.model,
                    phrase_lookup.as_ref(),
                ),
                true,
                if quality_mode == QualityMode::Pro {
                    "Pro".to_string()
                } else {
                    "Flash".to_string()
                },
                Some(markdown.model),
            )
        } else {
            let fallback = dictionary_entry
                .as_ref()
                .map(|entry| build_full_analysis_from_dictionary(&prototype, &entry.raw_data))
                .unwrap_or_else(|| build_unavailable_analysis(&prototype));
            let source = if dictionary_entry.is_some() {
                "字典兜底"
            } else {
                "本地兜底"
            };
            (fallback, false, source.to_string(), None)
        }
    };

    let final_response = if dictionary_entry.is_none() && !used_ai_generation {
        AnalyzeResponse {
            entry_id: 0,
            query_text: if is_phrase_query {
                query.to_string()
            } else {
                prototype.clone()
            },
            analysis_markdown: analysis.markdown.clone(),
            phrase_lookup: analysis.phrase_lookup.clone(),
            phrase_usage_preview: analysis.phrase_usage_preview.clone(),
            attached_phrase_modules: analysis.attached_phrase_modules.clone(),
            source,
            model,
            quality_mode: Some(quality_mode),
            follow_ups: Vec::new(),
        }
    } else {
        if is_phrase_query {
            AnalyzeResponse {
                entry_id: 0,
                query_text: query.to_string(),
                analysis_markdown: analysis.markdown.clone(),
                phrase_lookup: analysis.phrase_lookup.clone(),
                phrase_usage_preview: analysis.phrase_usage_preview.clone(),
                attached_phrase_modules: analysis.attached_phrase_modules.clone(),
                source,
                model,
                quality_mode: Some(quality_mode),
                follow_ups: Vec::new(),
            }
        } else {
            let new_entry = NewKnowledgeEntry {
                query_text: prototype.clone(),
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
                        &new_entry.analysis,
                        &new_entry.tags,
                        &new_entry.aliases,
                    )
                    .await?
                }
                None => knowledge::insert(&state.pool, &new_entry).await?,
            };

            if !is_phrase_query && prototype != query {
                knowledge::add_alias(&state.pool, entry.id, query).await?;
            }

            AnalyzeResponse {
                entry_id: entry.id,
                query_text: entry.query_text,
                analysis_markdown: analysis.markdown.clone(),
                phrase_lookup: analysis.phrase_lookup.clone(),
                phrase_usage_preview: analysis.phrase_usage_preview.clone(),
                attached_phrase_modules: analysis.attached_phrase_modules.clone(),
                source,
                model,
                quality_mode: Some(quality_mode),
                follow_ups: Vec::<FollowUpItem>::new(),
            }
        }
    };

    tx.send(sse_complete(&final_response)).ok();
    Ok(())
}
