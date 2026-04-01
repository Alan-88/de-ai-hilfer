use crate::ai::{AiChatOptions, AiScene};
use crate::models::{
    AnalyzeRequest, AnalyzeResponse, KnowledgeEntry, PhraseHostCandidate, PhraseLookupConfidence,
    PhraseLookupInfo,
};
use crate::repositories::dictionary;
use crate::services::analysis_preview::analysis_markdown;
use crate::services::analyze;
use crate::services::embedding_lookup::infer_headword_by_embedding;
use crate::state::AppState;
use anyhow::{anyhow, Result};
use serde::Deserialize;
use serde_json::Value;
use std::collections::HashMap;
use std::time::Duration;

pub async fn infer_headword_with_hint(
    state: &AppState,
    term: &str,
    hint: &str,
) -> Result<Option<(String, &'static str)>> {
    match infer_headword_by_embedding(state, term, hint).await {
        Ok(Some(headword))
            if looks_like_german_candidate(&headword)
                && normalize_for_match(&headword) != normalize_for_match(term) =>
        {
            return Ok(Some((headword, "语义向量推断")));
        }
        Ok(_) => {}
        Err(err) => {
            tracing::warn!("intelligent_search hint embedding failed: {err:#}");
        }
    }

    if let Some(headword) = infer_headword_with_ai(state, term, hint).await? {
        return Ok(Some((headword, "AI推断")));
    }

    Ok(None)
}

pub async fn infer_headword_with_ai(
    state: &AppState,
    term: &str,
    hint: &str,
) -> Result<Option<String>> {
    let input = serde_json::json!({
        "term": term,
        "hint": hint,
    })
    .to_string();

    let parsed = state
        .ai_client
        .chat_with_options(
            AiScene::IntelligentSearch,
            &state.prompts.intelligent_search_prompt,
            &input,
            AiChatOptions {
                temperature: 0.0,
                max_tokens: 48,
                timeout: Duration::from_secs(8),
            },
        )
        .await
        .and_then(|response| extract_json::<IntelligentSearchResult>(&response))?;

    tracing::info!(
        "intelligent_search ai deduced result: term={term}, hint_present={}, result={}",
        !hint.is_empty(),
        parsed.result
    );

    if !looks_like_german_candidate(&parsed.result) {
        tracing::warn!(
            "intelligent_search rejected non-german ai result: term={term}, result={}",
            parsed.result
        );
        return Ok(None);
    }

    Ok(Some(parsed.result))
}

pub fn build_intelligent_search_pending_response(term: &str, hint: &str) -> AnalyzeResponse {
    let hint_note = if hint.is_empty() {
        "当前本地字典没有直接命中，且这个查询场景仍需要 AI 做语义推断。".to_string()
    } else {
        format!("当前本地字典没有直接命中，线索“{hint}”仍需要 AI 做语义推断。")
    };

    AnalyzeResponse {
        entry_id: 0,
        query_text: term.to_string(),
        analysis_markdown: format!(
            "## 高级查询暂未完成\n\n- {hint_note}\n- 现阶段本地已经能处理：字典直达、变形识别、近似拼写。\n- 如果你输入的是中文语义线索，请等待 AI 推断链路进一步稳定后再试，或先补一个更接近的德语拼写。 "
        ),
        phrase_lookup: None,
        phrase_usage_preview: None,
        attached_phrase_modules: Vec::new(),
        source: "需要AI推断".to_string(),
        model: None,
        quality_mode: None,
        follow_ups: Vec::new(),
    }
}

pub fn looks_like_german_candidate(value: &str) -> bool {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return false;
    }

    let mut has_latin_letter = false;

    for ch in trimmed.chars() {
        if ch.is_ascii_alphabetic() || matches!(ch, 'ä' | 'ö' | 'ü' | 'Ä' | 'Ö' | 'Ü' | 'ß')
        {
            has_latin_letter = true;
            continue;
        }

        if matches!(ch, ' ' | '-' | '\'' | '/' | '.') {
            continue;
        }

        if ch.is_ascii_digit() {
            continue;
        }

        return false;
    }

    has_latin_letter
}

