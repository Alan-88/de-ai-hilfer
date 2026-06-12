use crate::ai::AiChatOptions;
use crate::models::{
    AnalysisDocument, DictionaryRaw, PhraseLookupInfo, PhraseUsageModule, PhraseUsagePreview,
    QualityMode, StructuredAnalysisDocument, StructuredDeepInsight, StructuredMeaning,
    StructuredUsageModule, StructuredWordNetwork,
};
use crate::prompts::PromptConfig;
use crate::services::dictionary_render::summarize_dictionary_entry;
use crate::services::json_extract_repair::{
    json_candidate, repair_json_inner_quotes, repair_truncated_json,
};
use anyhow::Result;
use serde::Deserialize;
use std::time::Duration;

#[derive(Clone, Copy)]
pub enum AnalysisMode {
    Full,
    Compact,
}

pub fn build_analysis_prompt(
    prompts: &PromptConfig,
    dictionary_entry: Option<&DictionaryRaw>,
    mode: AnalysisMode,
    generation_hint: Option<&str>,
    phrase_lookup: Option<&PhraseLookupInfo>,
) -> String {
    if phrase_lookup.is_some() {
        return build_phrase_usage_preview_prompt(
            dictionary_entry,
            generation_hint,
            phrase_lookup.expect("phrase lookup present"),
        );
    }

    let dictionary_summary = dictionary_entry
        .map(|entry| summarize_dictionary_entry(&entry.raw_data))
        .unwrap_or_else(|| "未命中本地字典；请谨慎分析并明确不确定性。".to_string());

    let mode_requirements = analysis_markdown_requirements(mode);

    let hint_requirements = generation_hint
        .filter(|hint| !hint.trim().is_empty())
        .map(|hint| {
            format!(
                "\n11. 额外用户要求：{}。只有在不违背字典事实的前提下，尽量满足这些展示偏好。",
                hint.trim()
            )
        })
        .unwrap_or_default();
    let phrase_requirements = phrase_lookup
        .map(|lookup| {
            let host_note = lookup
                .best_host_headword
                .as_deref()
                .map(|host| format!("候选宿主主词是 `{host}`，字典事实仅把它当参考，不要把整篇写成这个主词的普通词条分析。"))
                .unwrap_or_else(|| "当前还没锁定宿主主词；如有不确定请明确说出。".to_string());
            format!(
                "\n12. 当前查询目标是德语短语/搭配 `{}`，不是单个词头。请围绕这个短语本身输出含义、使用条件、语气和例句。{}",
                lookup.phrase, host_note
            )
        })
        .unwrap_or_default();

    format!(
        "{}\n\n--- 额外系统要求 ---\n1. 直接输出 Markdown，不要输出 JSON、代码块包裹或额外解释。\n{}\n\n--- 字典事实 ---\n{}",
        prompts
            .analysis_prompt
            .replace("{vocabulary_list}", "Rust 迁移阶段暂不启用知识库工具列表"),
        format!("{mode_requirements}{hint_requirements}{phrase_requirements}"),
        dictionary_summary
    )
}

