use crate::db::DbPool;
use crate::repositories::dictionary_merge::DictionaryRawLookupRow;
use anyhow::Result;
use serde_json::{json, Value};
use std::collections::BTreeSet;

pub async fn load_raw_rows_by_headword(
    pool: &DbPool,
    headword: &str,
) -> Result<Vec<DictionaryRawLookupRow>, sqlx::Error> {
    sqlx::query_as::<_, DictionaryRawLookupRow>(
        r#"
        SELECT
            headword,
            raw_data,
            has_audio,
            created_at,
            is_form_of,
            pos
        FROM dictionary_raw_entries
        WHERE lower(headword) = lower($1)
        ORDER BY
            CASE WHEN headword = $1 THEN 0 ELSE 1 END,
            is_form_of ASC,
            COALESCE(pos, '') ASC,
            id ASC
        "#,
    )
    .bind(headword)
    .fetch_all(pool)
    .await
}

pub fn build_light_dictionary_facts_payload(word: &str, rows: &[DictionaryRawLookupRow]) -> String {
    let preferred_rows = preferred_headword_rows(rows);
    let payload = json!({
        "query": word,
        "case_signal": detect_case_signal(word),
        "dictionary_rows": preferred_rows
            .iter()
            .map(build_row_fact)
            .collect::<Vec<_>>(),
    });
    serde_json::to_string_pretty(&payload).unwrap_or_else(|_| "{}".to_string())
}

pub fn first_ipa_from_dictionary_facts(dictionary_facts: Option<&str>) -> Option<String> {
    let facts = dictionary_facts?;
    let value = serde_json::from_str::<Value>(facts).ok()?;
    value
        .get("dictionary_rows")?
        .as_array()?
        .iter()
        .filter_map(|row| row.get("ipas").and_then(Value::as_array))
        .flat_map(|ipas| ipas.iter())
        .find_map(|ipa| ipa.as_str().map(str::trim).filter(|ipa| !ipa.is_empty()))
        .map(ToString::to_string)
}

fn preferred_headword_rows(rows: &[DictionaryRawLookupRow]) -> Vec<&DictionaryRawLookupRow> {
    let Some(preferred_headword) = rows.first().map(|row| row.headword.as_str()) else {
        return Vec::new();
    };

    let preferred_rows = rows
        .iter()
        .filter(|row| row.headword == preferred_headword)
        .collect::<Vec<_>>();

    if preferred_rows.iter().any(|row| !row.is_form_of) {
        preferred_rows
            .into_iter()
            .filter(|row| !row.is_form_of)
            .collect()
    } else {
        preferred_rows
    }
}

fn detect_case_signal(word: &str) -> &'static str {
    let mut chars = word.chars().filter(|ch| ch.is_alphabetic());
    let Some(first) = chars.next() else {
        return "other";
    };
    if first.is_uppercase() {
        "titlecase_or_uppercase"
    } else if first.is_lowercase() {
        "lowercase"
    } else {
        "other"
    }
}

fn build_row_fact(row: &&DictionaryRawLookupRow) -> Value {
    let raw_data = &row.raw_data;
    json!({
        "word": raw_data.get("word").and_then(Value::as_str).unwrap_or(""),
        "pos": row.pos.as_deref().unwrap_or("unknown"),
        "is_form_of": row.is_form_of,
        "ipas": collect_ipas(raw_data),
        "head_templates": collect_head_templates(raw_data),
        "senses": collect_senses(raw_data),
        "forms": collect_fact_forms(raw_data),
    })
}

