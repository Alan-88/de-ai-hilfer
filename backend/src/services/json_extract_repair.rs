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

fn should_escape_inner_quote(candidate: &str, next_index: usize) -> bool {
    let next_non_ws = candidate[next_index..]
        .chars()
        .find(|ch| !ch.is_whitespace());
    !matches!(
        next_non_ws,
        None | Some(',') | Some('}') | Some(']') | Some(':')
    )
}

#[cfg(test)]
mod tests {
    use super::repair_json_inner_quotes;
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
    fn rejects_truncated_output_instead_of_cutting_or_closing_it() {
        let raw = r#"{
          "usage_modules": [
            {
              "title": "etwas nicht pauschal sagen",
              "explanation": "表示"不能一概而论"。",
              "example_de": "Das kann man nicht pauschal sagen."
        "#;

        assert!(extract_json::<serde_json::Value>(raw).is_err());
        assert!(extract_json::<serde_json::Value>(
            r#"{"deep_insights":[{"title":"词源","content_markdown":"半截"#
        )
        .is_err());
    }

    #[test]
    fn inner_quote_repair_only_changes_when_needed() {
        let raw = r#"{"ok":"plain","nested":"already \"escaped\""}"#;
        assert!(repair_json_inner_quotes(raw).is_none());
    }
}
