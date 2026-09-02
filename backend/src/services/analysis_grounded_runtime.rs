use crate::ai::{AiChatOptions, AiClient};
use crate::models::{
    AiModelOverride, AnalysisDocument, DictionaryRaw, QualityMode, StructuredAnalysisDocument,
};
use crate::services::ai_model_resolver::{
    fallback_model_for, resolve_task_model, resolve_task_model_with_override, AiModelTask,
};
use crate::services::analysis_grounded_ab_cache::{
    get_grounded_ab_cache, put_grounded_ab_cache, remove_grounded_ab_cache, GroundedAbCache,
    GroundedAbCacheKey,
};
use crate::services::analysis_grounded_assembly::assemble_grounded_structured_document;
use crate::services::analysis_grounded_generation::{
    generate_model_a, generate_stage2_markdown, stream_stage2_markdown,
};
use crate::services::analysis_grounded_model_a::ModelAOutput;
use crate::services::analysis_grounded_prompt::build_structure_prompt;
use crate::services::analysis_grounded_stage2_quality::validate_stage2_markdown_completeness;
use crate::services::analysis_structure_quality::validate_structured_capture;
use crate::services::analysis_structure_retry::{
    generate_structure_with_retry_policy, StructureRetryPolicy,
};
use crate::services::analysis_structure_seed::build_structure_seed;
use crate::services::analysis_structure_transform::{
    merge_structured_with_seed, normalize_structured_analysis,
};
use crate::services::analyze_support::extract_json_with_report;
use crate::services::dictionary_render::build_dictionary_excerpt;
use crate::services::stream_response::sse_delta;
use crate::state::AppState;
use anyhow::{anyhow, Result};
use std::time::Duration;
use tokio::sync::mpsc::UnboundedSender;

fn structure_chat_options() -> AiChatOptions {
    AiChatOptions {
        temperature: 0.0,
        max_tokens: None,
        timeout: Duration::from_secs(90),
    }
}

pub struct GroundedAnalysis {
    pub analysis: AnalysisDocument,
    pub model: String,
}

pub async fn generate_grounded_analysis(
    state: &AppState,
    target_query: &str,
    dictionary_entry: Option<&DictionaryRaw>,
    quality_mode: QualityMode,
    generation_hint: Option<&str>,
    model_override: Option<&AiModelOverride>,
) -> Result<GroundedAnalysis> {
    generate_grounded_analysis_with_stage2_fallback(
        state,
        target_query,
        dictionary_entry,
        quality_mode,
        true,
        generation_hint,
        model_override,
    )
    .await
}

pub async fn generate_grounded_analysis_strict_primary(
    state: &AppState,
    target_query: &str,
    dictionary_entry: Option<&DictionaryRaw>,
    quality_mode: QualityMode,
) -> Result<GroundedAnalysis> {
    generate_grounded_analysis_with_stage2_fallback(
        state,
        target_query,
        dictionary_entry,
        quality_mode,
        false,
        None,
        None,
    )
    .await
}

async fn generate_grounded_analysis_with_stage2_fallback(
    state: &AppState,
    target_query: &str,
    dictionary_entry: Option<&DictionaryRaw>,
    quality_mode: QualityMode,
    allow_stage2_fallback: bool,
    generation_hint: Option<&str>,
    model_override: Option<&AiModelOverride>,
) -> Result<GroundedAnalysis> {
    let stage2_primary =
        resolve_task_model_with_override(state, AiModelTask::Analyze, model_override).await?;
    let cache_key = GroundedAbCacheKey::new(
        target_query,
        quality_mode,
        generation_hint,
        &stage2_primary.model,
        model_override,
        allow_stage2_fallback,
    );
    let grounded_ab = if let Some(cached) = get_grounded_ab_cache(&cache_key) {
        tracing::info!("grounded AB cache hit: target={target_query}");
        cached
    } else {
        let stage1 = generate_model_a(state, target_query, quality_mode, model_override).await?;
        let fallback_model = if allow_stage2_fallback && !stage2_primary.persisted {
            fallback_model_for(state, quality_mode)
        } else {
            ""
        };
        let stage2 = generate_stage2_markdown(
            &stage2_primary.client,
            &state.prompts,
            target_query,
            stage1.dictionary_facts.as_deref(),
            &stage1.output,
            &stage2_primary.model,
            fallback_model,
            quality_mode,
            generation_hint,
        )
        .await?;
        validate_stage2_markdown_completeness(&stage2.markdown)?;

        let cached = GroundedAbCache {
            query_text: target_query.to_string(),
            dictionary_facts: stage1.dictionary_facts,
            model_a_output: stage1.output,
            stage2_markdown: stage2.markdown,
            stage2_model: stage2.model,
        };
        put_grounded_ab_cache(cache_key.clone(), cached.clone());
        cached
    };
    let (structured, _structure_model) = structure_and_assemble(
        state,
        target_query,
        grounded_ab.dictionary_facts.as_deref(),
        &grounded_ab.model_a_output,
        &grounded_ab.stage2_markdown,
        StructureRetryPolicy::runtime_default(),
        None,
    )
    .await?;
    remove_grounded_ab_cache(&cache_key);

    Ok(GroundedAnalysis {
        analysis: build_grounded_document(
            target_query,
            grounded_ab.stage2_markdown,
            structured,
            dictionary_entry,
            quality_mode,
            &grounded_ab.stage2_model,
            &_structure_model,
        ),
        model: grounded_ab.stage2_model,
    })
}

