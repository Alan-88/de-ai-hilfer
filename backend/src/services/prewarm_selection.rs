use crate::db::DbPool;
use crate::repositories::{dictionary, learning_order};
use crate::services::query_inference::is_form_reference_entry;
use anyhow::{Context, Result};
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

const DEFAULT_FREQUENCY_CACHE: &str = "target/embedding-cache/opensubtitles_cistem_freq.csv";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrewarmSelectionSource {
    LearningOrder,
    Frequency,
}

impl PrewarmSelectionSource {
    pub fn from_env() -> Self {
        match std::env::var("PREWARM_SELECTION_SOURCE")
            .ok()
            .as_deref()
            .map(str::trim)
            .map(str::to_ascii_lowercase)
            .as_deref()
        {
            Some("frequency") => Self::Frequency,
            _ => Self::LearningOrder,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::LearningOrder => "learning_order",
            Self::Frequency => "frequency",
        }
    }
}

#[derive(Debug, Clone)]
pub struct PrewarmSelectionReport {
    pub headwords: Vec<String>,
    pub scanned_candidates: usize,
    pub source: PrewarmSelectionSource,
}

pub async fn select_headwords_for_prewarm(
    pool: &DbPool,
    limit: usize,
) -> Result<PrewarmSelectionReport> {
    let source = PrewarmSelectionSource::from_env();
    let max_cefr_rank = prewarm_max_cefr_rank_from_env();

    let (headwords, scanned_candidates) = match source {
        PrewarmSelectionSource::LearningOrder => {
            select_from_learning_order(pool, limit, max_cefr_rank).await?
        }
        PrewarmSelectionSource::Frequency => {
            let frequency_path = std::env::var("EMBED_INIT_FREQUENCY_PATH")
                .map(PathBuf::from)
                .unwrap_or_else(|_| PathBuf::from(DEFAULT_FREQUENCY_CACHE));
            select_from_frequency(pool, &frequency_path, limit).await?
        }
    };

    Ok(PrewarmSelectionReport {
        headwords,
        scanned_candidates,
        source,
    })
}

async fn select_from_learning_order(
    pool: &DbPool,
    limit: usize,
    max_cefr_rank: Option<i32>,
) -> Result<(Vec<String>, usize)> {
    let fetch_limit = ((limit.max(1) * 64).min(5_000)) as i64;
    let candidates = learning_order::list_headwords_for_prewarm(pool, fetch_limit, max_cefr_rank)
        .await
        .context("failed to load learning-order headwords for prewarm selection")?;

    let mut selected = Vec::with_capacity(limit);
    let mut scanned = 0;

    for headword in candidates {
        scanned += 1;
        let Some(entry) = dictionary::find_by_headword(pool, &headword).await? else {
            continue;
        };

        if should_skip_prewarm_entry(&entry.headword, &entry.raw_data) {
            continue;
        }

        selected.push(entry.headword);
        if selected.len() >= limit {
            break;
        }
    }

    Ok((selected, scanned))
}

async fn select_from_frequency(
    pool: &DbPool,
    frequency_path: &Path,
    limit: usize,
) -> Result<(Vec<String>, usize)> {
    let content = tokio::fs::read_to_string(frequency_path)
        .await
        .with_context(|| {
            format!(
                "failed to read frequency file at {}",
                frequency_path.display()
            )
        })?;
    let headword_rows = sqlx::query_scalar::<_, String>(
        r#"
        SELECT headword
        FROM dictionary_raw
        ORDER BY headword ASC
        "#,
    )
    .fetch_all(pool)
    .await
    .context("failed to load dictionary headwords for prewarm selection")?;
    let exact_headwords = headword_rows
        .iter()
        .cloned()
        .map(|headword| (headword.clone(), headword))
        .collect::<HashMap<_, _>>();
    let lowercase_headwords = headword_rows.into_iter().fold(
        HashMap::new(),
        |mut map: HashMap<String, String>, headword| {
            map.entry(headword.to_lowercase()).or_insert(headword);
            map
        },
    );

    let mut selected = Vec::with_capacity(limit);
    let mut seen_forms = HashSet::new();
    let mut seen_headwords = HashSet::new();
    let mut scanned = 0;

    for (index, line) in content.lines().enumerate() {
        if index == 0 && line.starts_with("word,") {
            continue;
        }

        let word = line
            .split_once(',')
            .map(|(word, _)| word.trim())
            .unwrap_or_else(|| line.trim());
        if word.is_empty() || !looks_like_frequency_token(word) {
            continue;
        }

        let normalized_form = word.to_lowercase();
        if !seen_forms.insert(normalized_form) {
            continue;
        }
        scanned += 1;

        let Some(candidate_headword) = exact_headwords
            .get(word)
            .or_else(|| lowercase_headwords.get(&word.to_lowercase()))
        else {
            continue;
        };

        let Some(entry) = dictionary::find_by_headword(pool, candidate_headword).await? else {
            continue;
        };

        if should_skip_prewarm_entry(&entry.headword, &entry.raw_data) {
            continue;
        }

        let canonical_headword = entry.headword;
        let normalized_headword = canonical_headword.to_lowercase();
        if !seen_headwords.insert(normalized_headword) {
            continue;
        }

        selected.push(canonical_headword);
        if selected.len() >= limit {
            break;
        }
    }

    Ok((selected, scanned))
}

fn prewarm_max_cefr_rank_from_env() -> Option<i32> {
    std::env::var("PREWARM_MAX_CEFR")
        .ok()
        .as_deref()
        .map(str::trim)
        .and_then(|value| match value.to_ascii_uppercase().as_str() {
            "A1" => Some(1),
            "A2" => Some(2),
            "B1" => Some(3),
            "B2" => Some(4),
            "C1" => Some(5),
            "C2" => Some(6),
            _ => None,
        })
}

fn looks_like_frequency_token(word: &str) -> bool {
    let alphabetic_count = word.chars().filter(|ch| ch.is_alphabetic()).count();
    if alphabetic_count < 2 {
        return false;
    }

    !is_short_all_caps(word)
}

fn should_skip_prewarm_entry(headword: &str, raw_data: &Value) -> bool {
    let pos = raw_data
        .get("pos")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_ascii_lowercase();
    if matches!(pos.as_str(), "name" | "character") {
        return true;
    }

    if is_form_reference_entry(raw_data) {
        return true;
    }

    let tags = raw_data
        .get("senses")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .flat_map(|sense| {
            sense
                .get("tags")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .filter_map(Value::as_str)
        })
        .collect::<HashSet<_>>();

    if tags.contains("letter")
        || tags.contains("uppercase")
        || tags.contains("initialism")
        || tags.contains("acronym")
    {
        return true;
    }

    if tags.contains("abbreviation") && is_short_all_caps(headword) {
        return true;
    }

    let alphabetic_count = headword.chars().filter(|ch| ch.is_alphabetic()).count();
    if alphabetic_count > 2
        && (tags.contains("alt-of") || tags.contains("alternative") || tags.contains("contraction"))
    {
        return true;
    }

    tags.contains("colloquial") && (tags.contains("alt-of") || tags.contains("contraction"))
}

fn is_short_all_caps(word: &str) -> bool {
    let letters = word
        .chars()
        .filter(|ch| ch.is_alphabetic())
        .collect::<Vec<_>>();

    !letters.is_empty() && letters.len() <= 4 && letters.iter().all(|ch| ch.is_uppercase())
}
