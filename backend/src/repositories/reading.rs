use crate::db::DbPool;
use crate::reading_models::{ReadingArticle, ReadingArticleBookmark, ReadingFolder};
use chrono::{DateTime, Utc};
use sqlx::{FromRow, PgConnection};

#[derive(Debug, Clone, FromRow)]
pub struct ArticleSummaryRow {
    pub id: i64,
    pub folder_id: Option<i64>,
    pub title: String,
    pub completed: bool,
    pub bookmark_count: i64,
    pub last_opened_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow)]
pub struct ArticleMemoryRow {
    pub article_id: i64,
    pub stability: Option<f32>,
    pub difficulty: Option<f32>,
    pub last_review_at: Option<DateTime<Utc>>,
    pub review_count: Option<i32>,
}

#[derive(Debug, Clone, FromRow)]
pub struct BookmarkTargetCandidate {
    pub lexeme_id: i64,
    pub surface: String,
    pub entry_id: Option<i64>,
}

pub async fn list_folders(pool: &DbPool) -> Result<Vec<ReadingFolder>, sqlx::Error> {
    sqlx::query_as(
        "SELECT id, parent_id, name, created_at, updated_at FROM reading_folders ORDER BY id",
    )
    .fetch_all(pool)
    .await
}

pub async fn list_articles(pool: &DbPool) -> Result<Vec<ArticleSummaryRow>, sqlx::Error> {
    sqlx::query_as(
        r#"
        SELECT
            a.id, a.folder_id, a.title, a.completed,
            count(b.id) AS bookmark_count,
            a.last_opened_at, a.created_at, a.updated_at
        FROM reading_articles a
        LEFT JOIN reading_article_bookmarks b ON b.article_id = a.id
        GROUP BY a.id
        ORDER BY COALESCE(a.last_opened_at, a.updated_at) DESC, a.id DESC
        "#,
    )
    .fetch_all(pool)
    .await
}

pub async fn list_article_memory(pool: &DbPool) -> Result<Vec<ArticleMemoryRow>, sqlx::Error> {
    sqlx::query_as(
        r#"
        SELECT DISTINCT ON (b.article_id, COALESCE(b.lexeme_id, -b.id))
            b.article_id,
            lp.stability,
            lp.difficulty,
            lp.last_review_at,
            lp.review_count
        FROM reading_article_bookmarks b
        LEFT JOIN knowledge_entries ke ON ke.id = b.entry_id
        LEFT JOIN learning_progress lp ON lp.entry_id = ke.id
        ORDER BY b.article_id, COALESCE(b.lexeme_id, -b.id), b.occurrence_index
        "#,
    )
    .fetch_all(pool)
    .await
}

pub async fn find_article(
    pool: &DbPool,
    article_id: i64,
) -> Result<Option<ReadingArticle>, sqlx::Error> {
    sqlx::query_as(
        r#"
        SELECT id, folder_id, title, body, completed, last_opened_at, created_at, updated_at
        FROM reading_articles
        WHERE id = $1
        "#,
    )
    .bind(article_id)
    .fetch_optional(pool)
    .await
}

pub async fn list_bookmarks(
    pool: &DbPool,
    article_id: i64,
) -> Result<Vec<ReadingArticleBookmark>, sqlx::Error> {
    sqlx::query_as(
        r#"
        SELECT
            b.id, b.article_id, b.occurrence_index, b.surface, b.form_of,
            CASE WHEN dl.id IS NULL THEN NULL ELSE b.lexeme_id END AS lexeme_id,
            CASE WHEN ke.id IS NULL THEN NULL ELSE b.entry_id END AS entry_id
        FROM reading_article_bookmarks b
        LEFT JOIN dictionary_lexemes dl ON dl.id = b.lexeme_id
        LEFT JOIN knowledge_entries ke ON ke.id = b.entry_id
        WHERE b.article_id = $1
        ORDER BY b.occurrence_index
        "#,
    )
    .bind(article_id)
    .fetch_all(pool)
    .await
}

