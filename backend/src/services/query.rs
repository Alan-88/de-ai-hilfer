use crate::models::{
    AnalyzeResponse, DBSuggestion, IntelligentSearchRequest, LibraryEntriesPageResponse,
    LibraryQueryTab, RecentItem, StatusResponse, SuggestionResponse,
};
use crate::repositories::{dictionary, dictionary_lexemes, knowledge};
use crate::services::analysis_preview::{analysis_markdown, preview_from_analysis};
use crate::services::embedding_lookup::infer_headword_by_embedding;
use crate::services::query_inference::{
    build_intelligent_search_pending_response, infer_headword_locally, infer_headword_with_ai,
    infer_headword_with_hint, is_form_reference_analysis, is_form_reference_entry,
    normalize_for_match, run_compact_with_source, should_bypass_knowledge_hit,
};
use crate::services::query_suggestions::{rank_candidate, sort_and_limit};
use crate::state::AppState;
use anyhow::Result;
use serde_json::Value;
use std::collections::HashMap;

pub async fn get_recent_entries(state: &AppState) -> Result<Vec<RecentItem>> {
    let mut recent_queries = {
        let recent = state.recent_searches.lock().await;
        recent.iter().cloned().collect::<Vec<_>>()
    };

    if recent_queries.is_empty() {
        recent_queries = knowledge::list_query_texts(&state.pool, 8).await?;
    }

    if recent_queries.is_empty() {
        return Ok(Vec::new());
    }

    let entries = knowledge::list_by_query_texts(&state.pool, &recent_queries).await?;
    let entry_map = entries
        .into_iter()
        .map(|entry| (entry.query_text.clone(), entry))
        .collect::<HashMap<_, _>>();

    Ok(recent_queries
        .into_iter()
        .filter_map(|query| {
            entry_map.get(&query).map(|entry| RecentItem {
                entry_id: entry.id,
                query_text: entry.query_text.clone(),
                preview: preview_from_analysis(&entry.analysis),
            })
        })
        .collect())
}

pub async fn get_all_entries(state: &AppState) -> Result<Vec<RecentItem>> {
    let entries = knowledge::list_all(&state.pool).await?;
    Ok(entries
        .into_iter()
        .map(|entry| RecentItem {
            entry_id: entry.id,
            query_text: entry.query_text,
            preview: preview_from_analysis(&entry.analysis),
        })
        .collect())
}

pub async fn get_library_entries_page(
    state: &AppState,
    q: &str,
    tab: LibraryQueryTab,
    limit: i64,
    cursor: Option<&str>,
) -> Result<LibraryEntriesPageResponse> {
    let offset = parse_library_cursor(cursor)?;
    let limit = limit.clamp(1, 60);
    let page = knowledge::list_page(&state.pool, q, tab, limit, offset).await?;

    Ok(LibraryEntriesPageResponse {
        items: page
            .entries
            .into_iter()
            .map(|entry| RecentItem {
                entry_id: entry.id,
                query_text: entry.query_text,
                preview: preview_from_analysis(&entry.analysis),
            })
            .collect(),
        total: page.total,
        next_cursor: page.next_offset.map(|value| value.to_string()),
        limit,
    })
}

pub async fn get_suggestions(state: &AppState, query: &str) -> Result<SuggestionResponse> {
    let query = query.trim();
    if query.len() < 2 {
        return Ok(SuggestionResponse {
            suggestions: Vec::new(),
        });
    }

    let fuzzy_enabled = normalize_for_match(query).chars().count() >= 3;
    let (
        alias_prefix_matches,
        query_prefix_matches,
        alias_fuzzy_matches,
        query_fuzzy_matches,
        dictionary_lexeme_matches,
    ) = tokio::try_join!(
        knowledge::find_alias_prefix_matches(&state.pool, query, 10),
        knowledge::find_prefix_matches(&state.pool, query, 10),
        async {
            if fuzzy_enabled {
                knowledge::list_alias_fuzzy_matches(&state.pool, query, 20).await
            } else {
                Ok(Vec::new())
            }
        },
        async {
            if fuzzy_enabled {
                knowledge::list_fuzzy_matches(&state.pool, query, 20).await
            } else {
                Ok(Vec::new())
            }
        },
        dictionary_lexemes::find_lexeme_candidates_by_surface(&state.pool, query, 20)
    )?;

    let mut ranked = Vec::new();
    let prefer_dictionary_surface = !dictionary_lexeme_matches.is_empty();

    ranked.extend(query_prefix_matches.into_iter().filter_map(|entry| {
        let match_text = entry.query_text.clone();
        rank_candidate(
            query,
            &match_text,
            "知识库",
            build_knowledge_suggestion(entry, "knowledge_prefix"),
            0,
        )
    }));
    if !prefer_dictionary_surface {
        ranked.extend(
            alias_prefix_matches
                .into_iter()
                .filter_map(|(entry, alias_text)| {
                    let match_text = alias_text.clone();
                    rank_candidate(
                        query,
                        &match_text,
                        "知识库",
                        build_alias_suggestion(entry, alias_text, "knowledge_alias_prefix"),
                        0,
                    )
                }),
        );
    }
    ranked.extend(dictionary_lexeme_matches.into_iter().filter_map(|entry| {
        let match_text = entry.matched_surface.clone();
        rank_candidate(
            query,
            &match_text,
            "词典",
            build_lexeme_suggestion(&entry, "dictionary_lexeme"),
            lexeme_relation_tier(&entry.matched_source),
        )
    }));
    ranked.extend(query_fuzzy_matches.into_iter().filter_map(|entry| {
        let match_text = entry.query_text.clone();
        rank_candidate(
            query,
            &match_text,
            "知识库",
            build_knowledge_suggestion(entry, "knowledge_fuzzy"),
            0,
        )
    }));
    if !prefer_dictionary_surface {
        ranked.extend(
            alias_fuzzy_matches
                .into_iter()
                .filter_map(|(entry, alias_text)| {
                    let match_text = alias_text.clone();
                    rank_candidate(
                        query,
                        &match_text,
                        "知识库",
                        build_alias_suggestion(entry, alias_text, "knowledge_alias_fuzzy"),
                        0,
                    )
                }),
        );
    }
    let suggestions = sort_and_limit(ranked, 10);

    Ok(SuggestionResponse { suggestions })
}

