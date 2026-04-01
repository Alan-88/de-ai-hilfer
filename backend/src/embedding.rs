use crate::ai::{AiClient, AiEmbeddingOptions, AiScene};
use crate::config::Config;
use crate::db;
use crate::embedding_frequency::{compare_headwords, frequency_rank_of, load_frequency_ranks};
use crate::embedding_rate_limiter::SmoothRateLimiter;
use crate::models::NewDictionaryLexemeEmbedding;
use crate::repositories::dictionary_lexemes;
use crate::services::dictionary_render::summarize_dictionary_entry;
use anyhow::{anyhow, Context, Result};
use pgvector::Vector;
use reqwest::StatusCode;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Semaphore;
use tokio::task::JoinSet;

const DEFAULT_FREQUENCY_URL: &str =
    "https://raw.githubusercontent.com/olastor/german-word-frequencies/main/opensubtitles/opensubtitles_cistem_freq.csv";
const DEFAULT_FREQUENCY_CACHE: &str = "target/embedding-cache/opensubtitles_cistem_freq.csv";

#[derive(Debug, Clone)]
pub struct EmbeddingBackfillOptions {
    pub frequency_path: PathBuf,
    pub frequency_url: String,
    pub batch_size: usize,
    pub concurrency: usize,
    pub limit: Option<usize>,
    pub retries: usize,
    pub timeout: Duration,
    pub dimensions: Option<u32>,
    pub retriable_cooldown: Duration,
    pub inputs_per_minute: Option<usize>,
    pub requests_per_minute: Option<usize>,
}

#[derive(Debug, Default, Clone, Copy)]
pub struct EmbeddingBackfillReport {
    pub total_candidates: usize,
    pub attempted: usize,
    pub succeeded: usize,
    pub failed: usize,
}

#[derive(Debug, Clone)]
struct PendingDictionaryLexeme {
    lexeme_id: i64,
    surface: String,
    raw_data: serde_json::Value,
    frequency_rank: Option<i32>,
}

#[derive(Debug, Clone)]
struct EmbeddingInputRecord {
    lexeme_id: i64,
    surface: String,
    source_text: String,
    frequency_rank: Option<i32>,
}

impl EmbeddingBackfillOptions {
    pub fn from_env() -> Self {
        let batch_size = parse_env_usize("EMBED_INIT_BATCH_SIZE", 64);
        let inputs_per_minute = std::env::var("EMBED_INIT_INPUTS_PER_MINUTE")
            .ok()
            .and_then(|value| value.parse::<usize>().ok())
            .or(Some(2000))
            .filter(|value| *value > 0);

        Self {
            frequency_path: std::env::var("EMBED_INIT_FREQUENCY_PATH")
                .map(PathBuf::from)
                .unwrap_or_else(|_| PathBuf::from(DEFAULT_FREQUENCY_CACHE)),
            frequency_url: std::env::var("EMBED_INIT_FREQUENCY_URL")
                .unwrap_or_else(|_| DEFAULT_FREQUENCY_URL.to_string()),
            batch_size,
            concurrency: parse_env_usize("EMBED_INIT_CONCURRENCY", 4),
            limit: std::env::var("EMBED_INIT_LIMIT")
                .ok()
                .and_then(|value| value.parse::<usize>().ok()),
            retries: parse_env_usize("EMBED_INIT_RETRIES", 4),
            timeout: Duration::from_secs(parse_env_u64("EMBED_INIT_TIMEOUT_SECS", 90)),
            dimensions: std::env::var("EMBED_INIT_DIMENSIONS")
                .ok()
                .and_then(|value| value.parse::<u32>().ok()),
            retriable_cooldown: Duration::from_secs(parse_env_u64(
                "EMBED_INIT_RETRIABLE_COOLDOWN_SECS",
                60,
            )),
            inputs_per_minute,
            requests_per_minute: std::env::var("EMBED_INIT_REQUESTS_PER_MINUTE")
                .ok()
                .and_then(|value| value.parse::<usize>().ok())
                .or_else(|| {
                    inputs_per_minute.map(|limit| {
                        let batch = batch_size.max(1);
                        (limit / batch).max(1)
                    })
                })
                .filter(|value| *value > 0),
        }
    }
}

