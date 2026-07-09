use crate::models::{
    LearningProgress, LearningProgressMapResponse, LearningProgressView, LearningRecallRating,
    LearningSessionResponse, LearningSessionV3Response, LearningSessionWord, LearningStatsResponse,
    NewLearningProgress,
};
use crate::repositories::{knowledge, learning};
use crate::services::analysis_preview::{analysis_markdown, structured_analysis};
use crate::services::learning_session::{
    session_item, should_complete_today, LearningPhase, LearningSessionItem,
    LearningSessionRuntime, MAX_INTRADAY_APPEARANCES,
};
use crate::state::AppState;
use anyhow::{anyhow, Result};
use chrono::{DateTime, Duration, Local, NaiveDate, NaiveTime, TimeZone, Utc};
use fsrs::{MemoryState, DEFAULT_PARAMETERS, FSRS};
use std::collections::{HashMap, VecDeque};
use uuid::Uuid;

const TARGET_RETENTION: f32 = 0.9;

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
        structured_analysis: structured_analysis(&entry.analysis),
        repetitions_left: estimate_repetitions_left(&progress),
        progress: Some(progress_to_view(&progress)),
        phase: None,
        appearance_count_today: None,
    };

    Ok(LearningSessionResponse {
        current_word: Some(current_word),
        completed_count: 0,
        total_count: due_entries.len() as i32,
        is_completed: false,
    })
}

pub async fn start_session_v3(
    state: &AppState,
    limit_new_words: i64,
) -> Result<LearningSessionV3Response> {
    let now = Utc::now();
    let due_entries = learning::list_due_entries(&state.pool, now).await?;
    let mut review_items = VecDeque::new();
    let mut new_items = VecDeque::new();
    let new_limit = limit_new_words.max(0) as usize;

    for (progress, entry) in due_entries {
        if is_new_progress(&progress) {
            if new_items.len() < new_limit {
                new_items.push_back(session_item(progress, entry, LearningPhase::New, now));
            }
        } else {
            review_items.push_back(session_item(progress, entry, LearningPhase::Review, now));
        }
    }

    let session_id = Uuid::new_v4().to_string();
    let mut session = LearningSessionRuntime::new(
        session_id.clone(),
        business_date(now),
        review_items,
        new_items,
    );
    session.select_next(now);

    let response = session_to_response(&session);
    state
        .learning_sessions
        .lock()
        .await
        .insert(session_id, session);
    Ok(response)
}

pub async fn get_session_next_v3(
    state: &AppState,
    session_id: &str,
) -> Result<LearningSessionV3Response> {
    let mut sessions = state.learning_sessions.lock().await;
    let session = sessions
        .get_mut(session_id)
        .ok_or_else(|| anyhow!("learning session not found"))?;
    session.select_next(Utc::now());
    Ok(session_to_response(session))
}

