use chrono::{DateTime, Utc};
use pgvector::Vector;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct DictionaryRawEntry {
    pub id: i64,
    pub source_key: String,
    pub headword: String,
    pub normalized_headword: String,
    pub lang_code: String,
    pub pos: Option<String>,
    pub is_form_of: bool,
    pub form_of_words: Vec<String>,
    pub raw_data: serde_json::Value,
    pub has_audio: Option<bool>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct DictionaryLexeme {
    pub id: i64,
    pub bundle_key: String,
    pub surface: String,
    pub normalized_surface: String,
    pub gloss_preview: serde_json::Value,
    pub pos_summary: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct DictionaryLexemeRawEntry {
    pub lexeme_id: i64,
    pub raw_entry_id: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct DictionarySurfaceForm {
    pub id: i64,
    pub surface: String,
    pub normalized_surface: String,
    pub lexeme_id: i64,
    pub source: String,
    pub raw_entry_id: Option<i64>,
    pub confidence: f32,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct DictionaryLexemeEmbedding {
    pub lexeme_id: i64,
    pub model_id: String,
    pub source_text: String,
    pub dimensions: i32,
    pub embedding: Vector,
    pub frequency_rank: Option<i32>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewDictionaryLexemeEmbedding {
    pub lexeme_id: i64,
    pub model_id: String,
    pub source_text: String,
    pub dimensions: i32,
    pub embedding: Vector,
    pub frequency_rank: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct LexemeCandidate {
    pub lexeme_id: i64,
    pub surface: String,
    pub pos_summary: Vec<String>,
    pub gloss_preview: serde_json::Value,
    pub matched_surface: String,
    pub matched_source: String,
}
