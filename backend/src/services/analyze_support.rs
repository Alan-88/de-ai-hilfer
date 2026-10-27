use crate::services::json_extract_repair::{json_candidate, repair_json_inner_quotes};
use anyhow::Result;
use serde::Deserialize;

#[derive(Clone, Copy)]
pub enum AnalysisMode {
    Full,
    Compact,
}

pub fn extract_json<T: for<'de> Deserialize<'de>>(raw: &str) -> Result<T> {
    let candidate = json_candidate(raw);
    let parsed = serde_json::from_str(candidate);
    if parsed.is_ok() {
        return Ok(parsed?);
    }
    if let Some(escaped) = repair_json_inner_quotes(candidate) {
        if let Ok(value) = serde_json::from_str(&escaped) {
            tracing::warn!("extract_json escaped inner quotes: raw_len={}", raw.len());
            return Ok(value);
        }
    }
    Ok(parsed?)
}
