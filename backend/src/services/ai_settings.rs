use crate::ai::{AiChatOptions, AiClient, AiEmbeddingOptions};
use crate::models::{
    AiModelTestRequest, AiModelTestResponse, AiProviderProfileView, AiSettingsResponse,
    AiSettingsUpdateRequest, AiTaskModelSettingView,
};
use crate::repositories::ai_settings;
use crate::services::ai_model_resolver::env_structure_model;
use crate::state::AppState;
use anyhow::{anyhow, Result};
use std::collections::{BTreeSet, HashMap, HashSet};
use std::time::Duration;

const TASK_ANALYZE: &str = "analyze";
const TASK_PHRASE: &str = "phrase";
const TASK_STRUCTURE: &str = "structure";
const TASK_EMBEDDING: &str = "embedding";
const TASK_INTELLIGENT_SEARCH: &str = "intelligent_search";
const ALLOWED_TASKS: [&str; 5] = [
    TASK_ANALYZE,
    TASK_PHRASE,
    TASK_STRUCTURE,
    TASK_EMBEDDING,
    TASK_INTELLIGENT_SEARCH,
];

pub async fn get_ai_settings(state: &AppState) -> Result<AiSettingsResponse> {
    let profiles = ai_settings::list_provider_profiles(&state.pool).await?;
    let task_settings = ai_settings::list_task_model_settings(&state.pool).await?;

    if profiles.is_empty() {
        return Ok(env_backed_settings(state));
    }

    let profile_names = profiles
        .iter()
        .map(|profile| (profile.id, profile.name.clone()))
        .collect::<HashMap<_, _>>();

    Ok(AiSettingsResponse {
        profiles: profiles
            .into_iter()
            .map(|profile| AiProviderProfileView {
                id: Some(profile.id),
                name: profile.name,
                base_url: profile.base_url,
                model_ids: profile.model_ids,
                is_default: profile.is_default,
                api_key_set: !profile.api_key.is_empty(),
                api_key_preview: api_key_preview(&profile.api_key),
            })
            .collect(),
        task_settings: fill_missing_tasks(
            task_settings
                .into_iter()
                .map(|setting| AiTaskModelSettingView {
                    task_key: setting.task_key,
                    provider_name: setting
                        .provider_id
                        .and_then(|id| profile_names.get(&id).cloned()),
                    model_id: setting.model_id,
                    inherit_task_key: setting.inherit_task_key,
                })
                .collect(),
            state,
        ),
    })
}

pub async fn update_ai_settings(
    state: &AppState,
    mut request: AiSettingsUpdateRequest,
) -> Result<AiSettingsResponse> {
    hydrate_env_api_key_for_first_save(state, &mut request).await?;
    validate_update(&request)?;
    ai_settings::replace_ai_settings(&state.pool, &request.profiles, &request.task_settings)
        .await?;
    get_ai_settings(state).await
}

pub async fn test_ai_model(
    state: &AppState,
    request: AiModelTestRequest,
) -> Result<AiModelTestResponse> {
    let base_url = request.base_url.trim();
    let model = request.model_id.trim();
    if base_url.is_empty() || model.is_empty() {
        return Ok(test_response(false, "Base URL 或模型 ID 为空"));
    }

    let api_key = resolve_test_api_key(state, &request).await?;
    let Some(api_key) = api_key else {
        return Ok(test_response(false, "缺少 API Key"));
    };

    let client = AiClient::new(api_key, base_url.to_string());
    let result = if looks_like_embedding_model(model) {
        client
            .embed_model_with_options(
                model,
                &["ping".to_string()],
                AiEmbeddingOptions {
                    timeout: Duration::from_secs(20),
                    dimensions: None,
                },
            )
            .await
            .map(|_| ())
    } else {
        client
            .chat_model_with_options(
                model,
                "Reply with exactly: ok",
                "ok",
                AiChatOptions {
                    temperature: 0.0,
                    max_tokens: Some(8),
                    timeout: Duration::from_secs(20),
                },
            )
            .await
            .map(|_| ())
    };

    match result {
        Ok(()) => Ok(test_response(true, "模型可用")),
        Err(err) => Ok(test_response(false, format!("测试失败：{err}"))),
    }
}

async fn resolve_test_api_key(
    state: &AppState,
    request: &AiModelTestRequest,
) -> Result<Option<String>> {
    if let Some(api_key) = request
        .api_key
        .as_deref()
        .map(str::trim)
        .filter(|key| !key.is_empty())
    {
        return Ok(Some(api_key.to_string()));
    }

    if let Some(profile_id) = request.profile_id {
        let profiles = ai_settings::list_provider_profiles(&state.pool).await?;
        if let Some(profile) = profiles
            .into_iter()
            .find(|profile| profile.id == profile_id)
        {
            return Ok((!profile.api_key.is_empty()).then_some(profile.api_key));
        }
    }

    Ok(state
        .config
        .openai_api_key
        .as_deref()
        .map(str::trim)
        .filter(|key| !key.is_empty())
        .map(ToString::to_string))
}

fn looks_like_embedding_model(model: &str) -> bool {
    model.to_ascii_lowercase().contains("embedding")
}

fn test_response(message_ok: bool, message: impl Into<String>) -> AiModelTestResponse {
    AiModelTestResponse {
        success: message_ok,
        message: message.into(),
    }
}

