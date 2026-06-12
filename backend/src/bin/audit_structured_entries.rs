use anyhow::{Context, Result};
use chrono::Utc;
use de_ai_hilfer::config::Config;
use de_ai_hilfer::db;
use de_ai_hilfer::models::StructuredAnalysisDocument;
use de_ai_hilfer::services::analysis_structure_quality::validate_structured_capture;
use serde::Serialize;
use serde_json::Value;
use std::path::PathBuf;

#[derive(Debug, Serialize)]
struct AuditSummary {
    created_at: String,
    markdown_rows: i64,
    structured_rows: i64,
    grammar_branch_rows: i64,
    quality_ok_rows: usize,
    quality_failed_rows: usize,
}

#[derive(Debug, Serialize)]
struct StructuredHealthRow {
    id: i64,
    query_text: String,
    markdown_len: usize,
    usage_count: usize,
    insight_count: usize,
    grammar_count: usize,
    grammar_branch_count: usize,
    collocation_count: usize,
    quality_ok: bool,
    quality_error: Option<String>,
}

#[derive(Debug, Serialize)]
struct MissingStructuredRow {
    id: i64,
    query_text: String,
    markdown_len: usize,
}

#[derive(Debug, Serialize)]
struct AuditReport {
    summary: AuditSummary,
    structured_health: Vec<StructuredHealthRow>,
    missing_structured: Vec<MissingStructuredRow>,
}

#[derive(Debug, sqlx::FromRow)]
struct EntryAuditRow {
    id: i64,
    query_text: String,
    analysis: Value,
}

#[tokio::main]
async fn main() -> Result<()> {
    let config = Config::from_env()?;
    let pool = db::create_pool(&config.database_url).await?;

    let summary = load_summary(&pool).await?;
    let structured_health = load_structured_health(&pool).await?;
    let missing_structured = load_missing_structured(&pool).await?;

    let report = AuditReport {
        summary,
        structured_health,
        missing_structured,
    };

    let output_dir = PathBuf::from("tmp");
    tokio::fs::create_dir_all(&output_dir)
        .await
        .context("failed to create tmp directory")?;

    let timestamp = Utc::now().format("%Y%m%d_%H%M%S").to_string();
    let path = output_dir.join(format!("structured_audit_{timestamp}.json"));
    tokio::fs::write(&path, serde_json::to_vec_pretty(&report)?)
        .await
        .with_context(|| format!("failed to write {}", path.display()))?;

    println!("{}", path.display());
    Ok(())
}

async fn load_summary(pool: &db::DbPool) -> Result<AuditSummary> {
    let row = sqlx::query!(
        r#"
        select
          count(*) filter (where analysis ? 'markdown') as "markdown_rows!",
          count(*) filter (
            where jsonb_typeof(analysis->'structured') = 'object'
          ) as "structured_rows!",
          count(*) filter (
            where jsonb_typeof(analysis->'structured'->'grammar_branches') = 'array'
              and jsonb_array_length(analysis->'structured'->'grammar_branches') > 0
          ) as "grammar_branch_rows!"
        from knowledge_entries
        "#
    )
    .fetch_one(pool)
    .await?;

    let structured_health = load_structured_health(pool).await?;
    let quality_ok_rows = structured_health
        .iter()
        .filter(|row| row.quality_ok)
        .count();
    let quality_failed_rows = structured_health.len().saturating_sub(quality_ok_rows);

    Ok(AuditSummary {
        created_at: Utc::now().to_rfc3339(),
        markdown_rows: row.markdown_rows,
        structured_rows: row.structured_rows,
        grammar_branch_rows: row.grammar_branch_rows,
        quality_ok_rows,
        quality_failed_rows,
    })
}

async fn load_structured_health(pool: &db::DbPool) -> Result<Vec<StructuredHealthRow>> {
    let rows = sqlx::query_as::<_, EntryAuditRow>(
        r#"
        select id, query_text, analysis
        from knowledge_entries
        where jsonb_typeof(analysis->'structured') = 'object'
        order by updated_at desc, id desc
        "#,
    )
    .fetch_all(pool)
    .await?;

    let mut result = Vec::with_capacity(rows.len());
    for row in rows {
        let markdown = row
            .analysis
            .get("markdown")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string();
        let structured_value = row
            .analysis
            .get("structured")
            .cloned()
            .unwrap_or(Value::Null);
        let structured: StructuredAnalysisDocument =
            serde_json::from_value(structured_value).context("failed to decode structured")?;

        let quality = if markdown.trim().is_empty() {
            Ok(())
        } else {
            validate_structured_capture(&markdown, &structured)
        };

        result.push(StructuredHealthRow {
            id: row.id,
            query_text: row.query_text,
            markdown_len: markdown.len(),
            usage_count: structured.usage_modules.len(),
            insight_count: structured.deep_insights.len(),
            grammar_count: structured.grammar_rows.len(),
            grammar_branch_count: structured.grammar_branches.len(),
            collocation_count: structured.collocations.len(),
            quality_ok: quality.is_ok(),
            quality_error: quality.err(),
        });
    }

    Ok(result)
}

async fn load_missing_structured(pool: &db::DbPool) -> Result<Vec<MissingStructuredRow>> {
    let rows = sqlx::query!(
        r#"
        select
          id,
          query_text,
          length(coalesce(analysis->>'markdown','')) as "markdown_len!"
        from knowledge_entries
        where analysis ? 'markdown'
          and jsonb_typeof(analysis->'structured') is distinct from 'object'
        order by length(coalesce(analysis->>'markdown','')) desc, id desc
        "#
    )
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|row| MissingStructuredRow {
            id: row.id,
            query_text: row.query_text,
            markdown_len: row.markdown_len as usize,
        })
        .collect())
}
