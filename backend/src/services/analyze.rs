use crate::models::{
    AnalysisDocument, AnalyzeRequest, AnalyzeResponse, FollowUpItem, NewKnowledgeEntry, QualityMode,
};
use crate::repositories::{dictionary, knowledge};
use crate::services::analysis_preview::analysis_markdown;
use crate::services::analyze_runtime::generate_analysis_with_model;
use crate::services::analyze_support::{
    build_phrase_preview_analysis, parse_llm_analysis, parse_phrase_usage_preview, AnalysisMode,
};
use crate::services::dictionary_render::{
    build_compact_analysis_from_dictionary, build_dictionary_excerpt,
    build_full_analysis_from_dictionary, build_unavailable_analysis,
};
use crate::services::query_inference::{
    infer_phrase_lookup, is_form_reference_entry, is_phrase_like_query,
};
use crate::services::query_resolution::{
    attached_phrase_modules_from_analysis, build_phrase_unavailable_analysis, identify_prototype,
    maybe_correct_spelling, phrase_lookup_from_analysis, phrase_usage_preview_from_analysis,
};
use crate::state::AppState;
use anyhow::Result;
use serde_json::Value;
use std::collections::VecDeque;

pub async fn analyze(state: &AppState, request: AnalyzeRequest) -> Result<AnalyzeResponse> {
    analyze_with_mode(state, request, AnalysisMode::Full).await
}

pub async fn analyze_compact(state: &AppState, request: AnalyzeRequest) -> Result<AnalyzeResponse> {
    analyze_with_mode(state, request, AnalysisMode::Compact).await
}