fn lexeme_relation_tier(matched_source: &str) -> u8 {
    match matched_source {
        "headword" => 0,
        "form_of" => 1,
        "form" => 2,
        "alias" => 3,
        _ => 4,
    }
}

fn parse_library_cursor(cursor: Option<&str>) -> Result<i64> {
    let Some(cursor) = cursor.map(str::trim).filter(|value| !value.is_empty()) else {
        return Ok(0);
    };

    let offset = cursor
        .parse::<i64>()
        .map_err(|_| anyhow::anyhow!("invalid library cursor"))?;
    if offset < 0 {
        return Err(anyhow::anyhow!("invalid library cursor"));
    }

    Ok(offset)
}

fn build_knowledge_suggestion(
    entry: crate::models::KnowledgeEntry,
    suggestion_type: &str,
) -> DBSuggestion {
    DBSuggestion {
        suggestion_type: suggestion_type.to_string(),
        entry_id: entry.id,
        query_text: entry.query_text,
        preview: preview_from_analysis(&entry.analysis),
        analysis_markdown: analysis_markdown(&entry.analysis),
        source: "知识库".to_string(),
        follow_ups: Vec::new(),
    }
}

fn build_alias_suggestion(
    entry: crate::models::KnowledgeEntry,
    alias_text: String,
    suggestion_type: &str,
) -> DBSuggestion {
    DBSuggestion {
        suggestion_type: suggestion_type.to_string(),
        entry_id: entry.id,
        query_text: entry.query_text,
        preview: format!(
            "↪ {} · {}",
            alias_text,
            preview_from_analysis(&entry.analysis)
        ),
        analysis_markdown: analysis_markdown(&entry.analysis),
        source: "知识库".to_string(),
        follow_ups: Vec::new(),
    }
}

fn build_lexeme_suggestion(
    entry: &crate::models::LexemeCandidate,
    suggestion_type: &str,
) -> DBSuggestion {
    let preview = lexeme_suggestion_preview(&entry);
    DBSuggestion {
        suggestion_type: suggestion_type.to_string(),
        entry_id: 0,
        query_text: entry.surface.clone(),
        preview,
        analysis_markdown: String::new(),
        source: "词典".to_string(),
        follow_ups: Vec::new(),
    }
}

fn lexeme_suggestion_preview(entry: &crate::models::LexemeCandidate) -> String {
    let pos = if entry.pos_summary.is_empty() {
        "未知词性".to_string()
    } else {
        entry.pos_summary.join(" / ")
    };
    let senses = entry
        .gloss_preview
        .as_array()
        .into_iter()
        .flat_map(|items| items.iter())
        .filter_map(|item| item.get("glosses"))
        .flat_map(|value| value.as_array().into_iter().flatten())
        .filter_map(|item| item.as_str())
        .take(2)
        .collect::<Vec<_>>()
        .join(" / ");

    let mapped_from = if entry.matched_surface != entry.surface {
        format!(" ↪ {}", entry.matched_surface)
    } else {
        String::new()
    };

    if senses.is_empty() {
        format!("词典候选 · {pos}{mapped_from}")
    } else {
        format!("词典候选 · {pos} · {senses}{mapped_from}")
    }
}

