use crate::ai::AiClient;
use crate::models::{
    AnalysisDocument, DictionaryRaw, PhraseLookupInfo, QualityMode, StreamMetaPayload,
};
use crate::services::analysis_preview::analysis_markdown;
use crate::services::analyze_runtime::{fallback_model_for, primary_model_for};
use crate::services::analyze_support::{
    analysis_chat_options, build_analysis_prompt, build_phrase_preview_analysis,
    parse_phrase_usage_preview, stream_analysis_chat_options, AnalysisMode,
};
use crate::services::dictionary_render::build_dictionary_excerpt;
use crate::services::stream_generation::{
    request_and_stream_model, StreamModelOutcome, HARD_FAILURE_RETRY_CYCLES,
    HARD_FAILURE_RETRY_DELAY_MS,
};
use crate::services::stream_response::sse_meta;
use crate::state::AppState;
use anyhow::Result;
use serde_json::Value;
use tokio::sync::mpsc::UnboundedSender;

pub struct StreamedMarkdown {
    pub markdown: String,
    pub model: String,
}

pub fn send_meta(
    tx: &UnboundedSender<String>,
    kind: &str,
    model: &str,
    quality_mode: QualityMode,
    source: &str,
    fallback: bool,
) {
    tx.send(sse_meta(&StreamMetaPayload {
        kind: kind.to_string(),
        model: model.to_string(),
        quality_mode,
        source: source.to_string(),
        fallback,
    }))
    .ok();
}

pub fn cached_analysis_markdown(analysis: &Value) -> String {
    analysis_markdown(analysis)
}

pub fn build_stream_analysis_document(
    markdown: String,
    dictionary_entry: Option<&DictionaryRaw>,
    quality_mode: QualityMode,
    model: &str,
    phrase_lookup: Option<&PhraseLookupInfo>,
) -> AnalysisDocument {
    AnalysisDocument {
        markdown: markdown.trim().to_string(),
        tags: Vec::new(),
        aliases: Vec::new(),
        prototype: dictionary_entry.map(|entry| entry.headword.clone()),
        phrase_lookup: phrase_lookup.cloned(),
        phrase_usage_preview: None,
        attached_phrase_modules: Vec::new(),
        dictionary_excerpt: dictionary_entry.map(|entry| build_dictionary_excerpt(&entry.raw_data)),
        model: Some(model.to_string()),
        quality_mode: Some(quality_mode),
    }
}

pub async fn generate_phrase_preview_with_model(
    state: &AppState,
    target_query: &str,
    dictionary_entry: Option<&DictionaryRaw>,
    quality_mode: QualityMode,
    generation_hint: Option<&str>,
    phrase_lookup: Option<&PhraseLookupInfo>,
    tx: &UnboundedSender<String>,
) -> Result<AnalysisDocument> {
    let primary_model = primary_model_for(state, quality_mode);
    let fallback_model = fallback_model_for(state, quality_mode);
    let prompt = build_analysis_prompt(
        &state.prompts,
        dictionary_entry,
        AnalysisMode::Full,
        generation_hint,
        phrase_lookup,
    );
    let options = analysis_chat_options(AnalysisMode::Full);

    for cycle in 1..=HARD_FAILURE_RETRY_CYCLES {
        send_meta(
            tx,
            "analyze",
            primary_model,
            quality_mode,
            if quality_mode == QualityMode::Pro {
                "Pro"
            } else {
                "Flash"
            },
            false,
        );
        match state
            .ai_client
            .chat_model_with_options(primary_model, &prompt, target_query, options)
            .await
        {
            Ok(content) => {
                let (preview, tags, aliases) = parse_phrase_usage_preview(&content, target_query)?;
                return Ok(build_phrase_preview_analysis(
                    target_query,
                    preview,
                    phrase_lookup,
                    dictionary_entry,
                    tags,
                    aliases,
                    quality_mode,
                    primary_model,
                ));
            }
            Err(primary_err)
                if crate::ai::is_hard_failure(&primary_err)
                    && !fallback_model.is_empty()
                    && fallback_model != primary_model =>
            {
                send_meta(
                    tx,
                    "analyze",
                    fallback_model,
                    quality_mode,
                    if quality_mode == QualityMode::Pro {
                        "Pro"
                    } else {
                        "Flash"
                    },
                    true,
                );
                match state
                    .ai_client
                    .chat_model_with_options(fallback_model, &prompt, target_query, options)
                    .await
                {
                    Ok(content) => {
                        let (preview, tags, aliases) =
                            parse_phrase_usage_preview(&content, target_query)?;
                        return Ok(build_phrase_preview_analysis(
                            target_query,
                            preview,
                            phrase_lookup,
                            dictionary_entry,
                            tags,
                            aliases,
                            quality_mode,
                            fallback_model,
                        ));
                    }
                    Err(fallback_err)
                        if cycle < HARD_FAILURE_RETRY_CYCLES
                            && crate::ai::is_hard_failure(&fallback_err) =>
                    {
                        tokio::time::sleep(std::time::Duration::from_millis(
                            HARD_FAILURE_RETRY_DELAY_MS,
                        ))
                        .await;
                    }
                    Err(fallback_err) => return Err(fallback_err),
                }
            }
            Err(primary_err)
                if cycle < HARD_FAILURE_RETRY_CYCLES
                    && crate::ai::is_hard_failure(&primary_err) =>
            {
                tokio::time::sleep(std::time::Duration::from_millis(
                    HARD_FAILURE_RETRY_DELAY_MS,
                ))
                .await;
            }
            Err(primary_err) => return Err(primary_err),
        }
    }

    unreachable!("phrase preview retry loop should always return or error");
}

