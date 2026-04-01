use anyhow::{anyhow, Result};
use futures_util::StreamExt;
use serde::Deserialize;

pub fn is_hard_failure(err: &anyhow::Error) -> bool {
    if let Some(reqwest_err) = err
        .chain()
        .find_map(|cause| cause.downcast_ref::<reqwest::Error>())
    {
        if reqwest_err.is_timeout() || reqwest_err.is_connect() || reqwest_err.is_request() {
            return true;
        }

        if let Some(status) = reqwest_err.status() {
            return status.as_u16() == 429 || status.is_server_error();
        }
    }

    let message = err.to_string().to_lowercase();
    message.contains("timed out")
        || message.contains("deadline has elapsed")
        || message.contains("connection")
        || message.contains("429")
        || message.contains("500")
        || message.contains("502")
        || message.contains("503")
        || message.contains("504")
}

pub async fn stream_chat_response<F>(response: reqwest::Response, mut on_delta: F) -> Result<String>
where
    F: FnMut(String) -> Result<()>,
{
    let mut stream = response.bytes_stream();
    let mut buffer = String::new();
    let mut content = String::new();

    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        buffer.push_str(&String::from_utf8_lossy(&chunk));

        while let Some(separator) = buffer.find("\n\n") {
            let raw_event = buffer[..separator].to_string();
            buffer.drain(..separator + 2);

            if let Some(delta) = parse_sse_event(&raw_event)? {
                if !delta.is_empty() {
                    content.push_str(&delta);
                    on_delta(delta)?;
                }
            }
        }
    }

    let trailing = buffer.trim();
    if !trailing.is_empty() {
        if let Some(delta) = parse_sse_event(trailing)? {
            if !delta.is_empty() {
                content.push_str(&delta);
                on_delta(delta)?;
            }
        }
    }

    Ok(content)
}

fn parse_sse_event(raw_event: &str) -> Result<Option<String>> {
    let payload = raw_event
        .lines()
        .filter_map(|line| line.strip_prefix("data:"))
        .map(str::trim)
        .collect::<Vec<_>>()
        .join("\n");

    if payload.is_empty() {
        return Ok(None);
    }

    if payload == "[DONE]" {
        return Ok(None);
    }

    let parsed: ChatCompletionChunk = serde_json::from_str(&payload)
        .map_err(|err| anyhow!("failed to parse streaming chunk: {err}; payload={payload}"))?;

    Ok(parsed
        .choices
        .into_iter()
        .filter_map(|choice| choice.delta.content)
        .reduce(|mut acc, item| {
            acc.push_str(&item);
            acc
        }))
}

#[derive(Deserialize)]
struct ChatCompletionChunk {
    choices: Vec<ChunkChoice>,
}

#[derive(Deserialize)]
struct ChunkChoice {
    delta: ChunkDelta,
}

#[derive(Deserialize)]
struct ChunkDelta {
    content: Option<String>,
}
