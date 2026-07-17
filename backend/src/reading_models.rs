use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ReadingFolder {
    pub id: i64,
    pub parent_id: Option<i64>,
    pub name: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ReadingArticle {
    pub id: i64,
    pub folder_id: Option<i64>,
    pub title: String,
    pub body: String,
    pub completed: bool,
    pub last_opened_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ReadingArticleBookmark {
    pub id: i64,
    pub article_id: i64,
    pub occurrence_index: i32,
    pub surface: String,
    pub form_of: String,
    pub lexeme_id: Option<i64>,
    pub entry_id: Option<i64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ReadingArticleSummary {
    pub id: i64,
    pub folder_id: Option<i64>,
    pub title: String,
    pub completed: bool,
    pub bookmark_count: i64,
    pub memory_percent: Option<i32>,
    pub last_opened_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ReadingIndexResponse {
    pub folders: Vec<ReadingFolder>,
    pub articles: Vec<ReadingArticleSummary>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ReadingContentSegment {
    Text {
        text: String,
    },
    Bookmark {
        occurrence_index: i32,
        surface: String,
        form_of: String,
        lexeme_id: Option<i64>,
        entry_id: Option<i64>,
    },
}

#[derive(Debug, Clone, Serialize)]
pub struct ReadingArticleDetail {
    #[serde(flatten)]
    pub article: ReadingArticle,
    pub segments: Vec<ReadingContentSegment>,
    pub bookmark_count: usize,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ReadingFolderRequest {
    pub name: String,
    pub parent_id: Option<i64>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ReadingArticleSaveRequest {
    pub title: String,
    pub body: String,
    pub folder_id: Option<i64>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ReadingArticleTitleRequest {
    pub title: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ReadingArticleFolderRequest {
    pub folder_id: Option<i64>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ReadingArticleStatusRequest {
    pub completed: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct ReadingMutationResponse {
    pub id: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct BookmarkValidationIssue {
    pub occurrence_index: i32,
    pub surface: String,
    pub form_of: String,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ReadingErrorResponse {
    pub message: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub bookmarks: Vec<BookmarkValidationIssue>,
}
