use serde_json::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntryFormStatus {
    Independent,
    Mixed,
    PureForm,
}

pub fn classify_form_status(entry: &Value) -> EntryFormStatus {
    let Some(senses) = entry.get("senses").and_then(Value::as_array) else {
        return EntryFormStatus::Independent;
    };

    let mut has_form_sense = false;
    let mut has_independent_sense = false;

    for sense in senses {
        if sense_is_form_of(sense) {
            has_form_sense = true;
        } else {
            has_independent_sense = true;
        }
    }

    match (has_form_sense, has_independent_sense) {
        (true, true) => EntryFormStatus::Mixed,
        (true, false) => EntryFormStatus::PureForm,
        _ => EntryFormStatus::Independent,
    }
}

pub fn strip_form_of_senses(mut entry: Value) -> Value {
    if let Some(senses) = entry.get_mut("senses").and_then(Value::as_array_mut) {
        senses.retain(|sense| !sense_is_form_of(sense));
    }

    entry
}

fn sense_is_form_of(sense: &Value) -> bool {
    sense
        .get("form_of")
        .and_then(Value::as_array)
        .map(|items| !items.is_empty())
        .unwrap_or(false)
        || sense
            .get("tags")
            .and_then(Value::as_array)
            .map(|tags| tags.iter().any(|tag| tag.as_str() == Some("form-of")))
            .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::{classify_form_status, strip_form_of_senses, EntryFormStatus};
    use serde_json::json;

    #[test]
    fn mixed_entry_is_not_treated_as_pure_form() {
        let entry = json!({
            "senses": [
                {
                    "tags": ["form-of"],
                    "form_of": [{"word": "anziehen"}],
                    "glosses": ["verbal noun of anziehen"]
                },
                {
                    "tags": ["masculine"],
                    "glosses": ["suit"]
                }
            ]
        });

        assert_eq!(classify_form_status(&entry), EntryFormStatus::Mixed);
    }

    #[test]
    fn stripping_form_senses_keeps_independent_meanings() {
        let entry = json!({
            "senses": [
                {
                    "tags": ["form-of"],
                    "form_of": [{"word": "aufziehen"}],
                    "glosses": ["verbal noun of aufziehen"]
                },
                {
                    "tags": ["masculine"],
                    "glosses": ["elevator"]
                }
            ]
        });

        let stripped = strip_form_of_senses(entry);
        let senses = stripped["senses"].as_array().expect("senses array");

        assert_eq!(senses.len(), 1);
        assert_eq!(senses[0]["glosses"][0].as_str(), Some("elevator"));
    }

    #[test]
    fn pure_form_entry_stays_pure_form() {
        let entry = json!({
            "senses": [
                {
                    "tags": ["form-of", "participle"],
                    "form_of": [{"word": "fühlen"}],
                    "glosses": ["past participle of fühlen"]
                }
            ]
        });

        assert_eq!(classify_form_status(&entry), EntryFormStatus::PureForm);
    }
}