pub async fn find_bookmark_candidates(
    pool: &DbPool,
    form_of: &str,
) -> Result<Vec<BookmarkTargetCandidate>, sqlx::Error> {
    sqlx::query_as(
        r#"
        SELECT
            dl.id AS lexeme_id,
            dl.surface,
            (
                SELECT ke.id
                FROM knowledge_entries ke
                WHERE ke.lexeme_id = dl.id
                  AND ke.entry_type <> 'PHRASE'
                  AND jsonb_typeof(ke.analysis->'structured') = 'object'
                  AND (
                    jsonb_array_length(COALESCE(ke.analysis #> '{structured,grammar_branches}', '[]'::jsonb)) > 0
                    OR jsonb_array_length(COALESCE(ke.analysis #> '{structured,meanings}', '[]'::jsonb)) > 0
                  )
                ORDER BY ke.updated_at DESC, ke.id DESC
                LIMIT 1
            ) AS entry_id
        FROM dictionary_lexemes dl
        WHERE dl.surface = $1 OR lower(dl.surface) = lower($1)
        ORDER BY CASE WHEN dl.surface = $1 THEN 0 ELSE 1 END, dl.id
        "#,
    )
    .bind(form_of)
    .fetch_all(pool)
    .await
}

pub async fn folder_exists(pool: &DbPool, folder_id: i64) -> Result<bool, sqlx::Error> {
    sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM reading_folders WHERE id = $1)")
        .bind(folder_id)
        .fetch_one(pool)
        .await
}

pub async fn folder_name_exists(
    pool: &DbPool,
    parent_id: Option<i64>,
    name: &str,
    except_id: Option<i64>,
) -> Result<bool, sqlx::Error> {
    sqlx::query_scalar(
        r#"
        SELECT EXISTS(
            SELECT 1 FROM reading_folders
            WHERE parent_id IS NOT DISTINCT FROM $1
              AND lower(name) = lower($2)
              AND ($3::bigint IS NULL OR id <> $3)
        )
        "#,
    )
    .bind(parent_id)
    .bind(name)
    .bind(except_id)
    .fetch_one(pool)
    .await
}

pub async fn would_create_folder_cycle(
    pool: &DbPool,
    folder_id: i64,
    parent_id: i64,
) -> Result<bool, sqlx::Error> {
    sqlx::query_scalar(
        r#"
        WITH RECURSIVE descendants AS (
            SELECT id FROM reading_folders WHERE id = $1
            UNION ALL
            SELECT f.id
            FROM reading_folders f
            JOIN descendants d ON f.parent_id = d.id
        )
        SELECT EXISTS(SELECT 1 FROM descendants WHERE id = $2)
        "#,
    )
    .bind(folder_id)
    .bind(parent_id)
    .fetch_one(pool)
    .await
}

pub async fn folder_has_content(pool: &DbPool, folder_id: i64) -> Result<bool, sqlx::Error> {
    sqlx::query_scalar(
        r#"
        SELECT EXISTS(
            SELECT 1 FROM reading_folders WHERE parent_id = $1
            UNION ALL
            SELECT 1 FROM reading_articles WHERE folder_id = $1
        )
        "#,
    )
    .bind(folder_id)
    .fetch_one(pool)
    .await
}

pub async fn insert_folder(
    pool: &DbPool,
    name: &str,
    parent_id: Option<i64>,
) -> Result<ReadingFolder, sqlx::Error> {
    sqlx::query_as(
        r#"
        INSERT INTO reading_folders (name, parent_id)
        VALUES ($1, $2)
        RETURNING id, parent_id, name, created_at, updated_at
        "#,
    )
    .bind(name)
    .bind(parent_id)
    .fetch_one(pool)
    .await
}

pub async fn update_folder(
    pool: &DbPool,
    folder_id: i64,
    name: &str,
    parent_id: Option<i64>,
) -> Result<Option<ReadingFolder>, sqlx::Error> {
    sqlx::query_as(
        r#"
        UPDATE reading_folders SET name = $2, parent_id = $3
        WHERE id = $1
        RETURNING id, parent_id, name, created_at, updated_at
        "#,
    )
    .bind(folder_id)
    .bind(name)
    .bind(parent_id)
    .fetch_optional(pool)
    .await
}

pub async fn delete_folder(pool: &DbPool, folder_id: i64) -> Result<bool, sqlx::Error> {
    Ok(sqlx::query("DELETE FROM reading_folders WHERE id = $1")
        .bind(folder_id)
        .execute(pool)
        .await?
        .rows_affected()
        > 0)
}

pub async fn insert_article(
    connection: &mut PgConnection,
    title: &str,
    body: &str,
    folder_id: Option<i64>,
) -> Result<ReadingArticle, sqlx::Error> {
    sqlx::query_as(
        r#"
        INSERT INTO reading_articles (title, body, folder_id)
        VALUES ($1, $2, $3)
        RETURNING id, folder_id, title, body, completed, last_opened_at, created_at, updated_at
        "#,
    )
    .bind(title)
    .bind(body)
    .bind(folder_id)
    .fetch_one(connection)
    .await
}

pub async fn update_article(
    connection: &mut PgConnection,
    article_id: i64,
    title: &str,
    body: &str,
    folder_id: Option<i64>,
) -> Result<Option<ReadingArticle>, sqlx::Error> {
    sqlx::query_as(
        r#"
        UPDATE reading_articles SET title = $2, body = $3, folder_id = $4
        WHERE id = $1
        RETURNING id, folder_id, title, body, completed, last_opened_at, created_at, updated_at
        "#,
    )
    .bind(article_id)
    .bind(title)
    .bind(body)
    .bind(folder_id)
    .fetch_optional(connection)
    .await
}

pub async fn replace_bookmarks(
    connection: &mut PgConnection,
    article_id: i64,
    bookmarks: &[(i32, String, String, i64, i64)],
) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM reading_article_bookmarks WHERE article_id = $1")
        .bind(article_id)
        .execute(&mut *connection)
        .await?;

    for (occurrence, surface, form_of, lexeme_id, entry_id) in bookmarks {
        sqlx::query(
            r#"
            INSERT INTO reading_article_bookmarks (
                article_id, occurrence_index, surface, form_of, lexeme_id, entry_id
            ) VALUES ($1, $2, $3, $4, $5, $6)
            "#,
        )
        .bind(article_id)
        .bind(occurrence)
        .bind(surface)
        .bind(form_of)
        .bind(lexeme_id)
        .bind(entry_id)
        .execute(&mut *connection)
        .await?;
    }
    Ok(())
}

pub async fn update_article_title(
    pool: &DbPool,
    article_id: i64,
    title: &str,
) -> Result<bool, sqlx::Error> {
    Ok(
        sqlx::query("UPDATE reading_articles SET title = $2 WHERE id = $1")
            .bind(article_id)
            .bind(title)
            .execute(pool)
            .await?
            .rows_affected()
            > 0,
    )
}

pub async fn update_article_folder(
    pool: &DbPool,
    article_id: i64,
    folder_id: Option<i64>,
) -> Result<bool, sqlx::Error> {
    Ok(
        sqlx::query("UPDATE reading_articles SET folder_id = $2 WHERE id = $1")
            .bind(article_id)
            .bind(folder_id)
            .execute(pool)
            .await?
            .rows_affected()
            > 0,
    )
}

pub async fn update_article_status(
    pool: &DbPool,
    article_id: i64,
    completed: bool,
) -> Result<bool, sqlx::Error> {
    Ok(
        sqlx::query("UPDATE reading_articles SET completed = $2 WHERE id = $1")
            .bind(article_id)
            .bind(completed)
            .execute(pool)
            .await?
            .rows_affected()
            > 0,
    )
}

pub async fn mark_article_opened(pool: &DbPool, article_id: i64) -> Result<bool, sqlx::Error> {
    Ok(
        sqlx::query("UPDATE reading_articles SET last_opened_at = NOW() WHERE id = $1")
            .bind(article_id)
            .execute(pool)
            .await?
            .rows_affected()
            > 0,
    )
}

pub async fn delete_article(pool: &DbPool, article_id: i64) -> Result<bool, sqlx::Error> {
    Ok(sqlx::query("DELETE FROM reading_articles WHERE id = $1")
        .bind(article_id)
        .execute(pool)
        .await?
        .rows_affected()
        > 0)
}

pub async fn list_articles_for_backup(pool: &DbPool) -> Result<Vec<ReadingArticle>, sqlx::Error> {
    sqlx::query_as(
        r#"
        SELECT id, folder_id, title, body, completed, last_opened_at, created_at, updated_at
        FROM reading_articles ORDER BY id
        "#,
    )
    .fetch_all(pool)
    .await
}

pub async fn list_bookmarks_for_backup(
    pool: &DbPool,
) -> Result<Vec<ReadingArticleBookmark>, sqlx::Error> {
    sqlx::query_as(
        r#"
        SELECT id, article_id, occurrence_index, surface, form_of, lexeme_id, entry_id
        FROM reading_article_bookmarks ORDER BY id
        "#,
    )
    .fetch_all(pool)
    .await
}