pub async fn run_embedding_backfill(
    config: &Config,
    options: EmbeddingBackfillOptions,
) -> Result<EmbeddingBackfillReport> {
    let api_key = config
        .openai_api_key
        .clone()
        .filter(|value| !value.is_empty())
        .ok_or_else(|| anyhow!("OPENAI_API_KEY is required for embedding initialization"))?;
    let base_url = config
        .openai_base_url
        .clone()
        .unwrap_or_else(|| "https://api.openai.com/v1".to_string());

    let pool = db::create_pool(&config.database_url)
        .await
        .context("failed to connect to database for embedding backfill")?;
    db::run_migrations(&pool)
        .await
        .context("failed to run migrations before embedding backfill")?;

    let frequency_ranks =
        Arc::new(load_frequency_ranks(&options.frequency_path, &options.frequency_url).await?);
    let model_id = config.ai_models.embedding.clone();
    let client = AiClient::new(api_key, base_url, config.ai_models.clone());
    let pending_lexemes = dictionary_lexemes::list_pending_lexemes_for_embedding(&pool, &model_id)
        .await
        .context("failed to list pending lexemes for embeddings")?;

    if pending_lexemes.is_empty() {
        tracing::info!("embedding backfill skipped: no pending dictionary lexemes");
        return Ok(EmbeddingBackfillReport::default());
    }

    let mut ordered_lexemes = pending_lexemes;
    ordered_lexemes.sort_by(|left, right| compare_headwords(&left.surface, &right.surface, &frequency_ranks));
    if let Some(limit) = options.limit {
        ordered_lexemes.truncate(limit);
    }

    let total_candidates = ordered_lexemes.len();
    tracing::info!(
        "embedding backfill start: total_candidates={}, model={}, batch_size={}, concurrency={}, dimensions={:?}, inputs_per_minute={:?}, requests_per_minute={:?}",
        total_candidates,
        model_id,
        options.batch_size,
        options.concurrency,
        options.dimensions,
        options.inputs_per_minute,
        options.requests_per_minute
    );

    let semaphore = Arc::new(Semaphore::new(options.concurrency.max(1)));
    let input_rate_limiter = options
        .inputs_per_minute
        .map(|limit| Arc::new(SmoothRateLimiter::new("inputs", limit)));
    let request_rate_limiter = options
        .requests_per_minute
        .map(|limit| Arc::new(SmoothRateLimiter::new("requests", limit)));
    let mut join_set = JoinSet::new();
    let pool = Arc::new(pool);
    let options = Arc::new(options);
    let mut report = EmbeddingBackfillReport {
        total_candidates,
        ..EmbeddingBackfillReport::default()
    };

    for batch in ordered_lexemes.chunks(options.batch_size.max(1)) {
        let batch = batch.to_vec();
        let pool = pool.clone();
        let client = client.clone();
        let model_id = model_id.clone();
        let frequency_ranks = frequency_ranks.clone();
        let options = options.clone();
        let input_rate_limiter = input_rate_limiter.clone();
        let request_rate_limiter = request_rate_limiter.clone();
        let permit = semaphore.clone().acquire_owned().await?;

        join_set.spawn(async move {
            let _permit = permit;
            process_lexeme_batch(
                pool,
                client,
                model_id,
                batch,
                frequency_ranks,
                options,
                input_rate_limiter,
                request_rate_limiter,
            )
            .await
        });
    }

    while let Some(result) = join_set.join_next().await {
        let batch_report = result.context("embedding worker join failed")??;
        report.attempted += batch_report.attempted;
        report.succeeded += batch_report.succeeded;
        report.failed += batch_report.failed;
        tracing::info!(
            "embedding backfill progress: {}/{} succeeded, {} failed",
            report.succeeded,
            report.total_candidates,
            report.failed
        );
    }

    Ok(report)
}

