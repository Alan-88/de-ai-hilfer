use crate::models::{
    StructuredAnalysisDocument, StructuredDeepInsight, StructuredUsageModule,
    StructuredWordNetwork, StructuredWordNetworkItem,
};

pub fn build_structure_seed(query: &str, markdown: &str) -> StructuredAnalysisDocument {
    StructuredAnalysisDocument {
        headword: query.trim().to_string(),
        usage_modules: parse_usage_modules(markdown),
        word_network: parse_word_network(markdown),
        deep_insights: parse_deep_insights(markdown),
        ..StructuredAnalysisDocument::default()
    }
}

fn parse_usage_modules(markdown: &str) -> Vec<StructuredUsageModule> {
    section_by_heading(markdown, &["#### 应用与例句"])
        .map(|section| {
            section
                .split("\n\n")
                .filter_map(parse_usage_block)
                .collect()
        })
        .unwrap_or_default()
}

fn parse_usage_block(block: &str) -> Option<StructuredUsageModule> {
    let lines = block
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>();

    if lines.len() < 4 || lines.first().is_some_and(|line| line.starts_with("####")) {
        return None;
    }

    let title = lines[0].trim_end_matches(':').trim();
    let mut explanation = String::new();
    let mut example_de = String::new();
    let mut example_zh = String::new();

    for line in lines.iter().skip(1) {
        if let Some(value) = line.strip_prefix("用法解析：") {
            explanation = value.trim().to_string();
        } else if let Some(value) = line.strip_prefix("场景例句：") {
            example_de = value.trim().to_string();
        } else if let Some(value) = line.strip_prefix("例句翻译：") {
            example_zh = strip_wrapping_zh_parentheses(value.trim());
        }
    }

    if explanation.is_empty() && example_de.is_empty() && example_zh.is_empty() {
        explanation = lines.get(1).copied().unwrap_or_default().to_string();
        example_de = lines.get(2).copied().unwrap_or_default().to_string();
        example_zh = lines
            .get(3)
            .map(|line| strip_wrapping_zh_parentheses(line))
            .unwrap_or_default();
    }

    (!title.is_empty()
        && !explanation.is_empty()
        && !example_de.is_empty()
        && !example_zh.is_empty())
    .then(|| StructuredUsageModule {
        title: title.to_string(),
        explanation,
        example_de,
        example_zh,
    })
}

fn parse_word_network(markdown: &str) -> StructuredWordNetwork {
    let Some(section) = section_by_heading(markdown, &["#### 词汇网络"]) else {
        return StructuredWordNetwork::default();
    };

    let mut network = StructuredWordNetwork::default();
    let mut bucket = "";

    for line in section.lines().map(str::trim) {
        if line.starts_with("####") || line.is_empty() {
            continue;
        }

        if is_word_network_bucket_line(line, "词族") {
            bucket = "family";
            continue;
        }
        if is_word_network_bucket_line(line, "同义词") {
            bucket = "synonyms";
            continue;
        }
        if is_word_network_bucket_line(line, "反义词") {
            bucket = "antonyms";
            continue;
        }

        if let Some(item) = parse_word_network_item(line) {
            match bucket {
                "family" => network.family.push(item),
                "synonyms" => network.synonyms.push(item),
                "antonyms" => network.antonyms.push(item),
                _ => {}
            }
        }
    }

    network
}

fn is_word_network_bucket_line(line: &str, bucket: &str) -> bool {
    let trimmed = line.trim();
    if trimmed.contains(&format!("**{bucket}**")) {
        return true;
    }

    let label = trimmed
        .trim_start_matches(['*', '-', ' ', '\t'])
        .trim()
        .trim_matches('*')
        .trim();

    label.starts_with(bucket)
}

fn parse_word_network_item(line: &str) -> Option<StructuredWordNetworkItem> {
    let line = line.trim_start_matches(['*', '-', ' ']).trim();
    if line.is_empty() || line.contains("无高价值项") {
        return None;
    }

    let (term, rest) = if let Some(start) = line.find('[') {
        let end = line[start + 1..].find(']')? + start + 1;
        (&line[start + 1..end], line[end + 1..].trim())
    } else if let Some(paren_index) = line.find('(') {
        (
            line[..paren_index].trim(),
            line[paren_index..].trim_start_matches([' ', '-']).trim(),
        )
    } else {
        line.split_once(' ')
            .map(|(term, rest)| (term.trim(), rest.trim()))
            .unwrap_or((line, ""))
    };

    if term.is_empty() {
        return None;
    }

    let detail = clean_word_network_detail_text(rest);
    let (part_of_speech, chinese, english) = parse_word_network_detail(detail);

    Some(StructuredWordNetworkItem {
        term: term.to_string(),
        part_of_speech,
        chinese,
        english,
        note: String::new(),
    })
}

fn clean_word_network_detail_text(detail: &str) -> &str {
    let detail = detail.trim().trim_start_matches([' ', ',', '-']).trim();
    strip_wrapping_pair(detail).unwrap_or(detail).trim()
}

fn parse_word_network_detail(detail: &str) -> (String, String, String) {
    let parts = detail
        .split(',')
        .map(clean_word_network_detail_part)
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>();

    match parts.as_slice() {
        [] => (String::new(), String::new(), String::new()),
        [part_of_speech] => (part_of_speech.clone(), String::new(), String::new()),
        [part_of_speech, chinese] => (part_of_speech.clone(), chinese.clone(), String::new()),
        _ => {
            let english = parts.last().cloned().unwrap_or_default();
            let chinese = parts
                .get(parts.len().saturating_sub(2))
                .cloned()
                .unwrap_or_default();
            let part_of_speech = parts[..parts.len().saturating_sub(2)].join(", ");
            (part_of_speech, chinese, english)
        }
    }
}

