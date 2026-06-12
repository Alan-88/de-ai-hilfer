#[path = "../bin_support/staged_structured_backfill_support.rs"]
mod staged_structured_backfill_support;

use anyhow::Result;
use de_ai_hilfer::ai::AiClient;
use de_ai_hilfer::config::Config;
use de_ai_hilfer::db;
use de_ai_hilfer::models::{AnalysisDocument, QualityMode};
use de_ai_hilfer::prompts::PromptConfig;
use de_ai_hilfer::repositories::{dictionary, dictionary_lexemes};
use de_ai_hilfer::services::analysis_grounded_backfill::{
    complete_grounded_from_ab_cache_with_policy, prepare_grounded_ab_strict_primary,
};
use de_ai_hilfer::services::analysis_grounded_prompt::{build_model_a_prompt, build_stage2_prompt};
use de_ai_hilfer::services::analysis_grounded_runtime::GroundedAnalysis;
use de_ai_hilfer::services::analysis_structure_retry::StructureRetryPolicy;
use de_ai_hilfer::services::dictionary_facts::dictionary_pos;
use de_ai_hilfer::services::dictionary_tags::build_tags;
use de_ai_hilfer::state::AppState;
use staged_structured_backfill_support::{
    classify_error, ensure_dirs, is_retryable_run_c_error, list_artifacts, prompt_hash, read_json,
    write_deferred_with_retries, write_failure, write_failure_with_retries, write_json,
    write_report, AbArtifact, Command, Options, QueuePaths, ReadyArtifact, StageCase, StageReport,
    PIPELINE_VERSION,
};
use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::task::JoinSet;
use tokio::time::{sleep, Duration};

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "de_ai_hilfer=info".into()),
        )
        .init();

    let config = Config::from_env()?;
    let prompts = PromptConfig::load(&config.prompt_config_path)?;
    let pool = db::create_pool(&config.database_url).await?;
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
        recent_searches: Arc::new(Mutex::new(VecDeque::new())),
    };

    let options = Options::from_env();
    ensure_dirs(&options.queue_dir).await?;

    let report = match options.command {
        Command::PrepareAb => prepare_ab(&state, &options).await?,
        Command::RunC => run_c(&state, &options).await?,
        Command::ApplyReady => apply_ready(&state, &options).await?,
    };

    let report_path = write_report(&options.queue_dir, &options.command, &report).await?;
    println!("{}", report_path.display());
    Ok(())
}

async fn prepare_ab(state: &AppState, options: &Options) -> Result<StageReport> {
    let words = load_words(state, options).await?;
    let mut cases = Vec::new();
    let mut join_set = JoinSet::new();
    let state = state.clone();
    let options = options.clone();
    let mut submitted = 0usize;

    for word in words {
        let paths = QueuePaths::new(&options.queue_dir, &word);
        if deferred_still_cooling(&paths.ab_deferred_path, &options).await? {
            cases.push(StageCase::skipped(&word, "prepare_deferred_cooling"));
            continue;
        }
        if paths.ready_path.exists() || paths.ab_path.exists() {
            cases.push(StageCase::skipped(&word, "already_prepared"));
            continue;
        }

        if submitted > 0 && options.request_spacing_secs > 0 {
            sleep(Duration::from_secs(options.request_spacing_secs)).await;
        }

        let state = state.clone();
        let task_options = options.clone();
        join_set.spawn(async move { prepare_ab_case(state, task_options, word).await });
        submitted += 1;

        if join_set.len() >= options.prepare_concurrency {
            if let Some(result) = join_set.join_next().await {
                cases.push(result??);
            }
        }
    }

    while let Some(result) = join_set.join_next().await {
        cases.push(result??);
    }

    Ok(StageReport::from_cases(cases))
}

async fn prepare_ab_case(state: AppState, options: Options, word: String) -> Result<StageCase> {
    let paths = QueuePaths::new(&options.queue_dir, &word);
    match prepare_grounded_ab_strict_primary(&state, &word, QualityMode::Default).await {
        Ok(cache) => {
            let artifact = AbArtifact {
                pipeline_version: PIPELINE_VERSION.to_string(),
                query_text: word.clone(),
                model_a_prompt_hash: prompt_hash(&build_model_a_prompt(&state.prompts)),
                stage2_prompt_hash: prompt_hash(&build_stage2_prompt(&state.prompts)),
                cache,
            };
            write_json(&paths.ab_path, &artifact).await?;
            Ok(StageCase::success(&word))
        }
        Err(err) => {
            let error_kind = classify_error(&err);
            if is_retryable_run_c_error(&error_kind) {
                write_deferred_with_retries(&paths.ab_deferred_path, &word, "prepare_ab", &err, 0)
                    .await?;
                Ok(StageCase::deferred_with_retries(
                    &word,
                    &error_kind,
                    &err,
                    0,
                ))
            } else {
                write_failure(&paths.ab_failed_path, &word, "prepare_ab", &err).await?;
                Ok(StageCase::failed(&word, &error_kind, &err))
            }
        }
    }
}

