CREATE TABLE reading_folders (
    id BIGSERIAL PRIMARY KEY,
    parent_id BIGINT REFERENCES reading_folders(id) ON DELETE RESTRICT DEFERRABLE INITIALLY DEFERRED,
    name TEXT NOT NULL CHECK (btrim(name) <> ''),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE UNIQUE INDEX uq_reading_folders_parent_name
    ON reading_folders(parent_id, lower(name)) NULLS NOT DISTINCT;

CREATE TRIGGER update_reading_folders_updated_at
    BEFORE UPDATE ON reading_folders
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TABLE reading_articles (
    id BIGSERIAL PRIMARY KEY,
    folder_id BIGINT REFERENCES reading_folders(id) ON DELETE RESTRICT,
    title TEXT NOT NULL CHECK (btrim(title) <> ''),
    body TEXT NOT NULL CHECK (btrim(body) <> ''),
    completed BOOLEAN NOT NULL DEFAULT FALSE,
    last_opened_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_reading_articles_folder ON reading_articles(folder_id);
CREATE INDEX idx_reading_articles_status ON reading_articles(completed, updated_at DESC);

CREATE TRIGGER update_reading_articles_updated_at
    BEFORE UPDATE ON reading_articles
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

-- 正文中的 [词形](原型) 是事实来源；此表只是可随保存重建的查询索引。
-- lexeme_id / entry_id 故意不设外键：删除词条后正文书签仍应保留并显示失效状态。
CREATE TABLE reading_article_bookmarks (
    id BIGSERIAL PRIMARY KEY,
    article_id BIGINT NOT NULL REFERENCES reading_articles(id) ON DELETE CASCADE,
    occurrence_index INTEGER NOT NULL CHECK (occurrence_index >= 0),
    surface TEXT NOT NULL CHECK (btrim(surface) <> ''),
    form_of TEXT NOT NULL CHECK (btrim(form_of) <> ''),
    lexeme_id BIGINT,
    entry_id BIGINT,
    UNIQUE(article_id, occurrence_index)
);

CREATE INDEX idx_reading_bookmarks_article ON reading_article_bookmarks(article_id);
CREATE INDEX idx_reading_bookmarks_lexeme ON reading_article_bookmarks(lexeme_id);
CREATE INDEX idx_reading_bookmarks_entry ON reading_article_bookmarks(entry_id);
