use crate::ai::{is_hard_failure, AiClient};
use crate::models::{
    AiModelOverride, AnalysisDocument, DictionaryRaw, QualityMode, StructuredAnalysisDocument,
};
use crate::services::ai_model_resolver::{
    resolve_task_model, resolve_task_model_with_override, AiModelTask,
};
use crate::services::analysis_grounded_assembly::assemble_grounded_structured_document;
use crate::services::analysis_grounded_facts::{
    build_light_dictionary_facts_payload, load_raw_rows_by_headword,
};
use crate::services::analysis_grounded_model_a::{normalize_model_a_output, ModelAOutput};
use crate::services::analysis_grounded_options::{
    model_a_chat_options, stage2_chat_options, structure_chat_options,
};
use crate::services::analysis_grounded_prompt::{
    build_model_a_prompt, build_model_a_user_payload, build_stage2_prompt,
    build_stage2_user_payload,
};
use crate::services::analysis_grounded_stage2_quality::validate_stage2_markdown_completeness;
use crate::services::analysis_structure_quality::validate_structured_capture;
use crate::services::analysis_structure_retry::{
    generate_structure_with_retry_policy, StructureRetryPolicy,
};
use crate::services::analysis_structure_seed::build_structure_seed;
use crate::services::analysis_structure_transform::{
    merge_structured_with_seed, normalize_structured_analysis,
};
use crate::services::analyze_runtime::fallback_model_for;
use crate::services::analyze_support::{extract_json, extract_json_with_report};
use crate::services::dictionary_render::build_dictionary_excerpt;
use crate::services::stream_analyze_runtime::StreamedMarkdown;
use crate::services::stream_generation::{
    request_and_stream_model, StreamModelOutcome, HARD_FAILURE_RETRY_CYCLES,
    HARD_FAILURE_RETRY_DELAY_MS,
};
use crate::state::AppState;
use anyhow::{anyhow, Result};
use std::time::Duration;
use tokio::sync::mpsc::UnboundedSender;

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
    let stage1 = generate_model_a(state, target_query, quality_mode, model_override).await?;
    let dictionary_facts = stage1.dictionary_facts.as_deref();
    let stage2_primary =
        resolve_task_model_with_override(state, AiModelTask::Analyze, model_override).await?;
    let fallback_model = if allow_stage2_fallback && !stage2_primary.persisted {
        fallback_model_for(state, quality_mode)
    } else {
        ""
    };
    let stage2 = generate_stage2_markdown(
        &stage2_primary.client,
        &state.prompts,
        target_query,
        dictionary_facts,
        &stage1.output,
        &stage2_primary.model,
        fallback_model,
        quality_mode,
        generation_hint,
    )
    .await?;
    let (structured, _structure_model) = structure_and_assemble(
        state,
        target_query,
        dictionary_facts,
        &stage1.output,
        &stage2.markdown,
        StructureRetryPolicy::runtime_default(),
        None,
    )
    .await?;

    Ok(GroundedAnalysis {
        analysis: build_grounded_document(
            target_query,
            stage2.markdown,
            structured,
            dictionary_entry,
            quality_mode,
            &stage2.model,
            &_structure_model,
        ),
        model: stage2.model,
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
    let stage1 = generate_model_a(state, target_query, quality_mode, model_override).await?;
    if tx.is_closed() {
        return Err(anyhow!("stream analyze canceled by client"));
    }

    let dictionary_facts = stage1.dictionary_facts.as_deref();
    let stage2_primary =
        resolve_task_model_with_override(state, AiModelTask::Analyze, model_override).await?;
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
        dictionary_facts,
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

    let (structured, structure_model) = structure_and_assemble(
        state,
        target_query,
        dictionary_facts,
        &stage1.output,
        &stage2.markdown,
        StructureRetryPolicy::runtime_default(),
        None,
    )
    .await?;

    Ok(GroundedAnalysis {
        analysis: build_grounded_document(
            target_query,
            stage2.markdown,
            structured,
            dictionary_entry,
            quality_mode,
            &stage2.model,
            &structure_model,
        ),
        model: stage2.model,
    })
}

pub(crate) async fn generate_model_a(
    state: &AppState,
    target_query: &str,
    _quality_mode: QualityMode,
    model_override: Option<&AiModelOverride>,
) -> Result<GroundedStage1> {
    let raw_rows = load_raw_rows_by_headword(&state.pool, target_query).await?;
    let dictionary_facts = (!raw_rows.is_empty())
        .then(|| build_light_dictionary_facts_payload(target_query, &raw_rows));
    let prompt = build_model_a_prompt(&state.prompts);
    let user_payload = build_model_a_user_payload(target_query, dictionary_facts.as_deref());
    let resolved =
        resolve_task_model_with_override(state, AiModelTask::Analyze, model_override).await?;
    let raw = resolved
        .client
        .chat_model_with_options(
            &resolved.model,
            &prompt,
            &user_payload,
            model_a_chat_options(),
        )
        .await?;
    let output = extract_json::<ModelAOutput>(&raw).map(normalize_model_a_output)?;
    if output.entries.is_empty() {
        return Err(anyhow!("model A returned no lexical branches"));
    }
    Ok(GroundedStage1 {
        dictionary_facts,
        output,
    })
}

pub(crate) async fn generate_stage2_markdown(
    ai_client: &AiClient,
    prompts: &crate::prompts::PromptConfig,
    target_query: &str,
    dictionary_facts: Option<&str>,
    stage1_output: &ModelAOutput,
    primary_model: &str,
    fallback_model: &str,
    quality_mode: QualityMode,
    generation_hint: Option<&str>,
) -> Result<StreamedMarkdown> {
    let prompt = build_stage2_prompt(prompts);
    let user_payload = build_stage2_user_payload(
        target_query,
        dictionary_facts,
        stage1_output,
        generation_hint,
    );
    let options = stage2_chat_options(quality_mode);

    for cycle in 1..=HARD_FAILURE_RETRY_CYCLES {
        match ai_client
            .chat_model_with_options(primary_model, &prompt, &user_payload, options)
            .await
        {
            Ok(markdown) => {
                return Ok(StreamedMarkdown {
                    markdown,
                    model: primary_model.to_string(),
                });
            }
            Err(primary_err)
                if cycle < HARD_FAILURE_RETRY_CYCLES && is_hard_failure(&primary_err) =>
            {
                tracing::warn!(
                    "grounded stage2 retrying after primary hard failure: cycle={cycle}/{HARD_FAILURE_RETRY_CYCLES}, primary={primary_model}, target={target_query}, err={primary_err:#}"
                );
                tokio::time::sleep(Duration::from_millis(HARD_FAILURE_RETRY_DELAY_MS)).await;
            }
            Err(primary_err)
                if is_hard_failure(&primary_err)
                    && !fallback_model.is_empty()
                    && fallback_model != primary_model =>
            {
                tracing::warn!(
                    "grounded stage2 switching to fallback: primary={primary_model}, fallback={fallback_model}, target={target_query}, err={primary_err:#}"
                );
                match ai_client
                    .chat_model_with_options(fallback_model, &prompt, &user_payload, options)
                    .await
                {
                    Ok(markdown) => {
                        return Ok(StreamedMarkdown {
                            markdown,
                            model: fallback_model.to_string(),
                        });
                    }
                    Err(fallback_err) => return Err(fallback_err),
                }
            }
            Err(primary_err) => return Err(primary_err),
        }
    }

    unreachable!("grounded stage2 retry loop should always return or error");
}

async fn stream_stage2_markdown(
    ai_client: &AiClient,
    tx: &UnboundedSender<String>,
    prompts: &crate::prompts::PromptConfig,
    target_query: &str,
    dictionary_facts: Option<&str>,
    stage1_output: &ModelAOutput,
    primary_model: &str,
    fallback_model: &str,
    quality_mode: QualityMode,
    generation_hint: Option<&str>,
) -> Result<StreamedMarkdown> {
    let prompt = build_stage2_prompt(prompts);
    let user_payload = build_stage2_user_payload(
        target_query,
        dictionary_facts,
        stage1_output,
        generation_hint,
    );
    let options = stage2_chat_options(quality_mode);

    for cycle in 1..=HARD_FAILURE_RETRY_CYCLES {
        match request_and_stream_model(
            ai_client,
            primary_model,
            &prompt,
            &user_payload,
            options,
            tx,
        )
        .await
        {
            StreamModelOutcome::Success(markdown) => {
                return Ok(StreamedMarkdown {
                    markdown,
                    model: primary_model.to_string(),
                });
            }
            StreamModelOutcome::Canceled => {
                return Err(anyhow!("stream analyze canceled by client"));
            }
            StreamModelOutcome::Retriable(err)
                if !fallback_model.is_empty() && fallback_model != primary_model =>
            {
                tracing::warn!(
                    "grounded stream stage2 primary failed before deltas; switching to fallback: cycle={cycle}/{HARD_FAILURE_RETRY_CYCLES}, primary={primary_model}, fallback={fallback_model}, target={target_query}, err={err:#}"
                );
                match request_and_stream_model(
                    ai_client,
                    fallback_model,
                    &prompt,
                    &user_payload,
                    options,
                    tx,
                )
                .await
                {
                    StreamModelOutcome::Canceled => {
                        return Err(anyhow!("stream analyze canceled by client"));
                    }
                    StreamModelOutcome::Success(markdown) => {
                        return Ok(StreamedMarkdown {
                            markdown,
                            model: fallback_model.to_string(),
                        });
                    }
                    StreamModelOutcome::Retriable(fallback_err)
                        if cycle < HARD_FAILURE_RETRY_CYCLES =>
                    {
                        tracing::warn!(
                            "grounded stream stage2 retrying after fallback hard failure: cycle={cycle}/{HARD_FAILURE_RETRY_CYCLES}, target={target_query}, err={fallback_err:#}"
                        );
                        tokio::time::sleep(Duration::from_millis(HARD_FAILURE_RETRY_DELAY_MS))
                            .await;
                    }
                    StreamModelOutcome::Retriable(fallback_err)
                    | StreamModelOutcome::Fatal(fallback_err) => return Err(fallback_err),
                }
            }
            StreamModelOutcome::Retriable(err) if cycle < HARD_FAILURE_RETRY_CYCLES => {
                tracing::warn!(
                    "grounded stream stage2 retrying after primary hard failure: cycle={cycle}/{HARD_FAILURE_RETRY_CYCLES}, primary={primary_model}, target={target_query}, err={err:#}"
                );
                tokio::time::sleep(Duration::from_millis(HARD_FAILURE_RETRY_DELAY_MS)).await;
            }
            StreamModelOutcome::Retriable(err) | StreamModelOutcome::Fatal(err) => return Err(err),
        }
    }

    unreachable!("grounded stream stage2 retry loop should always return or error");
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

    let structure_prompt =
        crate::services::analysis_structure_prompt::build_structure_prompt(&state.prompts);
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

pub(crate) struct GroundedStage1 {
    pub dictionary_facts: Option<String>,
    pub output: ModelAOutput,
}

struct ResolvedStructureClient {
    client: AiClient,
    model: String,
}
