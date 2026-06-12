pub(crate) fn json_candidate(raw: &str) -> &str {
    if let Some(start) = raw.find('{') {
        if let Some(end) = raw.rfind('}') {
            return &raw[start..=end];
        }
        return &raw[start..];
    }

    raw
}

pub(crate) fn repair_json_inner_quotes(candidate: &str) -> Option<String> {
    let mut repaired = String::with_capacity(candidate.len() + 16);
    let mut in_string = false;
    let mut escape = false;
    let mut changed = false;

    for (index, ch) in candidate.char_indices() {
        if !in_string {
            repaired.push(ch);
            if ch == '"' {
                in_string = true;
            }
            continue;
        }

        if escape {
            repaired.push(ch);
            escape = false;
            continue;
        }

        match ch {
            '\\' => {
                repaired.push(ch);
                escape = true;
            }
            '"' => {
                if should_escape_inner_quote(candidate, index + ch.len_utf8()) {
                    repaired.push('\\');
                    repaired.push('"');
                    changed = true;
                } else {
                    repaired.push('"');
                    in_string = false;
                }
            }
            _ => repaired.push(ch),
        }
    }

    changed.then_some(repaired)
}

pub(crate) fn repair_truncated_json(candidate: &str) -> Option<String> {
    let trimmed = candidate.trim();
    if trimmed.is_empty() {
        return None;
    }

    let boundaries = trimmed
        .char_indices()
        .map(|(index, _)| index)
        .chain(std::iter::once(trimmed.len()))
        .collect::<Vec<_>>();
    let max_trim_steps = boundaries.len().saturating_sub(1).min(1200);

    for trim_count in 0..=max_trim_steps {
        let end = boundaries[boundaries.len() - 1 - trim_count];
        let prefix = &trimmed[..end];
        let repaired = close_truncated_json(prefix)?;
        if serde_json::from_str::<serde_json::Value>(&repaired).is_ok() {
            return Some(repaired);
        }
    }

    None
}

fn should_escape_inner_quote(candidate: &str, next_index: usize) -> bool {
    let next_non_ws = candidate[next_index..]
        .chars()
        .find(|ch| !ch.is_whitespace());
    !matches!(
        next_non_ws,
        None | Some(',') | Some('}') | Some(']') | Some(':')
    )
}

fn close_truncated_json(prefix: &str) -> Option<String> {
    let mut repaired = prefix.trim_end().to_string();
    if repaired.is_empty() {
        return None;
    }

    let mut in_string = false;
    let mut escape = false;
    let mut stack = Vec::new();

    for ch in repaired.chars() {
        if in_string {
            if escape {
                escape = false;
                continue;
            }
            match ch {
                '\\' => escape = true,
                '"' => in_string = false,
                _ => {}
            }
            continue;
        }

        match ch {
            '"' => in_string = true,
            '{' => stack.push('}'),
            '[' => stack.push(']'),
            '}' | ']' => {
                if stack.last().copied() == Some(ch) {
                    stack.pop();
                }
            }
            _ => {}
        }
    }

    if in_string {
        repaired.push('"');
    }

    loop {
        while repaired.ends_with(char::is_whitespace) {
            repaired.pop();
        }

        match repaired.chars().last() {
            Some(',') | Some(':') => {
                repaired.pop();
            }
            Some('{') => {
                repaired.pop();
                if stack.last().copied() == Some('}') {
                    stack.pop();
                }
            }
            Some('[') => {
                repaired.pop();
                if stack.last().copied() == Some(']') {
                    stack.pop();
                }
            }
            _ => break,
        }
    }

    if repaired.is_empty() {
        return None;
    }

    while let Some(closer) = stack.pop() {
        repaired.push(closer);
    }

    Some(repaired)
}

#[cfg(test)]
mod tests {
    use super::{repair_json_inner_quotes, repair_truncated_json};
    use crate::services::analyze_support::extract_json;
    use serde_json::json;

    #[test]
    fn extract_json_repairs_unescaped_inner_quotes() {
        let raw = r#"{
          "title": "Absolut! (als Antwort)",
          "explanation": "作为独立的感叹词，表示强烈的赞同。相当于"没错！"或"当然！"。"
        }"#;

        let parsed = extract_json::<serde_json::Value>(raw).expect("json should be repaired");
        assert_eq!(
            parsed,
            json!({
              "title": "Absolut! (als Antwort)",
              "explanation": "作为独立的感叹词，表示强烈的赞同。相当于\"没错！\"或\"当然！\"。"
            })
        );
    }

    #[test]
    fn extract_json_repairs_unescaped_quotes_before_truncation_repair() {
        let raw = r#"{
          "usage_modules": [
            {
              "title": "etwas nicht pauschal sagen",
              "explanation": "表示"不能一概而论"。",
              "example_de": "Das kann man nicht pauschal sagen."
        "#;

        let parsed = extract_json::<serde_json::Value>(raw).expect("json should be repaired");
        assert_eq!(
            parsed,
            json!({
              "usage_modules": [
                {
                  "title": "etwas nicht pauschal sagen",
                  "explanation": "表示\"不能一概而论\"。",
                  "example_de": "Das kann man nicht pauschal sagen."
                }
              ]
            })
        );
    }

    #[test]
    fn inner_quote_repair_only_changes_when_needed() {
        let raw = r#"{"ok":"plain","nested":"already \"escaped\""}"#;
        assert!(repair_json_inner_quotes(raw).is_none());
        assert!(repair_truncated_json(raw).is_some());
    }
}
