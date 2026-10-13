use crate::ai_settings_models::AiModelOverride;
use crate::models::{
    AnalyzeRequest, AnalyzeResponse, LearningProgress, LearningProgressView, LearningRecallRating,
    LexemeCandidate,
};
use crate::repositories::{
    agent_api as repository, dictionary_lexemes, knowledge, learning as learning_repository,
};
use crate::services::learning_session::{
    ACTIVE_INTRADAY_WORKSET_TARGET, INTRADAY_FORGOTTEN_DELAY_SECONDS, INTRADAY_FUZZY_DELAY_SECONDS,
    INTRADAY_KNOWN_DELAY_SECONDS, INTRADAY_RECOVERY_KNOWN_DELAY_SECONDS, MAX_INTRADAY_APPEARANCES,
};
use crate::services::{analyze, learning};
use crate::state::AppState;
use anyhow::{anyhow, Result};
use chrono::{DateTime, NaiveDate, Utc};
use serde::Serialize;
use serde_json::Value;
use std::collections::HashSet;
use uuid::Uuid;

#[derive(Debug, Serialize)]
pub struct LexemeResolveResponse {
    pub surface: String,
    pub candidates: Vec<LexemeCandidate>,
}

#[derive(Debug, Serialize)]
pub struct LexemeStateResponse {
    pub lexeme: LexemeSummary,
    pub knowledge: KnowledgeState,
    pub learning: LearningState,
}

#[derive(Debug, Serialize)]
pub struct LexemeSummary {
    pub id: i64,
    pub headword: String,
    pub pos: Vec<String>,
    pub gloss: Value,
}