async fn analyze_with_mode(
    state: &AppState,
    request: AnalyzeRequest,
    mode: AnalysisMode,
) -> Result<AnalyzeResponse> {
    let query = request.query_text.trim();
    let generation_hint = request
        .generation_hint
        .as_deref()
        .map(str::trim)
        .filter(|hint| !hint.is_empty());
    let quality_mode = request.quality_mode;
    anyhow::ensure!(!query.is_empty(), "query_text cannot be empty");
    tracing::info!(
        "analyze start: query={query}, mode={}",
        analysis_mode_name(mode)
    );

    if !request.force_refresh {
        if let Some(entry) = knowledge::find_by_query_text_exact(&state.pool, query).await? {
            if entry.entry_type == "PHRASE" {
                tracing::info!(
                    "analyze skip phrase cache preview: query={query}, entry_id={}",
                    entry.id
                );
            } else {
                tracing::info!(
                    "analyze cache hit by query: query={query}, entry_id={}",
                    entry.id
                );
                update_recent_searches(&state.recent_searches, &entry.query_text).await;
                return Ok(cached_response(
                    entry.id,
                    &entry.query_text,
                    &entry.analysis,
                    "知识库",
                ));
            }
        }
    }

    let exact_dictionary_hit = dictionary::find_by_headword(&state.pool, query).await?;
    tracing::info!(
        "analyze dictionary exact hit: query={query}, hit={}",
        exact_dictionary_hit.is_some()
    );
    let phrase_lookup = if exact_dictionary_hit.is_none() && is_phrase_like_query(query) {
        infer_phrase_lookup(state, query).await?
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
                        "analyze resolved exact form entry to prototype: query={query}, form_headword={}, prototype={}",
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
        let corrected_query = maybe_correct_spelling(state, query)
            .await
            .unwrap_or_else(|_| query.to_string());
        let prototype = identify_prototype(state, &corrected_query)
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

            tracing::info!(
                "analyze cache hit by prototype: query={query}, prototype={prototype}, entry_id={}",
                entry.id
            );
            update_recent_searches(&state.recent_searches, &entry.query_text).await;
            return Ok(cached_response(
                entry.id,
                &entry.query_text,
                &entry.analysis,
                "知识库",
            ));
        }
    }

    let (mut analysis, used_ai_generation, response_source, generated_model) = if matches!(
        mode,
        AnalysisMode::Compact
    ) {
        tracing::info!(
            "analyze compact branch: query={query}, prototype={prototype}, dictionary_hit={}",
            dictionary_entry.is_some()
        );
        (
            dictionary_entry
            .as_ref()
            .map(|entry| build_compact_analysis_from_dictionary(&prototype, &entry.raw_data))
            .unwrap_or_else(|| AnalysisDocument {
                markdown: format!(
                    "## {}\n\n- 核心判断：暂未命中字典或知识库。\n- 建议：请改用首页直接查询，或补充更准确的拼写线索。",
                    prototype
                ),
                tags: vec!["待确认".to_string()],
                aliases: Vec::new(),
                prototype: Some(prototype.clone()),
                phrase_lookup: None,
                phrase_usage_preview: None,
                attached_phrase_modules: Vec::new(),
                dictionary_excerpt: None,
                model: None,
                quality_mode: Some(QualityMode::Default),
            }),
            false,
            if dictionary_entry.is_some() {
                "generated".to_string()
            } else {
                "待确认".to_string()
            },
            None,
        )
    } else {
        tracing::info!(
            "analyze full ai generation: query={query}, prototype={prototype}, phrase_query={is_phrase_query}"
        );
        match generate_analysis_with_model(
            state,
            if is_phrase_query { query } else { &prototype },
            dictionary_entry.as_ref(),
            mode,
            quality_mode,
            generation_hint,
            phrase_lookup.as_ref(),
        )
        .await
        {
            Ok(generated) => {
                let target = if is_phrase_query { query } else { &prototype };
                if is_phrase_query {
                    let (preview, tags, aliases) =
                        parse_phrase_usage_preview(&generated.content, target)?;
                    (
                        build_phrase_preview_analysis(
                            target,
                            preview,
                            phrase_lookup.as_ref(),
                            dictionary_entry.as_ref(),
                            tags,
                            aliases,
                            quality_mode,
                            &generated.model,
                        ),
                        true,
                        match quality_mode {
                            QualityMode::Default => "Flash".to_string(),
                            QualityMode::Pro => "Pro".to_string(),
                        },
                        Some(generated.model),
                    )
                } else {
                    let mut parsed = parse_llm_analysis(&generated.content, target);
                    parsed.phrase_lookup = phrase_lookup.clone();
                    (
                        parsed,
                        true,
                        match quality_mode {
                            QualityMode::Default => "Flash".to_string(),
                            QualityMode::Pro => "Pro".to_string(),
                        },
                        Some(generated.model),
                    )
                }
            }
            Err(err) => {
                tracing::warn!(
                    "analyze fallback to deterministic dictionary rendering: query={query}, prototype={prototype}, err={err:#}"
                );
                if is_phrase_query {
                    (
                        build_phrase_unavailable_analysis(query, phrase_lookup.as_ref()),
                        false,
                        "短语待确认".to_string(),
                        None,
                    )
                } else {
                    let fallback = dictionary_entry
                        .as_ref()
                        .map(|entry| {
                            build_full_analysis_from_dictionary(&prototype, &entry.raw_data)
                        })
                        .unwrap_or_else(|| build_unavailable_analysis(&prototype));
                    let source = if dictionary_entry.is_some() {
                        "字典兜底"
                    } else {
                        "本地兜底"
                    };
                    (fallback, false, source.to_string(), None)
                }
            }
        }
    };

    analysis.prototype = dictionary_entry
        .as_ref()
        .map(|entry| entry.headword.clone());
    analysis.model = generated_model.clone();
    analysis.quality_mode = Some(quality_mode);
    analysis.phrase_lookup = analysis.phrase_lookup.or_else(|| phrase_lookup.clone());
    analysis.dictionary_excerpt = dictionary_entry
        .as_ref()
        .map(|entry| build_dictionary_excerpt(&entry.raw_data));

    if !is_phrase_query
        && prototype != query
        && !analysis.aliases.iter().any(|alias| alias == query)
    {
        analysis.aliases.push(query.to_string());
    }

    if dictionary_entry.is_none() && !used_ai_generation {
        tracing::info!(
            "analyze skip persistence for low-confidence fallback: query={query}, prototype={prototype}, mode={}",
            analysis_mode_name(mode)
        );
        return Ok(AnalyzeResponse {
            entry_id: 0,
            query_text: if is_phrase_query {
                query.to_string()
            } else {
                prototype
            },
            analysis_markdown: analysis.markdown,
            phrase_lookup: analysis.phrase_lookup,
            phrase_usage_preview: analysis.phrase_usage_preview,
            attached_phrase_modules: analysis.attached_phrase_modules,
            source: response_source,
            model: analysis.model,
            quality_mode: Some(quality_mode),
            follow_ups: Vec::new(),
        });
    }

    if is_phrase_query {
        tracing::info!(
            "analyze keep phrase preview transient: query={query}, prototype={prototype}"
        );
        return Ok(AnalyzeResponse {
            entry_id: 0,
            query_text: query.to_string(),
            analysis_markdown: analysis.markdown,
            phrase_lookup: analysis.phrase_lookup,
            phrase_usage_preview: analysis.phrase_usage_preview,
            attached_phrase_modules: analysis.attached_phrase_modules,
            source: response_source,
            model: analysis.model,
            quality_mode: Some(quality_mode),
            follow_ups: Vec::new(),
        });
    }

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

    tracing::info!(
        "analyze persist entry: query={query}, prototype={prototype}, used_ai_generation={used_ai_generation}"
    );
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
    tracing::info!("analyze insert complete: entry_id={}", entry.id);
    if !is_phrase_query && prototype != query {
        knowledge::add_alias(&state.pool, entry.id, query).await?;
    }
    update_recent_searches(&state.recent_searches, &entry.query_text).await;

    Ok(AnalyzeResponse {
        entry_id: entry.id,
        query_text: entry.query_text,
        analysis_markdown: analysis.markdown,
        phrase_lookup: analysis.phrase_lookup,
        phrase_usage_preview: analysis.phrase_usage_preview,
        attached_phrase_modules: analysis.attached_phrase_modules,
        source: response_source,
        model: analysis.model,
        quality_mode: Some(quality_mode),
        follow_ups: Vec::<FollowUpItem>::new(),
    })
}

