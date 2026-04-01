use serde_json::Value;

pub fn collect_examples(raw_data: &Value, example_limit: usize) -> Vec<String> {
    raw_data
        .get("senses")
        .and_then(Value::as_array)
        .map(|senses| {
            senses
                .iter()
                .flat_map(|sense| {
                    sense
                        .get("examples")
                        .and_then(Value::as_array)
                        .into_iter()
                        .flatten()
                })
                .filter_map(|example| {
                    let text = example.get("text").and_then(Value::as_str)?.trim();
                    if text.is_empty() {
                        return None;
                    }
                    let translation = example
                        .get("translation")
                        .and_then(Value::as_str)
                        .or_else(|| example.get("english").and_then(Value::as_str))
                        .map(str::trim)
                        .filter(|value| !value.is_empty());
                    Some(match translation {
                        Some(translation) => format!("- `{text}`\n  释义：{translation}"),
                        None => format!("- `{text}`"),
                    })
                })
                .take(example_limit)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

pub fn usage_note(raw_data: &Value) -> Option<String> {
    raw_data
        .get("senses")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .find_map(|sense| {
            let tags = sense.get("tags").and_then(Value::as_array)?;
            let visible_tags = tags
                .iter()
                .filter_map(Value::as_str)
                .filter(|tag| {
                    matches!(
                        *tag,
                        "colloquial" | "formal" | "regional" | "dated" | "impersonal"
                    )
                })
                .collect::<Vec<_>>();
            if visible_tags.is_empty() {
                None
            } else {
                Some(format!(
                    "部分义项带有语域/用法标签：{}。",
                    visible_tags.join(", ")
                ))
            }
        })
}