async fn run_c(state: &AppState, options: &Options) -> Result<StageReport> {
    let paths = list_pending_ab_artifacts(state, options).await?;
    let mut cases = Vec::new();
    let mut join_set = JoinSet::new();
    let state = state.clone();
    let options = options.clone();
    let mut submitted = 0usize;

    for path in paths {
        if submitted > 0 && options.request_spacing_secs > 0 {
            sleep(Duration::from_secs(options.request_spacing_secs)).await;
        }

        let state = state.clone();
        let task_options = options.clone();
        join_set.spawn(async move { run_c_case(state, task_options, path).await });
        submitted += 1;

        if join_set.len() >= options.c_concurrency {
            if let Some(result) = join_set.join_next().await {
                cases.push(result??);
            }
        }
    }

    while let Some(result) = join_set.join_next().await {
        cases.push(result??);
    }

    Ok(StageReport::from_cases(cases))
}

async fn run_c_case(
    state: AppState,
    options: Options,
    path: std::path::PathBuf,
) -> Result<StageCase> {
    let artifact: AbArtifact = read_json(&path).await?;
    let queue_paths = QueuePaths::new(&options.queue_dir, &artifact.query_text);
    if queue_paths.ready_path.exists() {
        return Ok(StageCase::skipped(&artifact.query_text, "already_ready"));
    }

    let dictionary_entry = dictionary::find_by_headword(&state.pool, &artifact.query_text).await?;
    match complete_grounded_from_ab_cache_with_retries(
        &state,
        &options,
        &artifact,
        dictionary_entry.as_ref(),
    )
    .await
    {
        Ok((grounded, transient_retries)) => {
            let analysis = serde_json::to_value(&grounded.analysis)?;
            let ready = ReadyArtifact {
                pipeline_version: artifact.pipeline_version,
                query_text: artifact.query_text.clone(),
                stage2_model: artifact.cache.stage2_model,
                analysis,
            };
            write_json(&queue_paths.ready_path, &ready).await?;
            Ok(StageCase::success_with_retries(
                &artifact.query_text,
                transient_retries,
            ))
        }
        Err((err, transient_retries)) => {
            let error_kind = classify_error(&err);
            if is_retryable_run_c_error(&error_kind) {
                write_deferred_with_retries(
                    &queue_paths.c_deferred_path,
                    &artifact.query_text,
                    "run_c",
                    &err,
                    transient_retries,
                )
                .await?;
                Ok(StageCase::deferred_with_retries(
                    &artifact.query_text,
                    &error_kind,
                    &err,
                    transient_retries,
                ))
            } else {
                write_failure_with_retries(
                    &queue_paths.c_failed_path,
                    &artifact.query_text,
                    "run_c",
                    &err,
                    transient_retries,
                )
                .await?;
                Ok(StageCase::failed_with_retries(
                    &artifact.query_text,
                    &error_kind,
                    &err,
                    transient_retries,
                ))
            }
        }
    }
}

async fn list_pending_ab_artifacts(
    state: &AppState,
    options: &Options,
) -> Result<Vec<std::path::PathBuf>> {
    let paths = if options.words.is_empty() {
        list_artifacts(&options.queue_dir, "ab", i64::MAX).await?
    } else {
        options
            .words
            .iter()
            .map(|word| QueuePaths::new(&options.queue_dir, word).ab_path)
            .collect()
    };
    let mut pending = Vec::new();

    for path in paths {
        if !path.exists() {
            continue;
        }
        let artifact: AbArtifact = read_json(&path).await?;
        let queue_paths = QueuePaths::new(&options.queue_dir, &artifact.query_text);
        if deferred_still_cooling(&queue_paths.c_deferred_path, options).await? {
            continue;
        }
        if queue_paths.ready_path.exists()
            || queue_paths.applied_path.exists()
            || queue_paths.c_failed_path.exists()
            || (!options.ignore_db_existing
                && entry_already_has_grammar_branches(state, &artifact.query_text).await?)
        {
            continue;
        }
        pending.push(path);
        if pending.len() >= options.limit as usize {
            break;
        }
    }

    Ok(pending)
}

async fn deferred_still_cooling(path: &std::path::Path, options: &Options) -> Result<bool> {
    if !path.exists() {
        return Ok(false);
    }

    let metadata = tokio::fs::metadata(path).await?;
    let modified = metadata.modified().map_err(anyhow::Error::from)?;
    let age_secs = modified
        .elapsed()
        .map(|duration| duration.as_secs())
        .unwrap_or(options.deferred_cooldown_secs);

    if age_secs < options.deferred_cooldown_secs {
        return Ok(true);
    }

    tokio::fs::remove_file(path).await?;
    Ok(false)
}

async fn entry_already_has_grammar_branches(state: &AppState, word: &str) -> Result<bool> {
    sqlx::query_scalar::<_, bool>(
        r#"
        select coalesce(
          jsonb_typeof(analysis->'structured'->'grammar_branches') = 'array'
          and jsonb_array_length(analysis->'structured'->'grammar_branches') > 0,
          false
        )
        from knowledge_entries
        where query_text = $1
        order by id desc
        limit 1
        "#,
    )
    .bind(word)
    .fetch_optional(&state.pool)
    .await
    .map(|value| value.unwrap_or(false))
    .map_err(Into::into)
}

