use sqlx::PgPool;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();

    let database_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| {
        "postgres://server:HomeServer1447@localhost:5432/de_ai_hilfer".to_string()
    });

    println!("Connecting to database...");
    let pool = PgPool::connect(&database_url).await?;

    // 检查 dictionary_raw 表的记录数
    let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM dictionary_raw")
        .fetch_one(&pool)
        .await?;

    println!("✓ Dictionary entries in database: {}", count.0);

    // 显示前5个条目
    let entries: Vec<(String,)> =
        sqlx::query_as("SELECT headword FROM dictionary_raw ORDER BY headword LIMIT 5")
            .fetch_all(&pool)
            .await?;

    println!("\nFirst 5 entries:");
    for (headword,) in entries {
        println!("  - {}", headword);
    }

    Ok(())
}
