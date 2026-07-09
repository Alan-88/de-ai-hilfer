ALTER TABLE learning_progress
ADD COLUMN IF NOT EXISTS lapses INTEGER NOT NULL DEFAULT 0;

CREATE TABLE IF NOT EXISTS learning_review_logs (
    id BIGSERIAL PRIMARY KEY,
    session_id TEXT NOT NULL,
    entry_id BIGINT NOT NULL REFERENCES knowledge_entries(id) ON DELETE CASCADE,
    rating TEXT NOT NULL,
    phase TEXT NOT NULL,
    is_first_today BOOLEAN NOT NULL,
    appearance_count_today INTEGER NOT NULL,
    long_term_committed BOOLEAN NOT NULL DEFAULT FALSE,
    business_date DATE NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_learning_review_logs_entry_date
    ON learning_review_logs(entry_id, business_date);

CREATE INDEX IF NOT EXISTS idx_learning_review_logs_session
    ON learning_review_logs(session_id);