fn build_phrase_usage_preview_prompt(
    dictionary_entry: Option<&DictionaryRaw>,
    generation_hint: Option<&str>,
    phrase_lookup: &PhraseLookupInfo,
) -> String {
    let dictionary_summary = dictionary_entry
        .map(|entry| summarize_dictionary_entry(&entry.raw_data))
        .unwrap_or_else(|| "未命中本地字典；请谨慎分析并明确不确定性。".to_string());
    let host_note = phrase_lookup
        .best_host_headword
        .as_deref()
        .map(|host| format!("当前宿主主词候选是 `{host}`。请把它仅当作挂载上下文参考，不要把输出扩展成这个主词的完整词条分析。"))
        .unwrap_or_else(|| "当前宿主主词还不稳定；请围绕短语本身给出最有把握的一条高价值用法。".to_string());
    let hint_note = generation_hint
        .filter(|hint| !hint.trim().is_empty())
        .map(|hint| {
            format!(
                "额外用户要求：{}。仅在不违背字典事实时尽量满足。",
                hint.trim()
            )
        })
        .unwrap_or_default();

    format!(
        "你是一位专业德语教师。当前任务不是生成完整词条，而是为一个德语短语/固定搭配生成“可直接追加到主词应用与例句区”的单条用法模块。\n\n\
输出必须是 JSON 对象，且只允许包含以下字段：\n\
{{\n\
  \"meaning_zh\": \"短语的简短中文释义，10-24 字，不能展开成长解释\",\n\
  \"meaning_en\": \"简短英文释义，可留空字符串但不要省略\",\n\
  \"usage_module\": {{\n\
    \"title\": \"德语搭配/句型结构本身，可带必要的格或介词提示\",\n\
    \"explanation\": \"1-2 句中文用法解析，重点说明场景、语气或搭配限制，不要给 title 单独做中文直译\",\n\
    \"example_de\": \"一个自然、完整的德语例句\",\n\
    \"example_zh\": \"该例句的自然中文翻译\"\n\
  }},\n\
  \"tags\": [\"1-4 个简短标签\"],\n\
  \"aliases\": [\"必要时才给，通常可为空数组\"]\n\
}}\n\n\
强制要求：\n\
1. 只生成一条 usage module，不要输出词汇网络、词源、深度解析、完整语法表。\n\
2. `usage_module.title` 默认应直接使用短语本身或最自然的句型变体。\n\
3. `usage_module.explanation` 必须是帮助学习者理解“什么时候用”，而不是重复释义。\n\
4. `example_de` 与 `example_zh` 必须完整对应。\n\
5. 不要输出 Markdown，不要输出额外注释，不要包裹代码块。\n\
6. {host_note}\n\
7. {hint_note}\n\n\
--- 字典事实 ---\n{dictionary_summary}",
    )
}

pub fn analysis_chat_options(mode: AnalysisMode) -> AiChatOptions {
    match mode {
        AnalysisMode::Full => AiChatOptions {
            temperature: 0.2,
            max_tokens: Some(900),
            timeout: Duration::from_secs(40),
        },
        AnalysisMode::Compact => AiChatOptions {
            temperature: 0.1,
            max_tokens: Some(360),
            timeout: Duration::from_secs(12),
        },
    }
}

