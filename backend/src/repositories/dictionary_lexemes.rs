use crate::db::DbPool;
use crate::models::{LexemeCandidate, NewDictionaryLexemeEmbedding};
use crate::repositories::dictionary_merge::{merge_raw_rows, DictionaryRawLookupRow};
use pgvector::Vector;
use serde_json::Value;
use std::collections::HashMap;

#[derive(Debug, Clone, sqlx::FromRow)]
struct LexemeEmbeddingSearchRow {
    lexeme_id: i64,
    surface: String,
    distance: f64,
    frequency_rank: Option<i32>,
}

#[derive(Debug, Clone)]
pub struct LexemeEmbeddingSearchHit {
    pub lexeme_id: i64,
    pub surface: String,
    pub distance: f32,
    pub frequency_rank: Option<i32>,
}

#[derive(Debug, Clone)]
pub struct PendingLexemeEmbeddingTarget {
    pub lexeme_id: i64,
    pub surface: String,
}

#[derive(Debug, Clone)]
pub struct ResolvedDictionaryLexeme {
    pub lexeme_id: i64,
    pub surface: String,
    pub raw_data: Value,
    pub has_audio: Option<bool>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

pub async fn list_pending_lexemes_for_embedding(
    pool: &DbPool,
    model_id: &str,
) -> Result<Vec<PendingLexemeEmbeddingTarget>, sqlx::Error> {
    sqlx::query_as::<_, (i64, String)>(
        r#"
        SELECT dl.id, dl.surface
        FROM dictionary_lexemes dl
        LEFT JOIN dictionary_lexeme_embeddings dle
          ON dle.lexeme_id = dl.id
         AND dle.model_id = $1
        WHERE dle.lexeme_id IS NULL
        ORDER BY dl.surface ASC
        "#,
    )
    .bind(model_id)
    .fetch_all(pool)
    .await
    .map(|rows| {
        rows.into_iter()
            .map(|(lexeme_id, surface)| PendingLexemeEmbeddingTarget { lexeme_id, surface })
            .collect()
    })
}

pub async fn list_lexemes_by_ids(
    pool: &DbPool,
    lexeme_ids: &[i64],
) -> Result<Vec<ResolvedDictionaryLexeme>, sqlx::Error> {
    if lexeme_ids.is_empty() {
        return Ok(Vec::new());
    }

    let rows = sqlx::query_as::<_, (
        i64,
        String,
        String,
        Value,
        Option<bool>,
        chrono::DateTime<chrono::Utc>,
        bool,
        Option<String>,
    )>(
        r#"
        SELECT
            dl.id AS lexeme_id,
            dl.surface AS lexeme_surface,
            dre.headword,
            dre.raw_data,
            dre.has_audio,
            dre.created_at,
            dre.is_form_of,
            dre.pos
        FROM dictionary_lexemes dl
        JOIN dictionary_lexeme_raw_entries dlre
          ON dlre.lexeme_id = dl.id
        JOIN dictionary_raw_entries dre
          ON dre.id = dlre.raw_entry_id
        WHERE dl.id = ANY($1)
        ORDER BY
            dl.id ASC,
            dre.is_form_of ASC,
            COALESCE(dre.pos, '') ASC,
            dre.id ASC
        "#,
    )
    .bind(lexeme_ids)
    .fetch_all(pool)
    .await?;

    let mut grouped = HashMap::<i64, (String, Vec<DictionaryRawLookupRow>)>::new();
    for (lexeme_id, lexeme_surface, headword, raw_data, has_audio, created_at, is_form_of, pos) in rows
    {
        grouped
            .entry(lexeme_id)
            .or_insert_with(|| (lexeme_surface, Vec::new()))
            .1
            .push(DictionaryRawLookupRow {
                headword,
                raw_data,
                has_audio,
                created_at,
                is_form_of,
                pos,
            });
    }

    let mut resolved = lexeme_ids
        .iter()
        .filter_map(|lexeme_id| {
            let (surface, rows) = grouped.remove(lexeme_id)?;
            let merged = merge_raw_rows(rows)?;
            Some(ResolvedDictionaryLexeme {
                lexeme_id: *lexeme_id,
                surface,
                raw_data: merged.raw_data,
                has_audio: merged.has_audio,
                created_at: merged.created_at,
            })
        })
        .collect::<Vec<_>>();

    resolved.sort_by(|left, right| left.surface.cmp(&right.surface));
    Ok(resolved)
}

pub async fn upsert_lexeme_embeddings(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    embeddings: &[NewDictionaryLexemeEmbedding],
) -> Result<(), sqlx::Error> {
    for embedding in embeddings {
        sqlx::query(
            r#"
            INSERT INTO dictionary_lexeme_embeddings (
                lexeme_id,
                model_id,
                source_text,
                dimensions,
                embedding,
                frequency_rank
            )
            VALUES ($1, $2, $3, $4, $5, $6)
            ON CONFLICT (lexeme_id, model_id)
            DO UPDATE SET
                source_text = EXCLUDED.source_text,
                dimensions = EXCLUDED.dimensions,
                embedding = EXCLUDED.embedding,
                frequency_rank = EXCLUDED.frequency_rank,
                updated_at = NOW()
            "#,
        )
        .bind(embedding.lexeme_id)
        .bind(&embedding.model_id)
        .bind(&embedding.source_text)
        .bind(embedding.dimensions)
        .bind(&embedding.embedding)
        .bind(embedding.frequency_rank)
        .execute(&mut **tx)
        .await?;
    }

    Ok(())
}

pub async fn search_lexemes_by_embedding(
    pool: &DbPool,
    model_id: &str,
    embedding: &[f32],
    limit: i64,
) -> Result<Vec<LexemeEmbeddingSearchHit>, sqlx::Error> {
    if embedding.is_empty() || limit <= 0 {
        return Ok(Vec::new());
    }

    let query_vector = Vector::from(embedding.to_vec());
    let rows = sqlx::query_as::<_, LexemeEmbeddingSearchRow>(
        r#"
        SELECT
            dle.lexeme_id,
            dl.surface,
            (dle.embedding <=> $2::vector) AS distance,
            dle.frequency_rank
        FROM dictionary_lexeme_embeddings dle
        JOIN dictionary_lexemes dl
          ON dl.id = dle.lexeme_id
        WHERE dle.model_id = $1
        ORDER BY
            dle.embedding <=> $2::vector ASC,
            dle.frequency_rank ASC NULLS LAST,
            dl.surface ASC
        LIMIT $3
        "#,
    )
    .bind(model_id)
    .bind(query_vector)
    .bind(limit)
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|row| LexemeEmbeddingSearchHit {
            lexeme_id: row.lexeme_id,
            surface: row.surface,
            distance: row.distance as f32,
            frequency_rank: row.frequency_rank,
        })
        .collect())
}

