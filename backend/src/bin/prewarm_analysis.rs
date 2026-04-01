use anyhow::{Context, Result};
use chrono::Utc;
use de_ai_hilfer::ai::AiClient;
use de_ai_hilfer::config::Config;
use de_ai_hilfer::db;
use de_ai_hilfer::models::{AnalyzeRequest, QualityMode};
use de_ai_hilfer::prompts::PromptConfig;
use de_ai_hilfer::repositories::knowledge;
use de_ai_hilfer::services::analyze::analyze;
use de_ai_hilfer::services::prewarm_selection::select_headwords_for_prewarm;
use de_ai_hilfer::state::AppState;
use serde::Serialize;
use serde_json::Value;
use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{Mutex, Semaphore};
use tokio::task::JoinSet;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

const DEFAULT_RETRY_DELAY_MS: u64 = 20_000;

#[derive(Debug, Serialize)]
struct AttemptReport {
    attempt: usize,
    source: String,
    model: Option<String>,
    entry_id: i64,
    reused_existing_ai_entry: bool,
}

#[derive(Debug, Serialize)]
struct WordReport {
    index: usize,
    headword: String,
    attempts: Vec<AttemptReport>,
    final_ok: bool,
}

#[derive(Debug, Serialize)]
struct SummaryReport {
    created_at: String,
    requested_limit: usize,
    selected_count: usize,
    scanned_candidates: usize,
    selection_source: String,
    reused_existing_ai_entries: usize,
    ai_generated: usize,
    fallback_failures: usize,
    words: Vec<WordReport>,
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

    let mut config = Config::from_env()?;
    let required_model = config.ai_models.analyze.clone();
    config.ai_models.fallback_fast = config.ai_models.analyze.clone();
    config.ai_models.fallback_pro = config.ai_models.analyze_pro.clone();
    let pool = db::create_pool(&config.database_url).await?;
    db::run_migrations(&pool).await?;
    let prompts = PromptConfig::load(&config.prompt_config_path)?;
    let ai_client = AiClient::new(
        config.openai_api_key.clone().unwrap_or_default(),
        config
            .openai_base_url
            .clone()
            .unwrap_or_else(|| "https://api.openai.com/v1".to_string()),
        config.ai_models.clone(),
    );

    let state = AppState {
        pool,
        config,
        prompts,
        ai_client,
        recent_searches: Arc::new(Mutex::new(VecDeque::with_capacity(20))),
    };

    let limit = env_usize("PREWARM_LIMIT", 500);
    let retry_on_fallback = env_usize("PREWARM_FALLBACK_RETRIES", 1);
    let concurrency = env_usize("PREWARM_CONCURRENCY", 4);
    let retry_delay_ms = env_u64("PREWARM_RETRY_DELAY_MS", DEFAULT_RETRY_DELAY_MS);
    let selection = select_headwords_for_prewarm(&state.pool, limit).await?;
    let headwords = selection.headwords;

    tracing::info!(
        "analysis prewarm start: requested_limit={}, selected_count={}, scanned_candidates={}, selection_source={}, fallback_retries={}, retry_delay_ms={}, concurrency={}, required_model={}",
        limit,
        headwords.len(),
        selection.scanned_candidates,
        selection.source.as_str(),
        retry_on_fallback,
        retry_delay_ms,
        concurrency
        ,
        required_model
    );

    let semaphore = Arc::new(Semaphore::new(concurrency.max(1)));
    let mut join_set = JoinSet::new();

    for (index, headword) in headwords.iter().cloned().enumerate() {
        let state = state.clone();
        let semaphore = semaphore.clone();
        let required_model = required_model.clone();
        join_set.spawn(async move {
            let _permit = semaphore.acquire_owned().await?;
            process_headword(
                state,
                headword,
                index,
                retry_on_fallback,
                retry_delay_ms,
                required_model,
            )
            .await
        });
    }

    let mut reused_existing_ai_entries = 0;
    let mut ai_generated = 0;
    let mut fallback_failures = 0;
    let mut words = Vec::with_capacity(headwords.len());

    let total = headwords.len();
    let mut completed = 0;
    while let Some(result) = join_set.join_next().await {
        let report = result.context("prewarm worker join failed")??;
        if report
            .attempts
            .iter()
            .any(|attempt| attempt.reused_existing_ai_entry)
        {
            reused_existing_ai_entries += 1;
        } else if report.final_ok {
            ai_generated += 1;
        } else {
            fallback_failures += 1;
        }

        completed += 1;
        words.push(report);
        log_progress(completed, total);
    }

    words.sort_by_key(|word| word.index);

    let summary = SummaryReport {
        created_at: Utc::now().to_rfc3339(),
        requested_limit: limit,
        selected_count: headwords.len(),
        scanned_candidates: selection.scanned_candidates,
        selection_source: selection.source.as_str().to_string(),
        reused_existing_ai_entries,
        ai_generated,
        fallback_failures,
        words,
    };