pub async fn run_compact_with_source(
    state: &AppState,
    query_text: &str,
    generated_source: &str,
) -> Result<AnalyzeResponse> {
    let mut response = analyze::analyze_compact(
        state,
        AnalyzeRequest {
            query_text: query_text.to_string(),
            entry_type: Some("WORD".to_string()),
            generation_hint: None,
            quality_mode: crate::models::QualityMode::Default,
            force_refresh: false,
            entry_id: None,
        },
    )
    .await?;

    if response.source == "generated" {
        response.source = generated_source.to_string();
    }

    Ok(response)
}

pub async fn infer_headword_locally(state: &AppState, term: &str) -> Result<Option<String>> {
    let candidates = dictionary::list_fuzzy_headwords(&state.pool, term, 120).await?;
    let normalized_term = normalize_for_match(term);

    Ok(candidates
        .into_iter()
        .map(|candidate| {
            let score = fuzzy_score(&normalized_term, &normalize_for_match(&candidate));
            (candidate, score)
        })
        .filter(|(_, score)| *score <= fuzzy_threshold(normalized_term.chars().count()))
        .min_by_key(|(_, score)| *score)
        .map(|(candidate, _)| candidate))
}

pub fn is_phrase_like_query(value: &str) -> bool {
    let tokens = tokenize_phrase(value);
    let token_count = tokens.len();
    (2..=8).contains(&token_count)
}

pub async fn infer_phrase_lookup(
    state: &AppState,
    phrase: &str,
) -> Result<Option<PhraseLookupInfo>> {
    if !is_phrase_like_query(phrase) {
        return Ok(None);
    }

    let mut candidates = HashMap::<String, PhraseHostCandidate>::new();

    for token in content_tokens(phrase) {
        if let Some(entry) = dictionary::find_by_headword(&state.pool, &token).await? {
            merge_phrase_candidate(
                &mut candidates,
                PhraseHostCandidate {
                    headword: entry.headword,
                    source: "词典词头".to_string(),
                    score: 0.98,
                },
            );
            continue;
        }

        if let Some(entry) = dictionary::find_by_form(&state.pool, &token).await? {
            merge_phrase_candidate(
                &mut candidates,
                PhraseHostCandidate {
                    headword: entry.headword,
                    source: "词形还原".to_string(),
                    score: 0.93,
                },
            );
        }
    }

    match infer_headword_by_embedding(state, phrase, "").await {
        Ok(Some(headword)) => merge_phrase_candidate(
            &mut candidates,
            PhraseHostCandidate {
                headword,
                source: "语义向量".to_string(),
                score: 0.72,
            },
        ),
        Ok(None) => {}
        Err(err) => {
            tracing::warn!("phrase lookup embedding failed: phrase={phrase}, err={err:#}");
        }
    }

    let mut host_candidates = candidates.into_values().collect::<Vec<_>>();
    host_candidates.sort_by(|left, right| {
        right
            .score
            .total_cmp(&left.score)
            .then(left.headword.cmp(&right.headword))
    });

    let best_host_headword = host_candidates
        .first()
        .map(|candidate| candidate.headword.clone());
    let confidence = phrase_lookup_confidence(&host_candidates);

    Ok(Some(PhraseLookupInfo {
        phrase: phrase.trim().to_string(),
        best_host_headword,
        confidence,
        host_candidates,
    }))
}

pub fn normalize_for_match(value: &str) -> String {
    value
        .trim()
        .to_lowercase()
        .chars()
        .flat_map(fold_match_char)
        .collect()
}

fn merge_phrase_candidate(
    candidates: &mut HashMap<String, PhraseHostCandidate>,
    candidate: PhraseHostCandidate,
) {
    candidates
        .entry(candidate.headword.clone())
        .and_modify(|existing| {
            if candidate.score > existing.score {
                *existing = candidate.clone();
            }
        })
        .or_insert(candidate);
}

fn phrase_lookup_confidence(candidates: &[PhraseHostCandidate]) -> PhraseLookupConfidence {
    let Some(best) = candidates.first() else {
        return PhraseLookupConfidence::Low;
    };

    let gap = candidates
        .get(1)
        .map(|next| best.score - next.score)
        .unwrap_or(best.score);

    if best.score >= 0.95 && gap >= 0.08 {
        PhraseLookupConfidence::High
    } else if best.score >= 0.78 {
        PhraseLookupConfidence::Medium
    } else {
        PhraseLookupConfidence::Low
    }
}

fn content_tokens(value: &str) -> Vec<String> {
    tokenize_phrase(value)
        .into_iter()
        .filter(|token| !is_phrase_function_token(token))
        .collect()
}

