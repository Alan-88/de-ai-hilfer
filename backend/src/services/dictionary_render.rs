use crate::models::AnalysisDocument;
use crate::services::dictionary_examples::{collect_examples, usage_note};
use crate::services::dictionary_facts::{
    compact_forms, dictionary_ipa, dictionary_pos, find_auxiliary, truncate_for_prompt,
};
use crate::services::dictionary_senses::{compact_senses, detailed_senses};
use crate::services::dictionary_tags::build_tags;
use serde_json::Value;

pub fn build_compact_analysis_from_dictionary(query: &str, raw_data: &Value) -> AnalysisDocument {
    let pos = dictionary_pos(raw_data);
    let ipa = dictionary_ipa(raw_data);

    let senses = compact_senses(raw_data, 2, " / ");
    let forms = compact_forms(raw_data, 3, 3);

    let mut sections = vec![format!("## {}", query)];

    if !ipa.is_empty() {
        sections.push(format!("*/{ipa}/*"));
    }

    if senses.is_empty() {
        sections.push(format!("- 词性：{pos}"));
    } else {
        sections.push(format!("- 核心释义：{}（{}）", senses.join("；"), pos));
    }

    if !forms.is_empty() {
        sections.push(format!("- 关键形式：{}", forms.join("；")));
    }

    sections.push("- 说明：这是高级查询的快速结果，已优先基于本地字典事实压缩生成。".to_string());

    AnalysisDocument {
        markdown: sections.join("\n\n"),
        tags: vec![pos.to_string()],
        aliases: Vec::new(),
        prototype: Some(query.to_string()),
        phrase_lookup: None,
        phrase_usage_preview: None,
        attached_phrase_modules: Vec::new(),
        dictionary_excerpt: None,
        model: None,
        quality_mode: None,
    }
}

pub fn build_full_analysis_from_dictionary(query: &str, raw_data: &Value) -> AnalysisDocument {
    let pos = dictionary_pos(raw_data);
    let ipa = dictionary_ipa(raw_data);
    let senses = detailed_senses(raw_data, 3);
    let forms = compact_forms(raw_data, 6, 4);
    let examples = collect_examples(raw_data, 2);
    let note = usage_note(raw_data);

    let mut sections = vec![format!("## {}", query)];
    let mut summary_bits = vec![format!("词性：{pos}")];

    if !ipa.is_empty() {
        summary_bits.push(format!("音标：/{ipa}/"));
    }
    if let Some(auxiliary) = find_auxiliary(raw_data) {
        summary_bits.push(format!("完成时助动词：{auxiliary}"));
    }

    sections.push(format!("- {}", summary_bits.join(" | ")));

    if !senses.is_empty() {
        sections.push(format!("### 核心义项\n{}", senses.join("\n")));
    }

    if !forms.is_empty() {
        sections.push(format!("### 关键形式\n- {}", forms.join("\n- ")));
    }

    if !examples.is_empty() {
        sections.push(format!("### 例句\n{}", examples.join("\n")));
    }

    if let Some(note) = note {
        sections.push(format!("### 使用提示\n- {note}"));
    }

    sections.push(
        "### 说明\n- 本次结果基于本地字典事实自动整理，用于在模型不可用或超时时提供稳定结果。"
            .to_string(),
    );

    AnalysisDocument {
        markdown: sections.join("\n\n"),
        tags: build_tags(raw_data, pos),
        aliases: Vec::new(),
        prototype: Some(query.to_string()),
        phrase_lookup: None,
        phrase_usage_preview: None,
        attached_phrase_modules: Vec::new(),
        dictionary_excerpt: None,
        model: None,
        quality_mode: None,
    }
}

pub fn build_unavailable_analysis(query: &str) -> AnalysisDocument {
    AnalysisDocument {
        markdown: format!(
            "## {query}\n\n- 当前未命中本地字典，且模型暂时不可用，暂时无法生成完整分析。\n- 建议：检查拼写、尝试输入原形，或稍后重试。"
        ),
        tags: vec!["待确认".to_string()],
        aliases: Vec::new(),
        prototype: Some(query.to_string()),
        phrase_lookup: None,
        phrase_usage_preview: None,
        attached_phrase_modules: Vec::new(),
        dictionary_excerpt: None,
        model: None,
        quality_mode: None,
    }
}

pub fn summarize_dictionary_entry(raw_data: &Value) -> String {
    let lemma = raw_data
        .get("word")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let pos = dictionary_pos(raw_data);
    let sounds = dictionary_ipa(raw_data).to_string();
    let senses = compact_senses(raw_data, 3, "; ")
        .into_iter()
        .collect::<Vec<_>>()
        .join("\n- ");
    let forms = compact_forms(raw_data, 5, 4).join("; ");
    let etymology = truncate_for_prompt(
        raw_data
            .get("etymology_text")
            .and_then(Value::as_str)
            .unwrap_or_default(),
        220,
    );

    format!(
        "lemma: {lemma}\npos: {pos}\nipa: {sounds}\nkey senses:\n- {senses}\nforms: {forms}\netymology: {etymology}"
    )
}

pub fn build_dictionary_excerpt(raw_data: &Value) -> Value {
    let senses = raw_data
        .get("senses")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .take(3)
                .map(|sense| {
                    serde_json::json!({
                        "glosses": sense.get("glosses").cloned().unwrap_or(Value::Null),
                        "tags": sense.get("tags").cloned().unwrap_or(Value::Null)
                    })
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    serde_json::json!({
        "word": raw_data.get("word").cloned().unwrap_or(Value::Null),
        "pos": raw_data.get("pos").cloned().unwrap_or(Value::Null),
        "sounds": raw_data.get("sounds").cloned().unwrap_or(Value::Null),
        "forms": raw_data.get("forms").cloned().unwrap_or(Value::Null),
        "senses": senses,
    })
}
