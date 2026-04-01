use anyhow::{Context, Result};
use serde_json::Value;
use sqlx::{PgPool, Row};
use std::collections::{BTreeMap, BTreeSet};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

use crate::services::dictionary_lexeme_extract::{
    build_gloss_preview, clean_entry, extract_form_of_words, extract_forms, normalize_surface,
    sense_has_tag,
};

#[derive(Debug, Clone)]
struct ImportedRawEntry {
    id: i64,
    headword: String,
    normalized_headword: String,
    pos: String,
    is_form_of: bool,
    raw_data: Value,
}

#[derive(Debug, Clone)]
struct LexemeBundle {
    surface: String,
    normalized_surface: String,
    raw_entries: Vec<ImportedRawEntry>,
}

pub async fn run_rebuild(database_url: &str, dict_path: &Path) -> Result<(i64, i64, i64)> {
    let pool = PgPool::connect(database_url)
        .await
        .context("failed to connect to database")?;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .context("failed to run migrations")?;

    reset_new_dictionary_tables(&pool).await?;
    import_raw_entries(&pool, dict_path).await?;
    let raw_entries = load_imported_raw_entries(&pool).await?;
    let bundles = build_lexeme_bundles(raw_entries);
    persist_lexeme_bundles(&pool, &bundles).await?;
    persist_surface_forms(&pool, &bundles).await?;

    let raw_count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM dictionary_raw_entries")
        .fetch_one(&pool)
        .await?;
    let lexeme_count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM dictionary_lexemes")
        .fetch_one(&pool)
        .await?;
    let surface_count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM dictionary_surface_forms")
        .fetch_one(&pool)
        .await?;

    Ok((raw_count.0, lexeme_count.0, surface_count.0))
}

async fn reset_new_dictionary_tables(pool: &PgPool) -> Result<()> {
    sqlx::query(
        r#"
        TRUNCATE TABLE
            dictionary_lexeme_embeddings,
            dictionary_surface_forms,
            dictionary_lexeme_raw_entries,
            dictionary_lexemes,
            dictionary_raw_entries
        RESTART IDENTITY
        CASCADE
        "#,
    )
    .execute(pool)
    .await
    .context("failed to reset new dictionary tables")?;

    Ok(())
}

