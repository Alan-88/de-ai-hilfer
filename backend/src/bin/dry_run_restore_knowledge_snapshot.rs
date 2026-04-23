use anyhow::{Context, Result};
use de_ai_hilfer::config::Config;
use de_ai_hilfer::db;
use de_ai_hilfer::services::knowledge_snapshot::{
    build_restore_plan, KnowledgeSnapshotPayload, KNOWLEDGE_SNAPSHOT_FORMAT,
};
use std::path::PathBuf;

#[tokio::main]
async fn main() -> Result<()> {
    let mut args = std::env::args().skip(1);
    let snapshot_path = args
        .next()
        .map(PathBuf::from)
        .context("usage: dry_run_restore_knowledge_snapshot <snapshot.json> [plan-output.json]")?;
    let plan_output = args.next().map(PathBuf::from).unwrap_or_else(default_plan_output_path);

    let config = Config::from_env()?;
    let pool = db::create_pool(&config.database_url).await?;
    db::run_migrations(&pool).await?;

    let bytes = std::fs::read(&snapshot_path)
        .with_context(|| format!("failed to read {}", snapshot_path.display()))?;
    let snapshot: KnowledgeSnapshotPayload =
        serde_json::from_slice(&bytes).context("failed to parse snapshot json")?;

    if snapshot.format != KNOWLEDGE_SNAPSHOT_FORMAT {
        anyhow::bail!("unsupported snapshot format: {}", snapshot.format);
    }

    let plan = build_restore_plan(&pool, &snapshot).await?;

    if let Some(parent) = plan_output.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("failed to create output directory {}", parent.display()))?;
    }
    let content = serde_json::to_string_pretty(&plan).context("failed to serialize restore plan")?;
    std::fs::write(&plan_output, content)
        .with_context(|| format!("failed to write {}", plan_output.display()))?;

    println!("restore dry-run plan written to {}", plan_output.display());
    println!(
        "entries_total={}, restorable={}, rejected={}, retained_learning_progress={}, skipped_learning_progress={}",
        plan.total_entries,
        plan.restorable_entries.len(),
        plan.rejected_entries.len(),
        plan.retained_learning_progress.len(),
        plan.skipped_learning_progress.len()
    );

    if !plan.rejected_entries.is_empty() {
        println!("rejected entry samples:");
        for rejected in plan.rejected_entries.iter().take(20) {
            println!(
                "  - #{} {} :: {}",
                rejected.entry_id,
                rejected.query_text,
                rejected.reasons.join(" | ")
            );
        }
    }

    Ok(())
}

fn default_plan_output_path() -> PathBuf {
    let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
    PathBuf::from(format!(
        "target/snapshots/knowledge_restore_plan_{timestamp}.json"
    ))
}