pub fn analysis_document_from_markdown(markdown: &str, query: &str) -> AnalysisDocument {
    AnalysisDocument {
        markdown: markdown.trim().to_string(),
        structured: None,
        tags: Vec::new(),
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

pub fn parse_phrase_usage_preview(
    raw: &str,
    query: &str,
) -> Result<(PhraseUsagePreview, Vec<String>, Vec<String>)> {
    #[derive(Deserialize)]
    struct LlmPhraseUsagePreview {
        meaning_zh: String,
        #[serde(default)]
        meaning_en: String,
        usage_module: PhraseUsageModule,
        #[serde(default)]
        tags: Vec<String>,
        #[serde(default)]
        aliases: Vec<String>,
    }

    let parsed: LlmPhraseUsagePreview = extract_json(raw)?;
    let preview = PhraseUsagePreview {
        meaning_zh: parsed.meaning_zh.trim().to_string(),
        meaning_en: parsed.meaning_en.trim().to_string(),
        usage_module: PhraseUsageModule {
            title: if parsed.usage_module.title.trim().is_empty() {
                query.trim().to_string()
            } else {
                parsed.usage_module.title.trim().to_string()
            },
            explanation: parsed.usage_module.explanation.trim().to_string(),
            example_de: parsed.usage_module.example_de.trim().to_string(),
            example_zh: parsed.usage_module.example_zh.trim().to_string(),
        },
    };
    Ok((preview, parsed.tags, parsed.aliases))
}

pub fn build_phrase_preview_analysis(
    phrase: &str,
    preview: PhraseUsagePreview,
    phrase_lookup: Option<&PhraseLookupInfo>,
    dictionary_entry: Option<&DictionaryRaw>,
    tags: Vec<String>,
    aliases: Vec<String>,
    quality_mode: QualityMode,
    model: &str,
) -> AnalysisDocument {
    let markdown = render_phrase_preview_markdown(phrase, &preview);
    AnalysisDocument {
        markdown,
        structured: Some(structured_analysis_from_phrase_preview(phrase, &preview)),
        tags,
        aliases,
        prototype: dictionary_entry.map(|entry| entry.headword.clone()),
        phrase_lookup: phrase_lookup.cloned(),
        phrase_usage_preview: Some(preview),
        attached_phrase_modules: Vec::new(),
        dictionary_excerpt: dictionary_entry
            .map(|entry| summarize_dictionary_entry(&entry.raw_data).into()),
        model: Some(model.to_string()),
        quality_mode: Some(quality_mode),
    }
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

pub fn structured_analysis_from_phrase_preview(
    phrase: &str,
    preview: &PhraseUsagePreview,
) -> StructuredAnalysisDocument {
    StructuredAnalysisDocument {
        headword: phrase.trim().to_string(),
        phonetic: String::new(),
        meanings: vec![StructuredMeaning {
            part_of_speech: "短语".to_string(),
            chinese: preview.meaning_zh.trim().to_string(),
            english: preview.meaning_en.trim().to_string(),
        }],
        usage_modules: vec![StructuredUsageModule {
            title: preview.usage_module.title.trim().to_string(),
            explanation: preview.usage_module.explanation.trim().to_string(),
            example_de: preview.usage_module.example_de.trim().to_string(),
            example_zh: preview.usage_module.example_zh.trim().to_string(),
        }],
        collocations: Vec::new(),
        examples: Vec::new(),
        grammar_rows: Vec::new(),
        grammar_branches: Vec::new(),
        word_network: StructuredWordNetwork::default(),
        deep_insights: vec![StructuredDeepInsight {
            title: "短语说明".to_string(),
            content_markdown: preview.usage_module.explanation.trim().to_string(),
        }],
    }
}

fn analysis_markdown_requirements(mode: AnalysisMode) -> &'static str {
    match mode {
        AnalysisMode::Full => {
            "2. 内容面向中文学习者，但要控制长度，优先讲最常用的 1-3 个核心义项。\n3. 下面提供的是权威字典事实，请优先用于确保语法和变位准确，但不要机械复述所有义项。\n4. 在“应用与例句”部分，优先输出 2-3 个“场景用法模块”，优先采用三行块格式：`德语搭配/句型: 中文用法解析` + `德语例句` + `（中文翻译）`，模块之间空一行。\n5. 中文用法解析不要只写空泛解释，优先说明最典型的使用场景、语气色彩、常见对比对象，或后面为什么接某个格/介词。\n6. 如果不用三行块格式，也可以使用 `用法解析 / 场景例句 / 例句翻译` 标签式写法，但不要再写“固定搭配”“例句”子标题，也不要把“用法解析”挤进标题行。\n7. 不要使用方括号包裹搭配，也不要给搭配本身单独写中文直译。\n8. 如果词条信息不足，要明确说出不确定性，但仍尽量给出最有把握的结论。"
        }
        AnalysisMode::Compact => {
            "2. 内容面向中文学习者，但必须压缩到很短：优先只给核心释义、一个场景用法模块、一个关键语法点。\n3. 下面提供的是权威字典事实，请优先用于确保语法和变位准确，但不要展开词汇网络或长篇词源。\n4. 总长度控制在 220 个中文字符以内；不要输出多级小节，最多使用 3 个短段或短列表项。\n5. 如果写到搭配，优先用 `德语搭配: 中文用法解析` + `德语例句` + `（中文翻译）` 的紧凑三行块格式。\n6. 中文用法解析优先点出最关键的使用场景或搭配限制，不要只写抽象释义。\n7. 不要给搭配本身单独做中文直译。"
        }
    }
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