fn tokenize_phrase(value: &str) -> Vec<String> {
    value
        .split(|ch: char| {
            !(ch.is_ascii_alphabetic()
                || matches!(ch, 'ä' | 'ö' | 'ü' | 'Ä' | 'Ö' | 'Ü' | 'ß' | '-' | '\''))
        })
        .map(str::trim)
        .filter(|token| !token.is_empty())
        .map(ToString::to_string)
        .collect()
}

fn is_phrase_function_token(token: &str) -> bool {
    matches!(
        normalize_for_match(token).as_str(),
        "der"
            | "die"
            | "das"
            | "dem"
            | "den"
            | "des"
            | "ein"
            | "eine"
            | "einer"
            | "einem"
            | "einen"
            | "von"
            | "vor"
            | "fur"
            | "mit"
            | "bei"
            | "nach"
            | "zu"
            | "zum"
            | "zur"
            | "am"
            | "im"
            | "an"
            | "auf"
            | "aus"
            | "uber"
            | "unter"
            | "um"
            | "in"
            | "ins"
            | "ans"
            | "sich"
            | "mir"
            | "mich"
            | "dir"
            | "dich"
            | "uns"
            | "euch"
            | "ihm"
            | "ihn"
            | "ihr"
            | "ihnen"
    )
}

fn fold_match_char(ch: char) -> Vec<char> {
    match ch {
        'ä' => vec!['a'],
        'ö' => vec!['o'],
        'ü' => vec!['u'],
        'ß' => vec!['s', 's'],
        _ if ch.is_alphanumeric() => vec![ch],
        _ => Vec::new(),
    }
}

pub fn is_form_reference_entry(raw_data: &Value) -> bool {
    raw_data
        .get("senses")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .any(|sense| {
            sense
                .get("tags")
                .and_then(Value::as_array)
                .map(|tags| tags.iter().any(|tag| tag.as_str() == Some("form-of")))
                .unwrap_or(false)
        })
}

pub fn is_form_reference_analysis(analysis: &Value) -> bool {
    analysis
        .get("dictionary_excerpt")
        .and_then(|excerpt| excerpt.get("senses"))
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .any(|sense| {
            sense
                .get("tags")
                .and_then(Value::as_array)
                .map(|tags| tags.iter().any(|tag| tag.as_str() == Some("form-of")))
                .unwrap_or(false)
        })
}

pub fn should_bypass_knowledge_hit(term: &str, entry: &KnowledgeEntry) -> bool {
    !looks_like_german_candidate(term)
        && !looks_like_german_candidate(&entry.query_text)
        && analysis_markdown(&entry.analysis).contains("暂未命中字典或知识库")
}

fn extract_json<T: for<'de> Deserialize<'de>>(raw: &str) -> Result<T> {
    if let (Some(start), Some(end)) = (raw.find('{'), raw.rfind('}')) {
        let candidate = &raw[start..=end];
        return Ok(serde_json::from_str(candidate)?);
    }

    Err(anyhow!("failed to locate JSON object in AI response"))
}

#[derive(Deserialize)]
struct IntelligentSearchResult {
    result: String,
}

fn fuzzy_score(input: &str, candidate: &str) -> usize {
    let mut score = levenshtein_distance(input, candidate);

    if input.chars().next() != candidate.chars().next() {
        score += 2;
    }
    if input.chars().last() != candidate.chars().last() {
        score += 1;
    }
    if !candidate.starts_with(&input.chars().take(2).collect::<String>()) {
        score += 1;
    }

    score
}

fn fuzzy_threshold(term_len: usize) -> usize {
    match term_len {
        0..=4 => 1,
        5..=7 => 2,
        _ => 3,
    }
}

fn levenshtein_distance(left: &str, right: &str) -> usize {
    let left_chars = left.chars().collect::<Vec<_>>();
    let right_chars = right.chars().collect::<Vec<_>>();
    let mut prev = (0..=right_chars.len()).collect::<Vec<_>>();
    let mut curr = vec![0; right_chars.len() + 1];

    for (i, left_char) in left_chars.iter().enumerate() {
        curr[0] = i + 1;

        for (j, right_char) in right_chars.iter().enumerate() {
            let cost = usize::from(left_char != right_char);
            curr[j + 1] = (prev[j + 1] + 1).min(curr[j] + 1).min(prev[j] + cost);
        }

        prev.clone_from(&curr);
    }

    prev[right_chars.len()]
}
