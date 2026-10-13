use crate::db::DbPool;
use chrono::{DateTime, Utc};
use serde_json::Value;
use sqlx::FromRow;

#[derive(Debug, Clone, FromRow)]
pub struct AgentLexemeStateRow {
    pub lexeme_id: i64,
    pub surface: String,
    pub pos_summary: Vec<String>,
    pub gloss_preview: Value,
    pub entry_id: Option<i64>,
    pub query_text: Option<String>,
    pub learning_state: Option<i32>,
    pub due_date: Option<DateTime<Utc>>,
    pub review_count: Option<i32>,
    pub lapses: Option<i32>,
}

#[derive(Debug, Clone, FromRow)]
pub struct AgentLearningRow {
    pub entry_id: i64,
    pub lexeme_id: Option<i64>,
    pub query_text: String,
    pub learning_state: i32,
    pub due_date: DateTime<Utc>,
    pub review_count: i32,
    pub lapses: i32,
}

pub async fn find_lexeme_state(
    pool: &DbPool,
    lexeme_id: i64,
) -> Result<Option<AgentLexemeStateRow>, sqlx::Error> {
    sqlx::query_as::<_, AgentLexemeStateRow>(
        r#"
        SELECT
            dl.id AS lexeme_id,
            dl.surface,
            dl.pos_summary,
            dl.gloss_preview,
            ke.id AS entry_id,
            ke.query_text,
            lp.state AS learning_state,
            lp.due_date,
            lp.review_count,
            lp.lapses
        FROM dictionary_lexemes dl
        LEFT JOIN LATERAL (
            SELECT id, query_text
            FROM knowledge_entries
            WHERE lexeme_id = dl.id
            ORDER BY updated_at DESC, id DESC
            LIMIT 1
        ) ke ON TRUE
        LEFT JOIN learning_progress lp ON lp.entry_id = ke.id
        WHERE dl.id = $1
        "#,
    )
    .bind(lexeme_id)
    .fetch_optional(pool)
    .await
}

pub async fn list_learning(pool: &DbPool) -> Result<Vec<AgentLearningRow>, sqlx::Error> {
    sqlx::query_as::<_, AgentLearningRow>(
        r#"
        SELECT
            ke.id AS entry_id,
            ke.lexeme_id,
            ke.query_text,
            lp.state AS learning_state,
            lp.due_date,
            lp.review_count,
            lp.lapses
        FROM learning_progress lp
        JOIN knowledge_entries ke ON ke.id = lp.entry_id
        ORDER BY lp.due_date ASC, ke.query_text ASC
        "#,
    )
    .fetch_all(pool)
    .await
}

pub async fn remove_learning<'e>(
    executor: impl sqlx::Executor<'e, Database = sqlx::Postgres>,
    entry_id: i64,
) -> Result<bool, sqlx::Error> {
    Ok(
        sqlx::query("DELETE FROM learning_progress WHERE entry_id = $1")
            .bind(entry_id)
            .execute(executor)
            .await?
            .rows_affected()
            > 0,
    )
}

pub async fn reset_learning<'e>(
    executor: impl sqlx::Executor<'e, Database = sqlx::Postgres>,
    entry_id: i64,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        INSERT INTO learning_progress (entry_id)
        VALUES ($1)
        ON CONFLICT (entry_id) DO UPDATE SET
            stability = 0,
            difficulty = 0,
            elapsed_days = 0,
            scheduled_days = 0,
            state = 0,
            last_review_at = NULL,
            due_date = NOW(),
            review_count = 0,
            lapses = 0
        "#,
    )
    .bind(entry_id)
    .execute(executor)
    .await?;
    Ok(())
}
