use crate::db::DbPool;
use crate::models::{FollowUp, NewFollowUp};

pub async fn list_by_entry_id(pool: &DbPool, entry_id: i64) -> Result<Vec<FollowUp>, sqlx::Error> {
    sqlx::query_as::<_, FollowUp>(
        r#"
        SELECT id, entry_id, question, answer, created_at
        FROM follow_ups
        WHERE entry_id = $1
        ORDER BY created_at ASC
        "#,
    )
    .bind(entry_id)
    .fetch_all(pool)
    .await
}

pub async fn insert(pool: &DbPool, follow_up: &NewFollowUp) -> Result<FollowUp, sqlx::Error> {
    sqlx::query_as::<_, FollowUp>(
        r#"
        INSERT INTO follow_ups (entry_id, question, answer)
        VALUES ($1, $2, $3)
        RETURNING id, entry_id, question, answer, created_at
        "#,
    )
    .bind(follow_up.entry_id)
    .bind(&follow_up.question)
    .bind(&follow_up.answer)
    .fetch_one(pool)
    .await
}

pub async fn list_all(pool: &DbPool) -> Result<Vec<FollowUp>, sqlx::Error> {
    sqlx::query_as::<_, FollowUp>(
        r#"
        SELECT id, entry_id, question, answer, created_at
        FROM follow_ups
        ORDER BY entry_id ASC, created_at ASC
        "#,
    )
    .fetch_all(pool)
    .await
}
