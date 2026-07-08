use crate::models::PhraseUsagePreview;
use crate::services::json_extract_repair::{
    json_candidate, repair_json_inner_quotes, repair_truncated_json,
};
use anyhow::Result;
use serde::Deserialize;

#[derive(Clone, Copy)]
pub enum AnalysisMode {
    Full,
    Compact,
}

pub fn render_phrase_preview_markdown(phrase: &str, preview: &PhraseUsagePreview) -> String {
    let meaning_en = if preview.meaning_en.trim().is_empty() {
        String::new()
    } else {
        format!(" * *{}*", preview.meaning_en.trim())
    };

    format!(
        "### {phrase}\n\n#### 核心释义 (Bedeutung)\n* **Phrase** **{}**{}\n\n#### 应用与例句 (Anwendung & Beispiele)\n\n{}: {}\n{}\n（{}）",
        preview.meaning_zh.trim(),
        meaning_en,
        preview.usage_module.title.trim(),
        preview.usage_module.explanation.trim(),
        preview.usage_module.example_de.trim(),
        preview.usage_module.example_zh.trim(),
    )
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JsonRepairKind {
    InnerQuotes,
    InnerQuotesAndTruncation,
    TruncatedPayload,
}

impl JsonRepairKind {
    pub fn as_str(self) -> &'static str {
        match self {
            JsonRepairKind::InnerQuotes => "inner_quotes",
            JsonRepairKind::InnerQuotesAndTruncation => "inner_quotes_and_truncation",
            JsonRepairKind::TruncatedPayload => "truncated_payload",
        }
    }
}

#[derive(Debug)]
pub struct JsonExtractOutcome<T> {
    pub value: T,
    pub repair: Option<JsonRepairKind>,
    pub raw_len: usize,
    pub candidate_len: usize,
    pub repaired_len: Option<usize>,
}

pub fn extract_json<T: for<'de> Deserialize<'de>>(raw: &str) -> Result<T> {
    let outcome = extract_json_with_report(raw)?;
    if let Some(repair) = outcome.repair {
        tracing::warn!(
            "extract_json repaired {}: raw_len={}, candidate_len={}, repaired_len={}",
            repair.as_str(),
            outcome.raw_len,
            outcome.candidate_len,
            outcome.repaired_len.unwrap_or(0)
        );
    }
    Ok(outcome.value)
}

pub fn extract_json_with_report<T: for<'de> Deserialize<'de>>(
    raw: &str,
) -> Result<JsonExtractOutcome<T>> {
    let candidate = json_candidate(raw);
    let raw_len = raw.len();
    let candidate_len = candidate.len();

    if let Ok(parsed) = serde_json::from_str(candidate) {
        return Ok(JsonExtractOutcome {
            value: parsed,
            repair: None,
            raw_len,
            candidate_len,
            repaired_len: None,
        });
    }

    if let Some(repaired) = repair_json_inner_quotes(candidate) {
        if let Ok(parsed) = serde_json::from_str(&repaired) {
            return Ok(JsonExtractOutcome {
                value: parsed,
                repair: Some(JsonRepairKind::InnerQuotes),
                raw_len,
                candidate_len,
                repaired_len: Some(repaired.len()),
            });
        }
    }

    if let Some(repaired) =
        repair_json_inner_quotes(candidate).and_then(|repaired| repair_truncated_json(&repaired))
    {
        if let Ok(parsed) = serde_json::from_str(&repaired) {
            return Ok(JsonExtractOutcome {
                value: parsed,
                repair: Some(JsonRepairKind::InnerQuotesAndTruncation),
                raw_len,
                candidate_len,
                repaired_len: Some(repaired.len()),
            });
        }
    }

    if let Some(repaired) = repair_truncated_json(candidate) {
        if let Ok(parsed) = serde_json::from_str(&repaired) {
            return Ok(JsonExtractOutcome {
                value: parsed,
                repair: Some(JsonRepairKind::TruncatedPayload),
                raw_len,
                candidate_len,
                repaired_len: Some(repaired.len()),
            });
        }
    }

    Ok(JsonExtractOutcome {
        value: serde_json::from_str(candidate)?,
        repair: None,
        raw_len,
        candidate_len,
        repaired_len: None,
    })
}
