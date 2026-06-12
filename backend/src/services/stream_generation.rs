use crate::ai::{is_hard_failure, stream_chat_response, AiChatOptions, AiClient};
use crate::services::stream_response::sse_delta;
use tokio::sync::mpsc::UnboundedSender;

pub const HARD_FAILURE_RETRY_CYCLES: usize = 2;
pub const HARD_FAILURE_RETRY_DELAY_MS: u64 = 1200;

pub enum StreamModelOutcome {
    Success(String),
    Canceled,
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
    if tx.is_closed() {
        return StreamModelOutcome::Canceled;
    }

    let response = tokio::select! {
        response = client.chat_model_stream_with_options(model, system_prompt, user_message, options) => response,
        _ = tx.closed() => return StreamModelOutcome::Canceled,
    };

    match response {
        Ok(response) => {
            let mut emitted_delta = false;
            let mut receiver_dropped = false;
            match stream_chat_response(response, |delta| {
                emitted_delta = true;
                if tx.send(sse_delta(&delta)).is_err() {
                    receiver_dropped = true;
                    return Err(anyhow::anyhow!("stream receiver dropped"));
                }
                Ok(())
            })
            .await
            {
                Ok(markdown) => StreamModelOutcome::Success(markdown),
                Err(_) if receiver_dropped => StreamModelOutcome::Canceled,
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
