use super::analysis_structure_seed::{build_structure_seed, is_word_network_bucket_line};
use crate::models::StructuredAnalysisDocument;

pub fn validate_structured_capture(
    markdown: &str,
    structured: &StructuredAnalysisDocument,
) -> Result<(), String> {
    let normalized = markdown.replace('\r', "");
    let source = build_structure_seed("", &normalized);

    {
        let module_like_blocks = source.usage_modules.len();
        let complete_usage_modules = structured
            .usage_modules
            .iter()
            .filter(|module| {
                !module.title.trim().is_empty()
                    && !module.explanation.trim().is_empty()
                    && !module.example_de.trim().is_empty()
                    && !module.example_zh.trim().is_empty()
            })
            .count();
        if module_like_blocks > 0 && complete_usage_modules < module_like_blocks {
            return Err(format!(
                "usage_modules incomplete: expected at least {module_like_blocks}, got {complete_usage_modules}"
            ));
        }
    }

    if let Some(section) = grammar_section(&normalized) {
        let markdown_rows = count_markdown_table_rows(section);
        let covered_rows = structured
            .grammar_rows
            .iter()
            .filter(|row| !row.key.trim().is_empty() && !row.value.trim().is_empty())
            .count();
        if markdown_rows > 0 && covered_rows < markdown_rows {
            return Err(format!(
                "grammar_rows incomplete: expected at least {markdown_rows}, got {covered_rows}"
            ));
        }
    }

    if let Some(section) = word_network_section(&normalized) {
        let has_family_bucket = has_word_network_bucket(section, "词族");
        let has_synonym_bucket = has_word_network_bucket(section, "同义词");
        let has_antonym_bucket = has_word_network_bucket(section, "反义词");
        let has_any_bucket = has_family_bucket || has_synonym_bucket || has_antonym_bucket;
        let has_visible_items = has_word_network_items(section);

        if has_visible_items && !has_any_bucket {
            return Err("word_network section has list items but no relation buckets".to_string());
        }

        if has_family_bucket
            && !bucket_declares_empty(section, "词族")
            && structured.word_network.family.is_empty()
        {
            return Err("word_network.family missing despite family subsection".to_string());
        }
        if has_synonym_bucket
            && !bucket_declares_empty(section, "同义词")
            && structured.word_network.synonyms.is_empty()
        {
            return Err("word_network.synonyms missing despite synonym subsection".to_string());
        }
        if has_antonym_bucket
            && !bucket_declares_empty(section, "反义词")
            && structured.word_network.antonyms.is_empty()
        {
            return Err("word_network.antonyms missing despite antonym subsection".to_string());
        }

        if has_visible_items
            && structured.word_network.family.is_empty()
            && structured.word_network.synonyms.is_empty()
            && structured.word_network.antonyms.is_empty()
        {
            return Err("word_network structured buckets empty despite visible items".to_string());
        }
        for (name, expected, actual) in [
            (
                "family",
                &source.word_network.family,
                &structured.word_network.family,
            ),
            (
                "synonyms",
                &source.word_network.synonyms,
                &structured.word_network.synonyms,
            ),
            (
                "antonyms",
                &source.word_network.antonyms,
                &structured.word_network.antonyms,
            ),
        ] {
            let expected = count_network_items(expected);
            let captured = count_network_items(actual);
            if captured < expected {
                return Err(format!(
                    "word_network.{name} incomplete: expected at least {expected}, got {captured}"
                ));
            }
        }
    }

    if let Some(section) = deep_insight_section(&normalized) {
        let markdown_items = count_top_level_deep_insight_items(section);
        let captured = structured
            .deep_insights
            .iter()
            .filter(|item| !item.content_markdown.trim().is_empty())
            .count();
        if markdown_items > 0 && captured < markdown_items {
            return Err(format!(
                "deep_insights incomplete: expected at least {markdown_items}, got {}",
                captured
            ));
        }
    }

    Ok(())
}

