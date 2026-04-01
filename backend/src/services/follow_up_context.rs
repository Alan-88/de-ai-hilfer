use serde_json::Value;

pub fn dictionary_senses(excerpt: &Value) -> Option<String> {
    excerpt
        .get("senses")
        .and_then(Value::as_array)
        .map(|senses| {
            senses
                .iter()
                .filter_map(|sense| {
                    sense
                        .get("glosses")
                        .and_then(Value::as_array)
                        .map(|glosses| {
                            glosses
                                .iter()
                                .filter_map(Value::as_str)
                                .take(2)
                                .collect::<Vec<_>>()
                                .join(" / ")
                        })
                })
                .filter(|item| !item.is_empty())
                .take(2)
                .collect::<Vec<_>>()
        })
        .filter(|items| !items.is_empty())
        .map(|items| items.join("；"))
}

pub fn dictionary_forms(excerpt: &Value) -> Option<String> {
    excerpt
        .get("forms")
        .and_then(Value::as_array)
        .map(|forms| {
            forms
                .iter()
                .filter_map(|form| {
                    let form_value = form.get("form").and_then(Value::as_str)?;
                    let tags = form
                        .get("tags")
                        .and_then(Value::as_array)
                        .map(|tags| compact_tags(tags))
                        .unwrap_or_default();
                    if matches!(form_value, "strong" | "weak" | "de-conj") {
                        return None;
                    }
                    Some(if tags.is_empty() {
                        form_value.to_string()
                    } else {
                        format!("{form_value} ({tags})")
                    })
                })
                .take(4)
                .collect::<Vec<_>>()
        })
        .filter(|items| !items.is_empty())
        .map(|items| items.join("；"))
}

pub fn dictionary_ipa(excerpt: &Value) -> Option<String> {
    excerpt
        .get("sounds")
        .and_then(Value::as_array)
        .and_then(|sounds| {
            sounds
                .iter()
                .find_map(|sound| sound.get("ipa").and_then(Value::as_str))
        })
        .map(str::to_string)
}

pub fn section_lines(markdown: &str, title: &str) -> Vec<String> {
    let mut in_section = false;
    let mut lines = Vec::new();

    for line in markdown.lines().map(str::trim) {
        if line.starts_with("### ") || line.starts_with("## ") {
            in_section = line.contains(title);
            continue;
        }
        if !in_section || line.is_empty() {
            continue;
        }
        if line.starts_with("### ") {
            break;
        }
        lines.push(line.trim_start_matches("- ").to_string());
    }

    lines
}

pub fn first_summary_line(markdown: &str) -> Option<String> {
    markdown
        .lines()
        .map(str::trim)
        .find(|line| {
            !line.is_empty()
                && !line.starts_with('#')
                && !line.starts_with("```")
                && !line.starts_with("*/")
        })
        .map(ToString::to_string)
}

fn compact_tags(tags: &[Value]) -> String {
    tags.iter()
        .filter_map(Value::as_str)
        .filter(|tag| !matches!(*tag, "table-tags" | "inflection-template" | "class"))
        .take(3)
        .collect::<Vec<_>>()
        .join(", ")
}
