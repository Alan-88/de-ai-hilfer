use crate::prompts::PromptConfig;

const DEFAULT_STRUCTURE_PROMPT: &str = include_str!("../../config/prompts/structure.md");

pub fn build_structure_prompt(prompts: &PromptConfig) -> String {
    let prompt = prompts.structure_prompt.trim();
    if prompt.is_empty() {
        return DEFAULT_STRUCTURE_PROMPT.trim().to_string();
    }
    prompt.to_string()
}
