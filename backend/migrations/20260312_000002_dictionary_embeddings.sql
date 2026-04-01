CREATE TABLE dictionary_embeddings (
    headword TEXT NOT NULL REFERENCES dictionary_raw(headword) ON DELETE CASCADE,
    model_id TEXT NOT NULL,
    source_text TEXT NOT NULL,
    dimensions INTEGER NOT NULL,
    embedding VECTOR NOT NULL,
    frequency_rank INTEGER,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (headword, model_id)
);

CREATE INDEX idx_dictionary_embeddings_model_id
    ON dictionary_embeddings(model_id);

CREATE TRIGGER update_dictionary_embeddings_updated_at
    BEFORE UPDATE ON dictionary_embeddings
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();
