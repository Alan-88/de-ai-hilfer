use crate::db::DbPool;
use crate::models::{KnowledgeEntry, LearningProgress, NewLearningProgress};
use chrono::{DateTime, Utc};

pub async fn find_progress(
    pool: &DbPool,
    entry_id: i64,
) -> Result<Option<LearningProgress>, sqlx::Error> {
    sqlx::query_as::<_, LearningProgress>(
        r#"
        SELECT entry_id, stability, difficulty, elapsed_days, scheduled_days, state, last_review_at, due_date, review_count
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
        RETURNING entry_id, stability, difficulty, elapsed_days, scheduled_days, state, last_review_at, due_date, review_count
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
            review_count = $9
        WHERE entry_id = $1
        RETURNING entry_id, stability, difficulty, elapsed_days, scheduled_days, state, last_review_at, due_date, review_count
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
    .fetch_one(pool)
    .await
}

pub async fn list_due_entries(
    pool: &DbPool,
    now: DateTime<Utc>,
) -> Result<Vec<(LearningProgress, KnowledgeEntry)>, sqlx::Error> {
    let rows = sqlx::query!(
        r#"
        SELECT
            lp.entry_id as "lp_entry_id!",
            lp.stability as "lp_stability!",
            lp.difficulty as "lp_difficulty!",
            lp.elapsed_days as "lp_elapsed_days!",
            lp.scheduled_days as "lp_scheduled_days!",
            lp.state as "lp_state!",
            lp.last_review_at as "lp_last_review_at?",
            lp.due_date as "lp_due_date!",
            lp.review_count as "lp_review_count!",
            ke.id as "ke_id!",
            ke.query_text as "ke_query_text!",
            ke.prototype as "ke_prototype?",
            ke.entry_type as "ke_entry_type!",
            ke.analysis as "ke_analysis!",
            ke.tags as "ke_tags?",
            ke.aliases as "ke_aliases?",
            ke.created_at as "ke_created_at!",
            ke.updated_at as "ke_updated_at!"
        FROM learning_progress lp
        JOIN knowledge_entries ke ON ke.id = lp.entry_id
        WHERE lp.due_date <= $1
        ORDER BY lp.due_date ASC, ke.updated_at DESC
        "#,
        now
    )
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|row| {
            (
                LearningProgress {
                    entry_id: row.lp_entry_id,
                    stability: row.lp_stability,
                    difficulty: row.lp_difficulty,
                    elapsed_days: row.lp_elapsed_days,
                    scheduled_days: row.lp_scheduled_days,
                    state: row.lp_state,
                    last_review_at: row.lp_last_review_at,
                    due_date: row.lp_due_date,
                    review_count: row.lp_review_count,
                },
                KnowledgeEntry {
                    id: row.ke_id,
                    query_text: row.ke_query_text,
                    prototype: row.ke_prototype,
                    entry_type: row.ke_entry_type,
                    analysis: row.ke_analysis,
                    tags: row.ke_tags,
                    aliases: row.ke_aliases,
                    created_at: row.ke_created_at,
                    updated_at: row.ke_updated_at,
                },
            )
        })
        .collect())
}

pub async fn list_all_progress(pool: &DbPool) -> Result<Vec<LearningProgress>, sqlx::Error> {
    sqlx::query_as::<_, LearningProgress>(
        r#"
        SELECT entry_id, stability, difficulty, elapsed_days, scheduled_days, state, last_review_at, due_date, review_count
        FROM learning_progress
        ORDER BY due_date ASC
        "#,
    )
    .fetch_all(pool)
    .await
}
