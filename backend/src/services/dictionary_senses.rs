use crate::services::dictionary_tags::is_noise_tag;
use serde_json::Value;

pub fn compact_senses(
    raw_data: &Value,
    sense_limit: usize,
    gloss_limit: &'static str,
) -> Vec<String> {
    raw_data
        .get("senses")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .take(sense_limit)
                .filter_map(|sense| {
                    let glosses = sense.get("glosses").and_then(Value::as_array)?;
                    let joined = glosses
                        .iter()
                        .filter_map(Value::as_str)
                        .take(2)
                        .collect::<Vec<_>>()
                        .join(gloss_limit);
                    if joined.is_empty() {
                        None
                    } else {
                        let tags = sense
                            .get("tags")
                            .and_then(Value::as_array)
                            .map(|tags| {
                                tags.iter()
                                    .filter_map(Value::as_str)
                                    .take(3)
                                    .collect::<Vec<_>>()
                                    .join(", ")
                            })
                            .unwrap_or_default();
                        Some(if tags.is_empty() {
                            joined
                        } else {
                            format!("{joined} [{tags}]")
                        })
                    }
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

pub fn detailed_senses(raw_data: &Value, sense_limit: usize) -> Vec<String> {
    raw_data
        .get("senses")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .take(sense_limit)
                .filter_map(|sense| {
                    let glosses = sense
                        .get("glosses")
                        .and_then(Value::as_array)
                        .map(|items| {
                            items
                                .iter()
                                .filter_map(Value::as_str)
                                .take(2)
                                .collect::<Vec<_>>()
                        })
                        .unwrap_or_default();
                    if glosses.is_empty() {
                        return None;
                    }

                    let mut line = format!("- {}", glosses.join(" / "));
                    let tags = sense
                        .get("tags")
                        .and_then(Value::as_array)
                        .map(|items| {
                            items
                                .iter()
                                .filter_map(Value::as_str)
                                .filter(|tag| !is_noise_tag(tag))
                                .take(4)
                                .collect::<Vec<_>>()
                        })
                        .unwrap_or_default();
                    if !tags.is_empty() {
                        line.push_str(&format!("（{}）", tags.join(", ")));
                    }
                    Some(line)
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}
