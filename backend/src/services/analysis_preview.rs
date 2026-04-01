use serde_json::Value;

pub fn analysis_markdown(analysis: &Value) -> String {
    analysis
        .get("markdown")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string()
}

pub fn preview_from_analysis(analysis: &Value) -> String {
    let markdown = analysis_markdown(analysis);
    if markdown.is_empty() {
        return "无法生成预览".to_string();
    }

    let lines = markdown
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .filter(|line| !line.starts_with('#'))
        .filter(|line| !line.starts_with("*/"))
        .collect::<Vec<_>>();

    let preview = if let Some(line) = lines
        .iter()
        .find(|line| line.contains("**") || line.contains("核心释义") || line.contains("* "))
    {
        line
    } else {
        lines.first().copied().unwrap_or("无法生成预览")
    };

    truncate(&strip_markdown_preview(preview), 90)
}

fn truncate(value: &str, max_len: usize) -> String {
    if value.chars().count() <= max_len {
        value.to_string()
    } else {
        value.chars().take(max_len).collect::<String>() + "..."
    }
}

fn strip_markdown_preview(value: &str) -> String {
    let mut cleaned = String::with_capacity(value.len());
    let mut previous_was_space = false;

    for ch in value.chars() {
        let normalized = match ch {
            '*' | '`' | '#' | '[' | ']' | '(' | ')' | '>' | '_' => None,
            '\n' | '\r' | '\t' => Some(' '),
            _ => Some(ch),
        };

        if let Some(ch) = normalized {
            let ch = if ch.is_whitespace() { ' ' } else { ch };
            if ch == ' ' {
                if previous_was_space {
                    continue;
                }
                previous_was_space = true;
            } else {
                previous_was_space = false;
            }
            cleaned.push(ch);
        }
    }

    cleaned
        .trim()
        .trim_start_matches("- ")
        .trim_start_matches("* ")
        .trim()
        .to_string()
}
