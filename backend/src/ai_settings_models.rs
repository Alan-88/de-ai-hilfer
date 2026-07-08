use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct AiProviderProfileRecord {
    pub id: i64,
    pub name: String,
    pub base_url: String,
    pub api_key: String,
    pub model_ids: Vec<String>,
    pub is_default: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct AiTaskModelSettingRecord {
    pub task_key: String,
    pub provider_id: Option<i64>,
    pub model_id: Option<String>,
    pub inherit_task_key: Option<String>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiProviderProfileView {
    pub id: Option<i64>,
    pub name: String,
    pub base_url: String,
    #[serde(default)]
    pub model_ids: Vec<String>,
    pub is_default: bool,
    pub api_key_set: bool,
    pub api_key_preview: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiTaskModelSettingView {
    pub task_key: String,
    pub provider_name: Option<String>,
    pub model_id: Option<String>,
    pub inherit_task_key: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiSettingsResponse {
    pub profiles: Vec<AiProviderProfileView>,
    pub task_settings: Vec<AiTaskModelSettingView>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiProviderProfileInput {
    #[serde(default)]
    pub id: Option<i64>,
    pub name: String,
    pub base_url: String,
    #[serde(default)]
    pub api_key: Option<String>,
    #[serde(default)]
    pub model_ids: Vec<String>,
    #[serde(default)]
    pub is_default: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiTaskModelSettingInput {
    pub task_key: String,
    #[serde(default)]
    pub provider_name: Option<String>,
    #[serde(default)]
    pub model_id: Option<String>,
    #[serde(default)]
    pub inherit_task_key: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiSettingsUpdateRequest {
    #[serde(default)]
    pub profiles: Vec<AiProviderProfileInput>,
    #[serde(default)]
    pub task_settings: Vec<AiTaskModelSettingInput>,
}
