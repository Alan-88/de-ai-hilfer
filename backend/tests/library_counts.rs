use de_ai_hilfer::{models::LibraryQueryTab, repositories::knowledge};
use sqlx::postgres::PgPoolOptions;

#[tokio::test]
#[ignore = "requires TEST_DATABASE_URL pointing to a disposable PostgreSQL database"]
async fn counts_match_categories_search_and_pagination() {
    let pool = PgPoolOptions::new()
        .max_connections(1)
        .connect(&std::env::var("TEST_DATABASE_URL").expect("TEST_DATABASE_URL"))
        .await
        .unwrap();
    // Temporary tables shadow application tables and disappear with this connection.
    sqlx::raw_sql(
        r#"
        CREATE TEMP TABLE knowledge_entries (
            id bigint PRIMARY KEY, query_text text NOT NULL, lexeme_id bigint,
            prototype text, entry_type text NOT NULL DEFAULT 'word',
            analysis jsonb NOT NULL DEFAULT '{}', tags text[], aliases text[],
            created_at timestamptz NOT NULL DEFAULT NOW(),
            updated_at timestamptz NOT NULL DEFAULT NOW()
        );
        CREATE TEMP TABLE learning_progress (
            entry_id bigint PRIMARY KEY, due_date timestamptz NOT NULL
        );
        INSERT INTO knowledge_entries (id, query_text) VALUES
            (1, 'Apfel'), (2, 'Äpfel'), (3, 'Banane'),
            (4, 'Birne'), (5, 'Biene'), (6, 'Zitrone');
        INSERT INTO learning_progress (entry_id, due_date) VALUES
            (1, NOW() - INTERVAL '1 day'), (2, NOW() + INTERVAL '1 day'),
            (3, NOW() - INTERVAL '1 minute'), (4, NOW() + INTERVAL '1 hour');
        "#,
    )
    .execute(&pool)
    .await
    .unwrap();

    for (tab, ids) in [
        (LibraryQueryTab::All, vec![1, 2, 3, 4, 5, 6]),
        (LibraryQueryTab::Learning, vec![1, 2, 3, 4]),
        (LibraryQueryTab::Review, vec![1, 3]),
        (LibraryQueryTab::New, vec![5, 6]),
    ] {
        let page = knowledge::list_page(&pool, "", tab, 10, 0).await.unwrap();
        assert_eq!(
            serde_json::to_value(&page.counts).unwrap(),
            serde_json::json!({"all":6,"learning":4,"review":2,"new":2})
        );
        let mut actual = page.entries.iter().map(|e| e.id).collect::<Vec<_>>();
        actual.sort_unstable();
        assert_eq!(actual, ids);
        assert_eq!(page.total, ids.len() as i64);
        assert_eq!(page.next_offset, None);
    }

    for (q, expected) in [
        ("apfel", [2, 2, 1, 0]),
        (" bi ", [2, 1, 0, 1]),
        ("nonexistent", [0, 0, 0, 0]),
        ("' OR TRUE --", [0, 0, 0, 0]),
    ] {
        let page = knowledge::list_page(&pool, q, LibraryQueryTab::All, 10, 0)
            .await
            .unwrap();
        let c = page.counts;
        assert_eq!([c.all, c.learning, c.review, c.new], expected, "{q}");
        assert_eq!(page.total, page.entries.len() as i64);
    }

    let first = knowledge::list_page(&pool, "", LibraryQueryTab::All, 1, 0)
        .await
        .unwrap();
    let next = knowledge::list_page(&pool, "", LibraryQueryTab::All, 1, 1)
        .await
        .unwrap();
    assert_eq!(first.total, 6);
    assert_eq!(first.next_offset, Some(1));
    assert_ne!(first.entries[0].id, next.entries[0].id);
    assert_eq!(
        serde_json::to_value(first.counts).unwrap(),
        serde_json::to_value(next.counts).unwrap()
    );
    pool.close().await;
}
