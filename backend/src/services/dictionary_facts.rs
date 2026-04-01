use serde_json::Value;

pub fn dictionary_pos(raw_data: &Value) -> &str {
    raw_data
        .get("pos")
        .and_then(Value::as_str)
        .unwrap_or("未知词性")
}

pub fn dictionary_ipa(raw_data: &Value) -> &str {
    raw_data
        .get("sounds")
        .and_then(Value::as_array)
        .and_then(|sounds| {
            sounds
                .iter()
                .find_map(|sound| sound.get("ipa").and_then(Value::as_str))
        })
        .unwrap_or("")
}

pub fn compact_forms(raw_data: &Value, form_limit: usize, tag_limit: usize) -> Vec<String> {
    raw_data
        .get("forms")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter(|form| {
                    form.get("source").is_none()
                        || form.get("source").and_then(Value::as_str) == Some("conjugation")
                })
                .filter_map(|form| {
                    let form_value = form.get("form").and_then(Value::as_str)?;
                    let tags = form
                        .get("tags")
                        .and_then(Value::as_array)
                        .map(|tags| {
                            tags.iter()
                                .filter_map(Value::as_str)
                                .take(tag_limit)
                                .collect::<Vec<_>>()
                                .join(", ")
                        })
                        .unwrap_or_default();
                    Some(if tags.is_empty() {
                        form_value.to_string()
                    } else {
                        format!("{form_value} ({tags})")
                    })
                })
                .take(form_limit)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

pub fn find_auxiliary(raw_data: &Value) -> Option<&str> {
    raw_data
        .get("forms")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .find_map(|form| {
            let is_auxiliary = form
                .get("tags")
                .and_then(Value::as_array)
                .map(|tags| tags.iter().any(|tag| tag.as_str() == Some("auxiliary")))
                .unwrap_or(false);
            if is_auxiliary {
                form.get("form").and_then(Value::as_str)
            } else {
                None
            }
        })
}

pub fn truncate_for_prompt(value: &str, max_len: usize) -> String {
    if value.chars().count() <= max_len {
        value.to_string()
    } else {
        value.chars().take(max_len).collect::<String>() + "..."
    }
}
