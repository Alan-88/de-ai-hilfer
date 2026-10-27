use crate::models::{
    StructuredAnalysisDocument, StructuredDeepInsight, StructuredExample, StructuredGrammarRow,
    StructuredMeaning, StructuredUsageModule, StructuredWordNetwork, StructuredWordNetworkItem,
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
        && structured.grammar_branches.is_empty()
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
    let missing_items = values.len() < seed.len();
    for module in seed {
        let key = module.title.trim().to_lowercase();
        if key.is_empty() {
            continue;
        }
        if let Some(existing) = values
            .iter_mut()
            .find(|item| item.title.trim().to_lowercase() == key)
        {
            if existing.explanation.is_empty() {
                existing.explanation.clone_from(&module.explanation);
            }
            if existing.example_de.is_empty() {
                existing.example_de.clone_from(&module.example_de);
            }
            if existing.example_zh.is_empty() {
                existing.example_zh.clone_from(&module.example_zh);
            }
        } else if missing_items {
            values.push(module.clone());
        }
    }
    values
}

fn merge_word_network_items(
    values: Vec<StructuredWordNetworkItem>,
    seed: &[StructuredWordNetworkItem],
) -> Vec<StructuredWordNetworkItem> {
    if values.len() >= dedupe_word_network_items(seed.to_vec()).len() {
        return values;
    }
    dedupe_word_network_items(values.into_iter().chain(seed.iter().cloned()).collect())
}

fn merge_deep_insights(
    mut values: Vec<StructuredDeepInsight>,
    seed: &[StructuredDeepInsight],
) -> Vec<StructuredDeepInsight> {
    if values.len() >= seed.len() {
        return values;
    }
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
            let key = (
                item.term.clone(),
                item.part_of_speech.clone(),
                item.chinese.clone(),
                item.english.clone(),
                item.note.clone(),
            );
            seen.insert(key)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn network_deduplication_keeps_distinct_forms_and_senses() {
        let markdown = "#### 词汇网络\n**词族**\n* [duzen] (*v.*, *称呼*, *address*)\n* [Duzen] (*n.*, *称呼行为*, *addressing*)\n**同义词**\n* [Wort] (*n.*, *同义词*, *synonym*)\n* [Wort] (*n.*, *词汇*, *word*)\n* [Wort] (*n.*, *词汇*, *word*)";
        let seed = crate::services::analysis_structure_seed::build_structure_seed("du", markdown);
        let normalized = normalize_structured_analysis(Some(seed), "du").unwrap();
        assert_eq!(normalized.word_network.family.len(), 2);
        assert_eq!(normalized.word_network.synonyms.len(), 2);
        assert!(
            crate::services::analysis_structure_quality::validate_structured_capture(
                markdown,
                &normalized
            )
            .is_ok()
        );
    }

    #[test]
    fn fills_only_missing_usage_fields_and_keeps_empty_grammar_branches() {
        let mut source = StructuredAnalysisDocument::default();
        source.usage_modules.push(StructuredUsageModule {
            title: "gar nicht".into(),
            explanation: "原有解释".into(),
            example_de: "原有例句".into(),
            example_zh: String::new(),
        });
        let mut seed = source.clone();
        seed.usage_modules[0].explanation = "备用解释".into();
        seed.usage_modules[0].example_zh = "完整翻译".into();
        let merged = merge_structured_with_seed(source, &seed);
        assert_eq!(merged.usage_modules.len(), 1);
        assert_eq!(merged.usage_modules[0].explanation, "原有解释");
        assert_eq!(merged.usage_modules[0].example_zh, "完整翻译");
        let branch_only = StructuredAnalysisDocument {
            grammar_branches: vec![crate::models::StructuredGrammarBranch {
                pos: "adverb".into(),
                ..Default::default()
            }],
            ..Default::default()
        };
        assert!(normalize_structured_analysis(Some(branch_only), "gar").is_some());
    }

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
