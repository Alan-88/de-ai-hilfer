use crate::db::DbPool;
use crate::models::{KnowledgeEntry, LearningProgress};
use crate::repositories::{knowledge, learning};
use anyhow::{bail, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::Row;
use std::collections::HashSet;

pub const KNOWLEDGE_SNAPSHOT_FORMAT: &str = "de_ai_hilfer_knowledge_snapshot_v1";
pub const KNOWLEDGE_RESTORE_PLAN_FORMAT: &str = "de_ai_hilfer_restore_plan_v1";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeSnapshotPayload {
    pub format: String,
    pub exported_at: DateTime<Utc>,
    pub knowledge_entries: Vec<KnowledgeEntry>,
    pub learning_progress: Vec<LearningProgress>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotValidationIssue {
    pub entry_id: i64,
    pub query_text: String,
    pub prototype: Option<String>,
    pub reasons: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotValidationSummary {
    pub total_entries: usize,
    pub valid_entries: usize,
    pub invalid_entries: usize,
    pub issues: Vec<SnapshotValidationIssue>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RestoreRejectedEntry {
    pub entry_id: i64,
    pub query_text: String,
    pub prototype: Option<String>,
    pub reasons: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RestoreSkippedLearningProgress {
    pub entry_id: i64,
    pub reasons: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RestorePreparedEntry {
    pub entry: KnowledgeEntry,
    pub resolution_mode: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeRestorePlan {
    pub format: String,
    pub generated_at: DateTime<Utc>,
    pub source_snapshot_format: String,
    pub source_exported_at: DateTime<Utc>,
    pub total_entries: usize,
    pub restorable_entries: Vec<RestorePreparedEntry>,
    pub rejected_entries: Vec<RestoreRejectedEntry>,
    pub retained_learning_progress: Vec<LearningProgress>,
    pub skipped_learning_progress: Vec<RestoreSkippedLearningProgress>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApplyRestoreOptions {
    pub allow_drop_follow_ups: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApplyRestoreSummary {
    pub restored_entries: usize,
    pub restored_learning_progress: usize,
    pub dropped_follow_ups: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ResolutionMode {
    Exact,
    CaseInsensitive,
}

#[derive(Debug, Clone, Copy)]
struct ResolvedLexeme {
    lexeme_id: i64,
    mode: ResolutionMode,
}

pub async fn export_snapshot(pool: &DbPool) -> Result<KnowledgeSnapshotPayload> {
    Ok(KnowledgeSnapshotPayload {
        format: KNOWLEDGE_SNAPSHOT_FORMAT.to_string(),
        exported_at: Utc::now(),
        knowledge_entries: knowledge::list_all(pool).await?,
        learning_progress: learning::list_all_progress(pool).await?,
    })
}

pub fn validate_snapshot(snapshot: &KnowledgeSnapshotPayload) -> SnapshotValidationSummary {
    let mut issues = Vec::new();

    for entry in &snapshot.knowledge_entries {
        let reasons = validate_snapshot_entry(entry);
        if !reasons.is_empty() {
            issues.push(SnapshotValidationIssue {
                entry_id: entry.id,
                query_text: entry.query_text.clone(),
                prototype: entry.prototype.clone(),
                reasons,
            });
        }
    }

    SnapshotValidationSummary {
        total_entries: snapshot.knowledge_entries.len(),
        valid_entries: snapshot
            .knowledge_entries
            .len()
            .saturating_sub(issues.len()),
        invalid_entries: issues.len(),
        issues,
    }
}

pub async fn build_restore_plan(
    pool: &DbPool,
    snapshot: &KnowledgeSnapshotPayload,
) -> Result<KnowledgeRestorePlan> {
    let mut restorable_entries = Vec::new();
    let mut rejected_entries = Vec::new();
    let mut retained_entry_ids = HashSet::new();

    for entry in &snapshot.knowledge_entries {
        let mut reasons = validate_snapshot_entry(entry);

        if reasons.is_empty() {
            let Some(prototype) = entry.prototype.as_deref() else {
                reasons.push("missing prototype".to_string());
                rejected_entries.push(RestoreRejectedEntry {
                    entry_id: entry.id,
                    query_text: entry.query_text.clone(),
                    prototype: entry.prototype.clone(),
                    reasons,
                });
                continue;
            };

            match resolve_target_lexeme(pool, prototype).await? {
                Some(resolution) => {
                    let mut next_entry = entry.clone();
                    next_entry.lexeme_id = Some(resolution.lexeme_id);
                    restorable_entries.push(RestorePreparedEntry {
                        entry: next_entry,
                        resolution_mode: match resolution.mode {
                            ResolutionMode::Exact => "exact".to_string(),
                            ResolutionMode::CaseInsensitive => "case_insensitive".to_string(),
                        },
                    });
                    retained_entry_ids.insert(entry.id);
                }
                None => {
                    reasons.push(
                        "prototype did not resolve to a unique restorable lexeme".to_string(),
                    );
                    rejected_entries.push(RestoreRejectedEntry {
                        entry_id: entry.id,
                        query_text: entry.query_text.clone(),
                        prototype: entry.prototype.clone(),
                        reasons,
                    });
                }
            }
        } else {
            rejected_entries.push(RestoreRejectedEntry {
                entry_id: entry.id,
                query_text: entry.query_text.clone(),
                prototype: entry.prototype.clone(),
                reasons,
            });
        }
    }

    let mut retained_learning_progress = Vec::new();
    let mut skipped_learning_progress = Vec::new();

    for progress in &snapshot.learning_progress {
        if retained_entry_ids.contains(&progress.entry_id) {
            retained_learning_progress.push(progress.clone());
        } else {
            skipped_learning_progress.push(RestoreSkippedLearningProgress {
                entry_id: progress.entry_id,
                reasons: vec!["owning knowledge entry is not restorable".to_string()],
            });
        }
    }

    Ok(KnowledgeRestorePlan {
        format: KNOWLEDGE_RESTORE_PLAN_FORMAT.to_string(),
        generated_at: Utc::now(),
        source_snapshot_format: snapshot.format.clone(),
        source_exported_at: snapshot.exported_at,
        total_entries: snapshot.knowledge_entries.len(),
        restorable_entries,
        rejected_entries,
        retained_learning_progress,
        skipped_learning_progress,
    })
}

pub async fn apply_restore_plan(
    pool: &DbPool,
    plan: &KnowledgeRestorePlan,
    options: ApplyRestoreOptions,
) -> Result<ApplyRestoreSummary> {
    let follow_up_count = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM follow_ups")
        .fetch_one(pool)
        .await?;

    if follow_up_count > 0 && !options.allow_drop_follow_ups {
        bail!(
            "refusing to restore because follow_ups still contains {follow_up_count} rows; rerun with --allow-drop-follow-ups after confirming they can be discarded"
        );
    }

    let mut tx = pool.begin().await?;

    sqlx::query(
        "TRUNCATE TABLE learning_progress, follow_ups, knowledge_entries RESTART IDENTITY CASCADE",
    )
    .execute(&mut *tx)
    .await?;

    for prepared in &plan.restorable_entries {
        let entry = &prepared.entry;
        sqlx::query(
            r#"
            INSERT INTO knowledge_entries (
                id, query_text, lexeme_id, prototype, entry_type, analysis, tags, aliases, created_at, updated_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            "#,
        )
        .bind(entry.id)
        .bind(&entry.query_text)
        .bind(entry.lexeme_id)
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

    for progress in &plan.retained_learning_progress {
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

    Ok(ApplyRestoreSummary {
        restored_entries: plan.restorable_entries.len(),
        restored_learning_progress: plan.retained_learning_progress.len(),
        dropped_follow_ups: follow_up_count as usize,
    })
}

pub fn dictionary_excerpt_word(analysis: &Value) -> Option<&str> {
    analysis
        .get("dictionary_excerpt")
        .and_then(Value::as_object)
        .and_then(|excerpt| excerpt.get("word"))
        .and_then(Value::as_str)
}

pub fn analysis_prototype(analysis: &Value) -> Option<&str> {
    analysis.get("prototype").and_then(Value::as_str)
}

fn validate_snapshot_entry(entry: &KnowledgeEntry) -> Vec<String> {
    let mut reasons = Vec::new();

    if entry.entry_type != "WORD" {
        reasons.push(format!("unsupported entry_type: {}", entry.entry_type));
    }

    match entry.prototype.as_deref() {
        Some(prototype) if prototype == entry.query_text => {}
        Some(prototype) => reasons.push(format!(
            "query_text/prototype mismatch: {} != {}",
            entry.query_text, prototype
        )),
        None => reasons.push("missing prototype".to_string()),
    }

    match (
        entry.prototype.as_deref(),
        dictionary_excerpt_word(&entry.analysis),
    ) {
        (Some(prototype), Some(word)) if prototype == word => {}
        (Some(prototype), Some(word)) => reasons.push(format!(
            "dictionary_excerpt.word mismatch: {} != {}",
            prototype, word
        )),
        (_, None) => reasons.push("missing analysis.dictionary_excerpt.word".to_string()),
        (None, Some(_)) => {}
    }

    if let (Some(expected), Some(actual)) = (
        entry.prototype.as_deref(),
        analysis_prototype(&entry.analysis),
    ) {
        if expected != actual {
            reasons.push(format!(
                "analysis.prototype mismatch: {} != {}",
                expected, actual
            ));
        }
    }

    reasons
}

async fn resolve_target_lexeme(
    pool: &DbPool,
    prototype: &str,
) -> Result<Option<ResolvedLexeme>, sqlx::Error> {
    if let Some(lexeme_id) = resolve_unique_exact_lexeme(pool, prototype).await? {
        if lexeme_has_independent_raw(pool, lexeme_id).await? {
            return Ok(Some(ResolvedLexeme {
                lexeme_id,
                mode: ResolutionMode::Exact,
            }));
        }
        return Ok(None);
    }

    if let Some(lexeme_id) = resolve_unique_ci_lexeme(pool, prototype).await? {
        if lexeme_has_independent_raw(pool, lexeme_id).await? {
            return Ok(Some(ResolvedLexeme {
                lexeme_id,
                mode: ResolutionMode::CaseInsensitive,
            }));
        }
    }

    Ok(None)
}

async fn resolve_unique_exact_lexeme(
    pool: &DbPool,
    surface: &str,
) -> Result<Option<i64>, sqlx::Error> {
    let rows = sqlx::query("SELECT id FROM dictionary_lexemes WHERE surface = $1 ORDER BY id ASC")
        .bind(surface)
        .fetch_all(pool)
        .await?;

    if rows.len() == 1 {
        Ok(Some(rows[0].get("id")))
    } else {
        Ok(None)
    }
}

async fn resolve_unique_ci_lexeme(
    pool: &DbPool,
    surface: &str,
) -> Result<Option<i64>, sqlx::Error> {
    let rows = sqlx::query(
        r#"
        SELECT id
        FROM dictionary_lexemes
        WHERE lower(surface) = lower($1)
        ORDER BY CASE WHEN surface = $1 THEN 0 ELSE 1 END, id ASC
        "#,
    )
    .bind(surface)
    .fetch_all(pool)
    .await?;

    if rows.len() == 1 {
        Ok(Some(rows[0].get("id")))
    } else {
        Ok(None)
    }
}

async fn lexeme_has_independent_raw(pool: &DbPool, lexeme_id: i64) -> Result<bool, sqlx::Error> {
    let count = sqlx::query_scalar::<_, i64>(
        r#"
        SELECT COUNT(*)
        FROM dictionary_lexeme_raw_entries dlre
        JOIN dictionary_raw_entries dre
          ON dre.id = dlre.raw_entry_id
        WHERE dlre.lexeme_id = $1
          AND dre.is_form_of = FALSE
        "#,
    )
    .bind(lexeme_id)
    .fetch_one(pool)
    .await?;

    Ok(count > 0)
}
