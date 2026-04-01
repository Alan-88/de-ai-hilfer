use crate::db::DbPool;
use crate::models::{KnowledgeEntry, LibraryQueryTab, NewKnowledgeEntry};
use sqlx::{FromRow, Postgres, QueryBuilder};

#[derive(Debug, FromRow)]
struct AliasMatchRow {
    id: i64,
    query_text: String,
    prototype: Option<String>,
    entry_type: String,
    analysis: serde_json::Value,
    tags: Option<Vec<String>>,
    aliases: Option<Vec<String>>,
    created_at: chrono::DateTime<chrono::Utc>,
    updated_at: chrono::DateTime<chrono::Utc>,
    alias_text: String,
}

pub struct KnowledgeListPage {
    pub entries: Vec<KnowledgeEntry>,
    pub total: i64,
    pub next_offset: Option<i64>,
}

const NORMALIZED_QUERY_TEXT_SQL: &str =
    "replace(translate(lower(ke.query_text), 'äöü', 'aou'), 'ß', 'ss')";

pub async fn find_by_query(
    pool: &DbPool,
    query: &str,
) -> Result<Option<KnowledgeEntry>, sqlx::Error> {
    sqlx::query_as::<_, KnowledgeEntry>(
        r#"
        SELECT id, query_text, prototype, entry_type, analysis, tags, aliases, created_at, updated_at
        FROM knowledge_entries
        WHERE query_text = $1 OR $1 = ANY(COALESCE(aliases, ARRAY[]::TEXT[]))
        ORDER BY created_at ASC
        LIMIT 1
        "#,
    )
    .bind(query)
    .fetch_optional(pool)
    .await
}

pub async fn find_by_query_text_exact(
    pool: &DbPool,
    query: &str,
) -> Result<Option<KnowledgeEntry>, sqlx::Error> {
    sqlx::query_as::<_, KnowledgeEntry>(
        r#"
        SELECT id, query_text, prototype, entry_type, analysis, tags, aliases, created_at, updated_at
        FROM knowledge_entries
        WHERE query_text = $1
        ORDER BY created_at ASC
        LIMIT 1
        "#,
    )
    .bind(query)
    .fetch_optional(pool)
    .await
}

