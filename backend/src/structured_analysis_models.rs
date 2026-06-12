use serde::de::DeserializeOwned;
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StructuredAnalysisDocument {
    #[serde(default)]
    pub headword: String,
    #[serde(default)]
    pub phonetic: String,
    #[serde(default, deserialize_with = "deserialize_vec_or_empty_string")]
    pub meanings: Vec<StructuredMeaning>,
    #[serde(default, deserialize_with = "deserialize_vec_or_empty_string")]
    pub usage_modules: Vec<StructuredUsageModule>,
    #[serde(default, deserialize_with = "deserialize_vec_or_empty_string")]
    pub collocations: Vec<String>,
    #[serde(default, deserialize_with = "deserialize_vec_or_empty_string")]
    pub examples: Vec<StructuredExample>,
    #[serde(default, deserialize_with = "deserialize_vec_or_empty_string")]
    pub grammar_rows: Vec<StructuredGrammarRow>,
    #[serde(default, deserialize_with = "deserialize_vec_or_empty_string")]
    pub grammar_branches: Vec<StructuredGrammarBranch>,
    #[serde(default)]
    pub word_network: StructuredWordNetwork,
    #[serde(default, deserialize_with = "deserialize_vec_or_empty_string")]
    pub deep_insights: Vec<StructuredDeepInsight>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StructuredMeaning {
    #[serde(default)]
    pub part_of_speech: String,
    #[serde(default)]
    pub chinese: String,
    #[serde(default)]
    pub english: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StructuredUsageModule {
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub explanation: String,
    #[serde(default)]
    pub example_de: String,
    #[serde(default)]
    pub example_zh: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StructuredExample {
    #[serde(default)]
    pub de: String,
    #[serde(default)]
    pub zh: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StructuredGrammarRow {
    #[serde(default)]
    pub key: String,
    #[serde(default)]
    pub value: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StructuredGrammarBranch {
    #[serde(default)]
    pub selector: String,
    #[serde(default)]
    pub pos: String,
    #[serde(default, deserialize_with = "deserialize_vec_or_empty_string")]
    pub meanings: Vec<StructuredBranchMeaning>,
    #[serde(default)]
    pub grammar: StructuredBranchGrammar,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StructuredBranchMeaning {
    #[serde(default)]
    pub zh: String,
    #[serde(default)]
    pub en: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StructuredBranchGrammar {
    #[serde(default, deserialize_with = "deserialize_vec_or_empty_string")]
    pub genders: Vec<String>,
    #[serde(default)]
    pub noun_class: String,
    #[serde(default, deserialize_with = "deserialize_vec_or_empty_string")]
    pub plural_forms: Vec<String>,
    #[serde(default, deserialize_with = "deserialize_vec_or_empty_string")]
    pub genitive_forms: Vec<String>,
    #[serde(default)]
    pub separable: String,
    #[serde(default)]
    pub transitivity: String,
    #[serde(default)]
    pub reflexive: String,
    #[serde(default, deserialize_with = "deserialize_vec_or_empty_string")]
    pub auxiliaries: Vec<String>,
    #[serde(default)]
    pub present_3sg: String,
    #[serde(default)]
    pub preterite_3sg: String,
    #[serde(default)]
    pub partizip_ii: String,
    #[serde(default)]
    pub comparative: String,
    #[serde(default)]
    pub superlative: String,
    #[serde(default, deserialize_with = "deserialize_vec_or_empty_string")]
    pub governs_cases: Vec<String>,
    #[serde(default)]
    pub word_order: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StructuredDeepInsight {
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub content_markdown: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StructuredWordNetwork {
    #[serde(default, deserialize_with = "deserialize_vec_or_empty_string")]
    pub family: Vec<StructuredWordNetworkItem>,
    #[serde(default, deserialize_with = "deserialize_vec_or_empty_string")]
    pub synonyms: Vec<StructuredWordNetworkItem>,
    #[serde(default, deserialize_with = "deserialize_vec_or_empty_string")]
    pub antonyms: Vec<StructuredWordNetworkItem>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StructuredWordNetworkItem {
    #[serde(default)]
    pub term: String,
    #[serde(default)]
    pub part_of_speech: String,
    #[serde(default)]
    pub chinese: String,
    #[serde(default)]
    pub english: String,
    #[serde(default)]
    pub note: String,
}

fn deserialize_vec_or_empty_string<'de, D, T>(deserializer: D) -> Result<Vec<T>, D::Error>
where
    D: Deserializer<'de>,
    T: DeserializeOwned,
{
    let value = Option::<Value>::deserialize(deserializer)?;
    let Some(value) = value else {
        return Ok(Vec::new());
    };

    match value {
        Value::Null => Ok(Vec::new()),
        Value::String(value) if value.trim().is_empty() => Ok(Vec::new()),
        Value::String(value) => serde_json::from_value(Value::String(value))
            .map(|item| vec![item])
            .or_else(|_| Ok(Vec::new())),
        Value::Array(values) => values
            .into_iter()
            .map(serde_json::from_value)
            .collect::<Result<Vec<T>, _>>()
            .map_err(serde::de::Error::custom),
        other => serde_json::from_value(other)
            .map(|item| vec![item])
            .or_else(|_| Ok(Vec::new())),
    }
}

#[cfg(test)]
mod tests {
    use super::StructuredAnalysisDocument;

    #[test]
    fn accepts_empty_string_for_array_fields() {
        let raw = r#"{
          "headword": "Million",
          "usage_modules": "",
          "collocations": "",
          "word_network": {
            "family": "",
            "synonyms": "",
            "antonyms": ""
          },
          "deep_insights": ""
        }"#;

        let document: StructuredAnalysisDocument = serde_json::from_str(raw).unwrap();

        assert!(document.usage_modules.is_empty());
        assert!(document.collocations.is_empty());
        assert!(document.word_network.family.is_empty());
        assert!(document.word_network.synonyms.is_empty());
        assert!(document.word_network.antonyms.is_empty());
        assert!(document.deep_insights.is_empty());
    }
}