async fn hydrate_env_api_key_for_first_save(
    state: &AppState,
    request: &mut AiSettingsUpdateRequest,
) -> Result<()> {
    if !ai_settings::list_provider_profiles(&state.pool)
        .await?
        .is_empty()
    {
        return Ok(());
    }

    let Some(env_api_key) = state
        .config
        .openai_api_key
        .as_deref()
        .map(str::trim)
        .filter(|key| !key.is_empty())
    else {
        return Ok(());
    };

    if let Some(profile) = request
        .profiles
        .iter_mut()
        .find(|profile| profile.id.is_none() && profile.is_default && profile.api_key.is_none())
    {
        profile.api_key = Some(env_api_key.to_string());
    }

    Ok(())
}

fn validate_update(request: &AiSettingsUpdateRequest) -> Result<()> {
    if request.profiles.is_empty() {
        return Err(anyhow!("at least one provider profile is required"));
    }

    let default_count = request
        .profiles
        .iter()
        .filter(|profile| profile.is_default)
        .count();
    if default_count != 1 {
        return Err(anyhow!("exactly one provider profile must be default"));
    }

    let provider_names = request
        .profiles
        .iter()
        .map(|profile| profile.name.trim().to_string())
        .collect::<HashSet<_>>();

    let mut seen_tasks = HashSet::new();
    for setting in &request.task_settings {
        let task_key = setting.task_key.trim();
        if !ALLOWED_TASKS.contains(&task_key) {
            return Err(anyhow!("unsupported AI task key: {task_key}"));
        }
        if !seen_tasks.insert(task_key.to_string()) {
            return Err(anyhow!("duplicate AI task key: {task_key}"));
        }

        if let Some(inherit) = setting.inherit_task_key.as_deref().map(str::trim) {
            if task_key != TASK_PHRASE || inherit != TASK_ANALYZE {
                return Err(anyhow!(
                    "only phrase may inherit analyze in the initial settings schema"
                ));
            }
            continue;
        }

        let provider_name = setting
            .provider_name
            .as_deref()
            .map(str::trim)
            .ok_or_else(|| anyhow!("provider_name is required for {task_key}"))?;
        if !provider_names.contains(provider_name) {
            return Err(anyhow!("unknown provider profile: {provider_name}"));
        }

        let model_id = setting
            .model_id
            .as_deref()
            .map(str::trim)
            .ok_or_else(|| anyhow!("model_id is required for {task_key}"))?;
        if model_id.is_empty() {
            return Err(anyhow!("model_id is required for {task_key}"));
        }
    }

    Ok(())
}

fn env_backed_settings(state: &AppState) -> AiSettingsResponse {
    let profile_name = "环境变量".to_string();
    let profile = AiProviderProfileView {
        id: None,
        name: profile_name.clone(),
        base_url: state
            .config
            .openai_base_url
            .clone()
            .unwrap_or_else(|| "https://api.openai.com/v1".to_string()),
        model_ids: env_model_ids(state),
        is_default: true,
        api_key_set: state
            .config
            .openai_api_key
            .as_deref()
            .map(|key| !key.is_empty())
            .unwrap_or(false),
        api_key_preview: state
            .config
            .openai_api_key
            .as_deref()
            .and_then(api_key_preview),
    };

    AiSettingsResponse {
        profiles: vec![profile],
        task_settings: vec![
            direct_task(TASK_ANALYZE, &profile_name, &state.config.ai_models.analyze),
            AiTaskModelSettingView {
                task_key: TASK_PHRASE.to_string(),
                provider_name: None,
                model_id: None,
                inherit_task_key: Some(TASK_ANALYZE.to_string()),
            },
            direct_task(TASK_STRUCTURE, &profile_name, &env_structure_model(state)),
            direct_task(
                TASK_EMBEDDING,
                &profile_name,
                &state.config.ai_models.embedding,
            ),
            direct_task(
                TASK_INTELLIGENT_SEARCH,
                &profile_name,
                &state.config.ai_models.intelligent_search,
            ),
        ],
    }
}

fn fill_missing_tasks(
    mut task_settings: Vec<AiTaskModelSettingView>,
    state: &AppState,
) -> Vec<AiTaskModelSettingView> {
    let existing = task_settings
        .iter()
        .map(|setting| setting.task_key.clone())
        .collect::<HashSet<_>>();
    let fallback = env_backed_settings(state);

    task_settings.extend(
        fallback
            .task_settings
            .into_iter()
            .filter(|setting| !existing.contains(&setting.task_key)),
    );
    task_settings
}

fn env_model_ids(state: &AppState) -> Vec<String> {
    let mut models = BTreeSet::new();
    models.insert(state.config.ai_models.analyze.clone());
    models.insert(state.config.ai_models.analyze_pro.clone());
    models.insert(state.config.ai_models.follow_up.clone());
    models.insert(state.config.ai_models.follow_up_pro.clone());
    models.insert(state.config.ai_models.intelligent_search.clone());
    models.insert(state.config.ai_models.embedding.clone());
    models.insert(state.config.ai_models.fallback_fast.clone());
    models.insert(state.config.ai_models.fallback_pro.clone());
    models.insert(env_structure_model(state));
    models
        .into_iter()
        .filter(|model| !model.is_empty())
        .collect()
}

fn direct_task(task_key: &str, provider_name: &str, model_id: &str) -> AiTaskModelSettingView {
    AiTaskModelSettingView {
        task_key: task_key.to_string(),
        provider_name: Some(provider_name.to_string()),
        model_id: Some(model_id.to_string()),
        inherit_task_key: None,
    }
}

fn api_key_preview(api_key: &str) -> Option<String> {
    if api_key.is_empty() {
        return None;
    }
    let chars = api_key.chars().collect::<Vec<_>>();
    if chars.len() <= 8 {
        return Some("********".to_string());
    }
    Some(format!(
        "{}...{}",
        chars.iter().take(4).collect::<String>(),
        chars
            .iter()
            .rev()
            .take(4)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect::<String>()
    ))
}
