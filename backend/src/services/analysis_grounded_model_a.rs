use crate::models::{StructuredBranchGrammar, StructuredBranchMeaning};
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value;
use std::collections::BTreeSet;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ModelAOutput {
    #[serde(default)]
    pub word: String,
    #[serde(default)]
    pub entries: Vec<ModelAEntry>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ModelAEntry {
    #[serde(default)]
    pub selector: String,
    #[serde(default)]
    pub pos: String,
    #[serde(default)]
    pub meanings: Vec<StructuredBranchMeaning>,
    #[serde(default)]
    pub grammar: ModelAGrammar,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ModelAGrammar {
    #[serde(default, deserialize_with = "deserialize_string_list")]
    pub genders: Vec<String>,
    #[serde(default, deserialize_with = "deserialize_string_value")]
    pub noun_class: String,
    #[serde(default, deserialize_with = "deserialize_string_list")]
    pub plural_forms: Vec<String>,
    #[serde(default, deserialize_with = "deserialize_string_list")]
    pub genitive_forms: Vec<String>,
    #[serde(default, deserialize_with = "deserialize_string_value")]
    pub separable: String,
    #[serde(default, deserialize_with = "deserialize_string_value")]
    pub transitivity: String,
    #[serde(default, deserialize_with = "deserialize_string_value")]
    pub reflexive: String,
    #[serde(default, deserialize_with = "deserialize_string_list")]
    pub auxiliaries: Vec<String>,
    #[serde(default, deserialize_with = "deserialize_string_value")]
    pub present_3sg: String,
    #[serde(default, deserialize_with = "deserialize_string_value")]
    pub preterite_3sg: String,
    #[serde(default, deserialize_with = "deserialize_string_value")]
    pub partizip_ii: String,
    #[serde(default, deserialize_with = "deserialize_string_value")]
    pub comparative: String,
    #[serde(default, deserialize_with = "deserialize_string_value")]
    pub superlative: String,
    #[serde(default, deserialize_with = "deserialize_string_list")]
    pub governs_cases: Vec<String>,
    #[serde(default, deserialize_with = "deserialize_string_value")]
    pub word_order: String,
}

impl From<&ModelAGrammar> for StructuredBranchGrammar {
    fn from(grammar: &ModelAGrammar) -> Self {
        Self {
            genders: grammar.genders.clone(),
            noun_class: grammar.noun_class.clone(),
            plural_forms: grammar.plural_forms.clone(),
            genitive_forms: grammar.genitive_forms.clone(),
            separable: grammar.separable.clone(),
            transitivity: grammar.transitivity.clone(),
            reflexive: grammar.reflexive.clone(),
            auxiliaries: grammar.auxiliaries.clone(),
            present_3sg: grammar.present_3sg.clone(),
            preterite_3sg: grammar.preterite_3sg.clone(),
            partizip_ii: grammar.partizip_ii.clone(),
            comparative: grammar.comparative.clone(),
            superlative: grammar.superlative.clone(),
            governs_cases: grammar.governs_cases.clone(),
            word_order: grammar.word_order.clone(),
        }
    }
}

pub fn normalize_model_a_output(mut output: ModelAOutput) -> ModelAOutput {
    output.word = output.word.trim().to_string();
    output.entries = output
        .entries
        .into_iter()
        .map(normalize_model_a_entry)
        .filter(|entry| {
            !entry.pos.is_empty()
                || !entry.selector.is_empty()
                || !entry.meanings.is_empty()
                || has_non_empty_grammar(&entry.grammar)
        })
        .collect();
    output
}

fn normalize_model_a_entry(mut entry: ModelAEntry) -> ModelAEntry {
    entry.selector = entry.selector.trim().to_string();
    entry.pos = normalize_pos(&entry.pos);
    entry.meanings = entry
        .meanings
        .into_iter()
        .map(|mut meaning| {
            meaning.zh = meaning.zh.trim().to_string();
            meaning.en = meaning.en.trim().to_string();
            meaning
        })
        .filter(|meaning| !meaning.zh.is_empty() || !meaning.en.is_empty())
        .collect();
    entry.grammar = normalize_grammar(entry.grammar);
    entry
}

fn normalize_grammar(mut grammar: ModelAGrammar) -> ModelAGrammar {
    grammar.genders = normalize_string_list(grammar.genders, normalize_gender);
    grammar.noun_class = normalize_noun_class(&grammar.noun_class);
    grammar.plural_forms = normalize_free_text_list(grammar.plural_forms);
    grammar.genitive_forms = normalize_free_text_list(grammar.genitive_forms);
    grammar.separable = normalize_separable(&grammar.separable);
    grammar.transitivity = normalize_transitivity(&grammar.transitivity);
    grammar.reflexive = normalize_reflexive(&grammar.reflexive);
    grammar.auxiliaries = normalize_string_list(grammar.auxiliaries, normalize_auxiliary);
    grammar.present_3sg = grammar.present_3sg.trim().to_string();
    grammar.preterite_3sg = grammar.preterite_3sg.trim().to_string();
    grammar.partizip_ii = grammar.partizip_ii.trim().to_string();
    grammar.comparative = grammar.comparative.trim().to_string();
    grammar.superlative = grammar.superlative.trim().to_string();
    grammar.governs_cases = normalize_string_list(grammar.governs_cases, normalize_case);
    if grammar.governs_cases.is_empty()
        && matches!(grammar.transitivity.as_str(), "transitive" | "both")
    {
        grammar.governs_cases.push("accusative".to_string());
    }
    grammar.word_order = normalize_word_order(&grammar.word_order);
    grammar
}

fn normalize_string_list(values: Vec<String>, mapper: fn(&str) -> String) -> Vec<String> {
    let mut seen = BTreeSet::new();
    let mut normalized = Vec::new();
    for value in values {
        let mapped = mapper(&value);
        if !mapped.is_empty() && seen.insert(mapped.clone()) {
            normalized.push(mapped);
        }
    }
    normalized
}

fn normalize_free_text_list(values: Vec<String>) -> Vec<String> {
    let mut seen = BTreeSet::new();
    let mut normalized = Vec::new();
    for value in values {
        let trimmed = value.trim();
        if !trimmed.is_empty() && seen.insert(trimmed.to_string()) {
            normalized.push(trimmed.to_string());
        }
    }
    normalized
}

fn normalize_pos(value: &str) -> String {
    let lower = value.trim().to_ascii_lowercase();
    match lower.as_str() {
        "noun" | "substantiv" | "nomen" => "noun".to_string(),
        "verb" => "verb".to_string(),
        "adjektiv" | "adjective" | "adj" => "adjective".to_string(),
        "adverb" | "adv" => "adverb".to_string(),
        "präposition" | "preposition" | "prep" => "preposition".to_string(),
        "konjunktion" | "subjunktion" | "conjunction" | "conj" => "conjunction".to_string(),
        "partikel" | "particle" => "particle".to_string(),
        "interjektion" | "interjection" => "interjection".to_string(),
        _ => value.trim().to_string(),
    }
}

fn normalize_gender(value: &str) -> String {
    let lower = value.trim().to_ascii_lowercase();
    match lower.as_str() {
        "m" | "maskulin" | "masculine" => "masculine".to_string(),
        "f" | "feminin" | "feminine" => "feminine".to_string(),
        "n" | "neutrum" | "neuter" => "neuter".to_string(),
        _ => String::new(),
    }
}

fn normalize_noun_class(value: &str) -> String {
    let lower = value.trim().to_ascii_lowercase();
    match lower.as_str() {
        "strong" | "stark" => "strong".to_string(),
        "weak" | "schwach" => "weak".to_string(),
        "mixed" | "gemischt" => "mixed".to_string(),
        _ => String::new(),
    }
}

fn normalize_separable(value: &str) -> String {
    let lower = value.trim().to_ascii_lowercase();
    match lower.as_str() {
        "yes" | "ja" | "true" | "trennbar" | "separable" => "separable".to_string(),
        "no" | "nein" | "false" | "untrennbar" | "inseparable" => "inseparable".to_string(),
        _ => String::new(),
    }
}

fn normalize_transitivity(value: &str) -> String {
    let lower = value.trim().to_ascii_lowercase().replace(' ', "");
    match lower.as_str() {
        "transitiv" | "transitive" => "transitive".to_string(),
        "intransitiv" | "intransitive" => "intransitive".to_string(),
        "transitiv/intransitiv"
        | "transitive/intransitive"
        | "transitiv,intransitiv"
        | "transitive,intransitive"
        | "both" => "both".to_string(),
        _ => String::new(),
    }
}

fn normalize_reflexive(value: &str) -> String {
    let lower = value.trim().to_ascii_lowercase();
    match lower.as_str() {
        "none" | "no" | "nein" | "false" => "none".to_string(),
        "optional" => "optional".to_string(),
        "required" | "yes" | "ja" | "true" => "required".to_string(),
        _ => String::new(),
    }
}

fn normalize_auxiliary(value: &str) -> String {
    let lower = value.trim().to_ascii_lowercase();
    match lower.as_str() {
        "haben" => "haben".to_string(),
        "sein" => "sein".to_string(),
        _ => String::new(),
    }
}

fn normalize_case(value: &str) -> String {
    let lower = value.trim().to_ascii_lowercase();
    if lower.contains("akk") || lower.contains("acc") {
        "accusative".to_string()
    } else if lower.contains("dat") {
        "dative".to_string()
    } else if lower.contains("gen") {
        "genitive".to_string()
    } else if lower.contains("nom") {
        "nominative".to_string()
    } else {
        String::new()
    }
}

fn normalize_word_order(value: &str) -> String {
    let lower = value.trim().to_ascii_lowercase();
    if lower.contains("nebensatz") || lower.contains("subordinate") {
        "subordinate_clause".to_string()
    } else if lower.contains("hauptsatz") || lower.contains("main") {
        "main_clause".to_string()
    } else {
        String::new()
    }
}

fn has_non_empty_grammar(grammar: &ModelAGrammar) -> bool {
    !grammar.genders.is_empty()
        || !grammar.noun_class.is_empty()
        || !grammar.plural_forms.is_empty()
        || !grammar.genitive_forms.is_empty()
        || !grammar.separable.is_empty()
        || !grammar.transitivity.is_empty()
        || !grammar.reflexive.is_empty()
        || !grammar.auxiliaries.is_empty()
        || !grammar.present_3sg.is_empty()
        || !grammar.preterite_3sg.is_empty()
        || !grammar.partizip_ii.is_empty()
        || !grammar.comparative.is_empty()
        || !grammar.superlative.is_empty()
        || !grammar.governs_cases.is_empty()
        || !grammar.word_order.is_empty()
}

fn deserialize_string_value<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    let value = Option::<Value>::deserialize(deserializer)?;
    Ok(value
        .map(value_to_string_list)
        .unwrap_or_default()
        .join("/"))
}

fn deserialize_string_list<'de, D>(deserializer: D) -> Result<Vec<String>, D::Error>
where
    D: Deserializer<'de>,
{
    let value = Option::<Value>::deserialize(deserializer)?;
    Ok(value.map(value_to_string_list).unwrap_or_default())
}

fn value_to_string_list(value: Value) -> Vec<String> {
    match value {
        Value::Null => Vec::new(),
        Value::String(value) => split_model_string(&value),
        Value::Array(values) => values
            .into_iter()
            .flat_map(value_to_string_list)
            .collect::<Vec<_>>(),
        other => split_model_string(&other.to_string()),
    }
}

fn split_model_string(value: &str) -> Vec<String> {
    value
        .split('/')
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .map(ToString::to_string)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{normalize_model_a_output, ModelAEntry, ModelAGrammar, ModelAOutput};

    fn normalized_grammar(grammar: ModelAGrammar) -> ModelAGrammar {
        normalize_model_a_output(ModelAOutput {
            word: "test".to_string(),
            entries: vec![ModelAEntry {
                pos: "verb".to_string(),
                grammar,
                ..ModelAEntry::default()
            }],
        })
        .entries
        .remove(0)
        .grammar
    }

    #[test]
    fn defaults_empty_transitive_government_to_accusative() {
        let grammar = normalized_grammar(ModelAGrammar {
            transitivity: "transitive".to_string(),
            governs_cases: Vec::new(),
            ..ModelAGrammar::default()
        });

        assert_eq!(grammar.governs_cases, ["accusative"]);
    }

    #[test]
    fn defaults_empty_mixed_transitivity_government_to_accusative() {
        let grammar = normalized_grammar(ModelAGrammar {
            transitivity: "both".to_string(),
            governs_cases: Vec::new(),
            ..ModelAGrammar::default()
        });

        assert_eq!(grammar.governs_cases, ["accusative"]);
    }

    #[test]
    fn preserves_explicit_non_default_government() {
        let grammar = normalized_grammar(ModelAGrammar {
            transitivity: "transitive".to_string(),
            governs_cases: vec!["dative".to_string()],
            ..ModelAGrammar::default()
        });

        assert_eq!(grammar.governs_cases, ["dative"]);
    }

    #[test]
    fn leaves_intransitive_government_empty() {
        let grammar = normalized_grammar(ModelAGrammar {
            transitivity: "intransitive".to_string(),
            governs_cases: Vec::new(),
            ..ModelAGrammar::default()
        });

        assert!(grammar.governs_cases.is_empty());
    }
}
