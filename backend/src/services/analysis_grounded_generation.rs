use crate::ai::{is_hard_failure, AiChatOptions, AiClient};
use crate::models::{AiModelOverride, QualityMode};
use crate::services::ai_model_resolver::{resolve_task_model_with_override, AiModelTask};
use crate::services::analysis_grounded_facts::{
    build_light_dictionary_facts_payload, load_raw_rows_by_headword,
};
use crate::services::analysis_grounded_model_a::{normalize_model_a_output, ModelAOutput};
use crate::services::analysis_grounded_prompt::{
    build_model_a_prompt, build_model_a_user_payload, build_stage2_prompt,
    build_stage2_user_payload,
};
use crate::services::analyze_support::extract_json;
use crate::services::stream_generation::{
    request_and_stream_model, StreamModelOutcome, HARD_FAILURE_RETRY_CYCLES,
    HARD_FAILURE_RETRY_DELAY_MS,
};
use crate::state::AppState;
use anyhow::{anyhow, Result};
use std::time::{Duration, Instant};
use tokio::sync::mpsc::UnboundedSender;

pub(crate) struct StreamedMarkdown {
    pub markdown: String,
    pub model: String,
}

fn model_a_chat_options() -> AiChatOptions {
    AiChatOptions {
        temperature: 0.1,
        max_tokens: Some(2200),
        timeout: Duration::from_secs(180),
    }
}

fn stage2_chat_options(_quality_mode: QualityMode) -> AiChatOptions {
    AiChatOptions {
        temperature: 0.2,
        max_tokens: Some(1800),
        timeout: Duration::from_secs(90),
    }
}

pub(crate) async fn generate_model_a(
    state: &AppState,
    target_query: &str,
    _quality_mode: QualityMode,
    model_override: Option<&AiModelOverride>,
) -> Result<GroundedStage1> {
    let raw_rows = load_raw_rows_by_headword(&state.pool, target_query).await?;
    let dictionary_facts = (!raw_rows.is_empty())
        .then(|| build_light_dictionary_facts_payload(target_query, &raw_rows));
    let prompt = build_model_a_prompt(&state.prompts);
    let user_payload = build_model_a_user_payload(target_query, dictionary_facts.as_deref());
    let resolved =
        resolve_task_model_with_override(state, AiModelTask::Analyze, model_override).await?;
    let started_at = Instant::now();
    let result = resolved
        .client
        .chat_model_with_options(
            &resolved.model,
            &prompt,
            &user_payload,
            model_a_chat_options(),
        )
        .await;
    let elapsed_ms = started_at.elapsed().as_millis();
    let raw = match result {
        Ok(raw) => {
            tracing::info!(
                "grounded model A complete: target={target_query}, model={}, elapsed_ms={elapsed_ms}",
                resolved.model
            );
            raw
        }
        Err(err) => {
            tracing::warn!(
                "grounded model A failed: target={target_query}, model={}, elapsed_ms={elapsed_ms}, err={err:#}",
                resolved.model
            );
            return Err(err);
        }
    };
    let output = extract_json::<ModelAOutput>(&raw).map(normalize_model_a_output)?;
    if output.entries.is_empty() {
        return Err(anyhow!("model A returned no lexical branches"));
    }
    Ok(GroundedStage1 {
        dictionary_facts,
        output,
    })
}

pub(crate) async fn generate_stage2_markdown(
    ai_client: &AiClient,
    prompts: &crate::prompts::PromptConfig,
    target_query: &str,
    dictionary_facts: Option<&str>,
    stage1_output: &ModelAOutput,
    primary_model: &str,
    fallback_model: &str,
    quality_mode: QualityMode,
    generation_hint: Option<&str>,
) -> Result<StreamedMarkdown> {
    let prompt = build_stage2_prompt(prompts);
    let user_payload = build_stage2_user_payload(
        target_query,
        dictionary_facts,
        stage1_output,
        generation_hint,
    );
    let options = stage2_chat_options(quality_mode);

    for cycle in 1..=HARD_FAILURE_RETRY_CYCLES {
        match ai_client
            .chat_model_with_options(primary_model, &prompt, &user_payload, options)
            .await
        {
            Ok(markdown) => {
                return Ok(StreamedMarkdown {
                    markdown,
                    model: primary_model.to_string(),
                });
            }
            Err(primary_err)
                if cycle < HARD_FAILURE_RETRY_CYCLES && is_hard_failure(&primary_err) =>
            {
                tracing::warn!(
                    "grounded stage2 retrying after primary hard failure: cycle={cycle}/{HARD_FAILURE_RETRY_CYCLES}, primary={primary_model}, target={target_query}, err={primary_err:#}"
                );
                tokio::time::sleep(Duration::from_millis(HARD_FAILURE_RETRY_DELAY_MS)).await;
            }
            Err(primary_err)
                if is_hard_failure(&primary_err)
                    && !fallback_model.is_empty()
                    && fallback_model != primary_model =>
            {
                tracing::warn!(
                    "grounded stage2 switching to fallback: primary={primary_model}, fallback={fallback_model}, target={target_query}, err={primary_err:#}"
                );
                match ai_client
                    .chat_model_with_options(fallback_model, &prompt, &user_payload, options)
                    .await
                {
                    Ok(markdown) => {
                        return Ok(StreamedMarkdown {
                            markdown,
                            model: fallback_model.to_string(),
                        });
                    }
                    Err(fallback_err) => return Err(fallback_err),
                }
            }
            Err(primary_err) => return Err(primary_err),
        }
    }

    unreachable!("grounded stage2 retry loop should always return or error");
}

