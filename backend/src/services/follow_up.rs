use crate::ai::{is_hard_failure, AiChatOptions};
use crate::models::{FollowUpRequest, FollowUpResponse, NewFollowUp, QualityMode};
use crate::repositories::{follow_up, knowledge};
use crate::services::follow_up_fallback::{build_follow_up_fallback, normalize_answer};
use crate::services::follow_up_prompt::build_follow_up_prompt;
use crate::state::AppState;
use anyhow::{anyhow, Result};
use std::time::Duration;

pub async fn create(state: &AppState, request: FollowUpRequest) -> Result<FollowUpResponse> {
    let question = request.question.trim();
    anyhow::ensure!(!question.is_empty(), "question cannot be empty");

    let entry = knowledge::find_by_id(&state.pool, request.entry_id)
        .await?
        .ok_or_else(|| anyhow!("knowledge entry not found"))?;
    let history = follow_up::list_by_entry_id(&state.pool, request.entry_id).await?;
    let vocabulary_list = knowledge::list_query_texts(&state.pool, 8).await?;
    let system_prompt = build_follow_up_prompt(
        &state.prompts.follow_up_prompt,
        &vocabulary_list,
        question,
        &entry.analysis,
        &history,
    );

    let primary_model = primary_model_for(state, request.quality_mode);
    let fallback_model = fallback_model_for(state, request.quality_mode);
    let result = match state
        .ai_client
        .chat_model_with_options(
            primary_model,
            &system_prompt,
            question,
            AiChatOptions {
                temperature: 0.1,
                max_tokens: 220,
                timeout: Duration::from_secs(12),
            },
        )
        .await
    {
        Ok(answer) => Some((normalize_answer(&answer), primary_model.to_string())),
        Err(err) if is_hard_failure(&err) && fallback_ready(primary_model, fallback_model) => {
            tracing::warn!(
                "follow-up switching to fallback model: primary={primary_model}, fallback={fallback_model}, entry_id={}, err={err:#}",
                request.entry_id
            );
            let answer = state
                .ai_client
                .chat_model_with_options(
                    fallback_model,
                    &system_prompt,
                    question,
                    AiChatOptions {
                        temperature: 0.1,
                        max_tokens: 220,
                        timeout: Duration::from_secs(12),
                    },
                )
                .await?;
            Some((normalize_answer(&answer), fallback_model.to_string()))
        }
        Err(err) => {
            tracing::warn!(
                "follow-up fallback to deterministic answer: entry_id={}, question={question}, err={err:#}",
                request.entry_id
            );
            None
        }
    };

    let (answer, model) = match result {
        Some((answer, model)) => (answer, Some(model)),
        None => (
            build_follow_up_fallback(&entry.query_text, question, &entry.analysis),
            None,
        ),
    };

    let record = follow_up::insert(
        &state.pool,
        &NewFollowUp {
            entry_id: request.entry_id,
            question: question.to_string(),
            answer: answer.clone(),
        },
    )
    .await?;

    Ok(FollowUpResponse {
        answer,
        follow_up: record,
        model,
        quality_mode: Some(request.quality_mode),
    })
}

fn primary_model_for(state: &AppState, quality_mode: QualityMode) -> &str {
    match quality_mode {
        QualityMode::Default => state.config.ai_models.follow_up.as_str(),
        QualityMode::Pro => state.config.ai_models.follow_up_pro.as_str(),
    }
}

fn fallback_model_for(state: &AppState, quality_mode: QualityMode) -> &str {
    match quality_mode {
        QualityMode::Default => state.config.ai_models.fallback_fast.as_str(),
        QualityMode::Pro => state.config.ai_models.fallback_pro.as_str(),
    }
}

fn fallback_ready(primary_model: &str, fallback_model: &str) -> bool {
    !fallback_model.is_empty() && fallback_model != primary_model
}