async fn import_raw_entries(pool: &PgPool, dict_path: &Path) -> Result<()> {
    println!("Reading dictionary from: {:?}", dict_path);
    let file = File::open(dict_path)?;
    let reader = BufReader::new(file);

    let mut total_lines = 0usize;
    let mut imported_count = 0usize;
    let mut skipped_count = 0usize;

    println!("\nImporting raw entries...\n");

    for (line_idx, line) in reader.lines().enumerate() {
        total_lines += 1;
        if line_idx > 0 && line_idx % 10_000 == 0 {
            println!(
                "Processed {} lines... (imported: {}, skipped: {})",
                line_idx, imported_count, skipped_count
            );
        }

        let line = line?;
        let entry: Value = match serde_json::from_str(&line) {
            Ok(value) => value,
            Err(err) => {
                eprintln!("Line {}: failed to parse JSON: {}", line_idx + 1, err);
                skipped_count += 1;
                continue;
            }
        };

        if entry.get("lang_code").and_then(Value::as_str) != Some("de") {
            skipped_count += 1;
            continue;
        }

        let Some(headword) = entry
            .get("word")
            .and_then(Value::as_str)
            .map(str::to_string)
        else {
            skipped_count += 1;
            continue;
        };

        let cleaned_entry = clean_entry(entry);
        let pos = cleaned_entry
            .get("pos")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        let form_of_words = extract_form_of_words(&cleaned_entry);
        let is_form_of = !form_of_words.is_empty() || sense_has_tag(&cleaned_entry, "form-of");
        let source_key = format!("de-line-{}", line_idx + 1);

        sqlx::query(
            r#"
            INSERT INTO dictionary_raw_entries (
                source_key,
                headword,
                normalized_headword,
                lang_code,
                pos,
                is_form_of,
                form_of_words,
                raw_data,
                has_audio
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            "#,
        )
        .bind(&source_key)
        .bind(&headword)
        .bind(normalize_surface(&headword))
        .bind("de")
        .bind(pos)
        .bind(is_form_of)
        .bind(&form_of_words)
        .bind(cleaned_entry)
        .bind(false)
        .execute(pool)
        .await
        .with_context(|| format!("failed to insert raw entry for '{headword}'"))?;

        imported_count += 1;
    }

    println!(
        "Imported {} raw entries (processed {}, skipped {})",
        imported_count, total_lines, skipped_count
    );

    Ok(())
}

async fn load_imported_raw_entries(pool: &PgPool) -> Result<Vec<ImportedRawEntry>> {
    let rows = sqlx::query(
        r#"
        SELECT id, headword, normalized_headword, pos, is_form_of, raw_data
        FROM dictionary_raw_entries
        ORDER BY headword ASC, id ASC
        "#,
    )
    .fetch_all(pool)
    .await
    .context("failed to load imported raw entries")?;

    Ok(rows
        .into_iter()
        .map(|row| ImportedRawEntry {
            id: row.get("id"),
            headword: row.get("headword"),
            normalized_headword: row.get("normalized_headword"),
            pos: row.get::<Option<String>, _>("pos").unwrap_or_default(),
            is_form_of: row.get("is_form_of"),
            raw_data: row.get("raw_data"),
        })
        .collect())
}

fn build_lexeme_bundles(raw_entries: Vec<ImportedRawEntry>) -> Vec<LexemeBundle> {
    let mut grouped = BTreeMap::<String, LexemeBundle>::new();

    for entry in raw_entries.into_iter().filter(|entry| !entry.is_form_of) {
        let bundle_key = format!("{}::{}", entry.headword, entry.normalized_headword);
        let bundle = grouped.entry(bundle_key).or_insert_with(|| LexemeBundle {
            surface: entry.headword.clone(),
            normalized_surface: entry.normalized_headword.clone(),
            raw_entries: Vec::new(),
        });
        bundle.raw_entries.push(entry);
    }

    grouped.into_values().collect()
}

async fn persist_lexeme_bundles(pool: &PgPool, bundles: &[LexemeBundle]) -> Result<()> {
    println!("Persisting {} lexeme bundles...", bundles.len());

    for bundle in bundles {
        let bundle_key = format!("{}::{}", bundle.surface, bundle.normalized_surface);
        let pos_summary = bundle
            .raw_entries
            .iter()
            .map(|entry| entry.pos.clone())
            .filter(|pos| !pos.is_empty())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect::<Vec<_>>();
        let gloss_preview = Value::Array(
            bundle
                .raw_entries
                .iter()
                .map(|entry| build_gloss_preview(&entry.pos, &entry.raw_data))
                .collect::<Vec<_>>(),
        );

        let row = sqlx::query(
            r#"
            INSERT INTO dictionary_lexemes (
                bundle_key,
                surface,
                normalized_surface,
                gloss_preview,
                pos_summary
            )
            VALUES ($1, $2, $3, $4, $5)
            RETURNING id
            "#,
        )
        .bind(&bundle_key)
        .bind(&bundle.surface)
        .bind(&bundle.normalized_surface)
        .bind(gloss_preview)
        .bind(&pos_summary)
        .fetch_one(pool)
        .await
        .with_context(|| format!("failed to persist lexeme bundle '{}'", bundle.surface))?;
        let lexeme_id: i64 = row.get("id");

        for raw_entry in &bundle.raw_entries {
            sqlx::query(
                r#"
                INSERT INTO dictionary_lexeme_raw_entries (lexeme_id, raw_entry_id)
                VALUES ($1, $2)
                "#,
            )
            .bind(lexeme_id)
            .bind(raw_entry.id)
            .execute(pool)
            .await?;
        }
    }

    Ok(())
}

async fn persist_surface_forms(pool: &PgPool, bundles: &[LexemeBundle]) -> Result<()> {
    println!("Persisting surface links...");

    let mut lexeme_ids = BTreeMap::<String, i64>::new();
    let rows = sqlx::query("SELECT id, surface, normalized_surface FROM dictionary_lexemes")
        .fetch_all(pool)
        .await?;
    for row in rows {
        let surface: String = row.get("surface");
        let normalized_surface: String = row.get("normalized_surface");
        let key = format!("{surface}::{normalized_surface}");
        lexeme_ids.insert(key, row.get("id"));
    }

    for bundle in bundles {
        let bundle_key = format!("{}::{}", bundle.surface, bundle.normalized_surface);
        let Some(&lexeme_id) = lexeme_ids.get(&bundle_key) else {
            continue;
        };

        insert_surface_form(
            pool,
            &bundle.surface,
            &bundle.normalized_surface,
            lexeme_id,
            "headword",
            None,
            1.0,
        )
        .await?;

        for raw_entry in &bundle.raw_entries {
            for form in extract_forms(&raw_entry.raw_data) {
                insert_surface_form(
                    pool,
                    &form,
                    &normalize_surface(&form),
                    lexeme_id,
                    "form",
                    Some(raw_entry.id),
                    0.88,
                )
                .await?;
            }
        }
    }

    let form_of_rows = sqlx::query(
        r#"
        SELECT id, headword, normalized_headword, form_of_words
        FROM dictionary_raw_entries
        WHERE is_form_of = TRUE
        "#,
    )
    .fetch_all(pool)
    .await?;

    for row in form_of_rows {
        let raw_entry_id: i64 = row.get("id");
        let surface: String = row.get("headword");
        let normalized_surface: String = row.get("normalized_headword");
        let form_of_words: Vec<String> = row.get("form_of_words");

        for form_of_word in form_of_words {
            if let Some(lexeme_id) = find_lexeme_id_by_surface(pool, &form_of_word).await? {
                insert_surface_form(
                    pool,
                    &surface,
                    &normalized_surface,
                    lexeme_id,
                    "form_of",
                    Some(raw_entry_id),
                    0.94,
                )
                .await?;
            }
        }
    }

    Ok(())
}

async fn find_lexeme_id_by_surface(pool: &PgPool, surface: &str) -> Result<Option<i64>> {
    let row = sqlx::query(
        r#"
        SELECT id
        FROM dictionary_lexemes
        WHERE surface = $1
        ORDER BY id ASC
        LIMIT 1
        "#,
    )
    .bind(surface)
    .fetch_optional(pool)
    .await?;

    Ok(row.map(|row| row.get("id")))
}

async fn insert_surface_form(
    pool: &PgPool,
    surface: &str,
    normalized_surface: &str,
    lexeme_id: i64,
    source: &str,
    raw_entry_id: Option<i64>,
    confidence: f32,
) -> Result<()> {
    sqlx::query(
        r#"
        INSERT INTO dictionary_surface_forms (
            surface,
            normalized_surface,
            lexeme_id,
            source,
            raw_entry_id,
            confidence
        )
        VALUES ($1, $2, $3, $4, $5, $6)
        ON CONFLICT (surface, lexeme_id, source, raw_entry_id)
        DO NOTHING
        "#,
    )
    .bind(surface)
    .bind(normalized_surface)
    .bind(lexeme_id)
    .bind(source)
    .bind(raw_entry_id)
    .bind(confidence)
    .execute(pool)
    .await?;

    Ok(())
}
