CREATE TABLE dictionary_learning_order (
    headword TEXT PRIMARY KEY REFERENCES dictionary_raw(headword) ON DELETE CASCADE,
    cefr_level TEXT,
    cefr_rank INTEGER,
    frequency_rank INTEGER,
    learning_order INTEGER,
    source TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_dictionary_learning_order_cefr_rank
    ON dictionary_learning_order(cefr_rank ASC NULLS LAST, learning_order ASC NULLS LAST, headword ASC);

CREATE INDEX idx_dictionary_learning_order_learning_order
    ON dictionary_learning_order(learning_order ASC NULLS LAST, headword ASC);

CREATE INDEX idx_dictionary_learning_order_source
    ON dictionary_learning_order(source);

CREATE TRIGGER update_dictionary_learning_order_updated_at
    BEFORE UPDATE ON dictionary_learning_order
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();