async fn process_lexeme_batch(
    pool: Arc<db::DbPool>,
    client: AiClient,
    model_id: String,
    lexemes: Vec<dictionary_lexemes::PendingLexemeEmbeddingTarget>,
    frequency_ranks: Arc<HashMap<String, i32>>,
    options: Arc<EmbeddingBackfillOptions>,
    input_rate_limiter: Option<Arc<SmoothRateLimiter>>,
    request_rate_limiter: Option<Arc<SmoothRateLimiter>>,
) -> Result<EmbeddingBackfillReport> {
    let entries = load_pending_lexemes(&pool, &lexemes, &frequency_ranks).await?;
    if entries.is_empty() {
        return Ok(EmbeddingBackfillReport::default());
    }

    let records = entries
        .iter()
        .map(|entry| EmbeddingInputRecord {
            lexeme_id: entry.lexeme_id,
            surface: entry.surface.clone(),
            source_text: summarize_dictionary_entry(&entry.raw_data),
            frequency_rank: entry.frequency_rank,
        })
        .collect::<Vec<_>>();

    match try_embed_records(
        &pool,
        &client,
        &model_id,
        &records,
        &options,
        input_rate_limiter.as_deref(),
        request_rate_limiter.as_deref(),
    )
    .await
    {
        Ok(success_count) => Ok(EmbeddingBackfillReport {
            attempted: records.len(),
            succeeded: success_count,
            failed: records.len().saturating_sub(success_count),
            total_candidates: 0,
        }),
        Err(batch_err) if records.len() > 1 => {
            tracing::warn!(
                "embedding batch failed; retrying item-by-item: batch_size={}, err={batch_err:#}",
                records.len()
            );
            let mut report = EmbeddingBackfillReport::default();
            for record in records {
                report.attempted += 1;
                match try_embed_records(
                    &pool,
                    &client,
                    &model_id,
                    std::slice::from_ref(&record),
                    &options,
                    input_rate_limiter.as_deref(),
                    request_rate_limiter.as_deref(),
                )
                .await
                {
                    Ok(success_count) => report.succeeded += success_count,
                    Err(single_err) => {
                        report.failed += 1;
                        tracing::error!(
                            "embedding single item failed: lexeme_id={}, surface={}, err={single_err:#}",
                            record.lexeme_id,
                            record.surface
                        );
                    }
                }
            }
            Ok(report)
        }
        Err(err) => Err(err),
    }
}

async fn load_pending_lexemes(
    pool: &db::DbPool,
    lexemes: &[dictionary_lexemes::PendingLexemeEmbeddingTarget],
    frequency_ranks: &HashMap<String, i32>,
) -> Result<Vec<PendingDictionaryLexeme>> {
    let lexeme_ids = lexemes.iter().map(|entry| entry.lexeme_id).collect::<Vec<_>>();
    let entries = dictionary_lexemes::list_lexemes_by_ids(pool, &lexeme_ids)
        .await
        .context("failed to load dictionary lexemes for embedding batch")?;
    let mut entry_map = entries
        .into_iter()
        .map(|entry| (entry.lexeme_id, entry))
        .collect::<HashMap<_, _>>();

    Ok(lexemes
        .iter()
        .filter_map(|lexeme| {
            entry_map
                .remove(&lexeme.lexeme_id)
                .map(|entry| PendingDictionaryLexeme {
                    lexeme_id: entry.lexeme_id,
                    surface: entry.surface.clone(),
                    raw_data: entry.raw_data,
                    frequency_rank: frequency_rank_of(&entry.surface, frequency_ranks),
                })
        })
        .collect())
}

