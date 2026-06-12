use serde::Deserialize;
use std::fs;
use std::path::Path;

const BUNDLED_MODEL_A_PROMPT: &str = include_str!("../config/prompts/model_a.md");
const BUNDLED_STAGE2_PROMPT: &str = include_str!("../config/prompts/stage2.md");
const BUNDLED_STRUCTURE_PROMPT: &str = include_str!("../config/prompts/structure.md");

#[derive(Debug, Clone, Deserialize)]
pub struct PromptConfig {
    pub prototype_identification_prompt: String,
    pub spell_checker_prompt: String,
    pub analysis_prompt: String,
    #[serde(default)]
    pub model_a_prompt: String,
    #[serde(default)]
    pub stage2_prompt: String,
    #[serde(default)]
    pub structure_prompt: String,
    pub follow_up_prompt: String,
    pub intelligent_search_prompt: String,
}

impl PromptConfig {
    pub fn load(path: impl AsRef<Path>) -> anyhow::Result<Self> {
        let path = path.as_ref();
        let content = fs::read_to_string(path)?;
        let mut config: Self = serde_yaml::from_str(&content)?;
        if let Some(base_dir) = path.parent() {
            fill_prompt_from_file_or_bundle(
                &mut config.model_a_prompt,
                base_dir,
                "model_a.md",
                BUNDLED_MODEL_A_PROMPT,
            )?;
            fill_prompt_from_file_or_bundle(
                &mut config.stage2_prompt,
                base_dir,
                "stage2.md",
                BUNDLED_STAGE2_PROMPT,
            )?;
            fill_prompt_from_file_or_bundle(
                &mut config.structure_prompt,
                base_dir,
                "structure.md",
                BUNDLED_STRUCTURE_PROMPT,
            )?;
        }
        Ok(config)
    }
}

fn fill_prompt_from_file_or_bundle(
    prompt: &mut String,
    base_dir: &Path,
    file_name: &str,
    bundled: &str,
) -> anyhow::Result<()> {
    if !prompt.trim().is_empty() {
        return Ok(());
    }

    let prompt_path = base_dir.join(file_name);
    *prompt = if prompt_path.exists() {
        fs::read_to_string(prompt_path)?
    } else {
        bundled.to_string()
    };
    Ok(())
}
