use crate::models::{QualityMode, StreamMetaPayload};
use crate::services::analysis_preview::analysis_markdown;
use crate::services::stream_response::sse_meta;
use serde_json::Value;
use tokio::sync::mpsc::UnboundedSender;

pub struct StreamedMarkdown {
    pub markdown: String,
    pub model: String,
}

pub fn send_meta(
    tx: &UnboundedSender<String>,
    kind: &str,
    model: &str,
    quality_mode: QualityMode,
    source: &str,
    fallback: bool,
) {
    tx.send(sse_meta(&StreamMetaPayload {
        kind: kind.to_string(),
        model: model.to_string(),
        quality_mode,
        source: source.to_string(),
        fallback,
    }))
    .ok();
}

pub fn cached_analysis_markdown(analysis: &Value) -> String {
    analysis_markdown(analysis)
}