pub async fn submit_review_v3(
    state: &AppState,
    session_id: &str,
    entry_id: i64,
    rating: LearningRecallRating,
) -> Result<LearningSessionV3Response> {
    let mut sessions = state.learning_sessions.lock().await;
    let session = sessions
        .get_mut(session_id)
        .ok_or_else(|| anyhow!("learning session not found"))?;
    let mut item = session
        .current
        .take()
        .ok_or_else(|| anyhow!("no active learning card"))?;

    if item.entry.id != entry_id {
        session.current = Some(item);
        return Err(anyhow!(
            "reviewed entry does not match active learning card"
        ));
    }

    let is_first_today = item.first_rating.is_none();
    if is_first_today {
        item.first_rating = Some(rating);
    }

    let phase = item.phase;
    let mut long_term_committed = false;

    if is_first_today {
        if let Err(err) = commit_long_term(state, &mut item.progress, rating).await {
            session.current = Some(item);
            return Err(err);
        }
        long_term_committed = true;
    }

    if should_complete_today(&mut item, rating, is_first_today) {
        session.completed_count += 1;
    } else {
        session.requeue_intraday(item.clone(), rating);
    }

    learning::insert_review_log(
        &state.pool,
        &session.id,
        entry_id,
        rating.as_str(),
        phase.as_str(),
        is_first_today,
        item.appearance_count_today,
        long_term_committed,
        session.business_date,
    )
    .await?;

    session.select_next(Utc::now());
    Ok(session_to_response(session))
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

    let next_states = fsrs.next_states(current_memory, TARGET_RETENTION, elapsed_days)?;
    let selected = match map_quality_to_rating(quality) {
        1 => next_states.again,
        2 => next_states.hard,
        3 => next_states.good,
        4 => next_states.easy,
        _ => unreachable!(),
    };

    let now = Utc::now();
    let scheduled_days = selected.interval.round().max(0.0) as i64;
    let next_due = next_due_date(now, scheduled_days.max(1));

    progress.stability = selected.memory.stability;
    progress.difficulty = selected.memory.difficulty;
    progress.elapsed_days = elapsed_days as i64;
    progress.scheduled_days = scheduled_days;
    progress.state = next_state_value(quality, scheduled_days);
    progress.last_review_at = Some(now);
    progress.due_date = next_due;
    progress.review_count += 1;
    if quality <= 1 && progress.review_count > 1 {
        progress.lapses += 1;
    }

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

fn session_to_response(session: &LearningSessionRuntime) -> LearningSessionV3Response {
    LearningSessionV3Response {
        session_id: session.id.clone(),
        business_date: session.business_date,
        current_word: session.current.as_ref().map(item_to_word),
        completed_count: session.completed_count,
        total_count: session.total_count,
        intraday_queue_count: session.intraday_items.len() as i32,
        is_completed: session.is_completed(),
    }
}

fn item_to_word(item: &LearningSessionItem) -> LearningSessionWord {
    LearningSessionWord {
        entry_id: item.entry.id,
        query_text: item.entry.query_text.clone(),
        analysis_markdown: analysis_markdown(&item.entry.analysis),
        structured_analysis: structured_analysis(&item.entry.analysis),
        repetitions_left: (MAX_INTRADAY_APPEARANCES - item.appearance_count_today).max(0),
        progress: Some(progress_to_view(&item.progress)),
        phase: Some(item.phase.as_str().to_string()),
        appearance_count_today: Some(item.appearance_count_today),
    }
}

async fn commit_long_term(
    state: &AppState,
    progress: &mut LearningProgress,
    first_rating: LearningRecallRating,
) -> Result<()> {
    let now = Utc::now();
    let had_prior_review = progress.review_count > 0;
    let fsrs = FSRS::new(Some(&DEFAULT_PARAMETERS))?;
    let elapsed_days = progress
        .last_review_at
        .map(|last| (now - last).num_days().max(0) as u32)
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

    let next_states = fsrs.next_states(current_memory, TARGET_RETENTION, elapsed_days)?;
    let selected = match first_rating {
        LearningRecallRating::Known => next_states.good,
        LearningRecallRating::Fuzzy => next_states.hard,
        LearningRecallRating::Forgotten => next_states.again,
    };
    let scheduled_days = selected.interval.round().max(1.0) as i64;

    progress.stability = selected.memory.stability;
    progress.difficulty = selected.memory.difficulty;
    progress.elapsed_days = elapsed_days as i64;
    progress.scheduled_days = scheduled_days;
    progress.state = long_term_state(first_rating, had_prior_review);
    progress.last_review_at = Some(now);
    progress.due_date = next_due_date(now, scheduled_days);
    progress.review_count += 1;
    if first_rating == LearningRecallRating::Forgotten && had_prior_review {
        progress.lapses += 1;
    }

    let updated = learning::update_progress(&state.pool, progress).await?;
    *progress = updated;
    Ok(())
}

fn long_term_state(first_rating: LearningRecallRating, had_prior_review: bool) -> i32 {
    if first_rating == LearningRecallRating::Forgotten && had_prior_review {
        3
    } else {
        2
    }
}

fn progress_to_view(progress: &LearningProgress) -> LearningProgressView {
    LearningProgressView {
        entry_id: progress.entry_id,
        review_count: progress.review_count,
        next_review_at: progress.due_date,
        last_reviewed_at: progress.last_review_at,
        scheduled_days: progress.scheduled_days,
        stability: progress.stability,
        difficulty: progress.difficulty,
        state: progress.state,
        lapses: progress.lapses,
    }
}

fn estimate_repetitions_left(progress: &LearningProgress) -> i32 {
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

fn is_new_progress(progress: &LearningProgress) -> bool {
    progress.review_count == 0 || progress.state == 0
}

fn business_date(now: DateTime<Utc>) -> NaiveDate {
    let local = now.with_timezone(&Local);
    let day = local.date_naive();
    if local.time() < NaiveTime::from_hms_opt(4, 0, 0).expect("valid time") {
        day.checked_sub_signed(Duration::days(1)).unwrap_or(day)
    } else {
        day
    }
}

fn next_due_date(now: DateTime<Utc>, scheduled_days: i64) -> DateTime<Utc> {
    let due_business_date = business_date(now)
        .checked_add_signed(Duration::days(scheduled_days.max(1)))
        .unwrap_or_else(|| business_date(now));
    business_day_start_utc(due_business_date)
}

fn business_day_start_utc(day: NaiveDate) -> DateTime<Utc> {
    let naive = day.and_hms_opt(4, 0, 0).expect("valid business day start");
    let local = Local
        .from_local_datetime(&naive)
        .single()
        .or_else(|| Local.from_local_datetime(&naive).earliest())
        .expect("local business day start");
    local.with_timezone(&Utc)
}

impl LearningRecallRating {
    fn as_str(self) -> &'static str {
        match self {
            LearningRecallRating::Known => "known",
            LearningRecallRating::Fuzzy => "fuzzy",
            LearningRecallRating::Forgotten => "forgotten",
        }
    }
}