    let report_path = PathBuf::from(format!(
        "logs/prewarm_analysis_{}.json",
        Utc::now().format("%Y%m%d_%H%M%S")
    ));
    tokio::fs::write(&report_path, serde_json::to_vec_pretty(&summary)?)
        .await
        .with_context(|| format!("failed to write report to {}", report_path.display()))?;

    tracing::info!(
        "analysis prewarm finished: selected_count={}, reused_existing_ai_entries={}, ai_generated={}, fallback_failures={}, report={}",
        summary.selected_count,
        summary.reused_existing_ai_entries,
        summary.ai_generated,
        summary.fallback_failures,
        report_path.display()
    );

    Ok(())
}

async fn process_headword(
    state: AppState,
    headword: String,
    index: usize,
    retry_on_fallback: usize,
    retry_delay_ms: u64,
    required_model: String,
) -> Result<WordReport> {
    if let Some(existing) = knowledge::find_by_query_text_exact(&state.pool, &headword).await? {
        if stored_model(&existing.analysis) == Some(required_model.as_str()) {
            return Ok(WordReport {
                index,
                headword,
                attempts: vec![AttemptReport {
                    attempt: 1,
                    source: "知识库".to_string(),
                    model: existing
                        .analysis
                        .get("model")
                        .and_then(Value::as_str)
                        .map(ToString::to_string),
                    entry_id: existing.id,
                    reused_existing_ai_entry: true,
                }],
                final_ok: true,
            });
        }

        if has_ai_model(&existing.analysis) {
            let deleted = knowledge::delete_by_id(&state.pool, existing.id).await?;
            tracing::warn!(
                "analysis prewarm removed cached entry generated by non-primary model: headword={}, entry_id={}, model={:?}, deleted={deleted}, required_model={}",
                headword,
                existing.id,
                stored_model(&existing.analysis),
                required_model
            );
        }
    }

    let mut attempts = Vec::new();
    let mut final_ok = false;

    for attempt in 1..=(retry_on_fallback + 1) {
        let response = analyze(
            &state,
            AnalyzeRequest {
                query_text: headword.clone(),
                entry_type: Some("WORD".to_string()),
                generation_hint: None,
                quality_mode: QualityMode::Default,
                force_refresh: true,
                entry_id: None,
            },
        )
        .await
        .with_context(|| format!("failed to prewarm analysis for {headword}"))?;

        let fallback_source = matches!(response.source.as_str(), "字典兜底" | "本地兜底");
        let wrong_model = response.model.as_deref() != Some(required_model.as_str());
        attempts.push(AttemptReport {
            attempt,
            source: response.source.clone(),
            model: response.model.clone(),
            entry_id: response.entry_id,
            reused_existing_ai_entry: false,
        });

        if fallback_source || wrong_model {
            if response.entry_id > 0 {
                let deleted = knowledge::delete_by_id(&state.pool, response.entry_id).await?;
                if fallback_source {
                    tracing::warn!(
                        "analysis prewarm removed fallback cache entry: headword={}, entry_id={}, deleted={deleted}",
                        headword,
                        response.entry_id
                    );
                } else {
                    tracing::warn!(
                        "analysis prewarm removed non-primary model cache entry: headword={}, entry_id={}, model={:?}, required_model={}, deleted={deleted}",
                        headword,
                        response.entry_id,
                        response.model,
                        required_model
                    );
                }
            }

            if attempt <= retry_on_fallback {
                let delay_ms = retry_delay_ms.saturating_mul(attempt as u64);
                tracing::warn!(
                    "analysis prewarm waiting before retry: headword={}, attempt={}/{}, reason={}, wait_ms={}",
                    headword,
                    attempt,
                    retry_on_fallback + 1,
                    if fallback_source {
                        "fallback"
                    } else {
                        "non_primary_model"
                    },
                    delay_ms
                );
                tokio::time::sleep(Duration::from_millis(delay_ms)).await;
                continue;
            }
            break;
        }

        final_ok = true;
        break;
    }

    Ok(WordReport {
        index,
        headword,
        attempts,
        final_ok,
    })
}

fn has_ai_model(analysis: &Value) -> bool {
    stored_model(analysis)
        .map(|value| !value.trim().is_empty())
        .unwrap_or(false)
}

fn stored_model(analysis: &Value) -> Option<&str> {
    analysis.get("model").and_then(Value::as_str)
}

fn env_usize(name: &str, default: usize) -> usize {
    std::env::var(name)
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(default)
}

fn env_u64(name: &str, default: u64) -> u64 {
    std::env::var(name)
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(default)
}

fn log_progress(current: usize, total: usize) {
    if current == total || current % 25 == 0 {
        tracing::info!("analysis prewarm progress: {current}/{total}");
    }
}