pub async fn insert(
    pool: &DbPool,
    entry: &NewKnowledgeEntry,
) -> Result<KnowledgeEntry, sqlx::Error> {
    sqlx::query_as::<_, KnowledgeEntry>(
        r#"
        INSERT INTO knowledge_entries (query_text, prototype, entry_type, analysis, tags, aliases)
        VALUES ($1, $2, $3, $4, $5, $6)
        RETURNING id, query_text, prototype, entry_type, analysis, tags, aliases, created_at, updated_at
        "#,
    )
    .bind(&entry.query_text)
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
    analysis: &serde_json::Value,
    tags: &Option<Vec<String>>,
    aliases: &Option<Vec<String>>,
) -> Result<KnowledgeEntry, sqlx::Error> {
    sqlx::query_as::<_, KnowledgeEntry>(
        r#"
        UPDATE knowledge_entries
        SET analysis = $2,
            tags = $3,
            aliases = $4
        WHERE id = $1
        RETURNING id, query_text, prototype, entry_type, analysis, tags, aliases, created_at, updated_at
        "#,
    )
    .bind(entry_id)
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

pub async fn find_by_id(
    pool: &DbPool,
    entry_id: i64,
) -> Result<Option<KnowledgeEntry>, sqlx::Error> {
    sqlx::query_as::<_, KnowledgeEntry>(
        r#"
        SELECT id, query_text, prototype, entry_type, analysis, tags, aliases, created_at, updated_at
        FROM knowledge_entries
        WHERE id = $1
        "#,
    )
    .bind(entry_id)
    .fetch_optional(pool)
    .await
}

pub async fn delete_by_id(pool: &DbPool, entry_id: i64) -> Result<u64, sqlx::Error> {
    let result = sqlx::query(
        r#"
        DELETE FROM knowledge_entries
        WHERE id = $1
        "#,
    )
    .bind(entry_id)
    .execute(pool)
    .await?;

    Ok(result.rows_affected())
}

pub async fn list_query_texts(pool: &DbPool, limit: i64) -> Result<Vec<String>, sqlx::Error> {
    sqlx::query_scalar::<_, String>(
        r#"
        SELECT query_text
        FROM knowledge_entries
        WHERE entry_type <> 'PHRASE'
        ORDER BY updated_at DESC
        LIMIT $1
        "#,
    )
    .bind(limit)
    .fetch_all(pool)
    .await
}

pub async fn list_by_query_texts(
    pool: &DbPool,
    queries: &[String],
) -> Result<Vec<KnowledgeEntry>, sqlx::Error> {
    sqlx::query_as::<_, KnowledgeEntry>(
        r#"
        SELECT id, query_text, prototype, entry_type, analysis, tags, aliases, created_at, updated_at
        FROM knowledge_entries
        WHERE query_text = ANY($1)
          AND entry_type <> 'PHRASE'
        "#,
    )
    .bind(queries)
    .fetch_all(pool)
    .await
}

pub async fn list_all(pool: &DbPool) -> Result<Vec<KnowledgeEntry>, sqlx::Error> {
    sqlx::query_as::<_, KnowledgeEntry>(
        r#"
        SELECT id, query_text, prototype, entry_type, analysis, tags, aliases, created_at, updated_at
        FROM knowledge_entries
        WHERE entry_type <> 'PHRASE'
        ORDER BY query_text ASC
        "#,
    )
    .fetch_all(pool)
    .await
}

pub async fn list_page(
    pool: &DbPool,
    query: &str,
    tab: LibraryQueryTab,
    limit: i64,
    offset: i64,
) -> Result<KnowledgeListPage, sqlx::Error> {
    let query_lower = query.trim().to_lowercase();
    let normalized_query = normalize_library_query(query);
    let limit = limit.max(1);
    let offset = offset.max(0);

    let mut count_query = QueryBuilder::<Postgres>::new("SELECT COUNT(*)");
    push_library_filters(&mut count_query, &query_lower, &normalized_query, tab);
    let total = count_query
        .build_query_scalar::<i64>()
        .fetch_one(pool)
        .await?;

    let mut list_query = QueryBuilder::<Postgres>::new(
        "SELECT ke.id, ke.query_text, ke.prototype, ke.entry_type, ke.analysis, ke.tags, ke.aliases, ke.created_at, ke.updated_at",
    );
    push_library_filters(&mut list_query, &query_lower, &normalized_query, tab);
    push_library_ordering(&mut list_query, &query_lower, &normalized_query);
    list_query.push(" LIMIT ");
    list_query.push_bind(limit);
    list_query.push(" OFFSET ");
    list_query.push_bind(offset);

    let entries = list_query
        .build_query_as::<KnowledgeEntry>()
        .fetch_all(pool)
        .await?;
    let next_offset = if offset + (entries.len() as i64) < total {
        Some(offset + entries.len() as i64)
    } else {
        None
    };

    Ok(KnowledgeListPage {
        entries,
        total,
        next_offset,
    })
}

pub async fn find_prefix_matches(
    pool: &DbPool,
    query: &str,
    limit: i64,
) -> Result<Vec<KnowledgeEntry>, sqlx::Error> {
    sqlx::query_as::<_, KnowledgeEntry>(
        r#"
        SELECT id, query_text, prototype, entry_type, analysis, tags, aliases, created_at, updated_at
        FROM knowledge_entries
        WHERE replace(translate(lower(query_text), 'äöü', 'aou'), 'ß', 'ss') LIKE $1
        ORDER BY query_text ASC
        LIMIT $2
        "#,
    )
    .bind(format!("{}%", query.trim().to_lowercase()))
    .bind(limit)
    .fetch_all(pool)
    .await
}

pub async fn find_alias_prefix_matches(
    pool: &DbPool,
    query: &str,
    limit: i64,
) -> Result<Vec<(KnowledgeEntry, String)>, sqlx::Error> {
    let rows = sqlx::query_as::<_, AliasMatchRow>(
        r#"
        SELECT
            id,
            query_text,
            prototype,
            entry_type,
            analysis,
            tags,
            aliases,
            created_at,
            updated_at,
            alias_text
        FROM (
            SELECT
                ke.id,
                ke.query_text,
                ke.prototype,
                ke.entry_type,
                ke.analysis,
                ke.tags,
                ke.aliases,
                ke.created_at,
                ke.updated_at,
                alias_text
            FROM knowledge_entries ke
            CROSS JOIN LATERAL unnest(COALESCE(ke.aliases, ARRAY[]::TEXT[])) AS alias_text
            WHERE replace(translate(lower(alias_text), 'äöü', 'aou'), 'ß', 'ss') LIKE $1
            ORDER BY alias_text ASC
            LIMIT $2
        ) matched_aliases
        "#,
    )
    .bind(format!("{}%", query.trim().to_lowercase()))
    .bind(limit)
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|row| {
            (
                KnowledgeEntry {
                    id: row.id,
                    query_text: row.query_text,
                    prototype: row.prototype,
                    entry_type: row.entry_type,
                    analysis: row.analysis,
                    tags: row.tags,
                    aliases: row.aliases,
                    created_at: row.created_at,
                    updated_at: row.updated_at,
                },
                row.alias_text,
            )
        })
        .collect())
}

pub async fn list_fuzzy_matches(
    pool: &DbPool,
    query: &str,
    limit: i64,
) -> Result<Vec<KnowledgeEntry>, sqlx::Error> {
    let normalized = query.trim().to_lowercase();
    let query_len = normalized.chars().count();
    let prefix_len = if query_len >= 5 { 2 } else { 1 };
    let prefix = normalized
        .chars()
        .take(prefix_len.max(1))
        .collect::<String>();

    sqlx::query_as::<_, KnowledgeEntry>(
        r#"
        SELECT id, query_text, prototype, entry_type, analysis, tags, aliases, created_at, updated_at
        FROM knowledge_entries
        WHERE replace(translate(lower(query_text), 'äöü', 'aou'), 'ß', 'ss') LIKE $1
          AND abs(char_length(query_text) - $2) <= 3
        ORDER BY abs(char_length(query_text) - $2) ASC, query_text ASC
        LIMIT $3
        "#,
    )
    .bind(format!("{prefix}%"))
    .bind(query_len as i32)
    .bind(limit)
    .fetch_all(pool)
    .await
}

