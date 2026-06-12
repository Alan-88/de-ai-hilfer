use crate::models::{
    PhraseUsagePreview, StructuredAnalysisDocument, StructuredDeepInsight, StructuredExample,
    StructuredGrammarRow, StructuredMeaning, StructuredUsageModule, StructuredWordNetwork,
    StructuredWordNetworkItem,
};
use std::collections::HashSet;

pub fn normalize_structured_analysis(
    structured: Option<StructuredAnalysisDocument>,
    fallback_headword: &str,
) -> Option<StructuredAnalysisDocument> {
    let mut structured = structured?;

    structured.headword = if structured.headword.trim().is_empty() {
        fallback_headword.trim().to_string()
    } else {
        structured.headword.trim().to_string()
    };
    structured.phonetic = structured.phonetic.trim().to_string();

    structured.meanings = structured
        .meanings
        .into_iter()
        .map(|meaning| StructuredMeaning {
            part_of_speech: meaning.part_of_speech.trim().to_string(),
            chinese: meaning.chinese.trim().to_string(),
            english: meaning.english.trim().to_string(),
        })
        .filter(|meaning| !meaning.chinese.is_empty() || !meaning.english.is_empty())
        .collect();

    structured.usage_modules = structured
        .usage_modules
        .into_iter()
        .map(|module| StructuredUsageModule {
            title: module.title.trim().to_string(),
            explanation: normalize_usage_explanation(&module.explanation),
            example_de: module.example_de.trim().to_string(),
            example_zh: module.example_zh.trim().to_string(),
        })
        .filter(|module| {
            !module.title.is_empty()
                || !module.explanation.is_empty()
                || !module.example_de.is_empty()
                || !module.example_zh.is_empty()
        })
        .collect();

    structured.examples = structured
        .examples
        .into_iter()
        .map(|example| StructuredExample {
            de: example.de.trim().to_string(),
            zh: example.zh.trim().to_string(),
        })
        .filter(|example| !example.de.is_empty() || !example.zh.is_empty())
        .collect();

    structured.grammar_rows = structured
        .grammar_rows
        .into_iter()
        .map(|row| StructuredGrammarRow {
            key: row.key.trim().to_string(),
            value: row.value.trim().to_string(),
        })
        .filter(|row| !row.key.is_empty() && !row.value.is_empty())
        .collect();

    structured.word_network = normalize_word_network(structured.word_network);

    structured.deep_insights = structured
        .deep_insights
        .into_iter()
        .map(|insight| StructuredDeepInsight {
            title: if insight.title.trim().is_empty() {
                "分析片段".to_string()
            } else {
                insight.title.trim().to_string()
            },
            content_markdown: insight.content_markdown.trim().to_string(),
        })
        .filter(|insight| !insight.content_markdown.is_empty())
        .collect();

    structured.collocations = dedupe_strings(structured.collocations);

    let is_empty = structured.meanings.is_empty()
        && structured.usage_modules.is_empty()
        && structured.collocations.is_empty()
        && structured.examples.is_empty()
        && structured.grammar_rows.is_empty()
        && structured.word_network.family.is_empty()
        && structured.word_network.synonyms.is_empty()
        && structured.word_network.antonyms.is_empty()
        && structured.deep_insights.is_empty()
        && structured.phonetic.is_empty();

    if is_empty {
        None
    } else {
        Some(structured)
    }
}

pub fn structured_from_phrase_preview(
    query: &str,
    preview: &PhraseUsagePreview,
) -> StructuredAnalysisDocument {
    normalize_structured_analysis(
        Some(StructuredAnalysisDocument {
            headword: query.trim().to_string(),
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
        }),
        query,
    )
    .unwrap_or_default()
}

pub fn merge_structured_with_seed(
    mut structured: StructuredAnalysisDocument,
    seed: &StructuredAnalysisDocument,
) -> StructuredAnalysisDocument {
    structured.usage_modules = merge_usage_modules(structured.usage_modules, &seed.usage_modules);
    structured.word_network.family =
        merge_word_network_items(structured.word_network.family, &seed.word_network.family);
    structured.word_network.synonyms = merge_word_network_items(
        structured.word_network.synonyms,
        &seed.word_network.synonyms,
    );
    structured.word_network.antonyms = merge_word_network_items(
        structured.word_network.antonyms,
        &seed.word_network.antonyms,
    );
    structured.deep_insights = merge_deep_insights(structured.deep_insights, &seed.deep_insights);
    structured
}