#[derive(Debug, Serialize)]
pub struct KnowledgeState {
    pub status: &'static str,
    pub entry_id: Option<i64>,
    pub query_text: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct LearningState {
    pub status: &'static str,
    pub due_date: Option<DateTime<Utc>>,
    pub review_count: Option<i32>,
    pub lapses: Option<i32>,
}

#[derive(Debug, Serialize)]
pub struct LearningListItem {
    pub entry_id: i64,
    pub lexeme_id: Option<i64>,
    pub query_text: String,
    pub status: &'static str,
    pub due_date: DateTime<Utc>,
    pub review_count: i32,
    pub lapses: i32,
}

#[derive(Debug, Serialize)]
pub struct LearningMutationResponse {
    pub entry_id: i64,
    pub status: &'static str,
}

#[derive(Debug, Serialize)]
pub struct LearningLogsResponse {
    pub business_date: NaiveDate,
    pub entry_id: Option<i64>,
    pub session_id: Option<String>,
    pub limit: i64,
    pub summary: LearningLogsSummary,
    pub logs: Vec<LearningLogItem>,
}

#[derive(Debug, Default, Serialize)]
pub struct LearningLogsSummary {
    pub returned_logs: i64,
    pub distinct_cards: i64,
    pub distinct_sessions: i64,
    pub first_reviews: i64,
    pub intraday_reviews: i64,
    pub long_term_commits: i64,
    pub known: i64,
    pub fuzzy: i64,
    pub forgotten: i64,
}

#[derive(Debug, Serialize)]
pub struct LearningLogItem {
    pub id: i64,
    pub session_id: String,
    pub entry_id: i64,
    pub query_text: String,
    pub rating: String,
    pub effective_rating: Option<String>,
    pub correction_status: String,
    pub correction_action: Option<String>,
    pub phase: String,
    pub is_first_today: bool,
    pub appearance_count_today: i32,
    pub long_term_committed: bool,
    pub business_date: NaiveDate,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct LearningCorrectionResponse {
    pub entry_id: i64,
    pub business_date: NaiveDate,
    pub action: String,
    pub operation_id: String,
    pub affected_log_ids: Vec<i64>,
    pub invalidated_sessions: usize,
    pub effective_first_rating: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct LearningPlanResponse {
    pub mode: &'static str,
    pub as_of: DateTime<Utc>,
    pub business_date: NaiveDate,
    pub scheduler: LearningPlanScheduler,
    pub summary: LearningPlanSummary,
    pub cards: Vec<LearningPlanCard>,
}

#[derive(Debug, Serialize)]
pub struct LearningPlanScheduler {
    pub active_workset_target: usize,
    pub max_intraday_appearances: i32,
    pub fallback: &'static str,
    pub forgotten_delay_seconds: i64,
    pub fuzzy_delay_seconds: i64,
    pub known_delay_seconds: i64,
    pub forgotten_then_known_delay_seconds: i64,
}

#[derive(Debug, Serialize)]
pub struct LearningPlanSummary {
    pub total_cards: i64,
    pub ready_cards: i64,
    pub waiting_cards: i64,
    pub new_cards: i64,
    pub long_term_due_cards: i64,
    pub intraday_cards: i64,
    pub next_available_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize)]
pub struct LearningPlanCard {
    pub entry_id: i64,
    pub query_text: String,
    pub source: &'static str,
    pub phase: &'static str,
    pub first_rating: Option<String>,
    pub appearance_count_today: i32,
    pub success_streak: i32,
    pub next_review_at: DateTime<Utc>,
    pub ready: bool,
    pub long_term_due_at: Option<DateTime<Utc>>,
}

pub async fn resolve_lexeme(
    state: &AppState,
    surface: &str,
    limit: i64,
) -> Result<LexemeResolveResponse> {
    let surface = surface.trim();
    anyhow::ensure!(!surface.is_empty(), "surface cannot be empty");
    let candidates = dictionary_lexemes::find_lexeme_candidates_by_surface(
        &state.pool,
        surface,
        limit.clamp(1, 20),
    )
    .await?;
    Ok(LexemeResolveResponse {
        surface: surface.to_string(),
        candidates,
    })
}

pub async fn inspect_lexeme(state: &AppState, lexeme_id: i64) -> Result<LexemeStateResponse> {
    let row = repository::find_lexeme_state(&state.pool, lexeme_id)
        .await?
        .ok_or_else(|| anyhow!("lexeme not found"))?;
    Ok(LexemeStateResponse {
        lexeme: LexemeSummary {
            id: row.lexeme_id,
            headword: row.surface,
            pos: row.pos_summary,
            gloss: row.gloss_preview,
        },
        knowledge: KnowledgeState {
            status: if row.entry_id.is_some() {
                "ready"
            } else {
                "missing"
            },
            entry_id: row.entry_id,
            query_text: row.query_text,
        },
        learning: LearningState {
            status: learning_status(row.learning_state),
            due_date: row.due_date,
            review_count: row.review_count,
            lapses: row.lapses,
        },
    })
}

pub async fn initialize_lexeme(
    state: &AppState,
    lexeme_id: i64,
    model_override: Option<AiModelOverride>,
) -> Result<AnalyzeResponse> {
    let current = inspect_lexeme(state, lexeme_id).await?;
    let result = analyze::analyze(
        state,
        AnalyzeRequest {
            query_text: current.lexeme.headword,
            entry_type: Some("WORD".to_string()),
            generation_hint: None,
            quality_mode: Default::default(),
            force_refresh: false,
            entry_id: None,
            model_override,
        },
    )
    .await?;
    anyhow::ensure!(
        result.entry_id > 0,
        "lexeme initialization did not create an entry"
    );
    Ok(result)
}

pub async fn list_learning(state: &AppState) -> Result<Vec<LearningListItem>> {
    Ok(repository::list_learning(&state.pool)
        .await?
        .into_iter()
        .map(|row| LearningListItem {
            entry_id: row.entry_id,
            lexeme_id: row.lexeme_id,
            query_text: row.query_text,
            status: learning_status(Some(row.learning_state)),
            due_date: row.due_date,
            review_count: row.review_count,
            lapses: row.lapses,
        })
        .collect())
}

pub async fn list_learning_logs(
    state: &AppState,
    business_date: Option<NaiveDate>,
    entry_id: Option<i64>,
    session_id: Option<&str>,
    limit: i64,
) -> Result<LearningLogsResponse> {
    let business_date = business_date.unwrap_or_else(|| learning::business_date(Utc::now()));
    let limit = limit.clamp(1, 500);
    let rows = learning_repository::list_review_logs(
        &state.pool,
        business_date,
        entry_id,
        session_id,
        limit,
    )
    .await?;

    let mut summary = LearningLogsSummary::default();
    let mut cards = HashSet::new();
    let mut sessions = HashSet::new();
    for row in &rows {
        cards.insert(row.entry_id);
        sessions.insert(row.session_id.as_str());
        if row.effective_rating.is_none() {
            continue;
        }
        if row.is_first_today {
            summary.first_reviews += 1;
        } else {
            summary.intraday_reviews += 1;
        }
        if row.long_term_committed {
            summary.long_term_commits += 1;
        }
        match row.effective_rating.as_deref() {
            Some("known") => summary.known += 1,
            Some("fuzzy") => summary.fuzzy += 1,
            Some("forgotten") => summary.forgotten += 1,
            _ => {}
        }
    }
    summary.returned_logs = rows.len() as i64;
    summary.distinct_cards = cards.len() as i64;
    summary.distinct_sessions = sessions.len() as i64;

    let logs = rows
        .into_iter()
        .map(|row| LearningLogItem {
            id: row.id,
            session_id: row.session_id,
            entry_id: row.entry_id,
            query_text: row.query_text,
            rating: row.rating,
            effective_rating: row.effective_rating,
            correction_status: row.correction_status,
            correction_action: row.correction_action,
            phase: row.phase,
            is_first_today: row.is_first_today,
            appearance_count_today: row.appearance_count_today,
            long_term_committed: row.long_term_committed,
            business_date: row.business_date,
            created_at: row.created_at,
        })
        .collect();

    Ok(LearningLogsResponse {
        business_date,
        entry_id,
        session_id: session_id.map(str::to_owned),
        limit,
        summary,
        logs,
    })
}

pub async fn get_learning_plan(state: &AppState) -> Result<LearningPlanResponse> {
    let now = Utc::now();
    let business_date = learning::business_date(now);
    let recoveries = learning_repository::list_intraday_recoveries(
        &state.pool,
        business_date,
        MAX_INTRADAY_APPEARANCES,
        INTRADAY_RECOVERY_KNOWN_DELAY_SECONDS,
        INTRADAY_KNOWN_DELAY_SECONDS,
        INTRADAY_FORGOTTEN_DELAY_SECONDS,
        INTRADAY_FUZZY_DELAY_SECONDS,
    )
    .await?;
    let recovered_entry_ids = recoveries
        .iter()
        .map(|recovery| recovery.entry.id)
        .collect::<HashSet<_>>();
    let due_entries = learning_repository::list_due_entries(&state.pool, now).await?;
    let mut cards = Vec::with_capacity(due_entries.len() + recoveries.len());

    for (progress, entry) in due_entries {
        if recovered_entry_ids.contains(&entry.id) {
            continue;
        }
        let is_new = progress.review_count == 0 || progress.state == 0;
        cards.push(LearningPlanCard {
            entry_id: entry.id,
            query_text: entry.query_text,
            source: if is_new { "new_card" } else { "long_term_due" },
            phase: if is_new { "new" } else { "review" },
            first_rating: None,
            appearance_count_today: 0,
            success_streak: 0,
            next_review_at: progress.due_date,
            ready: true,
            long_term_due_at: Some(progress.due_date),
        });
    }

    for recovery in recoveries {
        let ready = recovery.next_review_at <= now;
        cards.push(LearningPlanCard {
            entry_id: recovery.entry.id,
            query_text: recovery.entry.query_text,
            source: "intraday_recovery",
            phase: "intraday",
            first_rating: Some(recovery.first_rating),
            appearance_count_today: recovery.appearance_count_today,
            success_streak: recovery.success_streak,
            next_review_at: recovery.next_review_at,
            ready,
            long_term_due_at: Some(recovery.progress.due_date),
        });
    }

    cards.sort_by_key(|card| card.next_review_at);
    let ready_cards = cards.iter().filter(|card| card.ready).count() as i64;
    let waiting_cards = cards.len() as i64 - ready_cards;
    let new_cards = cards
        .iter()
        .filter(|card| card.source == "new_card")
        .count() as i64;
    let long_term_due_cards = cards
        .iter()
        .filter(|card| card.source == "long_term_due")
        .count() as i64;
    let intraday_cards = cards
        .iter()
        .filter(|card| card.source == "intraday_recovery")
        .count() as i64;
    let next_available_at = cards
        .iter()
        .filter(|card| !card.ready)
        .map(|card| card.next_review_at)
        .min();

    Ok(LearningPlanResponse {
        mode: "reconstructible",
        as_of: now,
        business_date,
        scheduler: LearningPlanScheduler {
            active_workset_target: ACTIVE_INTRADAY_WORKSET_TARGET,
            max_intraday_appearances: MAX_INTRADAY_APPEARANCES,
            fallback: "count_when_workset_is_full",
            forgotten_delay_seconds: INTRADAY_FORGOTTEN_DELAY_SECONDS,
            fuzzy_delay_seconds: INTRADAY_FUZZY_DELAY_SECONDS,
            known_delay_seconds: INTRADAY_KNOWN_DELAY_SECONDS,
            forgotten_then_known_delay_seconds: INTRADAY_RECOVERY_KNOWN_DELAY_SECONDS,
        },
        summary: LearningPlanSummary {
            total_cards: cards.len() as i64,
            ready_cards,
            waiting_cards,
            new_cards,
            long_term_due_cards,
            intraday_cards,
            next_available_at,
        },
        cards,
    })
}

pub async fn add_learning(state: &AppState, entry_id: i64) -> Result<LearningProgressView> {
    learning::add_word(state, entry_id).await
}

pub(crate) async fn membership_once(
    state: &AppState,
    key: &str,
    action: crate::batch_models::MembershipAction,
    entry_id: i64,
) -> Result<Value> {
    use crate::batch_models::MembershipAction;
    use crate::repositories::agent_batches;
    let mut tx = state.pool.begin().await?;
    agent_batches::lock_operation(&mut tx, key).await?;
    if let Some(value) = agent_batches::receipt(&mut tx, key).await? {
        return Ok(value);
    }
    knowledge::find_by_id(&state.pool, entry_id)
        .await?
        .ok_or_else(|| anyhow!("knowledge entry not found"))?;
    let status = match action {
        MembershipAction::Add => {
            learning_repository::insert_progress(
                &mut *tx,
                &crate::models::NewLearningProgress { entry_id },
            )
            .await?;
            "added"
        }
        MembershipAction::Remove => {
            repository::remove_learning(&mut *tx, entry_id).await?;
            "not_added"
        }
        MembershipAction::Reset => {
            repository::reset_learning(&mut *tx, entry_id).await?;
            "new"
        }
    };
    let result = serde_json::json!({"entry_id":entry_id,"status":status});
    agent_batches::save_receipt(&mut tx, key, &result).await?;
    tx.commit().await?;
    Ok(result)
}

pub async fn reset_intraday(state: &AppState, entry_id: i64) -> Result<LearningCorrectionResponse> {
    apply_intraday_correction(state, entry_id, "intraday_reset", true).await
}

pub async fn undo_last_intraday(
    state: &AppState,
    entry_id: i64,
) -> Result<LearningCorrectionResponse> {
    apply_intraday_correction(state, entry_id, "intraday_undo", false).await
}

async fn apply_intraday_correction(
    state: &AppState,
    entry_id: i64,
    action: &str,
    reset_all: bool,
) -> Result<LearningCorrectionResponse> {
    knowledge::find_by_id(&state.pool, entry_id)
        .await?
        .ok_or_else(|| anyhow!("knowledge entry not found"))?;

    let business_date = learning::business_date(Utc::now());
    let logs = learning_repository::list_review_logs(
        &state.pool,
        business_date,
        Some(entry_id),
        None,
        1000,
    )
    .await?;
    let mut active_intraday = logs
        .iter()
        .filter(|log| !log.is_first_today && log.effective_rating.is_some())
        .collect::<Vec<_>>();
    if active_intraday.is_empty() {
        return Err(anyhow!("no active intraday feedback to correct"));
    }
    active_intraday.sort_by_key(|log| (log.created_at, log.id));
    let target_log_ids = if reset_all {
        active_intraday.iter().map(|log| log.id).collect::<Vec<_>>()
    } else {
        vec![active_intraday.last().expect("non-empty").id]
    };
    let operation_id = Uuid::new_v4().to_string();
    learning_repository::insert_correction_rows(
        &state.pool,
        &operation_id,
        entry_id,
        business_date,
        action,
        &target_log_ids,
        None,
        None,
        None,
    )
    .await?;
    let invalidated_sessions =
        learning::invalidate_sessions_for_business_date(state, business_date).await;
    let effective_first_rating = logs
        .iter()
        .find(|log| log.is_first_today && log.effective_rating.is_some())
        .and_then(|log| log.effective_rating.clone());

    Ok(LearningCorrectionResponse {
        entry_id,
        business_date,
        action: action.to_string(),
        operation_id,
        affected_log_ids: target_log_ids,
        invalidated_sessions,
        effective_first_rating,
    })
}

pub async fn correct_first_rating(
    state: &AppState,
    entry_id: i64,
    replacement_rating: &str,
) -> Result<LearningCorrectionResponse> {
    let replacement = parse_recall_rating(replacement_rating)?;
    knowledge::find_by_id(&state.pool, entry_id)
        .await?
        .ok_or_else(|| anyhow!("knowledge entry not found"))?;

    let business_date = learning::business_date(Utc::now());
    let logs = learning_repository::list_review_logs(
        &state.pool,
        business_date,
        Some(entry_id),
        None,
        1000,
    )
    .await?;
    let first = logs
        .iter()
        .find(|log| log.is_first_today && log.effective_rating.is_some())
        .ok_or_else(|| anyhow!("no first feedback found for this business day"))?;
    let previous_rating = first.effective_rating.as_deref().unwrap_or_default();
    if previous_rating == replacement_rating {
        return Err(anyhow!("replacement rating is unchanged"));
    }
    let (progress_before, expected_progress) = parse_progress_snapshots(
        first.progress_before.as_ref(),
        first.effective_progress_after.as_ref(),
    )?;
    let current_progress = learning_repository::find_progress(&state.pool, entry_id)
        .await?
        .ok_or_else(|| anyhow!("learning progress not found"))?;
    if serde_json::to_value(&current_progress)? != serde_json::to_value(&expected_progress)? {
        return Err(anyhow!(
            "learning progress changed after the recorded feedback; refusing automatic correction"
        ));
    }

    let corrected_progress =
        learning::recompute_long_term_progress(&progress_before, replacement, first.created_at)?;
    let current_progress_value = serde_json::to_value(&current_progress)?;
    let corrected_value = serde_json::to_value(&corrected_progress)?;
    let operation_id = Uuid::new_v4().to_string();
    learning_repository::update_progress_and_insert_correction(
        &state.pool,
        &corrected_progress,
        &operation_id,
        business_date,
        first.id,
        replacement_rating,
        &current_progress_value,
        &corrected_value,
    )
    .await?;
    let invalidated_sessions =
        learning::invalidate_sessions_for_business_date(state, business_date).await;

    Ok(LearningCorrectionResponse {
        entry_id,
        business_date,
        action: "first_rating_correction".to_string(),
        operation_id,
        affected_log_ids: vec![first.id],
        invalidated_sessions,
        effective_first_rating: Some(replacement_rating.to_string()),
    })
}

fn parse_recall_rating(value: &str) -> Result<LearningRecallRating> {
    match value {
        "known" => Ok(LearningRecallRating::Known),
        "fuzzy" => Ok(LearningRecallRating::Fuzzy),
        "forgotten" => Ok(LearningRecallRating::Forgotten),
        _ => Err(anyhow!(
            "invalid rating: {value}; expected known, fuzzy, or forgotten"
        )),
    }
}

fn parse_progress_snapshots(
    progress_before: Option<&Value>,
    progress_after: Option<&Value>,
) -> Result<(LearningProgress, LearningProgress)> {
    let before =
        progress_before.ok_or_else(|| anyhow!("first feedback has no progress snapshot"))?;
    let after =
        progress_after.ok_or_else(|| anyhow!("first feedback has no current progress snapshot"))?;
    let before = serde_json::from_value(before.clone())
        .map_err(|error| anyhow!("invalid first-feedback progress snapshot: {error}"))?;
    let after = serde_json::from_value(after.clone())
        .map_err(|error| anyhow!("invalid current progress snapshot: {error}"))?;
    Ok((before, after))
}

pub async fn remove_learning(state: &AppState, entry_id: i64) -> Result<LearningMutationResponse> {
    knowledge::find_by_id(&state.pool, entry_id)
        .await?
        .ok_or_else(|| anyhow!("knowledge entry not found"))?;
    repository::remove_learning(&state.pool, entry_id).await?;
    Ok(LearningMutationResponse {
        entry_id,
        status: "not_added",
    })
}

pub async fn reset_learning(state: &AppState, entry_id: i64) -> Result<LearningMutationResponse> {
    knowledge::find_by_id(&state.pool, entry_id)
        .await?
        .ok_or_else(|| anyhow!("knowledge entry not found"))?;
    repository::reset_learning(&state.pool, entry_id).await?;
    Ok(LearningMutationResponse {
        entry_id,
        status: "new",
    })
}

fn learning_status(state: Option<i32>) -> &'static str {
    match state {
        None => "not_added",
        Some(0) => "new",
        Some(1) => "learning",
        Some(2) => "review",
        Some(3) => "relearning",
        Some(_) => "unknown",
    }
}

#[cfg(test)]
mod tests {
    use super::{learning_status, parse_progress_snapshots, parse_recall_rating};
    use crate::models::LearningRecallRating;
    use serde_json::json;

    #[test]
    fn maps_learning_states() {
        assert_eq!(learning_status(None), "not_added");
        assert_eq!(learning_status(Some(0)), "new");
        assert_eq!(learning_status(Some(3)), "relearning");
        assert_eq!(learning_status(Some(99)), "unknown");
    }

    #[test]
    fn parses_supported_recall_ratings_only() {
        assert_eq!(
            parse_recall_rating("known").unwrap(),
            LearningRecallRating::Known
        );
        assert_eq!(
            parse_recall_rating("fuzzy").unwrap(),
            LearningRecallRating::Fuzzy
        );
        assert_eq!(
            parse_recall_rating("forgotten").unwrap(),
            LearningRecallRating::Forgotten
        );
        assert!(parse_recall_rating("again").is_err());
    }

    #[test]
    fn rejects_first_feedback_without_snapshots() {
        let error = parse_progress_snapshots(None, Some(&json!({}))).unwrap_err();
        assert!(error.to_string().contains("no progress snapshot"));
    }
}
