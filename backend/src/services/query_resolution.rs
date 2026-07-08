use crate::models::{AnalysisDocument, AttachedPhraseModule, PhraseLookupInfo, PhraseUsagePreview};
use serde_json::Value;

pub fn phrase_lookup_from_analysis(analysis: &Value) -> Option<PhraseLookupInfo> {
    analysis
        .get("phrase_lookup")
        .cloned()
        .and_then(|value| serde_json::from_value(value).ok())
}

pub fn attached_phrase_modules_from_analysis(analysis: &Value) -> Vec<AttachedPhraseModule> {
    analysis
        .get("attached_phrase_modules")
        .cloned()
        .and_then(|value| serde_json::from_value(value).ok())
        .unwrap_or_default()
}

pub fn phrase_usage_preview_from_analysis(analysis: &Value) -> Option<PhraseUsagePreview> {
    analysis
        .get("phrase_usage_preview")
        .cloned()
        .and_then(|value| serde_json::from_value(value).ok())
}

pub fn build_phrase_unavailable_analysis(
    phrase: &str,
    phrase_lookup: Option<&PhraseLookupInfo>,
) -> AnalysisDocument {
    let host_line = phrase_lookup
        .and_then(|lookup| lookup.best_host_headword.as_deref())
        .map(|host| format!("- 当前已锁定的候选主词：{host}"))
        .unwrap_or_else(|| "- 当前尚未锁定可靠主词。".to_string());

    AnalysisDocument {
        markdown: format!(
            "## {phrase}\n\n- 这是一次短语/搭配查询。\n{host_line}\n- 但本轮模型暂时不可用，无法稳定生成该短语的场景化解释。\n- 建议：稍后重试，或先切换查看候选主词。"
        ),
        structured: None,
        tags: vec!["短语待确认".to_string()],
        aliases: Vec::new(),
        prototype: phrase_lookup.and_then(|lookup| lookup.best_host_headword.clone()),
        phrase_lookup: phrase_lookup.cloned(),
        phrase_usage_preview: None,
        attached_phrase_modules: Vec::new(),
        dictionary_excerpt: None,
        model: None,
        quality_mode: None,
    }
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
        phrase_lookup: None,
        phrase_usage_preview: None,
        attached_phrase_modules: Vec::new(),
        dictionary_excerpt: None,
        model: None,
        quality_mode: None,
    }
}