fn count_network_items(items: &[crate::models::StructuredWordNetworkItem]) -> usize {
    items
        .iter()
        .filter(|item| !item.term.trim().is_empty())
        .map(|item| {
            (
                item.term.trim(),
                item.part_of_speech.trim(),
                item.chinese.trim(),
                item.english.trim(),
                item.note.trim(),
            )
        })
        .collect::<std::collections::HashSet<_>>()
        .len()
}

fn grammar_section(markdown: &str) -> Option<&str> {
    section_by_headings(markdown, &["#### 语法详情"])
}

fn word_network_section(markdown: &str) -> Option<&str> {
    section_by_headings(markdown, &["#### 词汇网络"])
}

fn deep_insight_section(markdown: &str) -> Option<&str> {
    section_by_headings(
        markdown,
        &["#### 深度解析与避坑", "#### 深度解析", "#### 避坑"],
    )
}

fn section_by_headings<'a>(markdown: &'a str, headings: &[&str]) -> Option<&'a str> {
    let start = headings.iter().find_map(|heading| markdown.find(heading))?;
    let section = &markdown[start..];
    let next_heading = section
        .char_indices()
        .skip(1)
        .find_map(|(index, _)| section[index..].starts_with("\n#### ").then_some(index));

    Some(match next_heading {
        Some(end) => &section[..end],
        None => section,
    })
}

fn count_markdown_table_rows(section: &str) -> usize {
    section
        .lines()
        .map(str::trim)
        .filter(|line| line.starts_with('|'))
        .filter(|line| {
            !line
                .chars()
                .all(|ch| ch.is_whitespace() || matches!(ch, '|' | ':' | '-'))
        })
        .count()
        .saturating_sub(1)
}

fn has_word_network_bucket(section: &str, bucket: &str) -> bool {
    section
        .lines()
        .any(|line| is_word_network_bucket_line(line, bucket))
}

fn has_word_network_items(section: &str) -> bool {
    section.lines().map(str::trim).any(|line| {
        (line.starts_with("*") || line.starts_with("-"))
            && !is_any_word_network_bucket_line(line)
            && !line.contains("无高价值项")
    })
}

fn bucket_declares_empty(section: &str, bucket: &str) -> bool {
    let mut in_bucket = false;
    for line in section.lines() {
        if is_word_network_bucket_line(line, bucket) {
            in_bucket = true;
            continue;
        }
        if in_bucket && is_any_word_network_bucket_line(line) {
            break;
        }
        if in_bucket && line.contains("无高价值项") {
            return true;
        }
    }
    false
}

fn is_any_word_network_bucket_line(line: &str) -> bool {
    ["词族", "同义词", "反义词"]
        .iter()
        .any(|bucket| is_word_network_bucket_line(line, bucket))
}

