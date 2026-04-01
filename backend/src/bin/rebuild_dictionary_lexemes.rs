use anyhow::Result;
use de_ai_hilfer::services::dictionary_lexeme_rebuild::run_rebuild;
use std::path::Path;

#[tokio::main]
async fn main() -> Result<()> {
    println!("De-AI-Hilfer Dictionary Lexeme Rebuild Tool");
    println!("===========================================\n");

    dotenvy::dotenv().ok();

    let database_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| {
        "postgres://server:HomeServer1447@localhost:5432/de_ai_hilfer".to_string()
    });
    let dict_path = Path::new("../assets/dictionary/kaikki.jsonl");

    if !dict_path.exists() {
        anyhow::bail!("dictionary file not found at: {:?}", dict_path);
    }

    let (raw_count, lexeme_count, surface_count) = run_rebuild(&database_url, dict_path).await?;

    println!("\n====================================");
    println!("Rebuild Summary:");
    println!("  Raw entries imported: {}", raw_count);
    println!("  Lexeme bundles built: {}", lexeme_count);
    println!("  Surface links built: {}", surface_count);
    println!("====================================\n");

    Ok(())
}