fn clean_word_network_detail_part(part: &str) -> String {
    let part = part.trim().trim_matches('*').trim();
    strip_wrapping_pair(part).unwrap_or(part).trim().to_string()
}

fn strip_wrapping_pair(value: &str) -> Option<&str> {
    value
        .strip_prefix('(')
        .and_then(|inner| inner.strip_suffix(')'))
        .or_else(|| {
            value
                .strip_prefix('（')
                .and_then(|inner| inner.strip_suffix('）'))
        })
}

fn parse_deep_insights(markdown: &str) -> Vec<StructuredDeepInsight> {
    let Some(section) = section_by_heading(
        markdown,
        &["#### 深度解析与避坑", "#### 深度解析", "#### 避坑"],
    ) else {
        return Vec::new();
    };

    let mut insights = Vec::new();
    let mut current_title = String::new();
    let mut current_content = Vec::new();

    for line in section.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("####") || trimmed.is_empty() {
            continue;
        }

        if is_top_level_list_item(line) {
            push_deep_insight(&mut insights, &mut current_title, &mut current_content);
            let (title, inline_content) = parse_deep_title(trimmed);
            current_title = title;
            if !inline_content.is_empty() {
                current_content.push(inline_content);
            }
        } else if !current_title.is_empty() {
            current_content.push(trimmed.to_string());
        }
    }

    push_deep_insight(&mut insights, &mut current_title, &mut current_content);
    insights
}

fn is_top_level_list_item(line: &str) -> bool {
    line.starts_with("* ") || line.starts_with("*   ") || line.starts_with("- ")
}

fn parse_deep_title(line: &str) -> (String, String) {
    let content = line
        .strip_prefix("*   ")
        .or_else(|| line.strip_prefix("* "))
        .or_else(|| line.strip_prefix("-   "))
        .or_else(|| line.strip_prefix("- "))
        .unwrap_or(line)
        .trim();
    if let Some(stripped) = content.strip_prefix("**") {
        if let Some(end) = stripped.find("**") {
            let title = stripped[..end].trim().to_string();
            let rest = stripped[end + 2..]
                .trim_start_matches([':', '：', ' '])
                .trim()
                .to_string();
            return (title, rest);
        }
    }

    (content.to_string(), String::new())
}

fn push_deep_insight(
    insights: &mut Vec<StructuredDeepInsight>,
    title: &mut String,
    content: &mut Vec<String>,
) {
    if title.trim().is_empty() {
        content.clear();
        return;
    }

    let content_markdown = content.join("\n").trim().to_string();
    if !content_markdown.is_empty() {
        insights.push(StructuredDeepInsight {
            title: title.trim().to_string(),
            content_markdown,
        });
    }
    title.clear();
    content.clear();
}

fn strip_wrapping_zh_parentheses(value: &str) -> String {
    value
        .trim()
        .trim_start_matches(['（', '('])
        .trim_end_matches(['）', ')'])
        .trim()
        .to_string()
}

fn section_by_heading<'a>(markdown: &'a str, headings: &[&str]) -> Option<&'a str> {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_bold_deep_insight_title_without_marker_leak() {
        let markdown =
            "#### 深度解析与避坑 (Einblicke)\n\n*   **词源理解**\n    从 `über` 到抽象转化。";

        let seed = build_structure_seed("übersetzen", markdown);

        assert_eq!(seed.deep_insights.len(), 1);
        assert_eq!(seed.deep_insights[0].title, "词源理解");
        assert_eq!(
            seed.deep_insights[0].content_markdown,
            "从 `über` 到抽象转化。"
        );
    }

    #[test]
    fn parses_word_network_part_of_speech_with_internal_comma() {
        let markdown = "#### 词汇网络 (Wortnetz)\n\n**词族**\n*   [die Nähe] (*Substantiv, f.*, 附近/亲近, proximity/closeness)";

        let seed = build_structure_seed("nah", markdown);

        assert_eq!(seed.word_network.family.len(), 1);
        assert_eq!(seed.word_network.family[0].term, "die Nähe");
        assert_eq!(seed.word_network.family[0].part_of_speech, "Substantiv, f.");
        assert_eq!(seed.word_network.family[0].chinese, "附近/亲近");
        assert_eq!(seed.word_network.family[0].english, "proximity/closeness");
    }

    #[test]
    fn parses_unbracketed_word_network_item_with_article_term() {
        let markdown = "#### 词汇网络 (Wortnetz)\n\n**词族**\n*   die Leitung (noun, 领导层/导线/管道, management/line/pipe)";

        let seed = build_structure_seed("Leiter", markdown);

        assert_eq!(seed.word_network.family.len(), 1);
        assert_eq!(seed.word_network.family[0].term, "die Leitung");
        assert_eq!(seed.word_network.family[0].part_of_speech, "noun");
        assert_eq!(seed.word_network.family[0].chinese, "领导层/导线/管道");
        assert_eq!(seed.word_network.family[0].english, "management/line/pipe");
    }

    #[test]
    fn preserves_parenthetical_notes_inside_word_network_detail() {
        let markdown = "#### 词汇网络 (Wortnetz)\n\n**同义词**\n*   [Föhre] (*f.*, *松树（多见于南德/奥地利）*, *pine tree*)";

        let seed = build_structure_seed("Kiefer", markdown);

        assert_eq!(seed.word_network.synonyms.len(), 1);
        assert_eq!(seed.word_network.synonyms[0].term, "Föhre");
        assert_eq!(seed.word_network.synonyms[0].part_of_speech, "f.");
        assert_eq!(
            seed.word_network.synonyms[0].chinese,
            "松树（多见于南德/奥地利）"
        );
        assert_eq!(seed.word_network.synonyms[0].english, "pine tree");
    }
}