fn count_top_level_deep_insight_items(section: &str) -> usize {
    build_structure_seed("", section).deep_insights.len()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{StructuredWordNetwork, StructuredWordNetworkItem};

    #[test]
    fn preserves_etymology_in_original_section_and_rejects_actual_omissions() {
        let markdown = "#### 应用与例句\n\ngar nicht\n用法解析：加强否定\n场景例句：Das stimmt gar nicht.\n例句翻译：这完全不对。\n\n#### 深度解析与避坑\n\n* **语义演变**\n词源解释在这里，不必改名。\n\n* **辨析**\n第二段正文。";
        let mut source = build_structure_seed("gar", markdown);
        assert!(validate_structured_capture(markdown, &source).is_ok());
        source.usage_modules[0].example_zh.clear();
        assert!(validate_structured_capture(markdown, &source)
            .unwrap_err()
            .contains("usage_modules"));
        source = build_structure_seed("gar", markdown);
        source.deep_insights.pop();
        assert!(validate_structured_capture(markdown, &source)
            .unwrap_err()
            .contains("deep_insights"));
    }

    #[test]
    fn unrelated_collocation_does_not_cover_missing_grammar() {
        let markdown = "#### 语法详情\n| 特征 | 值 |\n| --- | --- |\n| 固定搭配 | mit jemandem |";
        let structured = StructuredAnalysisDocument {
            collocations: vec!["unrelated".into()],
            ..Default::default()
        };
        assert!(validate_structured_capture(markdown, &structured).is_err());
    }

    #[test]
    fn rejects_partially_missing_network_bucket() {
        let markdown = "#### 词汇网络\n\n**词族**\n* [garen] (Verb, 烹饪, cook)\n* [Garzeit] (Noun, 烹饪时间, cooking time)";
        let mut source = build_structure_seed("gar", markdown);
        source.word_network.family.pop();
        assert!(validate_structured_capture(markdown, &source)
            .unwrap_err()
            .contains("family incomplete"));
    }

    #[test]
    fn accepts_declared_empty_word_network_bucket() {
        let markdown = "#### 词汇网络 (Wortnetz)\n\n**词族**\n*   无高价值项\n\n**同义词**\n*   [helfen] (*Verb*, 帮助, to help)\n\n**反义词**\n*   无高价值项";
        let structured = StructuredAnalysisDocument {
            word_network: StructuredWordNetwork {
                synonyms: vec![StructuredWordNetworkItem {
                    term: "helfen".to_string(),
                    part_of_speech: "Verb".to_string(),
                    chinese: "帮助".to_string(),
                    english: "to help".to_string(),
                    note: String::new(),
                }],
                ..StructuredWordNetwork::default()
            },
            ..StructuredAnalysisDocument::default()
        };

        assert!(validate_structured_capture(markdown, &structured).is_ok());
    }

    #[test]
    fn rejects_flat_word_network_items_without_buckets() {
        let markdown = "#### 词汇网络 (Wortnetz)\n\n*   [helfen] (*Verb*, 帮助, to help)";
        let structured = StructuredAnalysisDocument::default();

        let err = validate_structured_capture(markdown, &structured).unwrap_err();
        assert!(err.contains("no relation buckets"));
    }

    #[test]
    fn accepts_legacy_word_network_bucket_list_items() {
        let markdown = "#### 词汇网络 (Wortnetz)\n\n* 词族 (Wortfamilie)\n    * Architekt (m. 建筑师)\n* 同义词 (Synonyme)\n    * Baukunst (强调艺术性)";
        let structured = StructuredAnalysisDocument {
            word_network: StructuredWordNetwork {
                family: vec![StructuredWordNetworkItem {
                    term: "Architekt".to_string(),
                    part_of_speech: "m.".to_string(),
                    chinese: "建筑师".to_string(),
                    english: String::new(),
                    note: String::new(),
                }],
                synonyms: vec![StructuredWordNetworkItem {
                    term: "Baukunst".to_string(),
                    part_of_speech: String::new(),
                    chinese: "强调艺术性".to_string(),
                    english: String::new(),
                    note: String::new(),
                }],
                ..StructuredWordNetwork::default()
            },
            ..StructuredAnalysisDocument::default()
        };

        assert!(validate_structured_capture(markdown, &structured).is_ok());
    }

    #[test]
    fn ignores_indented_example_bullets_in_deep_insight_count() {
        let markdown = "#### 深度解析与避坑 (Einblicke)\n\n*   **逻辑对立：bereits vs. erst**\n德语学习者常混淆这两个词。\n例如：\n- Er ist **bereits** zehn Jahre alt.\n- Er ist **erst** zehn Jahre alt.\n\n*   **词源逻辑**\n`bereits` 来源于形容词 `bereit`。";

        assert_eq!(count_top_level_deep_insight_items(markdown), 2);
    }

    #[test]
    fn ignores_nested_deep_insight_bullets() {
        let markdown = "#### 深度解析与避坑 (Einblicke)\n\n*   **核心逻辑：从“重”到“轻”**\n说明：\n    *   **体力/精力上**：使工作不那么累。\n    *   **心理/情感上**：使心情不再沉重。\n\n*   **词源小记**\n说明。";

        assert_eq!(count_top_level_deep_insight_items(markdown), 2);
    }
}
