use crate::ai::{is_hard_failure, AiChatOptions, AiClient};
use anyhow::{Error, Result};
use std::time::Duration;

const STRUCTURE_TRANSIENT_RETRIES: usize = 2;
const STRUCTURE_TRANSIENT_INITIAL_DELAY_SECS: u64 = 45;
const STRUCTURE_TRANSIENT_MAX_DELAY_SECS: u64 = 180;

#[derive(Debug, Clone, Copy)]
pub struct StructureRetryPolicy {
    pub max_retries: usize,
    pub initial_delay: Duration,
    pub max_delay: Duration,
}

impl StructureRetryPolicy {
    pub const fn runtime_default() -> Self {
        Self {
            max_retries: STRUCTURE_TRANSIENT_RETRIES,
            initial_delay: Duration::from_secs(STRUCTURE_TRANSIENT_INITIAL_DELAY_SECS),
            max_delay: Duration::from_secs(STRUCTURE_TRANSIENT_MAX_DELAY_SECS),
        }
    }

    pub const fn no_retry() -> Self {
        Self {
            max_retries: 0,
            initial_delay: Duration::from_secs(0),
            max_delay: Duration::from_secs(0),
        }
    }
}

pub async fn generate_structure_with_transient_retries(
    ai_client: &AiClient,
    target_query: &str,
    model: &str,
    structure_prompt: &str,
    structure_user_payload: &str,
    options: AiChatOptions,
) -> Result<String> {
    generate_structure_with_retry_policy(
        ai_client,
        target_query,
        model,
        structure_prompt,
        structure_user_payload,
        options,
        StructureRetryPolicy::runtime_default(),
    )
    .await
}

pub async fn generate_structure_with_retry_policy(
    ai_client: &AiClient,
    target_query: &str,
    model: &str,
    structure_prompt: &str,
    structure_user_payload: &str,
    options: AiChatOptions,
    policy: StructureRetryPolicy,
) -> Result<String> {
    let mut transient_retries = 0;
    let mut wait = policy.initial_delay;

    loop {
        match ai_client
            .chat_model_with_options(model, structure_prompt, structure_user_payload, options)
            .await
        {
            Ok(raw) => return Ok(raw),
            Err(err) => {
                if !is_structure_transient_error(&err) || transient_retries >= policy.max_retries {
                    return Err(err);
                }

                transient_retries += 1;
                tracing::warn!(
                    "grounded structure transient failure: target={target_query}, model={model}, retry={transient_retries}/{}, wait_secs={}, err={err:#}",
                    policy.max_retries,
                    wait.as_secs()
                );
                tokio::time::sleep(wait).await;
                wait = wait.saturating_mul(2).min(policy.max_delay);
            }
        }
    }
}

fn is_structure_transient_error(err: &Error) -> bool {
    if is_hard_failure(err) {
        return true;
    }

    let message = format!("{err:#}").to_ascii_lowercase();
    [
        "rate limit",
        "rate_limit",
        "too many requests",
        "temporarily unavailable",
        "overloaded",
        "empty response",
        "did not contain message content",
        "timeout",
        "timed out",
        "504",
        "503",
        "502",
        "429",
    ]
    .iter()
    .any(|needle| message.contains(needle))
}
