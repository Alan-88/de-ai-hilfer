use crate::models::{StreamDeltaPayload, StreamErrorPayload, StreamMetaPayload};
use serde::Serialize;

pub fn sse_meta(payload: &StreamMetaPayload) -> String {
    sse_event("meta", payload)
}

pub fn sse_delta(delta: &str) -> String {
    sse_event(
        "delta",
        &StreamDeltaPayload {
            delta: delta.to_string(),
        },
    )
}

pub fn sse_complete<T: Serialize>(payload: &T) -> String {
    sse_event("complete", payload)
}

pub fn sse_error(message: impl Into<String>) -> String {
    sse_event(
        "error",
        &StreamErrorPayload {
            message: message.into(),
        },
    )
}

fn sse_event<T: Serialize>(event: &str, payload: &T) -> String {
    let json = serde_json::to_string(payload)
        .unwrap_or_else(|_| "{\"message\":\"serialization failed\"}".to_string());
    format!("event: {event}\ndata: {json}\n\n")
}
