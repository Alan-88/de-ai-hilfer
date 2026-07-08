use crate::models::{DictionaryRaw, QualityMode};
use crate::services::analysis_grounded_model_a::ModelAOutput;
use crate::services::analysis_grounded_runtime::{
    build_grounded_document, generate_model_a, generate_stage2_markdown, structure_and_assemble,
    GroundedAnalysis,
};
use crate::services::analysis_structure_retry::StructureRetryPolicy;
use crate::services::analyze_runtime::primary_model_for;
use crate::state::AppState;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::time::Instant;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroundedAbCache {
    pub query_text: String,
    pub dictionary_facts: Option<String>,
    pub model_a_output: ModelAOutput,
    pub stage2_markdown: String,
    pub stage2_model: String,
}

pub async fn prepare_grounded_ab_strict_primary(
    state: &AppState,
    target_query: &str,
    quality_mode: QualityMode,
) -> Result<GroundedAbCache> {
    let started_at = Instant::now();
    let stage1_started_at = Instant::now();
    let stage1 = generate_model_a(state, target_query, quality_mode, None).await?;
    let stage1_elapsed_ms = stage1_started_at.elapsed().as_millis();
    let dictionary_facts = stage1.dictionary_facts.as_deref();
    let primary_model = primary_model_for(state, quality_mode);
    let stage2_started_at = Instant::now();
    let stage2 = generate_stage2_markdown(
        &state.ai_client,
        &state.prompts,
        target_query,
        dictionary_facts,
        &stage1.output,
        primary_model,
        "",
        quality_mode,
    )
    .await?;
    let stage2_elapsed_ms = stage2_started_at.elapsed().as_millis();
    tracing::info!(
        "prepare_ab_timing query={} stage1_ms={} stage2_ms={} total_ms={}",
        target_query,
        stage1_elapsed_ms,
        stage2_elapsed_ms,
        started_at.elapsed().as_millis()
    );

    Ok(GroundedAbCache {
        query_text: target_query.to_string(),
        dictionary_facts: stage1.dictionary_facts,
        model_a_output: stage1.output,
        stage2_markdown: stage2.markdown,
        stage2_model: stage2.model,
    })
}

pub async fn complete_grounded_from_ab_cache(
    state: &AppState,
    cache: &GroundedAbCache,
    dictionary_entry: Option<&DictionaryRaw>,
    quality_mode: QualityMode,
) -> Result<GroundedAnalysis> {
    complete_grounded_from_ab_cache_with_policy(
        state,
        cache,
        dictionary_entry,
        quality_mode,
        StructureRetryPolicy::runtime_default(),
        None,
    )
    .await
}

pub async fn complete_grounded_from_ab_cache_with_policy(
    state: &AppState,
    cache: &GroundedAbCache,
    dictionary_entry: Option<&DictionaryRaw>,
    quality_mode: QualityMode,
    structure_retry_policy: StructureRetryPolicy,
    structure_model_override: Option<&str>,
) -> Result<GroundedAnalysis> {
    let (structured, structure_model) = structure_and_assemble(
        state,
        &cache.query_text,
        cache.dictionary_facts.as_deref(),
        &cache.model_a_output,
        &cache.stage2_markdown,
        structure_retry_policy,
        structure_model_override,
    )
    .await?;

    Ok(GroundedAnalysis {
        analysis: build_grounded_document(
            &cache.query_text,
            cache.stage2_markdown.clone(),
            structured,
            dictionary_entry,
            quality_mode,
            &cache.stage2_model,
            &structure_model,
        ),
        model: cache.stage2_model.clone(),
    })
}
