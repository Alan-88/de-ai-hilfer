use crate::db::DbPool;
use crate::models::{KnowledgeEntry, LearningProgress, NewLearningProgress};
use chrono::{DateTime, NaiveDate, Utc};
use sqlx::Row;

pub async fn find_progress(
    pool: &DbPool,
    entry_id: i64,
) -> Result<Option<LearningProgress>, sqlx::Error> {
    sqlx::query_as::<_, LearningProgress>(
        r#"
        SELECT entry_id, stability, difficulty, elapsed_days, scheduled_days, state, last_review_at, due_date, review_count, lapses
        FROM learning_progress
        WHERE entry_id = $1
        "#,
    )
    .bind(entry_id)
    .fetch_optional(pool)
    .await
}

pub async fn insert_progress(
    pool: &DbPool,
    progress: &NewLearningProgress,
) -> Result<LearningProgress, sqlx::Error> {
    sqlx::query_as::<_, LearningProgress>(
        r#"
        INSERT INTO learning_progress (entry_id)
        VALUES ($1)
        RETURNING entry_id, stability, difficulty, elapsed_days, scheduled_days, state, last_review_at, due_date, review_count, lapses
        "#,
    )
    .bind(progress.entry_id)
    .fetch_one(pool)
    .await
}

pub async fn update_progress(
    pool: &DbPool,
    progress: &LearningProgress,
) -> Result<LearningProgress, sqlx::Error> {
    sqlx::query_as::<_, LearningProgress>(
        r#"
        UPDATE learning_progress
        SET stability = $2,
            difficulty = $3,
            elapsed_days = $4,
            scheduled_days = $5,
            state = $6,
            last_review_at = $7,
            due_date = $8,
            review_count = $9,
            lapses = $10
        WHERE entry_id = $1
        RETURNING entry_id, stability, difficulty, elapsed_days, scheduled_days, state, last_review_at, due_date, review_count, lapses
        "#,
    )
    .bind(progress.entry_id)
    .bind(progress.stability)
    .bind(progress.difficulty)
    .bind(progress.elapsed_days)
    .bind(progress.scheduled_days)
    .bind(progress.state)
    .bind(progress.last_review_at)
    .bind(progress.due_date)
    .bind(progress.review_count)
    .bind(progress.lapses)
    .fetch_one(pool)
    .await
}

pub async fn list_due_entries(
    pool: &DbPool,
    now: DateTime<Utc>,
) -> Result<Vec<(LearningProgress, KnowledgeEntry)>, sqlx::Error> {
    let rows = sqlx::query(
        r#"
        SELECT
            lp.entry_id as lp_entry_id,
            lp.stability as lp_stability,
            lp.difficulty as lp_difficulty,
            lp.elapsed_days as lp_elapsed_days,
            lp.scheduled_days as lp_scheduled_days,
            lp.state as lp_state,
            lp.last_review_at as lp_last_review_at,
            lp.due_date as lp_due_date,
            lp.review_count as lp_review_count,
            lp.lapses as lp_lapses,
            ke.id as ke_id,
            ke.query_text as ke_query_text,
            ke.lexeme_id as ke_lexeme_id,
            ke.prototype as ke_prototype,
            ke.entry_type as ke_entry_type,
            ke.analysis as ke_analysis,
            ke.tags as ke_tags,
            ke.aliases as ke_aliases,
            ke.created_at as ke_created_at,
            ke.updated_at as ke_updated_at
        FROM learning_progress lp
        JOIN knowledge_entries ke ON ke.id = lp.entry_id
        WHERE lp.due_date <= $1
        ORDER BY lp.due_date ASC, ke.updated_at DESC
        "#,
    )
    .bind(now)
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|row| {
            (
                LearningProgress {
                    entry_id: row.get("lp_entry_id"),
                    stability: row.get("lp_stability"),
                    difficulty: row.get("lp_difficulty"),
                    elapsed_days: row.get("lp_elapsed_days"),
                    scheduled_days: row.get("lp_scheduled_days"),
                    state: row.get("lp_state"),
                    last_review_at: row.get("lp_last_review_at"),
                    due_date: row.get("lp_due_date"),
                    review_count: row.get("lp_review_count"),
                    lapses: row.get("lp_lapses"),
                },
                KnowledgeEntry {
                    id: row.get("ke_id"),
                    query_text: row.get("ke_query_text"),
                    lexeme_id: row.get("ke_lexeme_id"),
                    prototype: row.get("ke_prototype"),
                    entry_type: row.get("ke_entry_type"),
                    analysis: row.get("ke_analysis"),
                    tags: row.get("ke_tags"),
                    aliases: row.get("ke_aliases"),
                    created_at: row.get("ke_created_at"),
                    updated_at: row.get("ke_updated_at"),
                },
            )
        })
        .collect())
}

pub async fn list_all_progress(pool: &DbPool) -> Result<Vec<LearningProgress>, sqlx::Error> {
    sqlx::query_as::<_, LearningProgress>(
        r#"
        SELECT entry_id, stability, difficulty, elapsed_days, scheduled_days, state, last_review_at, due_date, review_count, lapses
        FROM learning_progress
        ORDER BY due_date ASC
        "#,
    )
    .fetch_all(pool)
    .await
}

#[allow(clippy::too_many_arguments)]
pub async fn insert_review_log(
    pool: &DbPool,
    session_id: &str,
    entry_id: i64,
    rating: &str,
    phase: &str,
    is_first_today: bool,
    appearance_count_today: i32,
    long_term_committed: bool,
    business_date: NaiveDate,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        INSERT INTO learning_review_logs (
            session_id, entry_id, rating, phase, is_first_today,
            appearance_count_today, long_term_committed, business_date
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
        "#,
    )
    .bind(session_id)
    .bind(entry_id)
    .bind(rating)
    .bind(phase)
    .bind(is_first_today)
    .bind(appearance_count_today)
    .bind(long_term_committed)
    .bind(business_date)
    .execute(pool)
    .await?;

    Ok(())
}
