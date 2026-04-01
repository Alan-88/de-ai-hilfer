use anyhow::{anyhow, Context, Result};
use std::cmp::Ordering;
use std::collections::HashMap;
use std::path::Path;

pub(crate) async fn load_frequency_ranks(path: &Path, url: &str) -> Result<HashMap<String, i32>> {
    let content = if path.exists() {
        tokio::fs::read_to_string(path)
            .await
            .with_context(|| format!("failed to read frequency file at {}", path.display()))?
    } else {
        download_frequency_file(path, url).await?
    };

    let mut ranks = HashMap::new();
    for (index, line) in content.lines().enumerate() {
        if index == 0 && line.starts_with("word,") {
            continue;
        }
        let word = line
            .split_once(',')
            .map(|(word, _)| word.trim())
            .unwrap_or_else(|| line.trim());
        if word.is_empty() {
            continue;
        }
        ranks.entry(word.to_lowercase()).or_insert(index as i32);
    }

    if ranks.is_empty() {
        return Err(anyhow!(
            "frequency file is empty or invalid: {}",
            path.display()
        ));
    }

    Ok(ranks)
}

pub(crate) fn compare_headwords(
    left: &str,
    right: &str,
    frequency_ranks: &HashMap<String, i32>,
) -> Ordering {
    let left_rank = frequency_rank_of(left, frequency_ranks).unwrap_or(i32::MAX);
    let right_rank = frequency_rank_of(right, frequency_ranks).unwrap_or(i32::MAX);

    left_rank
        .cmp(&right_rank)
        .then_with(|| left.len().cmp(&right.len()))
        .then_with(|| left.cmp(right))
}

pub(crate) fn frequency_rank_of(
    headword: &str,
    frequency_ranks: &HashMap<String, i32>,
) -> Option<i32> {
    frequency_ranks.get(&headword.to_lowercase()).copied()
}

async fn download_frequency_file(path: &Path, url: &str) -> Result<String> {
    tracing::info!(
        "frequency file missing; downloading default source: {} -> {}",
        url,
        path.display()
    );

    let response = reqwest::get(url)
        .await
        .with_context(|| format!("failed to download frequency file from {url}"))?
        .error_for_status()
        .with_context(|| format!("frequency source returned error status: {url}"))?;
    let content = response
        .text()
        .await
        .context("failed to read frequency file response body")?;

    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent).await.with_context(|| {
            format!(
                "failed to create frequency cache directory {}",
                parent.display()
            )
        })?;
    }
    tokio::fs::write(path, &content)
        .await
        .with_context(|| format!("failed to cache frequency file at {}", path.display()))?;

    Ok(content)
}
