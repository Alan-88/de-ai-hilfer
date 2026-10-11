ALTER TABLE learning_review_logs
    ADD COLUMN IF NOT EXISTS progress_before JSONB,
    ADD COLUMN IF NOT EXISTS progress_after JSONB;

CREATE TABLE IF NOT EXISTS learning_review_corrections (
    id BIGSERIAL PRIMARY KEY,
    operation_id TEXT NOT NULL,
    entry_id BIGINT NOT NULL REFERENCES knowledge_entries(id) ON DELETE CASCADE,
    business_date DATE NOT NULL,
    target_log_id BIGINT REFERENCES learning_review_logs(id) ON DELETE CASCADE,
    action TEXT NOT NULL,
    replacement_rating TEXT,
    progress_before JSONB,
    progress_after JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_learning_review_corrections_entry_date
    ON learning_review_corrections(entry_id, business_date);

CREATE INDEX IF NOT EXISTS idx_learning_review_corrections_target_log
    ON learning_review_corrections(target_log_id);
