use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

pub use crate::dictionary_lexeme_models::*;

// ==========================================
// 1. Dictionary Raw (权威数据层)
// ==========================================

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct DictionaryRaw {
    pub headword: String,
    pub raw_data: serde_json::Value,
    pub has_audio: Option<bool>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewDictionaryEntry {
    pub headword: String,
    pub raw_data: serde_json::Value,
    pub has_audio: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct DictionaryLearningOrder {
    pub headword: String,
    pub cefr_level: Option<String>,
    pub cefr_rank: Option<i32>,
    pub frequency_rank: Option<i32>,
    pub learning_order: Option<i32>,
    pub source: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewDictionaryLearningOrder {
    pub headword: String,
    pub cefr_level: Option<String>,
    pub cefr_rank: Option<i32>,
    pub frequency_rank: Option<i32>,
    pub learning_order: Option<i32>,
    pub source: String,
}

// ==========================================
// 2. Knowledge Entries (用户知识层)
// ==========================================

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct KnowledgeEntry {
    pub id: i64,
    pub query_text: String,
    pub prototype: Option<String>,
    pub entry_type: String,
    pub analysis: serde_json::Value,
    pub tags: Option<Vec<String>>,
    pub aliases: Option<Vec<String>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewKnowledgeEntry {
    pub query_text: String,
    pub prototype: Option<String>,
    pub entry_type: String,
    pub analysis: serde_json::Value,
    pub tags: Option<Vec<String>>,
    pub aliases: Option<Vec<String>>,
}

// ==========================================
// 3. Follow-ups (追问记录)
// ==========================================

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct FollowUp {
    pub id: i64,
    pub entry_id: i64,
    pub question: String,
    pub answer: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewFollowUp {
    pub entry_id: i64,
    pub question: String,
    pub answer: String,
}

// ==========================================
// 4. Learning Progress (FSRS 学习进度)
// ==========================================

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct LearningProgress {
    pub entry_id: i64,
    pub stability: f32,
    pub difficulty: f32,
    pub elapsed_days: i64,
    pub scheduled_days: i64,
    pub state: i32, // 0:New, 1:Learning, 2:Review, 3:Relearning
    pub last_review_at: Option<DateTime<Utc>>,
    pub due_date: DateTime<Utc>,
    pub review_count: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewLearningProgress {
    pub entry_id: i64,
}

// FSRS Rating enum (对应用户评分)
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum Rating {
    Again = 1,
    Hard = 2,
    Good = 3,
    Easy = 4,
}

impl Rating {
    pub fn from_i32(value: i32) -> Option<Self> {
        match value {
            1 => Some(Rating::Again),
            2 => Some(Rating::Hard),
            3 => Some(Rating::Good),
            4 => Some(Rating::Easy),
            _ => None,
        }
    }
}

// ==========================================
// API Request/Response DTOs
// ==========================================

#[derive(Debug, Serialize, Deserialize)]
pub struct AnalyzeRequest {
    pub query_text: String,
    pub entry_type: Option<String>,
    pub generation_hint: Option<String>,
    #[serde(default)]
    pub quality_mode: QualityMode,
    #[serde(default)]
    pub force_refresh: bool,
    pub entry_id: Option<i64>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AnalyzeResponse {
    pub entry_id: i64,
    pub query_text: String,
    pub analysis_markdown: String,
    #[serde(default)]
    pub phrase_lookup: Option<PhraseLookupInfo>,
    #[serde(default)]
    pub phrase_usage_preview: Option<PhraseUsagePreview>,
    #[serde(default)]
    pub attached_phrase_modules: Vec<AttachedPhraseModule>,
    pub source: String,
    pub model: Option<String>,
    pub quality_mode: Option<QualityMode>,
    pub follow_ups: Vec<FollowUpItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecentItem {
    pub entry_id: i64,
    pub query_text: String,
    pub preview: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum LibraryQueryTab {
    #[default]
    All,
    Learning,
    Review,
    New,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LibraryEntriesPageResponse {
    pub items: Vec<RecentItem>,
    pub total: i64,
    pub next_cursor: Option<String>,
    pub limit: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntryDetailResponse {
    pub entry_id: i64,
    pub query_text: String,
    pub entry_type: String,
    pub prototype: Option<String>,
    pub analysis_markdown: String,
    #[serde(default)]
    pub phrase_lookup: Option<PhraseLookupInfo>,
    #[serde(default)]
    pub phrase_usage_preview: Option<PhraseUsagePreview>,
    #[serde(default)]
    pub attached_phrase_modules: Vec<AttachedPhraseModule>,
    pub source: String,
    pub model: Option<String>,
    pub quality_mode: Option<QualityMode>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub aliases: Vec<String>,
    #[serde(default)]
    pub follow_ups: Vec<FollowUpItem>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntryDeleteResponse {
    pub message: String,
    pub deleted_entry_id: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DBSuggestion {
    pub suggestion_type: String,
    pub entry_id: i64,
    pub query_text: String,
    pub preview: String,
    pub analysis_markdown: String,
    pub source: String,
    pub follow_ups: Vec<FollowUpItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuggestionResponse {
    pub suggestions: Vec<DBSuggestion>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntelligentSearchRequest {
    pub term: String,
    pub hint: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusResponse {
    pub status: String,
    pub db_status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseBackupPayload {
    pub format: String,
    pub exported_at: DateTime<Utc>,
    pub knowledge_entries: Vec<KnowledgeEntry>,
    pub follow_ups: Vec<FollowUp>,
    pub learning_progress: Vec<LearningProgress>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseImportResponse {
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FollowUpItem {
    pub id: i64,
    pub question: String,
    pub answer: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FollowUpRequest {
    pub entry_id: i64,
    pub question: String,
    #[serde(default)]
    pub quality_mode: QualityMode,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FollowUpResponse {
    pub answer: String,
    pub follow_up: FollowUp,
    pub model: Option<String>,
    pub quality_mode: Option<QualityMode>,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum QualityMode {
    #[default]
    Default,
    Pro,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamMetaPayload {
    pub kind: String,
    pub model: String,
    pub quality_mode: QualityMode,
    pub source: String,
    pub fallback: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamDeltaPayload {
    pub delta: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamErrorPayload {
    pub message: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ReviewRequest {
    pub entry_id: i64,
    pub rating: i32, // 1-4
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ReviewResponse {
    pub progress: LearningProgress,
    pub next_due_date: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisDocument {
    pub markdown: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub aliases: Vec<String>,
    pub prototype: Option<String>,
    #[serde(default)]
    pub phrase_lookup: Option<PhraseLookupInfo>,
    #[serde(default)]
    pub phrase_usage_preview: Option<PhraseUsagePreview>,
    #[serde(default)]
    pub attached_phrase_modules: Vec<AttachedPhraseModule>,
    pub dictionary_excerpt: Option<serde_json::Value>,
    pub model: Option<String>,
    pub quality_mode: Option<QualityMode>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhraseUsageModule {
    pub title: String,
    pub explanation: String,
    pub example_de: String,
    pub example_zh: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhraseUsagePreview {
    pub meaning_zh: String,
    pub meaning_en: String,
    pub usage_module: PhraseUsageModule,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttachedPhraseModule {
    pub phrase: String,
    pub host_headword: String,
    pub source_phrase_entry_id: i64,
    #[serde(default)]
    pub usage_module: Option<PhraseUsageModule>,
    pub analysis_markdown: String,
    pub confidence: PhraseLookupConfidence,
    pub attached_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttachPhraseRequest {
    pub phrase_entry_id: Option<i64>,
    pub host_headword: String,
    #[serde(default)]
    pub phrase: Option<String>,
    #[serde(default)]
    pub phrase_lookup: Option<PhraseLookupInfo>,
    #[serde(default)]
    pub phrase_usage_preview: Option<PhraseUsagePreview>,
    #[serde(default)]
    pub analysis_markdown: Option<String>,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub quality_mode: Option<QualityMode>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetachPhraseRequest {
    pub host_entry_id: i64,
    pub source_phrase_entry_id: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhraseHostCandidate {
    pub headword: String,
    pub source: String,
    pub score: f32,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PhraseLookupConfidence {
    High,
    Medium,
    Low,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhraseLookupInfo {
    pub phrase: String,
    pub best_host_headword: Option<String>,
    pub confidence: PhraseLookupConfidence,
    #[serde(default)]
    pub host_candidates: Vec<PhraseHostCandidate>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearningProgressView {
    pub entry_id: i64,
    pub review_count: i32,
    pub next_review_at: DateTime<Utc>,
    pub last_reviewed_at: Option<DateTime<Utc>>,
    pub scheduled_days: i64,
    pub stability: f32,
    pub difficulty: f32,
    pub state: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearningSessionWord {
    pub entry_id: i64,
    pub query_text: String,
    pub analysis_markdown: String,
    pub repetitions_left: i32,
    pub progress: Option<LearningProgressView>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearningSessionResponse {
    pub current_word: Option<LearningSessionWord>,
    pub completed_count: i32,
    pub total_count: i32,
    pub is_completed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearningProgressMapResponse {
    pub progress: std::collections::HashMap<i64, LearningProgressView>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearningStatsResponse {
    pub total_words: i64,
    pub due_today: i64,
    pub average_stability: f32,
}
