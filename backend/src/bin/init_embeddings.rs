use de_ai_hilfer::config::Config;
use de_ai_hilfer::embedding::{run_embedding_backfill, EmbeddingBackfillOptions};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "de_ai_hilfer=info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let config = Config::from_env()?;
    let options = EmbeddingBackfillOptions::from_env();

    tracing::info!("starting embedding initialization; run with `cargo run --bin init_embeddings`");
    let report = run_embedding_backfill(&config, options).await?;
    tracing::info!(
        "embedding initialization finished: total_candidates={}, attempted={}, succeeded={}, failed={}",
        report.total_candidates,
        report.attempted,
        report.succeeded,
        report.failed
    );

    Ok(())
}
