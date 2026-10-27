use anyhow::{anyhow, Result};

pub fn validate_stage2_markdown_completeness(markdown: &str) -> Result<()> {
    let normalized = markdown.replace('\r', "");
    for title in ["应用与例句", "词汇网络", "深度解析与避坑"] {
        let heading = format!("#### {title}");
        let mut lines = normalized
            .lines()
            .skip_while(|line| !line.starts_with(&heading));
        if lines.next().is_none() {
            return Err(anyhow!(
                "grounded stage2 markdown missing section: {heading}"
            ));
        }
        if !lines
            .take_while(|line| !line.starts_with("#### "))
            .any(|line| !line.trim().is_empty())
        {
            return Err(anyhow!("grounded stage2 markdown empty section: {heading}"));
        }
    }
    let source = super::analysis_structure_seed::build_structure_seed("", &normalized);
    if source.usage_modules.is_empty() || source.deep_insights.is_empty() {
        return Err(anyhow!(
            "grounded stage2 markdown has no complete usage module or deep insight"
        ));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::validate_stage2_markdown_completeness;

    #[test]
    fn rejects_empty_sections_even_in_long_documents() {
        let markdown = format!(
            "#### 应用与例句\n{}\n#### 词汇网络\n\n#### 深度解析与避坑\n正文",
            "x".repeat(1000)
        );
        assert!(validate_stage2_markdown_completeness(&markdown)
            .unwrap_err()
            .to_string()
            .contains("empty section"));
    }

    #[test]
    fn accepts_short_content_without_terminal_punctuation() {
        validate_stage2_markdown_completeness("#### 应用与例句\n\ngar nicht\n用法解析：完全不\n场景例句：Gar nicht\n例句翻译：完全不\n\n#### 词汇网络\n**词族**\n* 无高价值项\n\n#### 深度解析与避坑\n\n* **否定**\n完整的无句号解释").unwrap();
        assert!(validate_stage2_markdown_completeness(
            "#### 应用与例句\nx\n#### 词汇网络\nx\n#### 深度解析与避坑\nx"
        )
        .is_err());
    }

    #[test]
    fn accepts_complete_markdown_ending_with_bold_markup() {
        let markdown = format!(
            "#### 应用与例句 (Anwendung & Beispiele)\n\ndas Radio anmachen\n用法解析：{}\n场景例句：Ich mache das Radio an.\n例句翻译：我打开收音机。\n\n#### 词汇网络 (Wortnetz)\n\n**词族**\n* 无高价值项\n\n#### 深度解析与避坑 (Einblicke)\n\n* **语法提醒**\n    作为可分动词，前缀位于句尾：*Ich habe das Radio **an**ge**macht**.*",
            "打开收音机的表达。".repeat(40)
        );

        validate_stage2_markdown_completeness(&markdown).unwrap();
    }
}
