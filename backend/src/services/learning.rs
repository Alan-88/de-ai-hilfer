use crate::models::{
    LearningProgressMapResponse, LearningProgressView, LearningSessionResponse,
    LearningSessionWord, LearningStatsResponse, NewLearningProgress,
};
use crate::repositories::{knowledge, learning};
use crate::services::analysis_preview::analysis_markdown;
use crate::state::AppState;
use anyhow::{anyhow, Result};
use chrono::{Duration, Utc};
use fsrs::{MemoryState, DEFAULT_PARAMETERS, FSRS};
use std::collections::HashMap;

pub async fn get_session(
    state: &AppState,
    _limit_new_words: i64,
) -> Result<LearningSessionResponse> {
    let now = Utc::now();
    let due_entries = learning::list_due_entries(&state.pool, now).await?;

    if due_entries.is_empty() {
        return Ok(LearningSessionResponse {
            current_word: None,
            completed_count: 0,
            total_count: 0,
            is_completed: true,
        });
    }

    let (progress, entry) = due_entries
        .first()
        .cloned()
        .ok_or_else(|| anyhow!("no due entries available"))?;

    let current_word = LearningSessionWord {
        entry_id: entry.id,
        query_text: entry.query_text,
        analysis_markdown: analysis_markdown(&entry.analysis),
        repetitions_left: estimate_repetitions_left(&progress),
        progress: Some(progress_to_view(&progress)),
    };

    Ok(LearningSessionResponse {
        current_word: Some(current_word),
        completed_count: 0,
        total_count: due_entries.len() as i32,
        is_completed: false,
    })
}

pub async fn add_word(state: &AppState, entry_id: i64) -> Result<LearningProgressView> {
    knowledge::find_by_id(&state.pool, entry_id)
        .await?
        .ok_or_else(|| anyhow!("knowledge entry not found"))?;

    let progress = match learning::find_progress(&state.pool, entry_id).await? {
        Some(progress) => progress,
        None => learning::insert_progress(&state.pool, &NewLearningProgress { entry_id }).await?,
    };

    Ok(progress_to_view(&progress))
}

pub async fn submit_review(
    state: &AppState,
    entry_id: i64,
    quality: i32,
) -> Result<LearningProgressView> {
    let mut progress = learning::find_progress(&state.pool, entry_id)
        .await?
        .ok_or_else(|| anyhow!("learning progress not found"))?;

    let fsrs = FSRS::new(Some(&DEFAULT_PARAMETERS))?;
    let elapsed_days = progress
        .last_review_at
        .map(|last| (Utc::now() - last).num_days().max(0) as u32)
        .unwrap_or(0);
    let current_memory =
        if progress.review_count == 0 || progress.stability <= 0.0 || progress.difficulty <= 0.0 {
            None
        } else {
            Some(MemoryState {
                stability: progress.stability,
                difficulty: progress.difficulty,
            })
        };

    let next_states = fsrs.next_states(current_memory, 0.9, elapsed_days)?;
    let selected = match map_quality_to_rating(quality) {
        1 => next_states.again,
        2 => next_states.hard,
        3 => next_states.good,
        4 => next_states.easy,
        _ => unreachable!(),
    };

    let now = Utc::now();
    let scheduled_days = selected.interval.round().max(0.0) as i64;
    let next_due = now + Duration::days(scheduled_days.max(1));

    progress.stability = selected.memory.stability;
    progress.difficulty = selected.memory.difficulty;
    progress.elapsed_days = elapsed_days as i64;
    progress.scheduled_days = scheduled_days;
    progress.state = next_state_value(quality, scheduled_days);
    progress.last_review_at = Some(now);
    progress.due_date = next_due;
    progress.review_count += 1;

    let updated = learning::update_progress(&state.pool, &progress).await?;
    Ok(progress_to_view(&updated))
}

pub async fn get_progress_map(state: &AppState) -> Result<LearningProgressMapResponse> {
    let list = learning::list_all_progress(&state.pool).await?;
    let progress = list
        .into_iter()
        .map(|item| (item.entry_id, progress_to_view(&item)))
        .collect::<HashMap<_, _>>();
    Ok(LearningProgressMapResponse { progress })
}

pub async fn get_stats(state: &AppState) -> Result<LearningStatsResponse> {
    let list = learning::list_all_progress(&state.pool).await?;
    let total_words = list.len() as i64;
    let due_today = list
        .iter()
        .filter(|item| item.due_date <= Utc::now())
        .count() as i64;
    let average_stability = if list.is_empty() {
        0.0
    } else {
        list.iter().map(|item| item.stability).sum::<f32>() / list.len() as f32
    };

    Ok(LearningStatsResponse {
        total_words,
        due_today,
        average_stability,
    })
}

fn progress_to_view(progress: &crate::models::LearningProgress) -> LearningProgressView {
    LearningProgressView {
        entry_id: progress.entry_id,
        review_count: progress.review_count,
        next_review_at: progress.due_date,
        last_reviewed_at: progress.last_review_at,
        scheduled_days: progress.scheduled_days,
        stability: progress.stability,
        difficulty: progress.difficulty,
        state: progress.state,
    }
}

fn estimate_repetitions_left(progress: &crate::models::LearningProgress) -> i32 {
    if progress.review_count == 0 {
        1
    } else if progress.scheduled_days <= 1 {
        2
    } else {
        1
    }
}

fn map_quality_to_rating(quality: i32) -> i32 {
    match quality {
        i32::MIN..=1 => 1,
        2 => 2,
        3 | 4 => 3,
        _ => 4,
    }
}

fn next_state_value(quality: i32, scheduled_days: i64) -> i32 {
    if quality <= 1 {
        3
    } else if scheduled_days <= 1 {
        1
    } else {
        2
    }
}