pub async fn intelligent_search(
    state: &AppState,
    request: IntelligentSearchRequest,
) -> Result<AnalyzeResponse> {
    let term = request.term.trim();
    let hint = request.hint.trim();
    anyhow::ensure!(!term.is_empty(), "term cannot be empty");
    tracing::info!(
        "intelligent_search start: term={term}, hint_present={}",
        !hint.is_empty()
    );

    if !hint.is_empty() {
        if let Some((headword, source)) = infer_headword_with_hint(state, term, hint).await? {
            if normalize_for_match(&headword) != normalize_for_match(term) {
                tracing::info!(
                    "intelligent_search semantic override: term={term}, hint={hint}, headword={headword}, source={source}"
                );
                return run_compact_with_source(state, &headword, source).await;
            }
        }
    }

    let knowledge_hit = knowledge::find_by_query_text_exact(&state.pool, term).await?;

    let direct_entry = dictionary::find_by_headword(&state.pool, term).await?;
    let form_entry = dictionary::find_by_form(&state.pool, term).await?;

    if let Some(entry) = knowledge_hit {
        if should_bypass_knowledge_hit(term, &entry) {
            tracing::info!(
                "intelligent_search bypass low-confidence knowledge hit: term={term}, entry_id={}",
                entry.id
            );
        } else {
            if let Some(form_entry) = &form_entry {
                if form_entry.headword != entry.query_text
                    && is_form_reference_analysis(&entry.analysis)
                {
                    tracing::info!(
                        "intelligent_search prefer headword over cached form entry: term={term}, cached={}, resolved={}",
                        entry.query_text,
                        form_entry.headword
                    );
                    return run_compact_with_source(state, &form_entry.headword, "字典变形推断")
                        .await;
                }
            }

            tracing::info!(
                "intelligent_search knowledge hit: term={term}, entry_id={}",
                entry.id
            );
            return Ok(AnalyzeResponse {
                entry_id: entry.id,
                query_text: entry.query_text,
                analysis_markdown: analysis_markdown(&entry.analysis),
                phrase_lookup: entry
                    .analysis
                    .get("phrase_lookup")
                    .cloned()
                    .and_then(|value| serde_json::from_value(value).ok()),
                phrase_usage_preview: entry
                    .analysis
                    .get("phrase_usage_preview")
                    .cloned()
                    .and_then(|value| serde_json::from_value(value).ok()),
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
                follow_ups: Vec::new(),
            });
        }
    }

    if let (Some(direct_entry), Some(form_entry)) = (&direct_entry, &form_entry) {
        if direct_entry.headword != form_entry.headword
            && is_form_reference_entry(&direct_entry.raw_data)
        {
            tracing::info!(
                "intelligent_search prefer form-derived headword over direct form entry: term={term}, direct={}, resolved={}",
                direct_entry.headword,
                form_entry.headword
            );
            return run_compact_with_source(state, &form_entry.headword, "字典变形推断").await;
        }
    }

    if let Some(entry) = direct_entry {
        tracing::info!(
            "intelligent_search dictionary direct hit: term={term}, headword={}",
            entry.headword
        );
        return run_compact_with_source(state, &entry.headword, "字典直达").await;
    }

    if let Some(entry) = form_entry {
        tracing::info!(
            "intelligent_search dictionary form hit: term={term}, headword={}",
            entry.headword
        );
        return run_compact_with_source(state, &entry.headword, "字典变形推断").await;
    }

    if let Some(headword) = infer_headword_locally(state, term).await? {
        tracing::info!("intelligent_search local fuzzy hit: term={term}, headword={headword}");
        return run_compact_with_source(state, &headword, "本地模糊推断").await;
    }

    match infer_headword_by_embedding(state, term, hint).await {
        Ok(Some(headword)) => {
            tracing::info!(
                "intelligent_search embedding hit: term={term}, headword={headword}, hint_present={}",
                !hint.is_empty()
            );
            return run_compact_with_source(state, &headword, "语义向量推断").await;
        }
        Ok(None) => {}
        Err(err) => {
            tracing::warn!("intelligent_search embedding retrieval failed: {err:#}");
        }
    }

    match infer_headword_with_ai(state, term, hint).await {
        Ok(Some(headword)) => run_compact_with_source(state, &headword, "AI推断").await,
        Ok(None) => Ok(build_intelligent_search_pending_response(term, hint)),
        Err(err) => {
            tracing::warn!(
                "intelligent_search fallback to analyze(term) after AI inference failed: {err:#}"
            );
            Ok(build_intelligent_search_pending_response(term, hint))
        }
    }
}

pub async fn status(state: &AppState) -> Result<StatusResponse> {
    sqlx::query("SELECT 1").execute(&state.pool).await?;
    Ok(StatusResponse {
        status: "ok".to_string(),
        db_status: "ok".to_string(),
    })
}
