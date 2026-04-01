use serde_json::{json, Value};
use std::collections::BTreeSet;

pub fn build_gloss_preview(pos: &str, raw_data: &Value) -> Value {
    let glosses = raw_data
        .get("senses")
        .and_then(Value::as_array)
        .and_then(|senses| senses.first())
        .and_then(|sense| sense.get("glosses"))
        .cloned()
        .unwrap_or_else(|| Value::Array(Vec::new()));

    json!({
        "pos": pos,
        "glosses": glosses
    })
}

pub fn clean_entry(mut entry: Value) -> Value {
    if let Some(senses) = entry.get_mut("senses").and_then(|s| s.as_array_mut()) {
        senses.retain(|sense| {
            if let Some(tags) = sense.get("tags").and_then(|t| t.as_array()) {
                !tags.iter().any(|tag| tag.as_str() == Some("obsolete"))
            } else {
                true
            }
        });
    }

    entry
}

pub fn sense_has_tag(entry: &Value, tag_name: &str) -> bool {
    entry
        .get("senses")
        .and_then(Value::as_array)
        .map(|senses| {
            senses.iter().any(|sense| {
                sense
                    .get("tags")
                    .and_then(Value::as_array)
                    .map(|tags| tags.iter().any(|tag| tag.as_str() == Some(tag_name)))
                    .unwrap_or(false)
            })
        })
        .unwrap_or(false)
}

pub fn extract_form_of_words(entry: &Value) -> Vec<String> {
    let mut words = BTreeSet::new();

    if let Some(senses) = entry.get("senses").and_then(Value::as_array) {
        for sense in senses {
            if let Some(items) = sense.get("form_of").and_then(Value::as_array) {
                for item in items {
                    if let Some(word) = item.get("word").and_then(Value::as_str) {
                        words.insert(word.to_string());
                    }
                }
            }
        }
    }

    words.into_iter().collect()
}

pub fn extract_forms(entry: &Value) -> Vec<String> {
    let mut forms = BTreeSet::new();
    if let Some(items) = entry.get("forms").and_then(Value::as_array) {
        for item in items {
            let skip_noise = item
                .get("tags")
                .and_then(Value::as_array)
                .map(|tags| {
                    tags.iter().any(|tag| {
                        matches!(
                            tag.as_str(),
                            Some("table-tags" | "inflection-template" | "class")
                        )
                    })
                })
                .unwrap_or(false);
            if skip_noise {
                continue;
            }

            if let Some(form) = item.get("form").and_then(Value::as_str) {
                let trimmed = form.trim();
                if !trimmed.is_empty() {
                    forms.insert(trimmed.to_string());
                }
            }
        }
    }
    forms.into_iter().collect()
}

pub fn normalize_surface(value: &str) -> String {
    value
        .trim()
        .to_lowercase()
        .chars()
        .flat_map(|ch| match ch {
            'ä' => ['a'].into_iter().collect::<Vec<_>>(),
            'ö' => ['o'].into_iter().collect::<Vec<_>>(),
            'ü' => ['u'].into_iter().collect::<Vec<_>>(),
            'ß' => ['s', 's'].into_iter().collect::<Vec<_>>(),
            _ => vec![ch],
        })
        .collect()
}
