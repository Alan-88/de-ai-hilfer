use crate::models::{AiModelOverride, DictionaryRaw, QualityMode};
use crate::services::ai_model_resolver::{
    fallback_model_for, resolve_task_model, resolve_task_model_with_override, AiModelTask,
};
use crate::services::analysis_grounded_generation::GroundedStage1;
use crate::services::analysis_grounded_model_a::ModelAOutput;
use crate::services::analysis_grounded_runtime::GroundedAnalysis;
use crate::state::AppState;
use serde::{Deserialize, Serialize};
use std::hash::{DefaultHasher, Hash, Hasher};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroundedAbCache {
    pub query_text: String,
    pub dictionary_facts: Option<String>,
    pub model_a_output: ModelAOutput,
    pub stage2_markdown: String,
    pub stage2_model: String,
}

#[derive(Default, Serialize, Deserialize)]
pub(crate) struct GenerationCheckpoint {
    pub stage1: Option<GroundedStage1>,
    pub ab: Option<GroundedAbCache>,
    pub complete: Option<GroundedAnalysis>,
}

// Include dictionary, prompt and routing identity so configuration changes cannot reuse old stages.
pub(crate) async fn checkpoint_key(
    state: &AppState,
    query: &str,
    dictionary: Option<&DictionaryRaw>,
    quality: QualityMode,
    hint: Option<&str>,
    model_override: Option<&AiModelOverride>,
    allow_fallback: bool,
) -> anyhow::Result<String> {
    let analyze =
        resolve_task_model_with_override(state, AiModelTask::Analyze, model_override).await?;
    let structure = resolve_task_model(state, AiModelTask::Structure).await?;
    let rows =
        crate::services::analysis_grounded_facts::load_raw_rows_by_headword(&state.pool, query)
            .await?;
    let facts = crate::services::analysis_grounded_facts::build_light_dictionary_facts_payload(
        query, &rows,
    );
    let identity = serde_json::json!({
        "version": 1, "query":query, "dictionary":dictionary.map(|entry| &entry.raw_data), "facts":facts,
        "quality":quality, "hint":hint, "override":model_override,
        "analyze_model":analyze.model,"analyze_route":analyze.client.routing_identity(),
        "structure_model":structure.model,"structure_route":structure.client.routing_identity(),
        "fallback":allow_fallback.then(|| fallback_model_for(state,quality)),
        "model_a_prompt":crate::services::analysis_grounded_prompt::build_model_a_prompt(&state.prompts),
        "stage2_prompt":crate::services::analysis_grounded_prompt::build_stage2_prompt(&state.prompts),
        "structure_prompt":crate::services::analysis_grounded_prompt::build_structure_prompt(&state.prompts),
    })
    .to_string();
    let mut hash = DefaultHasher::new();
    identity.hash(&mut hash);
    Ok(format!("generation-v1-{:016x}", hash.finish()))
}