async fn complete_grounded_from_ab_cache_with_retries(
    state: &AppState,
    options: &Options,
    artifact: &AbArtifact,
    dictionary_entry: Option<&de_ai_hilfer::models::DictionaryRaw>,
) -> std::result::Result<(GroundedAnalysis, usize), (anyhow::Error, usize)> {
    let mut transient_retries = 0;
    let mut wait_secs = options.c_retry_initial_delay_secs;

    loop {
        match complete_grounded_from_ab_cache_with_policy(
            state,
            &artifact.cache,
            dictionary_entry,
            QualityMode::Default,
            StructureRetryPolicy::no_retry(),
            options.structure_model.as_deref(),
        )
        .await
        {
            Ok(grounded) => return Ok((grounded, transient_retries)),
            Err(err) => {
                let error_kind = classify_error(&err);
                if !is_retryable_run_c_error(&error_kind)
                    || transient_retries >= options.c_retry_limit
                {
                    return Err((err, transient_retries));
                }

                transient_retries += 1;
                tracing::warn!(
                    "staged backfill Model C transient failure: query={}, kind={}, retry={}/{}, wait_secs={}, err={err:#}",
                    artifact.query_text,
                    error_kind,
                    transient_retries,
                    options.c_retry_limit,
                    wait_secs
                );
                sleep(Duration::from_secs(wait_secs)).await;
                wait_secs = wait_secs
                    .saturating_mul(2)
                    .min(options.c_retry_max_delay_secs);
            }
        }
    }
}

async fn apply_ready(state: &AppState, options: &Options) -> Result<StageReport> {
    let paths = list_artifacts(&options.queue_dir, "ready", options.limit).await?;
    let mut cases = Vec::new();

    for path in paths {
        let artifact: ReadyArtifact = read_json(&path).await?;
        let Some(entry_id) = find_entry_id(state, &artifact.query_text).await? else {
            cases.push(StageCase::failed_message(
                &artifact.query_text,
                "missing_entry",
                "knowledge entry not found",
            ));
            continue;
        };

        let mut analysis_doc: AnalysisDocument = serde_json::from_value(artifact.analysis.clone())?;
        let dictionary_entry =
            dictionary::find_by_headword(&state.pool, &artifact.query_text).await?;
        if analysis_doc.tags.is_empty() {
            if let Some(entry) = dictionary_entry.as_ref() {
                analysis_doc.tags = build_tags(&entry.raw_data, dictionary_pos(&entry.raw_data));
            }
        }
        let lexeme_id =
            dictionary_lexemes::find_unique_lexeme_id_by_surface(&state.pool, &artifact.query_text)
                .await?;
        let analysis = serde_json::to_value(&analysis_doc)?;
        let tags = (!analysis_doc.tags.is_empty()).then_some(analysis_doc.tags.clone());
        let aliases = (!analysis_doc.aliases.is_empty()).then_some(analysis_doc.aliases.clone());

        sqlx::query(
            r#"
            update knowledge_entries
            set lexeme_id = coalesce($2, lexeme_id),
                analysis = $3,
                tags = coalesce($4, tags),
                aliases = coalesce($5, aliases)
            where id = $1
            "#,
        )
        .bind(entry_id)
        .bind(lexeme_id)
        .bind(&analysis)
        .bind(&tags)
        .bind(&aliases)
        .execute(&state.pool)
        .await?;

        let applied_path = QueuePaths::new(&options.queue_dir, &artifact.query_text).applied_path;
        tokio::fs::rename(&path, applied_path).await?;
        cases.push(StageCase::success(&artifact.query_text));
    }

    Ok(StageReport::from_cases(cases))
}

async fn load_words(state: &AppState, options: &Options) -> Result<Vec<String>> {
    if !options.words.is_empty() {
        return Ok(options.words.clone());
    }

    sqlx::query_scalar::<_, String>(
        r#"
        select query_text
        from knowledge_entries
        where entry_type <> 'PHRASE'
          and analysis ? 'markdown'
          and length(coalesce(analysis->>'markdown','')) > 0
          and not coalesce(
            jsonb_typeof(analysis->'structured'->'grammar_branches') = 'array'
            and jsonb_array_length(analysis->'structured'->'grammar_branches') > 0,
            false
          )
        order by id desc
        limit $1
        "#,
    )
    .bind(options.limit)
    .fetch_all(&state.pool)
    .await
    .map_err(Into::into)
}

async fn find_entry_id(state: &AppState, word: &str) -> Result<Option<i64>> {
    sqlx::query_scalar::<_, i64>(
        r#"
        select id
        from knowledge_entries
        where query_text = $1
        order by id desc
        limit 1
        "#,
    )
    .bind(word)
    .fetch_optional(&state.pool)
    .await
    .map_err(Into::into)
}
