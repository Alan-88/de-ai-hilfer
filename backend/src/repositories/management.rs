use crate::db::DbPool;
use crate::models::DatabaseBackupPayload;

pub async fn replace_database_snapshot(
    pool: &DbPool,
    backup: &DatabaseBackupPayload,
) -> Result<(), sqlx::Error> {
    let mut tx = pool.begin().await?;

    sqlx::query(
        "TRUNCATE TABLE learning_progress, follow_ups, knowledge_entries RESTART IDENTITY CASCADE",
    )
    .execute(&mut *tx)
    .await?;

    for entry in &backup.knowledge_entries {
        sqlx::query(
            r#"
            INSERT INTO knowledge_entries (
                id, query_text, prototype, entry_type, analysis, tags, aliases, created_at, updated_at
            )
            VALUES (
                $1, $2, (SELECT headword FROM dictionary_raw WHERE headword = $3), $4, $5, $6, $7, $8, $9
            )
            "#,
        )
        .bind(entry.id)
        .bind(&entry.query_text)
        .bind(&entry.prototype)
        .bind(&entry.entry_type)
        .bind(&entry.analysis)
        .bind(&entry.tags)
        .bind(&entry.aliases)
        .bind(entry.created_at)
        .bind(entry.updated_at)
        .execute(&mut *tx)
        .await?;
    }

    for follow_up in &backup.follow_ups {
        sqlx::query(
            r#"
            INSERT INTO follow_ups (id, entry_id, question, answer, created_at)
            VALUES ($1, $2, $3, $4, $5)
            "#,
        )
        .bind(follow_up.id)
        .bind(follow_up.entry_id)
        .bind(&follow_up.question)
        .bind(&follow_up.answer)
        .bind(follow_up.created_at)
        .execute(&mut *tx)
        .await?;
    }

    for progress in &backup.learning_progress {
        sqlx::query(
            r#"
            INSERT INTO learning_progress (
                entry_id, stability, difficulty, elapsed_days, scheduled_days, state, last_review_at, due_date, review_count
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
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
        .execute(&mut *tx)
        .await?;
    }

    sqlx::query(
        r#"
        SELECT setval(
            pg_get_serial_sequence('knowledge_entries', 'id'),
            COALESCE((SELECT MAX(id) FROM knowledge_entries), 0) + 1,
            false
        )
        "#,
    )
    .execute(&mut *tx)
    .await?;

    sqlx::query(
        r#"
        SELECT setval(
            pg_get_serial_sequence('follow_ups', 'id'),
            COALESCE((SELECT MAX(id) FROM follow_ups), 0) + 1,
            false
        )
        "#,
    )
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;
    Ok(())
}
