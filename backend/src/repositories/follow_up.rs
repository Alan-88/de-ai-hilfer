use crate::db::DbPool;
use crate::models::FollowUp;

pub async fn get(pool: &DbPool, id: i64) -> Result<Option<FollowUp>, sqlx::Error> {
    sqlx::query_as("SELECT id,entry_id,question,answer,created_at FROM follow_ups WHERE id=$1")
        .bind(id)
        .fetch_optional(pool)
        .await
}

pub async fn delete(pool: &DbPool, id: i64) -> Result<bool, sqlx::Error> {
    Ok(sqlx::query("DELETE FROM follow_ups WHERE id=$1")
        .bind(id)
        .execute(pool)
        .await?
        .rows_affected()
        > 0)
}

pub async fn replace(
    connection: &mut sqlx::PgConnection,
    original: &FollowUp,
    question: &str,
    answer: &str,
) -> Result<Option<FollowUp>, sqlx::Error> {
    sqlx::query_as("UPDATE follow_ups SET question=$2,answer=$3 WHERE id=$1 AND question=$4 AND answer=$5 RETURNING id,entry_id,question,answer,created_at")
        .bind(original.id).bind(question).bind(answer).bind(&original.question).bind(&original.answer)
        .fetch_optional(connection).await
}

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

pub async fn insert(
    connection: &mut sqlx::PgConnection,
    entry_id: i64,
    question: &str,
    answer: &str,
) -> Result<FollowUp, sqlx::Error> {
    sqlx::query_as::<_, FollowUp>(
        r#"
        INSERT INTO follow_ups (entry_id, question, answer)
        VALUES ($1, $2, $3)
        RETURNING id, entry_id, question, answer, created_at
        "#,
    )
    .bind(entry_id)
    .bind(question)
    .bind(answer)
    .fetch_one(connection)
    .await
}

pub async fn list_items_by_entry_id(
    pool: &DbPool,
    entry_id: i64,
) -> Result<Vec<crate::models::FollowUpItem>, sqlx::Error> {
    Ok(list_by_entry_id(pool, entry_id)
        .await?
        .into_iter()
        .map(|item| crate::models::FollowUpItem {
            id: item.id,
            question: item.question,
            answer: item.answer,
            created_at: item.created_at,
        })
        .collect())
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
