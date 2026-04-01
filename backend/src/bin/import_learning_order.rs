use anyhow::{Context, Result};
use de_ai_hilfer::config::Config;
use de_ai_hilfer::db;
use de_ai_hilfer::models::NewDictionaryLearningOrder;
use de_ai_hilfer::repositories::{dictionary, learning_order};
use serde::Deserialize;
use std::collections::HashSet;
use std::path::PathBuf;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[derive(Debug, Deserialize)]
struct InputRow {
    headword: String,
    cefr_level: Option<String>,
    cefr_rank: Option<i32>,
    frequency_rank: Option<i32>,
    learning_order: Option<i32>,
    source: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "de_ai_hilfer=info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let config = Config::from_env()?;
    let pool = db::create_pool(&config.database_url).await?;
    db::run_migrations(&pool).await?;

    let options = parse_options()?;
    let path = options.path;
    let content = tokio::fs::read_to_string(&path)
        .await
        .with_context(|| format!("failed to read {}", path.display()))?;
    let rows: Vec<InputRow> = serde_json::from_str(&content)
        .with_context(|| format!("failed to parse JSON from {}", path.display()))?;

    let mut normalized = Vec::with_capacity(rows.len());
    let mut skipped = Vec::new();
    let mut seen_headwords = HashSet::new();

    for (index, row) in rows.into_iter().enumerate() {
        let Some(entry) = dictionary::find_by_headword(&pool, &row.headword).await? else {
            skipped.push(row.headword);
            continue;
        };

        if !seen_headwords.insert(entry.headword.clone()) {
            continue;
        }

        normalized.push(NewDictionaryLearningOrder {
            headword: entry.headword,
            cefr_level: normalize_cefr_level(row.cefr_level.as_deref()),
            cefr_rank: row
                .cefr_rank
                .or_else(|| row.cefr_level.as_deref().and_then(cefr_rank_for_level)),
            frequency_rank: row.frequency_rank,
            learning_order: row.learning_order.or(Some((index + 1) as i32)),
            source: row
                .source
                .unwrap_or_else(|| "manual_learning_order_json".to_string()),
        });
    }

    let mut tx = pool.begin().await?;
    if options.replace_all {
        learning_order::delete_all(&mut tx).await?;
    }
    learning_order::upsert_many(&mut tx, &normalized).await?;
    tx.commit().await?;

    tracing::info!(
        "learning order import finished: imported={}, skipped={}, replace_all={}, path={}",
        normalized.len(),
        skipped.len(),
        options.replace_all,
        path.display()
    );
    if !skipped.is_empty() {
        tracing::warn!("learning order skipped headwords: {}", skipped.join(", "));
    }

    Ok(())
}

struct ImportOptions {
    path: PathBuf,
    replace_all: bool,
}

fn parse_options() -> Result<ImportOptions> {
    let mut path: Option<PathBuf> = None;
    let mut replace_all = false;

    for arg in std::env::args().skip(1) {
        match arg.as_str() {
            "--replace-all" => replace_all = true,
            value if value.starts_with("--") => {
                anyhow::bail!("unknown option: {value}");
            }
            value if path.is_none() => path = Some(PathBuf::from(value)),
            value => {
                anyhow::bail!("unexpected extra argument: {value}");
            }
        }
    }

    let path = if let Some(path) = path {
        path
    } else {
        std::env::var("LEARNING_ORDER_JSON_PATH")
            .map(PathBuf::from)
            .context(
                "missing learning order source path; pass it as first arg or set LEARNING_ORDER_JSON_PATH",
            )?
    };

    Ok(ImportOptions { path, replace_all })
}

fn normalize_cefr_level(level: Option<&str>) -> Option<String> {
    level
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| value.to_ascii_uppercase())
}

fn cefr_rank_for_level(level: &str) -> Option<i32> {
    match level.trim().to_ascii_uppercase().as_str() {
        "A1" => Some(1),
        "A2" => Some(2),
        "B1" => Some(3),
        "B2" => Some(4),
        "C1" => Some(5),
        "C2" => Some(6),
        _ => None,
    }
}
