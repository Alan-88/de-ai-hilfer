use crate::models::AiModelOverride;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged, deny_unknown_fields)]
pub enum BatchId {
    Id(i64),
    Reference { step: String, path: String },
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum FailurePolicy {
    #[default]
    Continue,
    Stop,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MembershipAction {
    Add,
    Remove,
    Reset,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TargetType {
    #[default]
    Entry,
    Lexeme,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case", deny_unknown_fields)]
pub enum BatchOperation {
    LexemeInitialize {
        lexeme_ids: Vec<i64>,
        #[serde(default)]
        model_override: Option<AiModelOverride>,
    },
    LearningMembership {
        mutation: MembershipAction,
        #[serde(default)]
        target_type: TargetType,
        ids: Vec<BatchId>,
    },
    FolderCreate {
        name: String,
        #[serde(default)]
        parent_id: Option<BatchId>,
    },
    ArticleImport {
        title: String,
        body: String,
        #[serde(default)]
        folder_id: Option<BatchId>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BatchStep {
    pub id: String,
    #[serde(default)]
    pub depends_on: Vec<String>,
    pub operation: BatchOperation,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BatchRequest {
    pub idempotency_key: String,
    #[serde(default)]
    pub on_error: FailurePolicy,
    pub operations: Vec<BatchStep>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchStepState {
    pub id: String,
    pub status: String,
    pub attempts: u32,
    pub result: Option<Value>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct BatchJob {
    pub id: String,
    pub status: String,
    pub steps: Value,
    pub error: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}
