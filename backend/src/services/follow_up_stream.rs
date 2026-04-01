use crate::ai::{is_hard_failure, stream_chat_response, AiChatOptions};
use crate::models::{
    FollowUpRequest, FollowUpResponse, NewFollowUp, QualityMode, StreamMetaPayload,
};
use crate::repositories::{follow_up, knowledge};
use crate::services::follow_up_fallback::{build_follow_up_fallback, normalize_answer};
use crate::services::follow_up_prompt::build_follow_up_prompt;
use crate::services::stream_response::{sse_complete, sse_delta, sse_meta};
use crate::state::AppState;
use anyhow::Result;
use std::time::Duration;
use tokio::sync::mpsc::UnboundedSender;

pub async fn stream_follow_up(
    state: AppState,
    request: FollowUpRequest,
    tx: UnboundedSender<String>,
) -> Result<()> {
    let question = request.question.trim();
    anyhow::ensure!(!question.is_empty(), "question cannot be empty");

    let entry = knowledge::find_by_id(&state.pool, request.entry_id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("knowledge entry not found"))?;
    let history = follow_up::list_by_entry_id(&state.pool, request.entry_id).await?;
    let vocabulary_list = knowledge::list_query_texts(&state.pool, 8).await?;
    let system_prompt = build_follow_up_prompt(
        &state.prompts.follow_up_prompt,
        &vocabulary_list,
        question,
        &entry.analysis,
        &history,
    );

    let primary_model = match request.quality_mode {
        QualityMode::Default => state.config.ai_models.follow_up.as_str(),
        QualityMode::Pro => state.config.ai_models.follow_up_pro.as_str(),
    };
    let fallback_model = match request.quality_mode {
        QualityMode::Default => state.config.ai_models.fallback_fast.as_str(),
        QualityMode::Pro => state.config.ai_models.fallback_pro.as_str(),
    };
    let source = if request.quality_mode == QualityMode::Pro {
        "Pro"
    } else {
        "Flash"
    };

    tx.send(sse_meta(&StreamMetaPayload {
        kind: "follow_up".to_string(),
        model: primary_model.to_string(),
        quality_mode: request.quality_mode,
        source: source.to_string(),
        fallback: false,
    }))
    .ok();

    let streamed = match state
        .ai_client
        .chat_model_stream_with_options(
            primary_model,
            &system_prompt,
            question,
            AiChatOptions {
                temperature: 0.1,
                max_tokens: 220,
                timeout: Duration::from_secs(20),
            },
        )
        .await
    {
        Ok(response) => Some(StreamedAnswer {
            answer: normalize_answer(
                &stream_chat_response(response, |delta| {
                    tx.send(sse_delta(&delta))
                        .map_err(|_| anyhow::anyhow!("stream receiver dropped"))?;
                    Ok(())
                })
                .await?,
            ),
            model: primary_model.to_string(),
        }),
        Err(err)
            if is_hard_failure(&err)
                && !fallback_model.is_empty()
                && fallback_model != primary_model =>
        {
            tracing::warn!(
                "follow-up stream switching to fallback model: primary={primary_model}, fallback={fallback_model}, err={err:#}"
            );
            tx.send(sse_meta(&StreamMetaPayload {
                kind: "follow_up".to_string(),
                model: fallback_model.to_string(),
                quality_mode: request.quality_mode,
                source: source.to_string(),
                fallback: true,
            }))
            .ok();
            let response = state
                .ai_client
                .chat_model_stream_with_options(
                    fallback_model,
                    &system_prompt,
                    question,
                    AiChatOptions {
                        temperature: 0.1,
                        max_tokens: 220,
                        timeout: Duration::from_secs(20),
                    },
                )
                .await?;
            Some(StreamedAnswer {
                answer: normalize_answer(
                    &stream_chat_response(response, |delta| {
                        tx.send(sse_delta(&delta))
                            .map_err(|_| anyhow::anyhow!("stream receiver dropped"))?;
                        Ok(())
                    })
                    .await?,
                ),
                model: fallback_model.to_string(),
            })
        }
        Err(err) => {
            tracing::warn!(
                "follow-up stream fallback to deterministic answer: entry_id={}, err={err:#}",
                request.entry_id
            );
            None
        }
    };

    let (answer, model) = match streamed {
        Some(streamed) => (streamed.answer, Some(streamed.model)),
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

    tx.send(sse_complete(&FollowUpResponse {
        answer,
        follow_up: record,
        model,
        quality_mode: Some(request.quality_mode),
    }))
    .ok();
    Ok(())
}

struct StreamedAnswer {
    answer: String,
    model: String,
}
