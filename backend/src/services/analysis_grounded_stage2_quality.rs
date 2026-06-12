use anyhow::{anyhow, Result};

pub fn validate_stage2_markdown_completeness(markdown: &str) -> Result<()> {
    let normalized = markdown.replace('\r', "");
    for heading in [
        "#### 应用与例句 (Anwendung & Beispiele)",
        "#### 词汇网络 (Wortnetz)",
        "#### 深度解析与避坑 (Einblicke)",
    ] {
        if !normalized.contains(heading) {
            return Err(anyhow!(
                "grounded stage2 markdown missing section: {heading}"
            ));
        }
    }

    let trimmed = normalized.trim_end();
    if trimmed.len() < 800 {
        return Err(anyhow!(
            "grounded stage2 markdown unexpectedly short: len={}",
            trimmed.len()
        ));
    }

    let tail = trimmed.chars().rev().take(80).collect::<String>();
    let tail = tail.chars().rev().collect::<String>();
    let punctuation_tail = trimmed
        .trim_end_matches('*')
        .trim_end_matches('`')
        .trim_end_matches('"')
        .trim_end_matches('\'')
        .trim_end_matches('」')
        .trim_end_matches('』')
        .trim_end_matches('”')
        .trim_end_matches('’')
        .trim_end_matches('*');
    let complete_punctuation = ['。', '！', '？', '.', '!', '?', ')', '）', '`']
        .iter()
        .any(|punctuation| punctuation_tail.ends_with(*punctuation));
    if !complete_punctuation {
        return Err(anyhow!(
            "grounded stage2 markdown appears truncated: tail={tail}"
        ));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::validate_stage2_markdown_completeness;

    #[test]
    fn rejects_truncated_stage2_markdown() {
        let markdown = "#### 应用与例句 (Anwendung & Beispiele)\n\nx\n\n#### 词汇网络 (Wortnetz)\n\nx\n\n#### 深度解析与避坑 (Einblicke)\n\n* 格位\n    在德语逻辑中，D";

        let err = validate_stage2_markdown_completeness(markdown).unwrap_err();
        assert!(err.to_string().contains("unexpectedly short"));
    }

    #[test]
    fn accepts_complete_markdown_ending_with_bold_markup() {
        let markdown = format!(
            "#### 应用与例句 (Anwendung & Beispiele)\n\n{}\n\n#### 词汇网络 (Wortnetz)\n\n{}\n\n#### 深度解析与避坑 (Einblicke)\n\n* **语法提醒**\n    作为可分动词，前缀位于句尾：*Ich habe das Radio **an**ge**macht**.*",
            "x".repeat(400),
            "y".repeat(400)
        );

        validate_stage2_markdown_completeness(&markdown).unwrap();
    }
}