fn analysis_mode_name(mode: AnalysisMode) -> &'static str {
    match mode {
        AnalysisMode::Full => "full",
        AnalysisMode::Compact => "compact",
    }
}

fn cached_response(
    entry_id: i64,
    query_text: &str,
    analysis: &Value,
    source: &str,
) -> AnalyzeResponse {
    AnalyzeResponse {
        entry_id,
        query_text: query_text.to_string(),
        analysis_markdown: analysis_markdown(analysis),
        phrase_lookup: phrase_lookup_from_analysis(analysis),
        phrase_usage_preview: phrase_usage_preview_from_analysis(analysis),
        attached_phrase_modules: attached_phrase_modules_from_analysis(analysis),
        source: source.to_string(),
        model: analysis
            .get("model")
            .and_then(Value::as_str)
            .map(ToString::to_string),
        quality_mode: analysis
            .get("quality_mode")
            .and_then(|value| serde_json::from_value(value.clone()).ok()),
        follow_ups: Vec::new(),
    }
}

async fn update_recent_searches(
    recent_searches: &tokio::sync::Mutex<VecDeque<String>>,
    query: &str,
) {
    let mut recent = recent_searches.lock().await;
    if let Some(index) = recent.iter().position(|item| item == query) {
        recent.remove(index);
    }
    recent.push_front(query.to_string());
    while recent.len() > 20 {
        recent.pop_back();
    }
}
