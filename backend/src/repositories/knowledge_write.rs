use crate::db::DbPool;
use crate::models::{KnowledgeEntry, NewKnowledgeEntry};

pub async fn insert(
    pool: &DbPool,
    entry: &NewKnowledgeEntry,
) -> Result<KnowledgeEntry, sqlx::Error> {
    sqlx::query_as::<_, KnowledgeEntry>(
        r#"
        INSERT INTO knowledge_entries (query_text, lexeme_id, prototype, entry_type, analysis, tags, aliases)
        VALUES (
            $1,
            $2,
            (SELECT headword FROM dictionary_raw WHERE headword = $3),
            $4,
            $5,
            $6,
            $7
        )
        RETURNING id, query_text, lexeme_id, prototype, entry_type, analysis, tags, aliases, created_at, updated_at
        "#,
    )
    .bind(&entry.query_text)
    .bind(&entry.lexeme_id)
    .bind(&entry.prototype)
    .bind(&entry.entry_type)
    .bind(&entry.analysis)
    .bind(&entry.tags)
    .bind(&entry.aliases)
    .fetch_one(pool)
    .await
}

pub async fn update_analysis(
    pool: &DbPool,
    entry_id: i64,
    lexeme_id: Option<i64>,
    analysis: &serde_json::Value,
    tags: &Option<Vec<String>>,
    aliases: &Option<Vec<String>>,
) -> Result<KnowledgeEntry, sqlx::Error> {
    sqlx::query_as::<_, KnowledgeEntry>(
        r#"
        UPDATE knowledge_entries
        SET lexeme_id = COALESCE($2, lexeme_id),
            analysis = $3,
            tags = $4,
            aliases = $5
        WHERE id = $1
        RETURNING id, query_text, lexeme_id, prototype, entry_type, analysis, tags, aliases, created_at, updated_at
        "#,
    )
    .bind(entry_id)
    .bind(lexeme_id)
    .bind(analysis)
    .bind(tags)
    .bind(aliases)
    .fetch_one(pool)
    .await
}

pub async fn add_alias(pool: &DbPool, entry_id: i64, alias: &str) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        UPDATE knowledge_entries
        SET aliases = CASE
            WHEN aliases IS NULL THEN ARRAY[$2]::TEXT[]
            WHEN NOT ($2 = ANY(aliases)) THEN array_append(aliases, $2)
            ELSE aliases
        END
        WHERE id = $1
        "#,
    )
    .bind(entry_id)
    .bind(alias)
    .execute(pool)
    .await?;

    Ok(())
}
