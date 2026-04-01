use crate::models::DictionaryRaw;
use serde_json::{json, Value};
use std::collections::{BTreeSet, HashSet};

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct DictionaryRawLookupRow {
    pub headword: String,
    pub raw_data: Value,
    pub has_audio: Option<bool>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub is_form_of: bool,
    pub pos: Option<String>,
}

pub fn merge_preferred_raw_rows(rows: Vec<DictionaryRawLookupRow>) -> Option<DictionaryRaw> {
    let preferred_headword = rows.first()?.headword.clone();
    let filtered = rows
        .into_iter()
        .filter(|row| row.headword == preferred_headword)
        .collect::<Vec<_>>();
    merge_raw_rows(filtered)
}

pub fn merge_raw_rows(rows: Vec<DictionaryRawLookupRow>) -> Option<DictionaryRaw> {
    let preferred_rows = if rows.iter().any(|row| !row.is_form_of) {
        rows.into_iter()
            .filter(|row| !row.is_form_of)
            .collect::<Vec<_>>()
    } else {
        rows
    };

    let first = preferred_rows.first()?.clone();
    if preferred_rows.len() == 1 {
        return Some(DictionaryRaw {
            headword: first.headword,
            raw_data: first.raw_data,
            has_audio: first.has_audio,
            created_at: first.created_at,
        });
    }

    let pos_summary = preferred_rows
        .iter()
        .filter_map(|row| row.pos.as_deref())
        .filter(|pos| !pos.is_empty())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>()
        .join(" / ");

    let mut raw_data = first.raw_data.clone();
    raw_data["word"] = Value::String(first.headword.clone());
    if !pos_summary.is_empty() {
        raw_data["pos"] = Value::String(pos_summary);
    }

    raw_data["sounds"] = Value::Array(merge_json_values(preferred_rows.iter().flat_map(|row| {
        row.raw_data
            .get("sounds")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default()
    })));
    raw_data["forms"] = Value::Array(merge_json_values(preferred_rows.iter().flat_map(|row| {
        row.raw_data
            .get("forms")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default()
    })));
    raw_data["senses"] = Value::Array(merge_json_values(preferred_rows.iter().flat_map(|row| {
        row.raw_data
            .get("senses")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .map(|sense| annotate_sense_with_pos(sense, row.pos.as_deref()))
            .collect::<Vec<_>>()
    })));
    raw_data["merged_entries"] = Value::Array(
        preferred_rows
            .iter()
            .map(|row| {
                json!({
                    "pos": row.pos,
                    "is_form_of": row.is_form_of,
                    "word": row.raw_data.get("word").cloned().unwrap_or(Value::Null)
                })
            })
            .collect(),
    );

    Some(DictionaryRaw {
        headword: first.headword,
        raw_data,
        has_audio: Some(preferred_rows.iter().any(|row| row.has_audio.unwrap_or(false))),
        created_at: preferred_rows
            .iter()
            .map(|row| row.created_at)
            .min()
            .unwrap_or(first.created_at),
    })
}

fn annotate_sense_with_pos(mut sense: Value, pos: Option<&str>) -> Value {
    let Some(pos) = pos.filter(|value| !value.is_empty()) else {
        return sense;
    };

    let Some(object) = sense.as_object_mut() else {
        return sense;
    };

    let tags = object
        .entry("tags".to_string())
        .or_insert_with(|| Value::Array(Vec::new()));

    match tags {
        Value::Array(items) => {
            if !items.iter().any(|item| item.as_str() == Some(pos)) {
                items.insert(0, Value::String(pos.to_string()));
            }
        }
        _ => {
            *tags = Value::Array(vec![Value::String(pos.to_string())]);
        }
    }

    sense
}

fn merge_json_values(values: impl Iterator<Item = Value>) -> Vec<Value> {
    let mut seen = HashSet::new();
    let mut merged = Vec::new();

    for value in values {
        let Ok(key) = serde_json::to_string(&value) else {
            continue;
        };
        if seen.insert(key) {
            merged.push(value);
        }
    }

    merged
}
