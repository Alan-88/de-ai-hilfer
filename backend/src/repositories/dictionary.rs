use crate::db::DbPool;
use crate::models::DictionaryRaw;
use crate::repositories::dictionary_merge::{
    merge_preferred_raw_rows, merge_raw_rows, DictionaryRawLookupRow,
};

pub async fn find_by_headword(
    pool: &DbPool,
    headword: &str,
) -> Result<Option<DictionaryRaw>, sqlx::Error> {
    let rows = sqlx::query_as::<_, DictionaryRawLookupRow>(
        r#"
        SELECT
            headword,
            raw_data,
            has_audio,
            created_at,
            is_form_of,
            pos
        FROM dictionary_raw_entries
        WHERE lower(headword) = lower($1)
        ORDER BY CASE WHEN headword = $1 THEN 0 ELSE 1 END, headword ASC
        "#,
    )
    .bind(headword)
    .fetch_all(pool)
    .await?;

    Ok(merge_preferred_raw_rows(rows))
}

pub async fn find_by_form(pool: &DbPool, form: &str) -> Result<Option<DictionaryRaw>, sqlx::Error> {
    let rows = sqlx::query_as::<_, DictionaryRawLookupRow>(
        r#"
        WITH ranked_lexeme AS (
            SELECT
                dl.id AS lexeme_id,
                CASE WHEN dsf.surface = $1 THEN 0 ELSE 1 END AS case_rank,
                CASE dsf.source
                    WHEN 'headword' THEN 0
                    WHEN 'form_of' THEN 1
                    WHEN 'form' THEN 2
                    WHEN 'alias' THEN 3
                    ELSE 9
                END AS source_rank,
                CASE
                    WHEN dl.surface = $1 THEN 0
                    WHEN lower(dl.surface) = lower($1) THEN 1
                    ELSE 2
                END AS surface_rank,
                dsf.confidence,
                dl.surface
            FROM dictionary_surface_forms dsf
            JOIN dictionary_lexemes dl
              ON dl.id = dsf.lexeme_id
            WHERE lower(dsf.surface) = lower($1)
            ORDER BY
                case_rank ASC,
                source_rank ASC,
                surface_rank ASC,
                dsf.confidence DESC,
                dl.surface ASC
            LIMIT 1
        )
        SELECT
            dre.headword,
            dre.raw_data,
            dre.has_audio,
            dre.created_at,
            dre.is_form_of,
            dre.pos
        FROM ranked_lexeme rl
        JOIN dictionary_lexeme_raw_entries dlre
          ON dlre.lexeme_id = rl.lexeme_id
        JOIN dictionary_raw_entries dre
          ON dre.id = dlre.raw_entry_id
        ORDER BY
            dre.is_form_of ASC,
            COALESCE(dre.pos, '') ASC,
            dre.id ASC
        "#,
    )
    .bind(form)
    .fetch_all(pool)
    .await?;

    Ok(merge_raw_rows(rows))
}

pub async fn list_fuzzy_headwords(
    pool: &DbPool,
    query: &str,
    limit: i64,
) -> Result<Vec<String>, sqlx::Error> {
    let normalized = query.trim().to_lowercase();
    let query_len = normalized.chars().count();
    let preferred_prefix_len = if query_len >= 5 { 3 } else { 2 };
    let prefix_len = normalized.chars().take(preferred_prefix_len).count().max(1);
    let prefix = normalized.chars().take(prefix_len).collect::<String>();

    sqlx::query_scalar::<_, String>(
        r#"
        SELECT headword
        FROM dictionary_raw
        WHERE replace(translate(lower(headword), 'äöü', 'aou'), 'ß', 'ss') LIKE $1
          AND abs(char_length(headword) - $2) <= 3
        ORDER BY abs(char_length(headword) - $2) ASC, headword ASC
        LIMIT $3
        "#,
    )
    .bind(format!("{prefix}%"))
    .bind(normalized.chars().count() as i32)
    .bind(limit)
    .fetch_all(pool)
    .await
}

pub async fn find_prefix_matches(
    pool: &DbPool,
    query: &str,
    limit: i64,
) -> Result<Vec<DictionaryRaw>, sqlx::Error> {
    sqlx::query_as::<_, DictionaryRaw>(
        r#"
        SELECT headword, raw_data, has_audio, created_at
        FROM dictionary_raw
        WHERE replace(translate(lower(headword), 'äöü', 'aou'), 'ß', 'ss') LIKE $1
        ORDER BY
            CASE WHEN headword = $2 THEN 0 ELSE 1 END,
            CASE
                WHEN substring(headword from '^[[:alpha:]]') = lower(substring(headword from '^[[:alpha:]]'))
                 AND substring($2 from '^[[:alpha:]]') = lower(substring($2 from '^[[:alpha:]]'))
                THEN 0
                WHEN substring(headword from '^[[:alpha:]]') = upper(substring(headword from '^[[:alpha:]]'))
                 AND substring($2 from '^[[:alpha:]]') = upper(substring($2 from '^[[:alpha:]]'))
                THEN 0
                ELSE 1
            END,
            headword ASC
        LIMIT $3
        "#,
    )
    .bind(format!("{}%", query.trim().to_lowercase()))
    .bind(query.trim())
    .bind(limit)
    .fetch_all(pool)
    .await
}

pub async fn list_fuzzy_matches(
    pool: &DbPool,
    query: &str,
    limit: i64,
) -> Result<Vec<DictionaryRaw>, sqlx::Error> {
    let normalized = query.trim().to_lowercase();
    let query_len = normalized.chars().count();
    let prefix_len = if query_len >= 5 { 2 } else { 1 };
    let prefix = normalized
        .chars()
        .take(prefix_len.max(1))
        .collect::<String>();

    sqlx::query_as::<_, DictionaryRaw>(
        r#"
        SELECT headword, raw_data, has_audio, created_at
        FROM dictionary_raw
        WHERE replace(translate(lower(headword), 'äöü', 'aou'), 'ß', 'ss') LIKE $1
          AND abs(char_length(headword) - $2) <= 3
        ORDER BY
            CASE WHEN headword = $4 THEN 0 ELSE 1 END,
            abs(char_length(headword) - $2) ASC,
            CASE
                WHEN substring(headword from '^[[:alpha:]]') = lower(substring(headword from '^[[:alpha:]]'))
                 AND substring($4 from '^[[:alpha:]]') = lower(substring($4 from '^[[:alpha:]]'))
                THEN 0
                WHEN substring(headword from '^[[:alpha:]]') = upper(substring(headword from '^[[:alpha:]]'))
                 AND substring($4 from '^[[:alpha:]]') = upper(substring($4 from '^[[:alpha:]]'))
                THEN 0
                ELSE 1
            END,
            headword ASC
        LIMIT $3
        "#,
    )
    .bind(format!("{prefix}%"))
    .bind(query_len as i32)
    .bind(limit)
    .bind(query.trim())
    .fetch_all(pool)
    .await
}
