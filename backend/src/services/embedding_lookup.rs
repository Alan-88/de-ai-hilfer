use crate::ai::{AiEmbeddingOptions, AiScene};
use crate::repositories::dictionary_lexemes;
use crate::state::AppState;
use anyhow::Result;
use std::time::Duration;

const EMBEDDING_CANDIDATE_LIMIT: i64 = 5;
const EMBEDDING_DISTANCE_THRESHOLD_TERM_ONLY: f32 = 0.36;
const EMBEDDING_DISTANCE_THRESHOLD_WITH_HINT: f32 = 0.45;
const EMBEDDING_MIN_MARGIN: f32 = 0.025;

pub async fn infer_headword_by_embedding(
    state: &AppState,
    term: &str,
    hint: &str,
) -> Result<Option<String>> {
    let embedding_input = build_embedding_query_input(term, hint);
    let embedding = state
        .ai_client
        .embed_with_options(
            AiScene::Embedding,
            &[embedding_input],
            AiEmbeddingOptions {
                timeout: Duration::from_secs(12),
                dimensions: None,
            },
        )
        .await?
        .into_iter()
        .next()
        .unwrap_or_default();

    if embedding.is_empty() {
        return Ok(None);
    }

    let lexeme_candidates = dictionary_lexemes::search_lexemes_by_embedding(
        &state.pool,
        &state.config.ai_models.embedding,
        &embedding,
        EMBEDDING_CANDIDATE_LIMIT,
    )
    .await?;

    let best = match lexeme_candidates.first() {
        Some(hit) => hit,
        None => return Ok(None),
    };
    let threshold = embedding_distance_threshold(hint);
    let margin_ok = lexeme_candidates
        .get(1)
        .map(|next| next.distance - best.distance >= EMBEDDING_MIN_MARGIN)
        .unwrap_or(true);
    let candidate_allowed = is_reliable_embedding_candidate(term, hint, &best.surface);

    tracing::info!(
        "embedding lexeme candidates: best={}#{}, distance={:.4}, second_distance={:?}, threshold={:.4}, margin_ok={}, candidate_allowed={}, best_frequency_rank={:?}",
        best.surface,
        best.lexeme_id,
        best.distance,
        lexeme_candidates.get(1).map(|next| next.distance),
        threshold,
        margin_ok,
        candidate_allowed,
        best.frequency_rank
    );

    if best.distance <= threshold && margin_ok && candidate_allowed {
        Ok(Some(best.surface.clone()))
    } else {
        Ok(None)
    }
}

fn build_embedding_query_input(term: &str, hint: &str) -> String {
    let hint = hint.trim();
    if hint.is_empty() {
        return term.to_string();
    }

    if looks_like_german_candidate(term) {
        format!("{term}\n{hint}")
    } else {
        hint.to_string()
    }
}

fn embedding_distance_threshold(hint: &str) -> f32 {
    if hint.trim().is_empty() {
        EMBEDDING_DISTANCE_THRESHOLD_TERM_ONLY
    } else {
        EMBEDDING_DISTANCE_THRESHOLD_WITH_HINT
    }
}

fn is_reliable_embedding_candidate(term: &str, hint: &str, headword: &str) -> bool {
    if hint.trim().is_empty() || looks_like_german_candidate(term) {
        return true;
    }

    let len = headword.chars().count();
    if len <= 2 {
        return false;
    }

    !headword.chars().all(|ch| ch.is_ascii_uppercase())
}

fn looks_like_german_candidate(value: &str) -> bool {
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