pub async fn find_lexeme_candidates_by_surface(
    pool: &DbPool,
    query: &str,
    limit: i64,
) -> Result<Vec<LexemeCandidate>, sqlx::Error> {
    let trimmed = query.trim();
    if trimmed.is_empty() || limit <= 0 {
        return Ok(Vec::new());
    }

    let normalized = normalize_candidate_surface(trimmed);

    sqlx::query_as::<_, LexemeCandidate>(
        r#"
        WITH ranked AS (
            SELECT
                dl.id AS lexeme_id,
                dl.surface,
                dl.pos_summary,
                dl.gloss_preview,
                dsf.surface AS matched_surface,
                dsf.source AS matched_source,
                CASE
                    WHEN dsf.surface = $1 THEN 0
                    WHEN dsf.surface LIKE $3 THEN 1
                    ELSE 1
                END AS query_scope,
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
                dsf.confidence
            FROM dictionary_surface_forms dsf
            JOIN dictionary_lexemes dl
              ON dl.id = dsf.lexeme_id
            WHERE dsf.surface = $1
               OR dsf.surface LIKE $3
               OR dsf.normalized_surface LIKE $4
        ),
        scoped AS (
            SELECT *
            FROM ranked
            WHERE query_scope = CASE
                WHEN EXISTS (SELECT 1 FROM ranked WHERE query_scope = 0) THEN 0
                ELSE 1
            END
        ),
        dedup AS (
            SELECT *,
                row_number() OVER (
                    PARTITION BY lexeme_id
                    ORDER BY
                        source_rank ASC,
                        surface_rank ASC,
                        confidence DESC,
                        surface ASC
                ) AS rn
            FROM scoped
        )
        SELECT
            lexeme_id,
            surface,
            pos_summary,
            gloss_preview,
            matched_surface,
            matched_source
        FROM dedup
        WHERE rn = 1
        ORDER BY
            source_rank ASC,
            surface_rank ASC,
            confidence DESC,
            surface ASC
        LIMIT $5
        "#,
    )
    .bind(trimmed)
    .bind(&normalized)
    .bind(format!("{trimmed}%"))
    .bind(format!("{normalized}%"))
    .bind(limit)
    .fetch_all(pool)
    .await
}

fn normalize_candidate_surface(value: &str) -> String {
    value
        .trim()
        .to_lowercase()
        .chars()
        .flat_map(|ch| match ch {
            'ä' => ['a'].into_iter().collect::<Vec<_>>(),
            'ö' => ['o'].into_iter().collect::<Vec<_>>(),
            'ü' => ['u'].into_iter().collect::<Vec<_>>(),
            'ß' => ['s', 's'].into_iter().collect::<Vec<_>>(),
            _ => vec![ch],
        })
        .collect()
}
