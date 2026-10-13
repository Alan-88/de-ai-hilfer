use crate::reading_models::{
    BookmarkValidationIssue, ReadingArticleDetail, ReadingArticleSummary, ReadingContentSegment,
    ReadingFolder, ReadingIndexResponse,
};
use crate::repositories::reading::{self, BookmarkTargetCandidate};
use crate::services::reading_content::{parse_reading_content, ParsedPart, ParsedReadingContent};
use crate::state::AppState;
use chrono::Utc;
use fsrs::{current_retrievability, MemoryState, DEFAULT_PARAMETERS};
use std::collections::HashMap;

#[derive(Debug, thiserror::Error)]
pub enum ReadingError {
    #[error("{0}")]
    BadRequest(String),
    #[error("{0}")]
    NotFound(String),
    #[error("{0}")]
    Conflict(String),
    #[error("书签校验失败")]
    BookmarkValidation(Vec<BookmarkValidationIssue>),
    #[error(transparent)]
    Database(#[from] sqlx::Error),
}

pub async fn index(state: &AppState) -> Result<ReadingIndexResponse, ReadingError> {
    let folders = reading::list_folders(&state.pool).await?;
    let rows = reading::list_articles(&state.pool).await?;
    let memory_rows = reading::list_article_memory(&state.pool).await?;
    let mut memory = HashMap::<i64, Vec<f32>>::new();
    let now = Utc::now();

    for row in memory_rows {
        let score = match (
            row.stability,
            row.difficulty,
            row.last_review_at,
            row.review_count,
        ) {
            (Some(stability), Some(difficulty), Some(last), Some(count))
                if count > 0 && stability > 0.0 && difficulty > 0.0 =>
            {
                let days = (now - last).num_seconds().max(0) as f32 / 86_400.0;
                current_retrievability(
                    MemoryState {
                        stability,
                        difficulty,
                    },
                    days,
                    DEFAULT_PARAMETERS[20],
                )
                .clamp(0.0, 1.0)
            }
            _ => 0.0,
        };
        memory.entry(row.article_id).or_default().push(score);
    }

    let articles = rows
        .into_iter()
        .map(|row| {
            let scores = memory.remove(&row.id).unwrap_or_default();
            let memory_percent = if row.bookmark_count == 0 {
                None
            } else if scores.is_empty() {
                Some(0)
            } else {
                Some((scores.iter().sum::<f32>() / scores.len() as f32 * 100.0).round() as i32)
            };
            ReadingArticleSummary {
                id: row.id,
                folder_id: row.folder_id,
                title: row.title,
                completed: row.completed,
                bookmark_count: row.bookmark_count,
                memory_percent,
                last_opened_at: row.last_opened_at,
                created_at: row.created_at,
                updated_at: row.updated_at,
            }
        })
        .collect();

    Ok(ReadingIndexResponse { folders, articles })
}

pub async fn article_detail(
    state: &AppState,
    article_id: i64,
) -> Result<ReadingArticleDetail, ReadingError> {
    let article = reading::find_article(&state.pool, article_id)
        .await?
        .ok_or_else(|| ReadingError::NotFound("文章不存在".to_string()))?;
    let parsed = parse_reading_content(&article.body).map_err(ReadingError::BookmarkValidation)?;
    let stored = reading::list_bookmarks(&state.pool, article_id).await?;
    let targets = stored
        .into_iter()
        .map(|bookmark| (bookmark.occurrence_index, bookmark))
        .collect::<HashMap<_, _>>();
    let bookmark_count = parsed.bookmarks.len();
    let segments = parsed
        .parts
        .into_iter()
        .map(|part| match part {
            ParsedPart::Text(text) => ReadingContentSegment::Text { text },
            ParsedPart::Bookmark(bookmark) => {
                let target = targets.get(&bookmark.occurrence_index);
                ReadingContentSegment::Bookmark {
                    occurrence_index: bookmark.occurrence_index,
                    surface: bookmark.surface,
                    form_of: bookmark.form_of,
                    lexeme_id: target.and_then(|item| item.lexeme_id),
                    entry_id: target.and_then(|item| item.entry_id),
                }
            }
        })
        .collect();

    Ok(ReadingArticleDetail {
        article,
        segments,
        bookmark_count,
    })
}

pub async fn create_article(
    state: &AppState,
    title: &str,
    body: &str,
    folder_id: Option<i64>,
) -> Result<ReadingArticleDetail, ReadingError> {
    let id = create_article_once(state, title, body, folder_id, None).await?;
    article_detail(state, id).await
}

pub async fn create_article_once(
    state: &AppState,
    title: &str,
    body: &str,
    folder_id: Option<i64>,
    operation_key: Option<&str>,
) -> Result<i64, ReadingError> {
    let mut tx = state.pool.begin().await?;
    if let Some(key) = operation_key {
        crate::repositories::agent_batches::lock_operation(&mut tx, key).await?;
        if let Some(receipt) = crate::repositories::agent_batches::receipt(&mut tx, key).await? {
            return receipt["article_id"]
                .as_i64()
                .ok_or_else(|| ReadingError::Conflict("invalid stored article receipt".into()));
        }
    }
    let (title, body) = validate_article_input(title, body)?;
    validate_folder_target(state, folder_id).await?;
    let parsed = parse_reading_content(body).map_err(ReadingError::BookmarkValidation)?;
    let bookmarks = resolve_bookmarks(state, &parsed).await?;
    let article = reading::insert_article(&mut tx, title, body, folder_id).await?;
    reading::replace_bookmarks(&mut tx, article.id, &bookmarks).await?;
    if let Some(key) = operation_key {
        crate::repositories::agent_batches::save_receipt(
            &mut tx,
            key,
            &serde_json::json!({"article_id": article.id}),
        )
        .await?;
    }
    tx.commit().await?;
    Ok(article.id)
}

pub async fn save_article(
    state: &AppState,
    article_id: i64,
    title: &str,
    body: &str,
    folder_id: Option<i64>,
) -> Result<ReadingArticleDetail, ReadingError> {
    let (title, body) = validate_article_input(title, body)?;
    validate_folder_target(state, folder_id).await?;
    let parsed = parse_reading_content(body).map_err(ReadingError::BookmarkValidation)?;
    let bookmarks = resolve_bookmarks(state, &parsed).await?;
    let mut tx = state.pool.begin().await?;
    let article = reading::update_article(&mut tx, article_id, title, body, folder_id)
        .await?
        .ok_or_else(|| ReadingError::NotFound("文章不存在".to_string()))?;
    reading::replace_bookmarks(&mut tx, article.id, &bookmarks).await?;
    tx.commit().await?;
    article_detail(state, article.id).await
}

pub async fn create_folder(
    state: &AppState,
    name: &str,
    parent_id: Option<i64>,
) -> Result<ReadingFolder, ReadingError> {
    create_folder_once(state, name, parent_id, None).await
}

pub async fn create_folder_once(
    state: &AppState,
    name: &str,
    parent_id: Option<i64>,
    operation_key: Option<&str>,
) -> Result<ReadingFolder, ReadingError> {
    let mut tx = state.pool.begin().await?;
    if let Some(key) = operation_key {
        crate::repositories::agent_batches::lock_operation(&mut tx, key).await?;
        if let Some(receipt) = crate::repositories::agent_batches::receipt(&mut tx, key).await? {
            return serde_json::from_value(receipt)
                .map_err(|_| ReadingError::Conflict("invalid stored folder receipt".into()));
        }
    }
    let name = validate_title(name, "文件夹名称")?;
    validate_folder_target(state, parent_id).await?;
    ensure_unique_folder_name(state, parent_id, name, None).await?;
    let folder = reading::insert_folder(&mut tx, name, parent_id).await?;
    if let Some(key) = operation_key {
        crate::repositories::agent_batches::save_receipt(&mut tx, key, &serde_json::json!(&folder))
            .await?;
    }
    tx.commit().await?;
    Ok(folder)
}

pub async fn update_folder(
    state: &AppState,
    folder_id: i64,
    name: &str,
    parent_id: Option<i64>,
) -> Result<ReadingFolder, ReadingError> {
    let name = validate_title(name, "文件夹名称")?;
    if !reading::folder_exists(&state.pool, folder_id).await? {
        return Err(ReadingError::NotFound("文件夹不存在".to_string()));
    }
    validate_folder_target(state, parent_id).await?;
    if let Some(parent_id) = parent_id {
        if reading::would_create_folder_cycle(&state.pool, folder_id, parent_id).await? {
            return Err(ReadingError::Conflict(
                "文件夹不能移动到自身或子文件夹中".to_string(),
            ));
        }
    }
    ensure_unique_folder_name(state, parent_id, name, Some(folder_id)).await?;
    reading::update_folder(&state.pool, folder_id, name, parent_id)
        .await?
        .ok_or_else(|| ReadingError::NotFound("文件夹不存在".to_string()))
}

pub async fn delete_folder(state: &AppState, folder_id: i64) -> Result<(), ReadingError> {
    if reading::folder_has_content(&state.pool, folder_id).await? {
        return Err(ReadingError::Conflict("非空文件夹不能直接删除".to_string()));
    }
    if !reading::delete_folder(&state.pool, folder_id).await? {
        return Err(ReadingError::NotFound("文件夹不存在".to_string()));
    }
    Ok(())
}

pub async fn rename_article(
    state: &AppState,
    article_id: i64,
    title: &str,
) -> Result<(), ReadingError> {
    let title = validate_title(title, "文章标题")?;
    if !reading::update_article_title(&state.pool, article_id, title).await? {
        return Err(ReadingError::NotFound("文章不存在".to_string()));
    }
    Ok(())
}

pub async fn move_article(
    state: &AppState,
    article_id: i64,
    folder_id: Option<i64>,
) -> Result<(), ReadingError> {
    validate_folder_target(state, folder_id).await?;
    if !reading::update_article_folder(&state.pool, article_id, folder_id).await? {
        return Err(ReadingError::NotFound("文章不存在".to_string()));
    }
    Ok(())
}

pub async fn set_article_status(
    state: &AppState,
    article_id: i64,
    completed: bool,
) -> Result<(), ReadingError> {
    if !reading::update_article_status(&state.pool, article_id, completed).await? {
        return Err(ReadingError::NotFound("文章不存在".to_string()));
    }
    Ok(())
}

pub async fn mark_article_opened(state: &AppState, article_id: i64) -> Result<(), ReadingError> {
    if !reading::mark_article_opened(&state.pool, article_id).await? {
        return Err(ReadingError::NotFound("文章不存在".to_string()));
    }
    Ok(())
}

pub async fn delete_article(state: &AppState, article_id: i64) -> Result<(), ReadingError> {
    if !reading::delete_article(&state.pool, article_id).await? {
        return Err(ReadingError::NotFound("文章不存在".to_string()));
    }
    Ok(())
}

fn validate_article_input<'a>(
    title: &'a str,
    body: &'a str,
) -> Result<(&'a str, &'a str), ReadingError> {
    let title = validate_title(title, "文章标题")?;
    let body = body.trim();
    if body.is_empty() {
        return Err(ReadingError::BadRequest("文章正文不能为空".to_string()));
    }
    Ok((title, body))
}

