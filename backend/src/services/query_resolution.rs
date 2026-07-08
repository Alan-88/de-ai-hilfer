use crate::models::{AnalysisDocument, AttachedPhraseModule};
use serde_json::Value;

pub fn attached_phrase_modules_from_analysis(analysis: &Value) -> Vec<AttachedPhraseModule> {
    analysis
        .get("attached_phrase_modules")
        .cloned()
        .and_then(|value| serde_json::from_value(value).ok())
        .unwrap_or_default()
}

pub fn build_no_candidate_analysis(query: &str) -> AnalysisDocument {
    AnalysisDocument {
        markdown: format!(
            "## 未找到可靠候选\n\n- `{query}` 没有命中字典、知识库或可还原词形。\n- 普通查词不会再自动调用 AI 猜测原型，避免把一次搜索悄悄路由到不可控结果。\n- 可以从候选列表中选择最接近的一项，或使用 Tab 联想搜索补充语义线索。"
        ),
        structured: None,
        tags: vec!["未找到可靠候选".to_string()],
        aliases: Vec::new(),
        prototype: None,
        attached_phrase_modules: Vec::new(),
        dictionary_excerpt: None,
        model: None,
        quality_mode: None,
    }
}
