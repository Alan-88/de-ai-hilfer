use anyhow::{Context, Result};
use serde_json::Value;
use sqlx::PgPool;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

#[tokio::main]
async fn main() -> Result<()> {
    println!("De-AI-Hilfer Dictionary Import Tool");
    println!("====================================\n");

    // 加载环境变量
    dotenvy::dotenv().ok();

    let database_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| {
        "postgres://server:HomeServer1447@localhost:5432/de_ai_hilfer".to_string()
    });

    // 连接数据库
    println!("Connecting to database...");
    let pool = PgPool::connect(&database_url)
        .await
        .context("Failed to connect to database")?;
    println!("✓ Connected to database\n");

    // 字典文件路径
    let dict_path = Path::new("../assets/dictionary/kaikki.jsonl");

    if !dict_path.exists() {
        anyhow::bail!("Dictionary file not found at: {:?}", dict_path);
    }

    println!("Reading dictionary from: {:?}", dict_path);
    let file = File::open(dict_path)?;
    let reader = BufReader::new(file);

    let mut total_lines = 0;
    let mut imported_count = 0;
    let mut skipped_count = 0;

    println!("\nImporting entries...\n");

    for (line_num, line) in reader.lines().enumerate() {
        total_lines += 1;

        if line_num > 0 && line_num % 1000 == 0 {
            println!(
                "Processed {} lines... (imported: {}, skipped: {})",
                line_num, imported_count, skipped_count
            );
        }

        let line = line?;
        let entry: Value = match serde_json::from_str(&line) {
            Ok(v) => v,
            Err(e) => {
                eprintln!("Line {}: Failed to parse JSON: {}", line_num + 1, e);
                skipped_count += 1;
                continue;
            }
        };

        // 提取 headword (word field)
        let headword = match entry.get("word").and_then(|w| w.as_str()) {
            Some(w) => w.to_string(),
            None => {
                skipped_count += 1;
                continue;
            }
        };

        // 清洗数据：移除废弃的义项
        let cleaned_entry = clean_entry(entry);

        // 插入数据库
        match insert_entry(&pool, &headword, cleaned_entry).await {
            Ok(_) => imported_count += 1,
            Err(e) => {
                // 如果是重复键冲突，跳过
                if e.to_string().contains("duplicate key") {
                    skipped_count += 1;
                } else {
                    eprintln!(
                        "Line {}: Failed to import '{}': {}",
                        line_num + 1,
                        headword,
                        e
                    );
                    skipped_count += 1;
                }
            }
        }
    }

    println!("\n====================================");
    println!("Import Summary:");
    println!("  Total lines processed: {}", total_lines);
    println!("  Successfully imported: {}", imported_count);
    println!("  Skipped: {}", skipped_count);
    println!("====================================\n");

    Ok(())
}

fn clean_entry(mut entry: Value) -> Value {
    // 清洗 senses 字段：移除含有 "tags": ["obsolete"] 的义项
    if let Some(senses) = entry.get_mut("senses").and_then(|s| s.as_array_mut()) {
        senses.retain(|sense| {
            if let Some(tags) = sense.get("tags").and_then(|t| t.as_array()) {
                !tags.iter().any(|tag| tag.as_str() == Some("obsolete"))
            } else {
                true
            }
        });
    }

    // TODO: 可以添加更多清洗逻辑
    // 例如：移除过时的拼写、方言形式等

    entry
}

async fn insert_entry(pool: &PgPool, headword: &str, raw_data: Value) -> Result<()> {
    sqlx::query!(
        r#"
        INSERT INTO dictionary_raw (headword, raw_data, has_audio)
        VALUES ($1, $2, $3)
        ON CONFLICT (headword) DO NOTHING
        "#,
        headword,
        raw_data,
        false
    )
    .execute(pool)
    .await?;

    Ok(())
}
