use crate::ai::{is_hard_failure, stream_chat_response, AiChatOptions, AiClient};
use crate::services::stream_response::sse_delta;
use tokio::sync::mpsc::UnboundedSender;

pub const HARD_FAILURE_RETRY_CYCLES: usize = 2;
pub const HARD_FAILURE_RETRY_DELAY_MS: u64 = 1200;

pub enum StreamModelOutcome {
    Success(String),
    Retriable(anyhow::Error),
    Fatal(anyhow::Error),
}

pub async fn request_and_stream_model(
    client: &AiClient,
    model: &str,
    system_prompt: &str,
    user_message: &str,
    options: AiChatOptions,
    tx: &UnboundedSender<String>,
) -> StreamModelOutcome {
    match client
        .chat_model_stream_with_options(model, system_prompt, user_message, options)
        .await
    {
        Ok(response) => {
            let mut emitted_delta = false;
            match stream_chat_response(response, |delta| {
                emitted_delta = true;
                tx.send(sse_delta(&delta))
                    .map_err(|_| anyhow::anyhow!("stream receiver dropped"))?;
                Ok(())
            })
            .await
            {
                Ok(markdown) => StreamModelOutcome::Success(markdown),
                Err(err) if !emitted_delta && is_hard_failure(&err) => {
                    StreamModelOutcome::Retriable(err)
                }
                Err(err) => StreamModelOutcome::Fatal(err),
            }
        }
        Err(err) if is_hard_failure(&err) => StreamModelOutcome::Retriable(err),
        Err(err) => StreamModelOutcome::Fatal(err),
    }
}
