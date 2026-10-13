use crate::db::DbPool;
use crate::models::DatabaseBackupPayload;

pub async fn replace_database_snapshot(
    pool: &DbPool,
    backup: &DatabaseBackupPayload,
) -> Result<(), sqlx::Error> {
    let mut tx = pool.begin().await?;
    let available: bool = sqlx::query_scalar(
        "SELECT pg_try_advisory_xact_lock(hashtextextended('database-snapshot',0))",
    )
    .fetch_one(&mut *tx)
    .await?;
    if !available {
        return Err(sqlx::Error::Protocol(
            "background batch is still active; finish it before restoring a backup".into(),
        ));
    }
    let pending: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM agent_batches WHERE status IN ('queued','running'))",
    )
    .fetch_one(&mut *tx)
    .await?;
    if pending {
        return Err(sqlx::Error::Protocol(
            "background batch is still active; finish it before restoring a backup".into(),
        ));
    }
    // Operational receipts refer to the old IDs and are not part of a user-data backup.
    sqlx::query("TRUNCATE agent_batches, agent_operation_receipts, analysis_checkpoints")
        .execute(&mut *tx)
        .await?;

    sqlx::query(
        "TRUNCATE TABLE learning_review_corrections, learning_review_logs, reading_article_bookmarks, reading_articles, reading_folders, learning_progress, follow_ups, knowledge_entries RESTART IDENTITY CASCADE",
    )
    .execute(&mut *tx)
    .await?;

    for entry in &backup.knowledge_entries {
        sqlx::query(
            r#"
            INSERT INTO knowledge_entries (
                id, query_text, lexeme_id, prototype, entry_type, analysis, tags, aliases, created_at, updated_at
            )
            VALUES (
                $1, $2, $3, (SELECT headword FROM dictionary_raw WHERE headword = $4), $5, $6, $7, $8, $9, $10
            )
            "#,
        )
        .bind(entry.id)
        .bind(&entry.query_text)
        .bind(&entry.lexeme_id)
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
                entry_id, stability, difficulty, elapsed_days, scheduled_days, state, last_review_at, due_date, review_count, lapses
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
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
    }

    for log in &backup.learning_review_logs {
        sqlx::query(
            r#"
            INSERT INTO learning_review_logs (
                id, session_id, entry_id, rating, phase, is_first_today,
                appearance_count_today, long_term_committed, business_date,
                created_at, progress_before, progress_after
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
            "#,
        )
        .bind(log.id)
        .bind(&log.session_id)
        .bind(log.entry_id)
        .bind(&log.rating)
        .bind(&log.phase)
        .bind(log.is_first_today)
        .bind(log.appearance_count_today)
        .bind(log.long_term_committed)
        .bind(log.business_date)
        .bind(log.created_at)
        .bind(&log.progress_before)
        .bind(&log.progress_after)
        .execute(&mut *tx)
        .await?;
    }

    for correction in &backup.learning_review_corrections {
        sqlx::query(
            r#"
            INSERT INTO learning_review_corrections (
                id, operation_id, entry_id, business_date, target_log_id,
                action, replacement_rating, progress_before, progress_after,
                created_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            "#,
        )
        .bind(correction.id)
        .bind(&correction.operation_id)
        .bind(correction.entry_id)
        .bind(correction.business_date)
        .bind(correction.target_log_id)
        .bind(&correction.action)
        .bind(&correction.replacement_rating)
        .bind(&correction.progress_before)
        .bind(&correction.progress_after)
        .bind(correction.created_at)
        .execute(&mut *tx)
        .await?;
    }

    for folder in &backup.reading_folders {
        sqlx::query(
            r#"
            INSERT INTO reading_folders (id, parent_id, name, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5)
            "#,
        )
        .bind(folder.id)
        .bind(folder.parent_id)
        .bind(&folder.name)
        .bind(folder.created_at)
        .bind(folder.updated_at)
        .execute(&mut *tx)
        .await?;
    }

    for article in &backup.reading_articles {
        sqlx::query(
            r#"
            INSERT INTO reading_articles (
                id, folder_id, title, body, completed, last_opened_at, created_at, updated_at
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            "#,
        )
        .bind(article.id)
        .bind(article.folder_id)
        .bind(&article.title)
        .bind(&article.body)
        .bind(article.completed)
        .bind(article.last_opened_at)
        .bind(article.created_at)
        .bind(article.updated_at)
        .execute(&mut *tx)
        .await?;
    }

    for bookmark in &backup.reading_article_bookmarks {
        sqlx::query(
            r#"
            INSERT INTO reading_article_bookmarks (
                id, article_id, occurrence_index, surface, form_of, lexeme_id, entry_id
            ) VALUES ($1, $2, $3, $4, $5, $6, $7)
            "#,
        )
        .bind(bookmark.id)
        .bind(bookmark.article_id)
        .bind(bookmark.occurrence_index)
        .bind(&bookmark.surface)
        .bind(&bookmark.form_of)
        .bind(bookmark.lexeme_id)
        .bind(bookmark.entry_id)
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

    for table in [
        "learning_review_logs",
        "learning_review_corrections",
        "reading_folders",
        "reading_articles",
        "reading_article_bookmarks",
    ] {
        let statement = format!(
            "SELECT setval(pg_get_serial_sequence('{table}', 'id'), COALESCE((SELECT MAX(id) FROM {table}), 0) + 1, false)"
        );
        sqlx::query(&statement).execute(&mut *tx).await?;
    }

    tx.commit().await?;
    Ok(())
}