fn validate_title<'a>(value: &'a str, label: &str) -> Result<&'a str, ReadingError> {
    let value = value.trim();
    if value.is_empty() {
        return Err(ReadingError::BadRequest(format!("{label}不能为空")));
    }
    Ok(value)
}

async fn validate_folder_target(
    state: &AppState,
    folder_id: Option<i64>,
) -> Result<(), ReadingError> {
    if let Some(folder_id) = folder_id {
        if !reading::folder_exists(&state.pool, folder_id).await? {
            return Err(ReadingError::BadRequest("目标文件夹不存在".to_string()));
        }
    }
    Ok(())
}

async fn ensure_unique_folder_name(
    state: &AppState,
    parent_id: Option<i64>,
    name: &str,
    except_id: Option<i64>,
) -> Result<(), ReadingError> {
    if reading::folder_name_exists(&state.pool, parent_id, name, except_id).await? {
        return Err(ReadingError::Conflict(
            "同一层级已经有同名文件夹".to_string(),
        ));
    }
    Ok(())
}

async fn resolve_bookmarks(
    state: &AppState,
    parsed: &ParsedReadingContent,
) -> Result<Vec<(i32, String, String, i64, i64)>, ReadingError> {
    let mut resolved = HashMap::<String, Result<(i64, i64), String>>::new();
    let mut bookmarks = Vec::with_capacity(parsed.bookmarks.len());
    let mut issues = Vec::new();

    for bookmark in &parsed.bookmarks {
        if !resolved.contains_key(&bookmark.form_of) {
            let candidates =
                reading::find_bookmark_candidates(&state.pool, &bookmark.form_of).await?;
            resolved.insert(
                bookmark.form_of.clone(),
                resolve_candidate(&bookmark.form_of, candidates),
            );
        }

        match resolved
            .get(&bookmark.form_of)
            .expect("resolved target exists")
        {
            Ok((lexeme_id, entry_id)) => bookmarks.push((
                bookmark.occurrence_index,
                bookmark.surface.clone(),
                bookmark.form_of.clone(),
                *lexeme_id,
                *entry_id,
            )),
            Err(reason) => issues.push(BookmarkValidationIssue {
                occurrence_index: bookmark.occurrence_index,
                surface: bookmark.surface.clone(),
                form_of: bookmark.form_of.clone(),
                reason: reason.clone(),
            }),
        }
    }

    if issues.is_empty() {
        Ok(bookmarks)
    } else {
        Err(ReadingError::BookmarkValidation(issues))
    }
}

