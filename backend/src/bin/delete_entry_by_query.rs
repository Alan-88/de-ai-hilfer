use sqlx::PgPool;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();

    let query_text = std::env::args().nth(1).ok_or_else(|| {
        anyhow::anyhow!("usage: cargo run --bin delete_entry_by_query -- <query_text>")
    })?;

    let database_url = std::env::var("DATABASE_URL")?;
    let pool = PgPool::connect(&database_url).await?;

    let deleted = sqlx::query!(
        r#"
        DELETE FROM knowledge_entries
        WHERE query_text = $1
        "#,
        query_text
    )
    .execute(&pool)
    .await?;

    println!("deleted_rows={}", deleted.rows_affected());
    Ok(())
}
