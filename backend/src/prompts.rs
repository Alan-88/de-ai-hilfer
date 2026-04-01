use serde::Deserialize;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Deserialize)]
pub struct PromptConfig {
    pub prototype_identification_prompt: String,
    pub spell_checker_prompt: String,
    pub analysis_prompt: String,
    pub follow_up_prompt: String,
    pub intelligent_search_prompt: String,
}

impl PromptConfig {
    pub fn load(path: impl AsRef<Path>) -> anyhow::Result<Self> {
        let content = fs::read_to_string(path)?;
        Ok(serde_yaml::from_str(&content)?)
    }
}
