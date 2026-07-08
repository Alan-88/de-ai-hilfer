use crate::ai::AiClient;
use crate::models::{AiProviderProfileRecord, AiTaskModelSettingRecord};
use crate::repositories::ai_settings;
use crate::state::AppState;
use anyhow::{anyhow, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AiModelTask {
    Analyze,
    Phrase,
    Structure,
    Embedding,
    IntelligentSearch,
}

pub struct ResolvedAiModel {
    pub client: AiClient,
    pub model: String,
    pub persisted: bool,
}

pub async fn resolve_task_model(state: &AppState, task: AiModelTask) -> Result<ResolvedAiModel> {
    let profiles = ai_settings::list_provider_profiles(&state.pool).await?;
    if profiles.is_empty() {
        return Ok(env_model(state, task));
    }

    let settings = ai_settings::list_task_model_settings(&state.pool).await?;
    match resolve_persisted_setting(task, &profiles, &settings, 0)? {
        Some(resolved) => Ok(resolved),
        None => Ok(env_model(state, task)),
    }
}

fn resolve_persisted_setting(
    task: AiModelTask,
    profiles: &[AiProviderProfileRecord],
    settings: &[AiTaskModelSettingRecord],
    depth: usize,
) -> Result<Option<ResolvedAiModel>> {
    if depth > 2 {
        return Err(anyhow!("AI model setting inheritance is too deep"));
    }

    let Some(setting) = settings
        .iter()
        .find(|setting| setting.task_key == task.key())
    else {
        return Ok(None);
    };

    if let Some(inherit) = setting.inherit_task_key.as_deref() {
        let inherited_task = AiModelTask::from_key(inherit)
            .ok_or_else(|| anyhow!("unsupported inherited AI task key: {inherit}"))?;
        return resolve_persisted_setting(inherited_task, profiles, settings, depth + 1);
    }

    let provider_id = setting
        .provider_id
        .ok_or_else(|| anyhow!("AI task {} is missing provider", task.key()))?;
    let profile = profiles
        .iter()
        .find(|profile| profile.id == provider_id)
        .ok_or_else(|| anyhow!("AI provider profile not found: {provider_id}"))?;
    let model = setting
        .model_id
        .as_deref()
        .map(str::trim)
        .filter(|model| !model.is_empty())
        .ok_or_else(|| anyhow!("AI task {} is missing model", task.key()))?;

    Ok(Some(ResolvedAiModel {
        client: AiClient::new(profile.api_key.clone(), profile.base_url.clone()),
        model: model.to_string(),
        persisted: true,
    }))
}

fn env_model(state: &AppState, task: AiModelTask) -> ResolvedAiModel {
    ResolvedAiModel {
        client: state.ai_client.clone(),
        model: match task {
            AiModelTask::Analyze | AiModelTask::Phrase => state.config.ai_models.analyze.clone(),
            AiModelTask::Structure => env_structure_model(state),
            AiModelTask::Embedding => state.config.ai_models.embedding.clone(),
            AiModelTask::IntelligentSearch => state.config.ai_models.intelligent_search.clone(),
        },
        persisted: false,
    }
}

pub fn env_structure_model(_state: &AppState) -> String {
    std::env::var("AI_MODEL_STRUCTURE")
        .ok()
        .filter(|model| !model.trim().is_empty())
        .or_else(|| {
            std::env::var("STRUCTURE_ARCHIVE_MODEL")
                .ok()
                .filter(|model| !model.trim().is_empty())
        })
        .unwrap_or_else(|| "minimax-m2.5".to_string())
}

impl AiModelTask {
    fn key(self) -> &'static str {
        match self {
            AiModelTask::Analyze => "analyze",
            AiModelTask::Phrase => "phrase",
            AiModelTask::Structure => "structure",
            AiModelTask::Embedding => "embedding",
            AiModelTask::IntelligentSearch => "intelligent_search",
        }
    }

    fn from_key(key: &str) -> Option<Self> {
        match key {
            "analyze" => Some(Self::Analyze),
            "phrase" => Some(Self::Phrase),
            "structure" => Some(Self::Structure),
            "embedding" => Some(Self::Embedding),
            "intelligent_search" => Some(Self::IntelligentSearch),
            _ => None,
        }
    }
}