pub async fn list_alias_fuzzy_matches(
    pool: &DbPool,
    query: &str,
    limit: i64,
) -> Result<Vec<(KnowledgeEntry, String)>, sqlx::Error> {
    let normalized = query.trim().to_lowercase();
    let query_len = normalized.chars().count();
    let prefix_len = if query_len >= 5 { 2 } else { 1 };
    let prefix = normalized
        .chars()
        .take(prefix_len.max(1))
        .collect::<String>();

    let rows = sqlx::query_as::<_, AliasMatchRow>(
        r#"
        SELECT
            id,
            query_text,
            prototype,
            entry_type,
            analysis,
            tags,
            aliases,
            created_at,
            updated_at,
            alias_text
        FROM (
            SELECT
                ke.id,
                ke.query_text,
                ke.prototype,
                ke.entry_type,
                ke.analysis,
                ke.tags,
                ke.aliases,
                ke.created_at,
                ke.updated_at,
                alias_text
            FROM knowledge_entries ke
            CROSS JOIN LATERAL unnest(COALESCE(ke.aliases, ARRAY[]::TEXT[])) AS alias_text
            WHERE replace(translate(lower(alias_text), 'äöü', 'aou'), 'ß', 'ss') LIKE $1
              AND abs(char_length(alias_text) - $2) <= 3
            ORDER BY abs(char_length(alias_text) - $2) ASC, alias_text ASC
            LIMIT $3
        ) matched_aliases
        "#,
    )
    .bind(format!("{prefix}%"))
    .bind(query_len as i32)
    .bind(limit)
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|row| {
            (
                KnowledgeEntry {
                    id: row.id,
                    query_text: row.query_text,
                    prototype: row.prototype,
                    entry_type: row.entry_type,
                    analysis: row.analysis,
                    tags: row.tags,
                    aliases: row.aliases,
                    created_at: row.created_at,
                    updated_at: row.updated_at,
                },
                row.alias_text,
            )
        })
        .collect())
}

fn push_library_filters(
    query: &mut QueryBuilder<'_, Postgres>,
    query_lower: &str,
    normalized_query: &str,
    tab: LibraryQueryTab,
) {
    query.push(
        " FROM knowledge_entries ke LEFT JOIN learning_progress lp ON lp.entry_id = ke.id WHERE ke.entry_type <> 'PHRASE'",
    );

    match tab {
        LibraryQueryTab::All => {}
        LibraryQueryTab::Learning => {
            query.push(" AND lp.entry_id IS NOT NULL");
        }
        LibraryQueryTab::Review => {
            query.push(" AND lp.entry_id IS NOT NULL AND lp.due_date <= NOW()");
        }
        LibraryQueryTab::New => {
            query.push(" AND lp.entry_id IS NULL");
        }
    }

    if !normalized_query.is_empty() {
        query.push(" AND (lower(ke.query_text) LIKE ");
        query.push_bind(format!("{query_lower}%"));
        query.push(" OR ");
        query.push(NORMALIZED_QUERY_TEXT_SQL);
        query.push(" LIKE ");
        query.push_bind(format!("{normalized_query}%"));
        query.push(" OR ");
        query.push(NORMALIZED_QUERY_TEXT_SQL);
        query.push(" LIKE ");
        query.push_bind(format!("%{normalized_query}%"));
        query.push(")");
    }
}

fn push_library_ordering(
    query: &mut QueryBuilder<'_, Postgres>,
    query_lower: &str,
    normalized_query: &str,
) {
    if normalized_query.is_empty() {
        query.push(" ORDER BY ke.query_text ASC, ke.id ASC");
        return;
    }

    query.push(" ORDER BY CASE WHEN lower(ke.query_text) LIKE ");
    query.push_bind(format!("{query_lower}%"));
    query.push(" THEN 0 WHEN ");
    query.push(NORMALIZED_QUERY_TEXT_SQL);
    query.push(" LIKE ");
    query.push_bind(format!("{normalized_query}%"));
    query.push(" THEN 1 WHEN ");
    query.push(NORMALIZED_QUERY_TEXT_SQL);
    query.push(" LIKE ");
    query.push_bind(format!("%{normalized_query}%"));
    query.push(" THEN 2 ELSE 3 END, abs(char_length(");
    query.push(NORMALIZED_QUERY_TEXT_SQL);
    query.push(") - ");
    query.push_bind(normalized_query.chars().count() as i32);
    query.push("), ke.query_text ASC, ke.id ASC");
}

fn normalize_library_query(value: &str) -> String {
    value
        .trim()
        .to_lowercase()
        .replace('ä', "a")
        .replace('ö', "o")
        .replace('ü', "u")
        .replace('ß', "ss")
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric())
        .collect()
}
