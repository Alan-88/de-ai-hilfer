use anyhow::{Context, Result};
use de_ai_hilfer::config::Config;
use de_ai_hilfer::db;
use de_ai_hilfer::services::knowledge_snapshot::{export_snapshot, validate_snapshot};
use std::path::PathBuf;

#[tokio::main]
async fn main() -> Result<()> {
    let config = Config::from_env()?;
    let pool = db::create_pool(&config.database_url).await?;
    db::run_migrations(&pool).await?;

    let output_path = snapshot_output_path(std::env::args().nth(1));
    if let Some(parent) = output_path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("failed to create output directory {}", parent.display()))?;
    }

    let snapshot = export_snapshot(&pool).await?;
    let summary = validate_snapshot(&snapshot);
    let content = serde_json::to_string_pretty(&snapshot).context("failed to serialize snapshot")?;
    std::fs::write(&output_path, content)
        .with_context(|| format!("failed to write {}", output_path.display()))?;

    println!("knowledge snapshot written to {}", output_path.display());
    println!(
        "entries={}, learning_progress={}, valid_entries={}, invalid_entries={}",
        snapshot.knowledge_entries.len(),
        snapshot.learning_progress.len(),
        summary.valid_entries,
        summary.invalid_entries
    );
    if !summary.issues.is_empty() {
        println!("invalid entry samples:");
        for issue in summary.issues.iter().take(20) {
            println!(
                "  - #{} {} :: {}",
                issue.entry_id,
                issue.query_text,
                issue.reasons.join(" | ")
            );
        }
    }

    Ok(())
}

fn snapshot_output_path(arg: Option<String>) -> PathBuf {
    if let Some(path) = arg.filter(|value| !value.trim().is_empty()) {
        return PathBuf::from(path);
    }

    let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
    PathBuf::from(format!(
        "target/snapshots/knowledge_snapshot_{timestamp}.json"
    ))
}