fn resolve_candidate(
    form_of: &str,
    candidates: Vec<BookmarkTargetCandidate>,
) -> Result<(i64, i64), String> {
    let exact = candidates
        .iter()
        .filter(|candidate| candidate.surface == form_of)
        .collect::<Vec<_>>();
    let scoped = if exact.is_empty() {
        candidates.iter().collect::<Vec<_>>()
    } else {
        exact
    };
    if scoped.is_empty() {
        return Err("词典中没有这个原型".to_string());
    }

    let ready = scoped
        .into_iter()
        .filter_map(|candidate| {
            candidate
                .entry_id
                .map(|entry_id| (candidate.lexeme_id, entry_id))
        })
        .collect::<Vec<_>>();
    match ready.as_slice() {
        [target] => Ok(*target),
        [] => Err("词条尚未初始化，或缺少完整结构化释义".to_string()),
        _ => Err("该原型命中多个已初始化词条，请使用更明确的原型".to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::resolve_candidate;
    use crate::repositories::reading::BookmarkTargetCandidate;

    #[test]
    fn exact_case_candidate_wins_over_case_insensitive_homograph() {
        let result = resolve_candidate(
            "übersetzen",
            vec![
                BookmarkTargetCandidate {
                    lexeme_id: 1,
                    surface: "übersetzen".to_string(),
                    entry_id: Some(9),
                },
                BookmarkTargetCandidate {
                    lexeme_id: 2,
                    surface: "Übersetzen".to_string(),
                    entry_id: Some(10),
                },
            ],
        );
        assert_eq!(result.unwrap(), (1, 9));
    }
}