pub(crate) async fn stream_stage2_markdown(
    ai_client: &AiClient,
    tx: &UnboundedSender<String>,
    prompts: &crate::prompts::PromptConfig,
    target_query: &str,
    dictionary_facts: Option<&str>,
    stage1_output: &ModelAOutput,
    primary_model: &str,
    fallback_model: &str,
    quality_mode: QualityMode,
    generation_hint: Option<&str>,
) -> Result<StreamedMarkdown> {
    let prompt = build_stage2_prompt(prompts);
    let user_payload = build_stage2_user_payload(
        target_query,
        dictionary_facts,
        stage1_output,
        generation_hint,
    );
    let options = stage2_chat_options(quality_mode);

    for cycle in 1..=HARD_FAILURE_RETRY_CYCLES {
        match request_and_stream_model(
            ai_client,
            primary_model,
            &prompt,
            &user_payload,
            options,
            tx,
        )
        .await
        {
            StreamModelOutcome::Success(markdown) => {
                return Ok(StreamedMarkdown {
                    markdown,
                    model: primary_model.to_string(),
                });
            }
            StreamModelOutcome::Canceled => {
                return Err(anyhow!("stream analyze canceled by client"));
            }
            StreamModelOutcome::Retriable(err)
                if !fallback_model.is_empty() && fallback_model != primary_model =>
            {
                tracing::warn!(
                    "grounded stream stage2 primary failed before deltas; switching to fallback: cycle={cycle}/{HARD_FAILURE_RETRY_CYCLES}, primary={primary_model}, fallback={fallback_model}, target={target_query}, err={err:#}"
                );
                match request_and_stream_model(
                    ai_client,
                    fallback_model,
                    &prompt,
                    &user_payload,
                    options,
                    tx,
                )
                .await
                {
                    StreamModelOutcome::Canceled => {
                        return Err(anyhow!("stream analyze canceled by client"));
                    }
                    StreamModelOutcome::Success(markdown) => {
                        return Ok(StreamedMarkdown {
                            markdown,
                            model: fallback_model.to_string(),
                        });
                    }
                    StreamModelOutcome::Retriable(fallback_err)
                        if cycle < HARD_FAILURE_RETRY_CYCLES =>
                    {
                        tracing::warn!(
                            "grounded stream stage2 retrying after fallback hard failure: cycle={cycle}/{HARD_FAILURE_RETRY_CYCLES}, target={target_query}, err={fallback_err:#}"
                        );
                        tokio::time::sleep(Duration::from_millis(HARD_FAILURE_RETRY_DELAY_MS))
                            .await;
                    }
                    StreamModelOutcome::Retriable(fallback_err)
                    | StreamModelOutcome::Fatal(fallback_err) => return Err(fallback_err),
                }
            }
            StreamModelOutcome::Retriable(err) if cycle < HARD_FAILURE_RETRY_CYCLES => {
                tracing::warn!(
                    "grounded stream stage2 retrying after primary hard failure: cycle={cycle}/{HARD_FAILURE_RETRY_CYCLES}, primary={primary_model}, target={target_query}, err={err:#}"
                );
                tokio::time::sleep(Duration::from_millis(HARD_FAILURE_RETRY_DELAY_MS)).await;
            }
            StreamModelOutcome::Retriable(err) | StreamModelOutcome::Fatal(err) => return Err(err),
        }
    }

    unreachable!("grounded stream stage2 retry loop should always return or error");
}

pub(crate) struct GroundedStage1 {
    pub dictionary_facts: Option<String>,
    pub output: ModelAOutput,
}
