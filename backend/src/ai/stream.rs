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
    let mut completed = false;

    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        buffer.push_str(&String::from_utf8_lossy(&chunk));

        while let Some(separator) = buffer.find("\n\n") {
            let raw_event = buffer[..separator].to_string();
            buffer.drain(..separator + 2);

            if let Some(delta) = parse_sse_event(&raw_event, &mut completed)? {
                if !delta.is_empty() {
                    content.push_str(&delta);
                    on_delta(delta)?;
                }
            }
        }
    }

    let trailing = buffer.trim();
    if !trailing.is_empty() {
        if let Some(delta) = parse_sse_event(trailing, &mut completed)? {
            if !delta.is_empty() {
                content.push_str(&delta);
                on_delta(delta)?;
            }
        }
    }

    anyhow::ensure!(completed, "AI stream ended without a completion event");
    Ok(content)
}

fn parse_sse_event(raw_event: &str, completed: &mut bool) -> Result<Option<String>> {
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
        *completed = true;
        return Ok(None);
    }

    let parsed: ChatCompletionChunk = serde_json::from_str(&payload)
        .map_err(|err| anyhow!("failed to parse streaming chunk: {err}; payload={payload}"))?;

    for choice in &parsed.choices {
        if let Some(reason) = &choice.finish_reason {
            anyhow::ensure!(
                reason == "stop",
                "AI stream did not complete: finish_reason={reason}"
            );
            *completed = true;
        }
    }
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
    #[serde(default)]
    finish_reason: Option<String>,
}

#[derive(Deserialize)]
struct ChunkDelta {
    content: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn completion_is_transport_metadata_not_sentence_punctuation() {
        let mut completed = false;
        let delta = parse_sse_event(
            r#"data: {"choices":[{"delta":{"content":"完整的无句号列表"},"finish_reason":null}]}"#,
            &mut completed,
        )
        .unwrap();
        assert_eq!(delta.as_deref(), Some("完整的无句号列表"));
        assert!(!completed);
        assert!(parse_sse_event(
            r#"data: {"choices":[{"delta":{},"finish_reason":"length"}]}"#,
            &mut completed
        )
        .is_err());
        assert!(!completed);
        parse_sse_event(
            r#"data: {"choices":[{"delta":{},"finish_reason":"stop"}]}"#,
            &mut completed,
        )
        .unwrap();
        assert!(completed);
        completed = false;
        parse_sse_event("data: [DONE]", &mut completed).unwrap();
        assert!(completed);
    }
}
