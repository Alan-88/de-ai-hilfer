use crate::ai::is_hard_failure;
use crate::models::{DictionaryRaw, PhraseLookupInfo, QualityMode};
use crate::services::analyze_support::{
    analysis_chat_options, build_analysis_prompt, AnalysisMode,
};
use crate::state::AppState;
use anyhow::Result;
use std::time::Duration;

pub struct GeneratedAnalysis {
    pub content: String,
    pub model: String,
}

pub async fn generate_analysis_with_model(
    state: &AppState,
    target_query: &str,
    dictionary_entry: Option<&DictionaryRaw>,
    mode: AnalysisMode,
    quality_mode: QualityMode,
    generation_hint: Option<&str>,
    phrase_lookup: Option<&PhraseLookupInfo>,
) -> Result<GeneratedAnalysis> {
    let primary_model = primary_model_for(state, quality_mode);
    let fallback_model = fallback_model_for(state, quality_mode);
    let prompt = build_analysis_prompt(
        &state.prompts,
        dictionary_entry,
        mode,
        generation_hint,
        phrase_lookup,
    );
    let options = analysis_chat_options(mode);

    for cycle in 1..=2 {
        match state
            .ai_client
            .chat_model_with_options(primary_model, &prompt, target_query, options)
            .await
        {
            Ok(content) => {
                return Ok(GeneratedAnalysis {
                    content,
                    model: primary_model.to_string(),
                });
            }
            Err(primary_err) if cycle < 2 && is_hard_failure(&primary_err) => {
                tracing::warn!(
                    "analyze retrying after primary hard failure: cycle={cycle}/2, primary={primary_model}, target={target_query}, err={primary_err:#}"
                );
                tokio::time::sleep(Duration::from_millis(1200)).await;
            }
            Err(primary_err)
                if is_hard_failure(&primary_err)
                    && !fallback_model.is_empty()
                    && fallback_model != primary_model =>
            {
                tracing::warn!(
                    "analyze switching to fallback model after primary retries: primary={primary_model}, fallback={fallback_model}, target={target_query}, err={primary_err:#}"
                );
                match state
                    .ai_client
                    .chat_model_with_options(fallback_model, &prompt, target_query, options)
                    .await
                {
                    Ok(content) => {
                        return Ok(GeneratedAnalysis {
                            content,
                            model: fallback_model.to_string(),
                        });
                    }
                    Err(fallback_err) => return Err(fallback_err),
                }
            }
            Err(primary_err) => return Err(primary_err),
        }
    }

    unreachable!("analysis retry loop should always return or error");
}

pub fn primary_model_for(state: &AppState, quality_mode: QualityMode) -> &str {
    match quality_mode {
        QualityMode::Default => state.config.ai_models.analyze.as_str(),
        QualityMode::Pro => state.config.ai_models.analyze_pro.as_str(),
    }
}

pub fn fallback_model_for(state: &AppState, quality_mode: QualityMode) -> &str {
    match quality_mode {
        QualityMode::Default => state.config.ai_models.fallback_fast.as_str(),
        QualityMode::Pro => state.config.ai_models.fallback_pro.as_str(),
    }
}
