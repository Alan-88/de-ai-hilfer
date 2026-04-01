use serde_json::Value;

pub fn build_tags(raw_data: &Value, fallback_pos: &str) -> Vec<String> {
    let mut tags = raw_data
        .get("senses")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .flat_map(|sense| {
            sense
                .get("tags")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .filter_map(Value::as_str)
        })
        .filter(|tag| !is_noise_tag(tag))
        .take(4)
        .map(ToString::to_string)
        .collect::<Vec<_>>();

    if tags.is_empty() {
        tags.push(fallback_pos.to_string());
    } else if !tags.iter().any(|tag| tag == fallback_pos) {
        tags.insert(0, fallback_pos.to_string());
    }

    tags
}

pub fn is_noise_tag(tag: &str) -> bool {
    matches!(
        tag,
        "class-1"
            | "class-2"
            | "class-3"
            | "class-4"
            | "class-5"
            | "class-6"
            | "class-7"
            | "strong"
            | "weak"
            | "table-tags"
            | "inflection-template"
            | "often"
    )
}