fn merge_usage_modules(
    mut values: Vec<StructuredUsageModule>,
    seed: &[StructuredUsageModule],
) -> Vec<StructuredUsageModule> {
    let mut seen = values
        .iter()
        .map(|module| module.title.trim().to_lowercase())
        .collect::<HashSet<_>>();
    for module in seed {
        let key = module.title.trim().to_lowercase();
        if !key.is_empty() && seen.insert(key) {
            values.push(module.clone());
        }
    }
    values
}

fn merge_word_network_items(
    values: Vec<StructuredWordNetworkItem>,
    seed: &[StructuredWordNetworkItem],
) -> Vec<StructuredWordNetworkItem> {
    dedupe_word_network_items(values.into_iter().chain(seed.iter().cloned()).collect())
}

fn merge_deep_insights(
    mut values: Vec<StructuredDeepInsight>,
    seed: &[StructuredDeepInsight],
) -> Vec<StructuredDeepInsight> {
    let mut seen = values
        .iter()
        .map(|insight| insight.title.trim().to_lowercase())
        .collect::<HashSet<_>>();
    for insight in seed {
        let key = insight.title.trim().to_lowercase();
        if !key.is_empty() && seen.insert(key) {
            values.push(insight.clone());
        }
    }
    values
}

fn dedupe_strings(values: Vec<String>) -> Vec<String> {
    let mut seen = HashSet::new();
    values
        .into_iter()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .filter(|value| seen.insert(value.to_lowercase()))
        .collect()
}

fn normalize_usage_explanation(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn normalize_word_network(word_network: StructuredWordNetwork) -> StructuredWordNetwork {
    StructuredWordNetwork {
        family: dedupe_word_network_items(word_network.family),
        synonyms: dedupe_word_network_items(word_network.synonyms),
        antonyms: dedupe_word_network_items(word_network.antonyms),
    }
}

fn dedupe_word_network_items(
    values: Vec<StructuredWordNetworkItem>,
) -> Vec<StructuredWordNetworkItem> {
    let mut seen = HashSet::new();
    values
        .into_iter()
        .map(|item| StructuredWordNetworkItem {
            term: item.term.trim().to_string(),
            part_of_speech: item.part_of_speech.trim().to_string(),
            chinese: item.chinese.trim().to_string(),
            english: item.english.trim().to_string(),
            note: item.note.trim().to_string(),
        })
        .filter(|item| !item.term.is_empty())
        .filter(|item| {
            let key = item.term.to_lowercase();
            seen.insert(key)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_usage_explanation_line_breaks_only() {
        let structured = StructuredAnalysisDocument {
            headword: "helfen".to_string(),
            usage_modules: vec![StructuredUsageModule {
                title: "jemandem helfen".to_string(),
                explanation: "用于表达帮助某人。\n注意支配 Dativ，\n不要按中文直觉使用 Akkusativ。"
                    .to_string(),
                example_de: "Ich helfe dir.".to_string(),
                example_zh: "我帮你。".to_string(),
            }],
            deep_insights: vec![StructuredDeepInsight {
                title: "为什么用 Dativ".to_string(),
                content_markdown: "1. 第一层\n2. 第二层".to_string(),
            }],
            ..StructuredAnalysisDocument::default()
        };

        let normalized = normalize_structured_analysis(Some(structured), "helfen").unwrap();

        assert_eq!(
            normalized.usage_modules[0].explanation,
            "用于表达帮助某人。 注意支配 Dativ， 不要按中文直觉使用 Akkusativ。"
        );
        assert_eq!(
            normalized.deep_insights[0].content_markdown,
            "1. 第一层\n2. 第二层"
        );
    }

    #[test]
    fn fills_missing_deep_insights_from_seed() {
        let structured = StructuredAnalysisDocument {
            deep_insights: vec![StructuredDeepInsight {
                title: "辨析".to_string(),
                content_markdown: "已抽取内容".to_string(),
            }],
            ..StructuredAnalysisDocument::default()
        };
        let seed = StructuredAnalysisDocument {
            deep_insights: vec![
                StructuredDeepInsight {
                    title: "辨析".to_string(),
                    content_markdown: "规则解析内容".to_string(),
                },
                StructuredDeepInsight {
                    title: "词源".to_string(),
                    content_markdown: "词源内容".to_string(),
                },
            ],
            ..StructuredAnalysisDocument::default()
        };

        let merged = merge_structured_with_seed(structured, &seed);

        assert_eq!(merged.deep_insights.len(), 2);
        assert_eq!(merged.deep_insights[0].content_markdown, "已抽取内容");
        assert_eq!(merged.deep_insights[1].title, "词源");
    }
}
