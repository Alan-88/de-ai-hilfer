use crate::db::DbPool;
use crate::models::{
    KnowledgeEntry, LearningProgress, LearningReviewCorrection, LearningReviewLog,
    NewLearningProgress,
};
use chrono::{DateTime, NaiveDate, Utc};
use sqlx::Row;

#[derive(Debug, Clone)]
pub struct LearningIntradayRecovery {
    pub progress: LearningProgress,
    pub entry: KnowledgeEntry,
    pub first_rating: String,
    pub appearance_count_today: i32,
    pub success_streak: i32,
    pub next_review_at: DateTime<Utc>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct LearningReviewLogRow {
    pub id: i64,
    pub session_id: String,
    pub entry_id: i64,
    pub query_text: String,
    pub rating: String,
    pub phase: String,
    pub is_first_today: bool,
    pub appearance_count_today: i32,
    pub long_term_committed: bool,
    pub business_date: NaiveDate,
    pub created_at: DateTime<Utc>,
    pub progress_before: Option<serde_json::Value>,
    pub effective_rating: Option<String>,
    pub correction_status: String,
    pub correction_action: Option<String>,
    pub effective_progress_after: Option<serde_json::Value>,
}

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

pub async fn insert_progress<'e>(
    executor: impl sqlx::Executor<'e, Database = sqlx::Postgres>,
    progress: &NewLearningProgress,
) -> Result<LearningProgress, sqlx::Error> {
    sqlx::query_as::<_, LearningProgress>(
        r#"
        INSERT INTO learning_progress (entry_id)
        VALUES ($1)
        ON CONFLICT (entry_id) DO UPDATE SET entry_id = EXCLUDED.entry_id
        RETURNING entry_id, stability, difficulty, elapsed_days, scheduled_days, state, last_review_at, due_date, review_count, lapses
        "#,
    )
    .bind(progress.entry_id)
    .fetch_one(executor)
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

pub async fn list_intraday_recoveries(
    pool: &DbPool,
    business_date: NaiveDate,
    max_appearances: i32,
    recovery_known_delay_seconds: i64,
    known_delay_seconds: i64,
    forgotten_delay_seconds: i64,
    fuzzy_delay_seconds: i64,
) -> Result<Vec<LearningIntradayRecovery>, sqlx::Error> {
    let rows = sqlx::query(
        r#"
        WITH effective_logs AS (
            SELECT
                l.id,
                l.entry_id,
                l.created_at,
                CASE
                    WHEN c.id IS NOT NULL AND c.replacement_rating IS NULL THEN NULL
                    ELSE COALESCE(c.replacement_rating, l.rating)
                END AS rating
            FROM learning_review_logs l
            LEFT JOIN LATERAL (
                SELECT id, replacement_rating
                FROM learning_review_corrections
                WHERE target_log_id = l.id
                ORDER BY id DESC
                LIMIT 1
            ) c ON TRUE
            WHERE l.business_date = $1
        ), per_entry AS (
            SELECT
                entry_id,
                (array_agg(rating ORDER BY created_at, id))[1] AS first_rating,
                (array_agg(rating ORDER BY created_at DESC, id DESC))[1] AS last_rating,
                count(*)::INTEGER AS appearance_count_today,
                max(created_at) AS last_seen_at,
                max(created_at) FILTER (WHERE rating <> 'known') AS last_non_known_at
            FROM effective_logs
            WHERE rating IS NOT NULL
            GROUP BY entry_id
        ), recovered AS (
            SELECT
                p.entry_id,
                p.first_rating,
                p.appearance_count_today,
                count(l.id) FILTER (
                    WHERE l.rating = 'known' AND l.created_at > p.last_non_known_at
                )::INTEGER AS success_streak,
                CASE p.last_rating
                    WHEN 'known' THEN CASE p.first_rating
                        WHEN 'forgotten' THEN p.last_seen_at + ($3 * INTERVAL '1 second')
                        ELSE p.last_seen_at + ($4 * INTERVAL '1 second')
                    END
                    WHEN 'forgotten' THEN p.last_seen_at + ($5 * INTERVAL '1 second')
                    WHEN 'fuzzy' THEN p.last_seen_at + ($6 * INTERVAL '1 second')
                    ELSE p.last_seen_at + ($4 * INTERVAL '1 second')
                END AS next_review_at
            FROM per_entry p
            JOIN effective_logs l
              ON l.entry_id = p.entry_id
             AND l.rating IS NOT NULL
            GROUP BY
                p.entry_id,
                p.first_rating,
                p.appearance_count_today,
                p.last_rating,
                p.last_seen_at,
                p.last_non_known_at
        )
        SELECT
            r.entry_id,
            r.first_rating,
            r.appearance_count_today,
            r.success_streak,
            r.next_review_at,
            lp.stability AS lp_stability,
            lp.difficulty AS lp_difficulty,
            lp.elapsed_days AS lp_elapsed_days,
            lp.scheduled_days AS lp_scheduled_days,
            lp.state AS lp_state,
            lp.last_review_at AS lp_last_review_at,
            lp.due_date AS lp_due_date,
            lp.review_count AS lp_review_count,
            lp.lapses AS lp_lapses,
            ke.id AS ke_id,
            ke.query_text AS ke_query_text,
            ke.lexeme_id AS ke_lexeme_id,
            ke.prototype AS ke_prototype,
            ke.entry_type AS ke_entry_type,
            ke.analysis AS ke_analysis,
            ke.tags AS ke_tags,
            ke.aliases AS ke_aliases,
            ke.created_at AS ke_created_at,
            ke.updated_at AS ke_updated_at
        FROM recovered r
        JOIN learning_progress lp ON lp.entry_id = r.entry_id
        JOIN knowledge_entries ke ON ke.id = r.entry_id
        WHERE r.first_rating IN ('fuzzy', 'forgotten')
          AND r.appearance_count_today < $2
          AND r.success_streak < CASE r.first_rating
              WHEN 'forgotten' THEN 2
              ELSE 1
          END
        ORDER BY r.next_review_at ASC, r.entry_id ASC
        "#,
    )
    .bind(business_date)
    .bind(max_appearances)
    .bind(recovery_known_delay_seconds)
    .bind(known_delay_seconds)
    .bind(forgotten_delay_seconds)
    .bind(fuzzy_delay_seconds)
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|row| LearningIntradayRecovery {
            progress: LearningProgress {
                entry_id: row.get("entry_id"),
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
            entry: KnowledgeEntry {
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
            first_rating: row.get("first_rating"),
            appearance_count_today: row.get("appearance_count_today"),
            success_streak: row.get("success_streak"),
            next_review_at: row.get("next_review_at"),
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

pub async fn list_review_logs(
    pool: &DbPool,
    business_date: NaiveDate,
    entry_id: Option<i64>,
    session_id: Option<&str>,
    limit: i64,
) -> Result<Vec<LearningReviewLogRow>, sqlx::Error> {
    sqlx::query_as::<_, LearningReviewLogRow>(
        r#"
        SELECT
            l.id,
            l.session_id,
            l.entry_id,
            ke.query_text,
            l.rating,
            l.phase,
            l.is_first_today,
            l.appearance_count_today,
            l.long_term_committed,
            l.business_date,
            l.created_at,
            l.progress_before,
            CASE
                WHEN c.id IS NOT NULL AND c.replacement_rating IS NULL THEN NULL
                ELSE COALESCE(c.replacement_rating, l.rating)
            END AS effective_rating,
            CASE
                WHEN c.id IS NULL THEN 'active'
                WHEN c.replacement_rating IS NULL THEN 'voided'
                ELSE 'replaced'
            END AS correction_status,
            c.action AS correction_action,
            COALESCE(c.progress_after, l.progress_after) AS effective_progress_after
        FROM learning_review_logs l
        JOIN knowledge_entries ke ON ke.id = l.entry_id
        LEFT JOIN LATERAL (
            SELECT id, action, replacement_rating, progress_after
            FROM learning_review_corrections
            WHERE target_log_id = l.id
            ORDER BY id DESC
            LIMIT 1
        ) c ON TRUE
        WHERE l.business_date = $1
          AND ($2::BIGINT IS NULL OR l.entry_id = $2)
          AND ($3::TEXT IS NULL OR l.session_id = $3)
        ORDER BY l.created_at ASC, l.id ASC
        LIMIT $4
        "#,
    )
    .bind(business_date)
    .bind(entry_id)
    .bind(session_id)
    .bind(limit)
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
    progress_before: &LearningProgress,
    progress_after: &LearningProgress,
) -> Result<i64, sqlx::Error> {
    let progress_before = serde_json::to_value(progress_before).map_err(|error| {
        sqlx::Error::Protocol(format!("failed to serialize progress_before: {error}"))
    })?;
    let progress_after = serde_json::to_value(progress_after).map_err(|error| {
        sqlx::Error::Protocol(format!("failed to serialize progress_after: {error}"))
    })?;

    sqlx::query_scalar(
        r#"
        INSERT INTO learning_review_logs (
            session_id, entry_id, rating, phase, is_first_today,
            appearance_count_today, long_term_committed, business_date,
            progress_before, progress_after
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
        RETURNING id
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
    .bind(progress_before)
    .bind(progress_after)
    .fetch_one(pool)
    .await
}

pub async fn list_all_review_logs(pool: &DbPool) -> Result<Vec<LearningReviewLog>, sqlx::Error> {
    sqlx::query_as::<_, LearningReviewLog>(
        r#"
        SELECT id, session_id, entry_id, rating, phase, is_first_today,
               appearance_count_today, long_term_committed, business_date,
               created_at, progress_before, progress_after
        FROM learning_review_logs
        ORDER BY id ASC
        "#,
    )
    .fetch_all(pool)
    .await
}

pub async fn list_all_review_corrections(
    pool: &DbPool,
) -> Result<Vec<LearningReviewCorrection>, sqlx::Error> {
    sqlx::query_as::<_, LearningReviewCorrection>(
        r#"
        SELECT id, operation_id, entry_id, business_date, target_log_id,
               action, replacement_rating, progress_before, progress_after,
               created_at
        FROM learning_review_corrections
        ORDER BY id ASC
        "#,
    )
    .fetch_all(pool)
    .await
}

pub async fn insert_correction_rows(
    pool: &DbPool,
    operation_id: &str,
    entry_id: i64,
    business_date: NaiveDate,
    action: &str,
    target_log_ids: &[i64],
    replacement_rating: Option<&str>,
    progress_before: Option<&serde_json::Value>,
    progress_after: Option<&serde_json::Value>,
) -> Result<Vec<i64>, sqlx::Error> {
    let mut tx = pool.begin().await?;
    let mut ids = Vec::with_capacity(target_log_ids.len());
    for target_log_id in target_log_ids {
        let id = sqlx::query_scalar::<_, i64>(
            r#"
            INSERT INTO learning_review_corrections (
                operation_id, entry_id, business_date, target_log_id,
                action, replacement_rating, progress_before, progress_after
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            RETURNING id
            "#,
        )
        .bind(operation_id)
        .bind(entry_id)
        .bind(business_date)
        .bind(target_log_id)
        .bind(action)
        .bind(replacement_rating)
        .bind(progress_before)
        .bind(progress_after)
        .fetch_one(&mut *tx)
        .await?;
        ids.push(id);
    }
    tx.commit().await?;
    Ok(ids)
}

pub async fn update_progress_and_insert_correction(
    pool: &DbPool,
    progress: &LearningProgress,
    operation_id: &str,
    business_date: NaiveDate,
    target_log_id: i64,
    replacement_rating: &str,
    progress_before: &serde_json::Value,
    progress_after: &serde_json::Value,
) -> Result<i64, sqlx::Error> {
    let mut tx = pool.begin().await?;
    sqlx::query(
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
    .execute(&mut *tx)
    .await?;

    let correction_id = sqlx::query_scalar::<_, i64>(
        r#"
        INSERT INTO learning_review_corrections (
            operation_id, entry_id, business_date, target_log_id,
            action, replacement_rating, progress_before, progress_after
        )
        VALUES ($1, $2, $3, $4, 'first_rating_correction', $5, $6, $7)
        RETURNING id
        "#,
    )
    .bind(operation_id)
    .bind(progress.entry_id)
    .bind(business_date)
    .bind(target_log_id)
    .bind(replacement_rating)
    .bind(progress_before)
    .bind(progress_after)
    .fetch_one(&mut *tx)
    .await?;

    tx.commit().await?;
    Ok(correction_id)
}
