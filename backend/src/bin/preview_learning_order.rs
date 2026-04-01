use anyhow::Result;
use de_ai_hilfer::config::Config;
use de_ai_hilfer::db;
use de_ai_hilfer::repositories::learning_order;

#[tokio::main]
async fn main() -> Result<()> {
    let config = Config::from_env()?;
    let pool = db::create_pool(&config.database_url).await?;
    db::run_migrations(&pool).await?;

    let limit = std::env::var("LEARNING_ORDER_PREVIEW_LIMIT")
        .ok()
        .and_then(|value| value.parse::<i64>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(30);

    let total = learning_order::count(&pool).await?;
    if total == 0 {
        println!("dictionary_learning_order is empty");
        return Ok(());
    }

    let rows = learning_order::list_preview(&pool, limit).await?;
    println!("dictionary_learning_order total={total}, preview_limit={limit}");
    println!("headword\tcefr\tcefr_rank\tfrequency_rank\tlearning_order\tsource");
    for row in rows {
        println!(
            "{}\t{}\t{}\t{}\t{}\t{}",
            row.headword,
            row.cefr_level.unwrap_or_else(|| "-".to_string()),
            row.cefr_rank
                .map(|value| value.to_string())
                .unwrap_or_else(|| "-".to_string()),
            row.frequency_rank
                .map(|value| value.to_string())
                .unwrap_or_else(|| "-".to_string()),
            row.learning_order
                .map(|value| value.to_string())
                .unwrap_or_else(|| "-".to_string()),
            row.source
        );
    }

    Ok(())
}
