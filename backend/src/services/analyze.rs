use crate::models::{
    AnalysisDocument, AnalyzeRequest, AnalyzeResponse, FollowUpItem, NewKnowledgeEntry, QualityMode,
};
use crate::repositories::{dictionary, dictionary_lexemes, knowledge};
use crate::services::analysis_grounded_runtime::generate_grounded_analysis;
use crate::services::analysis_preview::analysis_markdown;
use crate::services::analyze_support::AnalysisMode;
use crate::services::dictionary_facts::dictionary_pos;
use crate::services::dictionary_render::{
    build_compact_analysis_from_dictionary, build_dictionary_excerpt,
};
use crate::services::dictionary_tags::build_tags;
use crate::services::query_inference::is_form_reference_entry;
use crate::services::query_resolution::{
    attached_phrase_modules_from_analysis, build_no_candidate_analysis,
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

    let exact_dictionary_hit = dictionary::find_by_headword(&state.pool, query).await?;
    tracing::info!(
        "analyze dictionary exact hit: query={query}, hit={}",
        exact_dictionary_hit.is_some()
    );
    let (prototype, dictionary_entry) = if let Some(entry) = exact_dictionary_hit {
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
    } else if let Some(form_entry) = dictionary::find_by_form(&state.pool, query).await? {
        tracing::info!(
            "analyze resolved surface form without AI: query={query}, prototype={}",
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

    if dictionary_entry.is_none() {
        tracing::info!(
            "analyze no reliable dictionary candidate: query={query}, mode={}",
            analysis_mode_name(mode)
        );
        let analysis = build_no_candidate_analysis(query);
        return Ok(AnalyzeResponse {
            entry_id: 0,
            query_text: query.to_string(),
            analysis_markdown: analysis.markdown,
            structured_analysis: analysis.structured,
            attached_phrase_modules: analysis.attached_phrase_modules,
            source: "未找到可靠候选".to_string(),
            model: None,
            quality_mode: Some(quality_mode),
            follow_ups: Vec::new(),
        });
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
                    structured: None,
                    tags: vec!["待确认".to_string()],
                    aliases: Vec::new(),
                    prototype: Some(prototype.clone()),
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
        tracing::info!("analyze full ai generation: query={query}, prototype={prototype}");
        match generate_grounded_analysis(
            state,
            &prototype,
            dictionary_entry.as_ref(),
            quality_mode,
            generation_hint,
            request.model_override.as_ref(),
        )
        .await
        {
            Ok(generated) => {
                let parsed = generated.analysis;
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
            Err(err) => {
                tracing::warn!(
                    "grounded analyze generation failed: query={query}, prototype={prototype}, err={err:#}"
                );
                return Err(err.context("grounded analysis generation failed"));
            }
        }
    };

    analysis.prototype = dictionary_entry
        .as_ref()
        .map(|entry| entry.headword.clone());
    analysis.model = generated_model.clone();
    analysis.quality_mode = Some(quality_mode);
    analysis.dictionary_excerpt = dictionary_entry
        .as_ref()
        .map(|entry| build_dictionary_excerpt(&entry.raw_data));

    if prototype != query && !analysis.aliases.iter().any(|alias| alias == query) {
        analysis.aliases.push(query.to_string());
    }
    if analysis.tags.is_empty() {
        if let Some(entry) = dictionary_entry.as_ref() {
            analysis.tags = build_tags(&entry.raw_data, dictionary_pos(&entry.raw_data));
        }
    }

    if dictionary_entry.is_none() && !used_ai_generation {
        tracing::info!(
            "analyze skip persistence for low-confidence fallback: query={query}, prototype={prototype}, mode={}",
            analysis_mode_name(mode)
        );
        return Ok(AnalyzeResponse {
            entry_id: 0,
            query_text: prototype,
            analysis_markdown: analysis.markdown,
            structured_analysis: analysis.structured,
            attached_phrase_modules: analysis.attached_phrase_modules,
            source: response_source,
            model: analysis.model,
            quality_mode: Some(quality_mode),
            follow_ups: Vec::new(),
        });
    }

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

    tracing::info!(
        "analyze persist entry: query={query}, prototype={prototype}, used_ai_generation={used_ai_generation}"
    );
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
    tracing::info!("analyze insert complete: entry_id={}", entry.id);
    if prototype != query {
        knowledge::add_alias(&state.pool, entry.id, query).await?;
    }
    update_recent_searches(&state.recent_searches, &entry.query_text).await;

    Ok(AnalyzeResponse {
        entry_id: entry.id,
        query_text: entry.query_text,
        analysis_markdown: analysis.markdown,
        structured_analysis: analysis.structured,
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
        structured_analysis: crate::services::analysis_preview::structured_analysis(analysis),
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
