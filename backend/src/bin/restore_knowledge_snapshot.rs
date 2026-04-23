use anyhow::{Context, Result};
use de_ai_hilfer::config::Config;
use de_ai_hilfer::db;
use de_ai_hilfer::services::knowledge_snapshot::{
    apply_restore_plan, build_restore_plan, ApplyRestoreOptions, KnowledgeSnapshotPayload,
    KNOWLEDGE_SNAPSHOT_FORMAT,
};
use std::path::PathBuf;

#[tokio::main]
async fn main() -> Result<()> {
    let args = CliArgs::parse(std::env::args().skip(1))?;

    let config = Config::from_env()?;
    let pool = db::create_pool(&config.database_url).await?;
    db::run_migrations(&pool).await?;

    let bytes = std::fs::read(&args.snapshot_path)
        .with_context(|| format!("failed to read {}", args.snapshot_path.display()))?;
    let snapshot: KnowledgeSnapshotPayload =
        serde_json::from_slice(&bytes).context("failed to parse snapshot json")?;

    if snapshot.format != KNOWLEDGE_SNAPSHOT_FORMAT {
        anyhow::bail!("unsupported snapshot format: {}", snapshot.format);
    }

    let plan = build_restore_plan(&pool, &snapshot).await?;
    if !plan.rejected_entries.is_empty() {
        anyhow::bail!(
            "restore aborted: {} entries are not restorable on the current dictionary; run dry_run_restore_knowledge_snapshot first and inspect the plan",
            plan.rejected_entries.len()
        );
    }

    if !args.apply {
        println!(
            "restore plan is ready but not applied: restorable_entries={}, retained_learning_progress={}",
            plan.restorable_entries.len(),
            plan.retained_learning_progress.len()
        );
        println!("rerun with --apply to execute the restore");
        return Ok(());
    }

    let summary = apply_restore_plan(
        &pool,
        &plan,
        ApplyRestoreOptions {
            allow_drop_follow_ups: args.allow_drop_follow_ups,
        },
    )
    .await?;

    println!(
        "restore applied: restored_entries={}, restored_learning_progress={}, dropped_follow_ups={}",
        summary.restored_entries, summary.restored_learning_progress, summary.dropped_follow_ups
    );

    Ok(())
}

struct CliArgs {
    snapshot_path: PathBuf,
    apply: bool,
    allow_drop_follow_ups: bool,
}

impl CliArgs {
    fn parse(args: impl IntoIterator<Item = String>) -> Result<Self> {
        let mut snapshot_path = None;
        let mut apply = false;
        let mut allow_drop_follow_ups = false;

        for arg in args {
            match arg.as_str() {
                "--apply" => apply = true,
                "--allow-drop-follow-ups" => allow_drop_follow_ups = true,
                value if value.starts_with("--") => {
                    anyhow::bail!("unknown flag: {value}");
                }
                value if snapshot_path.is_none() => {
                    snapshot_path = Some(PathBuf::from(value));
                }
                value => {
                    anyhow::bail!("unexpected extra argument: {value}");
                }
            }
        }

        let snapshot_path = snapshot_path.context(
            "usage: restore_knowledge_snapshot <snapshot.json> [--apply] [--allow-drop-follow-ups]",
        )?;

        Ok(Self {
            snapshot_path,
            apply,
            allow_drop_follow_ups,
        })
    }
}