fn collect_ipas(raw_data: &Value) -> Vec<String> {
    raw_data
        .get("sounds")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(|item| item.get("ipa").and_then(Value::as_str))
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(ToString::to_string)
                .collect::<BTreeSet<_>>()
                .into_iter()
                .take(4)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

fn collect_head_templates(raw_data: &Value) -> Vec<String> {
    raw_data
        .get("head_templates")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(|item| item.get("expansion").and_then(Value::as_str))
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(ToString::to_string)
                .collect::<BTreeSet<_>>()
                .into_iter()
                .take(4)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

fn collect_senses(raw_data: &Value) -> Vec<Value> {
    raw_data
        .get("senses")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(|sense| {
                    let glosses = sense
                        .get("glosses")
                        .and_then(Value::as_array)
                        .map(|glosses| {
                            glosses
                                .iter()
                                .filter_map(Value::as_str)
                                .map(str::trim)
                                .filter(|value| !value.is_empty())
                                .take(3)
                                .map(ToString::to_string)
                                .collect::<Vec<_>>()
                        })
                        .unwrap_or_default();
                    if glosses.is_empty() {
                        return None;
                    }

                    let tags = sense
                        .get("tags")
                        .and_then(Value::as_array)
                        .map(|tags| {
                            tags.iter()
                                .filter_map(Value::as_str)
                                .map(str::trim)
                                .filter(|value| !value.is_empty())
                                .map(ToString::to_string)
                                .take(8)
                                .collect::<Vec<_>>()
                        })
                        .unwrap_or_default();

                    Some(json!({
                        "glosses": glosses,
                        "tags": tags,
                    }))
                })
                .take(8)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

fn collect_fact_forms(raw_data: &Value) -> Value {
    let forms = raw_data
        .get("forms")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let pos = raw_data
        .get("pos")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_ascii_lowercase();

    json!({
        "auxiliaries": collect_forms_by_required_tag(&forms, "auxiliary", 4),
        "present_3sg": collect_forms_with_all_tags(&forms, &["present", "singular", "third-person"], 4),
        "preterite_3sg": collect_forms_with_all_tags_excluding(&forms, &["past"], &["participle", "subjunctive"], 4),
        "partizip_ii": collect_forms_with_all_tags(&forms, &["participle", "past"], 4),
        "plural_forms": if pos == "noun" {
            collect_noun_plural_forms(&forms, 6)
        } else {
            Vec::new()
        },
        "genitive_forms": if pos == "noun" {
            collect_noun_genitive_forms(&forms, 6)
        } else {
            Vec::new()
        },
        "comparative": collect_forms_with_any_tags(&forms, &["comparative"], 4),
        "superlative": collect_forms_with_any_tags(&forms, &["superlative"], 4),
    })
}

fn collect_forms_by_required_tag(forms: &[Value], required_tag: &str, limit: usize) -> Vec<String> {
    collect_form_values(
        forms
            .iter()
            .filter(|form| has_all_tags(form, &[required_tag])),
        limit,
    )
}

fn collect_forms_with_all_tags(forms: &[Value], tags: &[&str], limit: usize) -> Vec<String> {
    collect_form_values(forms.iter().filter(|form| has_all_tags(form, tags)), limit)
}

fn collect_forms_with_all_tags_excluding(
    forms: &[Value],
    required_tags: &[&str],
    excluded_tags: &[&str],
    limit: usize,
) -> Vec<String> {
    collect_form_values(
        forms
            .iter()
            .filter(|form| has_all_tags(form, required_tags) && !has_any_tag(form, excluded_tags)),
        limit,
    )
}

fn collect_forms_with_any_tags(forms: &[Value], tags: &[&str], limit: usize) -> Vec<String> {
    collect_form_values(forms.iter().filter(|form| has_any_tag(form, tags)), limit)
}

fn collect_noun_plural_forms(forms: &[Value], limit: usize) -> Vec<String> {
    collect_form_values(
        forms.iter().filter(|form| {
            has_any_tag(form, &["plural"])
                && (has_any_tag(form, &["nominative"])
                    || !has_any_tag(form, &["genitive", "dative", "accusative"]))
        }),
        limit,
    )
}

fn collect_noun_genitive_forms(forms: &[Value], limit: usize) -> Vec<String> {
    collect_form_values(
        forms.iter().filter(|form| {
            has_any_tag(form, &["genitive"])
                && (has_any_tag(form, &["singular"]) || !has_any_tag(form, &["plural"]))
        }),
        limit,
    )
}

fn collect_form_values<'a>(forms: impl Iterator<Item = &'a Value>, limit: usize) -> Vec<String> {
    let mut seen = BTreeSet::new();
    let mut values = Vec::new();

    for form in forms {
        let Some(value) = form.get("form").and_then(Value::as_str).map(str::trim) else {
            continue;
        };
        if value.is_empty() || value == "de-conj" || value == "de-ndecl" || value == "de-adecl" {
            continue;
        }
        if seen.insert(value.to_string()) {
            values.push(value.to_string());
        }
        if values.len() >= limit {
            break;
        }
    }

    values
}

fn has_all_tags(form: &Value, expected: &[&str]) -> bool {
    let tags = form
        .get("tags")
        .and_then(Value::as_array)
        .map(|items| items.iter().filter_map(Value::as_str).collect::<Vec<_>>())
        .unwrap_or_default();
    expected
        .iter()
        .all(|tag| tags.iter().any(|item| item == tag))
}

fn has_any_tag(form: &Value, expected: &[&str]) -> bool {
    let tags = form
        .get("tags")
        .and_then(Value::as_array)
        .map(|items| items.iter().filter_map(Value::as_str).collect::<Vec<_>>())
        .unwrap_or_default();
    expected
        .iter()
        .any(|tag| tags.iter().any(|item| item == tag))
}