async fn try_embed_records(
    pool: &db::DbPool,
    client: &AiClient,
    model_id: &str,
    records: &[EmbeddingInputRecord],
    options: &EmbeddingBackfillOptions,
    input_rate_limiter: Option<&SmoothRateLimiter>,
    request_rate_limiter: Option<&SmoothRateLimiter>,
) -> Result<usize> {
    if records.is_empty() {
        return Ok(0);
    }

    let inputs = records
        .iter()
        .map(|record| record.source_text.clone())
        .collect::<Vec<_>>();
    let embeddings = embed_with_retry(
        client,
        &inputs,
        options,
        input_rate_limiter,
        request_rate_limiter,
    )
    .await?;
    let mut new_embeddings = Vec::with_capacity(records.len());

    for (record, embedding) in records.iter().zip(embeddings) {
        let dimensions = embedding.len() as i32;
        new_embeddings.push(NewDictionaryLexemeEmbedding {
            lexeme_id: record.lexeme_id,
            model_id: model_id.to_string(),
            source_text: record.source_text.clone(),
            dimensions,
            embedding: Vector::from(embedding),
            frequency_rank: record.frequency_rank,
        });
    }

    let mut tx = pool.begin().await?;
    dictionary_lexemes::upsert_lexeme_embeddings(&mut tx, &new_embeddings).await?;
    tx.commit().await?;

    Ok(new_embeddings.len())
}

async fn embed_with_retry(
    client: &AiClient,
    inputs: &[String],
    options: &EmbeddingBackfillOptions,
    input_rate_limiter: Option<&SmoothRateLimiter>,
    request_rate_limiter: Option<&SmoothRateLimiter>,
) -> Result<Vec<Vec<f32>>> {
    let mut cooldown_round = 0usize;

    loop {
        let mut delay = Duration::from_secs(2);
        let mut last_retriable_error = None;

        for attempt in 0..=options.retries {
            if let Some(rate_limiter) = input_rate_limiter {
                rate_limiter.acquire(inputs.len()).await;
            }
            if let Some(rate_limiter) = request_rate_limiter {
                rate_limiter.acquire(1).await;
            }
            match client
                .embed_with_options(
                    AiScene::Embedding,
                    inputs,
                    AiEmbeddingOptions {
                        timeout: options.timeout,
                        dimensions: options.dimensions,
                    },
                )
                .await
            {
                Ok(response) => return Ok(response),
                Err(err) if attempt < options.retries && is_retriable_embedding_error(&err) => {
                    tracing::warn!(
                        "embedding request retrying: attempt={}, remaining={}, err={err:#}",
                        attempt + 1,
                        options.retries.saturating_sub(attempt)
                    );
                    tokio::time::sleep(delay).await;
                    delay = (delay * 2).min(Duration::from_secs(30));
                }
                Err(err) if is_retriable_embedding_error(&err) => {
                    last_retriable_error = Some(err);
                    break;
                }
                Err(err) => return Err(err),
            }
        }

        if let Some(err) = last_retriable_error {
            cooldown_round += 1;
            tracing::warn!(
                "embedding request exhausted retriable attempts; entering cooldown: round={}, sleep_secs={}, err={err:#}",
                cooldown_round,
                options.retriable_cooldown.as_secs()
            );
            tokio::time::sleep(options.retriable_cooldown).await;
            continue;
        }

        return Err(anyhow!("embedding request exhausted retries"));
    }
}

fn is_retriable_embedding_error(err: &anyhow::Error) -> bool {
    if let Some(reqwest_err) = err.downcast_ref::<reqwest::Error>() {
        if reqwest_err.is_timeout() || reqwest_err.is_connect() || reqwest_err.is_request() {
            return true;
        }
        if let Some(status) = reqwest_err.status() {
            return matches!(
                status,
                StatusCode::TOO_MANY_REQUESTS
                    | StatusCode::BAD_GATEWAY
                    | StatusCode::SERVICE_UNAVAILABLE
                    | StatusCode::GATEWAY_TIMEOUT
                    | StatusCode::INTERNAL_SERVER_ERROR
            );
        }
    }

    let message = err.to_string().to_lowercase();
    [
        "timeout",
        "timed out",
        "429",
        "500",
        "502",
        "503",
        "504",
        "connection reset",
    ]
    .iter()
    .any(|needle| message.contains(needle))
}

fn parse_env_usize(name: &str, default: usize) -> usize {
    std::env::var(name)
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(default)
}

fn parse_env_u64(name: &str, default: u64) -> u64 {
    std::env::var(name)
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(default)
}