pub async fn stream_model_markdown(
    client: &AiClient,
    tx: &UnboundedSender<String>,
    kind: &str,
    primary_model: &str,
    fallback_model: &str,
    quality_mode: QualityMode,
    system_prompt: &str,
    user_message: &str,
    source: &str,
) -> Result<StreamedMarkdown> {
    for cycle in 1..=HARD_FAILURE_RETRY_CYCLES {
        send_meta(tx, kind, primary_model, quality_mode, source, false);
        match request_and_stream_model(
            client,
            primary_model,
            system_prompt,
            user_message,
            stream_analysis_chat_options(),
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
            StreamModelOutcome::Retriable(err)
                if !fallback_model.is_empty() && fallback_model != primary_model =>
            {
                tracing::warn!(
                    "stream primary attempt failed before deltas; switching to fallback: cycle={cycle}/{HARD_FAILURE_RETRY_CYCLES}, primary={primary_model}, fallback={fallback_model}, err={err:#}"
                );
                send_meta(tx, kind, fallback_model, quality_mode, source, true);
                match request_and_stream_model(
                    client,
                    fallback_model,
                    system_prompt,
                    user_message,
                    stream_analysis_chat_options(),
                    tx,
                )
                .await
                {
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
                            "stream retrying after fallback hard failure before deltas: cycle={cycle}/{HARD_FAILURE_RETRY_CYCLES}, err={fallback_err:#}"
                        );
                        tokio::time::sleep(std::time::Duration::from_millis(
                            HARD_FAILURE_RETRY_DELAY_MS,
                        ))
                        .await;
                    }
                    StreamModelOutcome::Retriable(fallback_err)
                    | StreamModelOutcome::Fatal(fallback_err) => return Err(fallback_err),
                }
            }
            StreamModelOutcome::Retriable(err) if cycle < HARD_FAILURE_RETRY_CYCLES => {
                tracing::warn!(
                    "stream retrying after primary hard failure before deltas: cycle={cycle}/{HARD_FAILURE_RETRY_CYCLES}, primary={primary_model}, err={err:#}"
                );
                tokio::time::sleep(std::time::Duration::from_millis(
                    HARD_FAILURE_RETRY_DELAY_MS,
                ))
                .await;
            }
            StreamModelOutcome::Retriable(err) | StreamModelOutcome::Fatal(err) => return Err(err),
        }
    }

    unreachable!("stream retry loop should always return or error");
}