pub async fn stream_grounded_analysis(
    state: &AppState,
    tx: &UnboundedSender<String>,
    target_query: &str,
    dictionary_entry: Option<&DictionaryRaw>,
    quality_mode: QualityMode,
    generation_hint: Option<&str>,
    model_override: Option<&AiModelOverride>,
) -> Result<GroundedAnalysis> {
    let stage2_primary =
        resolve_task_model_with_override(state, AiModelTask::Analyze, model_override).await?;
    let cache_key = GroundedAbCacheKey::new(
        target_query,
        quality_mode,
        generation_hint,
        &stage2_primary.model,
        model_override,
        true,
    );
    let grounded_ab = if let Some(cached) = get_grounded_ab_cache(&cache_key) {
        tracing::info!("grounded stream AB cache hit: target={target_query}");
        tx.send(sse_delta(&cached.stage2_markdown)).ok();
        cached
    } else {
        let stage1 = generate_model_a(state, target_query, quality_mode, model_override).await?;
        if tx.is_closed() {
            return Err(anyhow!("stream analyze canceled by client"));
        }

        let fallback_model = if stage2_primary.persisted {
            ""
        } else {
            fallback_model_for(state, quality_mode)
        };
        let stage2 = stream_stage2_markdown(
            &stage2_primary.client,
            tx,
            &state.prompts,
            target_query,
            stage1.dictionary_facts.as_deref(),
            &stage1.output,
            &stage2_primary.model,
            fallback_model,
            quality_mode,
            generation_hint,
        )
        .await?;
        if tx.is_closed() {
            return Err(anyhow!("stream analyze canceled by client"));
        }
        validate_stage2_markdown_completeness(&stage2.markdown)?;

        let cached = GroundedAbCache {
            query_text: target_query.to_string(),
            dictionary_facts: stage1.dictionary_facts,
            model_a_output: stage1.output,
            stage2_markdown: stage2.markdown,
            stage2_model: stage2.model,
        };
        put_grounded_ab_cache(cache_key.clone(), cached.clone());
        cached
    };
    if tx.is_closed() {
        remove_grounded_ab_cache(&cache_key);
        return Err(anyhow!("stream analyze canceled by client"));
    }

    let (structured, structure_model) = structure_and_assemble(
        state,
        target_query,
        grounded_ab.dictionary_facts.as_deref(),
        &grounded_ab.model_a_output,
        &grounded_ab.stage2_markdown,
        StructureRetryPolicy::runtime_default(),
        None,
    )
    .await?;
    remove_grounded_ab_cache(&cache_key);

    Ok(GroundedAnalysis {
        analysis: build_grounded_document(
            target_query,
            grounded_ab.stage2_markdown,
            structured,
            dictionary_entry,
            quality_mode,
            &grounded_ab.stage2_model,
            &structure_model,
        ),
        model: grounded_ab.stage2_model,
    })
}

pub(crate) async fn structure_and_assemble(
    state: &AppState,
    target_query: &str,
    dictionary_facts: Option<&str>,
    stage1_output: &ModelAOutput,
    markdown: &str,
    retry_policy: StructureRetryPolicy,
    structure_model_override: Option<&str>,
) -> Result<(StructuredAnalysisDocument, String)> {
    validate_stage2_markdown_completeness(markdown)?;

    let structure_prompt = build_structure_prompt(&state.prompts);
    let preliminary_parse = build_structure_seed(target_query, markdown);
    let structure_user_payload = serde_json::json!({
        "query": target_query,
        "markdown": markdown,
        "preliminary_parse": preliminary_parse,
    })
    .to_string();

    let resolved_structure = match structure_model_override {
        Some(model) => ResolvedStructureClient {
            client: state.ai_client.clone(),
            model: model.to_string(),
        },
        None => {
            let resolved = resolve_task_model(state, AiModelTask::Structure).await?;
            ResolvedStructureClient {
                client: resolved.client,
                model: resolved.model,
            }
        }
    };
    let structure_model = if resolved_structure.model.trim().is_empty() {
        "minimax-m2.5".to_string()
    } else {
        resolved_structure.model
    };
    let raw = generate_structure_with_retry_policy(
        &resolved_structure.client,
        target_query,
        &structure_model,
        &structure_prompt,
        &structure_user_payload,
        structure_chat_options(),
        retry_policy,
    )
    .await?;
    let extracted = extract_json_with_report::<StructuredAnalysisDocument>(&raw)?;
    let module_document = normalize_structured_analysis(Some(extracted.value), target_query)
        .ok_or_else(|| anyhow!("structure extraction returned empty document"))?;
    let module_document = merge_structured_with_seed(module_document, &preliminary_parse);
    validate_structured_capture(markdown, &module_document)
        .map_err(|reason| anyhow!("structure quality gate rejected payload: {reason}"))?;

    let assembled = assemble_grounded_structured_document(
        target_query,
        dictionary_facts,
        stage1_output,
        module_document,
    );
    Ok((assembled, structure_model))
}

pub(crate) fn build_grounded_document(
    target_query: &str,
    markdown: String,
    structured: StructuredAnalysisDocument,
    dictionary_entry: Option<&DictionaryRaw>,
    quality_mode: QualityMode,
    model: &str,
    _structure_model: &str,
) -> AnalysisDocument {
    AnalysisDocument {
        markdown: markdown.trim().to_string(),
        structured: Some(structured),
        tags: Vec::new(),
        aliases: Vec::new(),
        prototype: Some(target_query.to_string()),
        attached_phrase_modules: Vec::new(),
        dictionary_excerpt: dictionary_entry.map(|entry| build_dictionary_excerpt(&entry.raw_data)),
        model: Some(model.to_string()),
        quality_mode: Some(quality_mode),
    }
}

struct ResolvedStructureClient {
    client: AiClient,
    model: String,
}
