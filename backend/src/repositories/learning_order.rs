use crate::db::DbPool;
use crate::models::{DictionaryLearningOrder, NewDictionaryLearningOrder};

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct LearningOrderPreviewRow {
    pub headword: String,
    pub cefr_level: Option<String>,
    pub cefr_rank: Option<i32>,
    pub frequency_rank: Option<i32>,
    pub learning_order: Option<i32>,
    pub source: String,
}

pub async fn upsert_many(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    entries: &[NewDictionaryLearningOrder],
) -> Result<(), sqlx::Error> {
    for entry in entries {
        sqlx::query(
            r#"
            INSERT INTO dictionary_learning_order (
                headword,
                cefr_level,
                cefr_rank,
                frequency_rank,
                learning_order,
                source
            )
            VALUES ($1, $2, $3, $4, $5, $6)
            ON CONFLICT (headword)
            DO UPDATE SET
                cefr_level = EXCLUDED.cefr_level,
                cefr_rank = EXCLUDED.cefr_rank,
                frequency_rank = EXCLUDED.frequency_rank,
                learning_order = EXCLUDED.learning_order,
                source = EXCLUDED.source,
                updated_at = NOW()
            "#,
        )
        .bind(&entry.headword)
        .bind(&entry.cefr_level)
        .bind(entry.cefr_rank)
        .bind(entry.frequency_rank)
        .bind(entry.learning_order)
        .bind(&entry.source)
        .execute(&mut **tx)
        .await?;
    }

    Ok(())
}

pub async fn delete_all(tx: &mut sqlx::Transaction<'_, sqlx::Postgres>) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM dictionary_learning_order")
        .execute(&mut **tx)
        .await?;
    Ok(())
}

pub async fn count(pool: &DbPool) -> Result<i64, sqlx::Error> {
    sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM dictionary_learning_order")
        .fetch_one(pool)
        .await
}

pub async fn list_preview(
    pool: &DbPool,
    limit: i64,
) -> Result<Vec<LearningOrderPreviewRow>, sqlx::Error> {
    sqlx::query_as::<_, LearningOrderPreviewRow>(
        r#"
        SELECT
            dlo.headword,
            dlo.cefr_level,
            dlo.cefr_rank,
            COALESCE(
                dlo.frequency_rank,
                (
                    SELECT MIN(dle.frequency_rank)
                    FROM dictionary_lexemes dl
                    JOIN dictionary_lexeme_embeddings dle
                      ON dle.lexeme_id = dl.id
                    WHERE dl.surface = dlo.headword
                )
            ) AS frequency_rank,
            dlo.learning_order,
            dlo.source
        FROM dictionary_learning_order dlo
        ORDER BY
            dlo.cefr_rank ASC NULLS LAST,
            dlo.learning_order ASC NULLS LAST,
            frequency_rank ASC NULLS LAST,
            dlo.headword ASC
        LIMIT $1
        "#,
    )
    .bind(limit)
    .fetch_all(pool)
    .await
}

pub async fn list_headwords_for_prewarm(
    pool: &DbPool,
    limit: i64,
    max_cefr_rank: Option<i32>,
) -> Result<Vec<String>, sqlx::Error> {
    sqlx::query_scalar::<_, String>(
        r#"
        SELECT dlo.headword
        FROM dictionary_learning_order dlo
        LEFT JOIN knowledge_entries ke
            ON ke.query_text = dlo.headword
           AND ke.entry_type = 'WORD'
        WHERE ($2::INT IS NULL OR dlo.cefr_rank <= $2)
          AND ke.id IS NULL
        ORDER BY
            dlo.cefr_rank ASC NULLS LAST,
            dlo.learning_order ASC NULLS LAST,
            COALESCE(
                dlo.frequency_rank,
                (
                    SELECT MIN(dle.frequency_rank)
                    FROM dictionary_lexemes dl
                    JOIN dictionary_lexeme_embeddings dle
                      ON dle.lexeme_id = dl.id
                    WHERE dl.surface = dlo.headword
                )
            ) ASC NULLS LAST,
            dlo.headword ASC
        LIMIT $1
        "#,
    )
    .bind(limit)
    .bind(max_cefr_rank)
    .fetch_all(pool)
    .await
}

pub async fn get(
    pool: &DbPool,
    headword: &str,
) -> Result<Option<DictionaryLearningOrder>, sqlx::Error> {
    sqlx::query_as::<_, DictionaryLearningOrder>(
        r#"
        SELECT
            headword,
            cefr_level,
            cefr_rank,
            frequency_rank,
            learning_order,
            source,
            created_at,
            updated_at
        FROM dictionary_learning_order
        WHERE headword = $1
        "#,
    )
    .bind(headword)
    .fetch_optional(pool)
    .await
}
