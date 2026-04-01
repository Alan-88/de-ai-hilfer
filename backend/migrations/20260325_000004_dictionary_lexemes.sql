CREATE TABLE dictionary_raw_entries (
    id BIGSERIAL PRIMARY KEY,
    source_key TEXT NOT NULL UNIQUE,
    headword TEXT NOT NULL,
    normalized_headword TEXT NOT NULL,
    lang_code TEXT NOT NULL,
    pos TEXT,
    is_form_of BOOLEAN NOT NULL DEFAULT FALSE,
    form_of_words TEXT[] NOT NULL DEFAULT '{}',
    raw_data JSONB NOT NULL,
    has_audio BOOLEAN DEFAULT FALSE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_dictionary_raw_entries_headword
    ON dictionary_raw_entries(headword);

CREATE INDEX idx_dictionary_raw_entries_normalized_headword
    ON dictionary_raw_entries(normalized_headword);

CREATE INDEX idx_dictionary_raw_entries_form_of_words
    ON dictionary_raw_entries USING GIN(form_of_words);

CREATE INDEX idx_dictionary_raw_entries_raw_data
    ON dictionary_raw_entries USING GIN(raw_data);

CREATE TABLE dictionary_lexemes (
    id BIGSERIAL PRIMARY KEY,
    bundle_key TEXT NOT NULL UNIQUE,
    surface TEXT NOT NULL,
    normalized_surface TEXT NOT NULL,
    gloss_preview JSONB NOT NULL DEFAULT '[]'::jsonb,
    pos_summary TEXT[] NOT NULL DEFAULT '{}',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_dictionary_lexemes_surface
    ON dictionary_lexemes(surface);

CREATE INDEX idx_dictionary_lexemes_normalized_surface
    ON dictionary_lexemes(normalized_surface);

CREATE TRIGGER update_dictionary_lexemes_updated_at
    BEFORE UPDATE ON dictionary_lexemes
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TABLE dictionary_lexeme_raw_entries (
    lexeme_id BIGINT NOT NULL REFERENCES dictionary_lexemes(id) ON DELETE CASCADE,
    raw_entry_id BIGINT NOT NULL REFERENCES dictionary_raw_entries(id) ON DELETE CASCADE,
    PRIMARY KEY (lexeme_id, raw_entry_id)
);

CREATE INDEX idx_dictionary_lexeme_raw_entries_raw_entry_id
    ON dictionary_lexeme_raw_entries(raw_entry_id);

CREATE TABLE dictionary_surface_forms (
    id BIGSERIAL PRIMARY KEY,
    surface TEXT NOT NULL,
    normalized_surface TEXT NOT NULL,
    lexeme_id BIGINT NOT NULL REFERENCES dictionary_lexemes(id) ON DELETE CASCADE,
    source TEXT NOT NULL,
    raw_entry_id BIGINT REFERENCES dictionary_raw_entries(id) ON DELETE SET NULL,
    confidence REAL NOT NULL DEFAULT 1.0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE UNIQUE INDEX uq_dictionary_surface_forms_link
    ON dictionary_surface_forms(surface, lexeme_id, source, raw_entry_id);

CREATE INDEX idx_dictionary_surface_forms_lookup
    ON dictionary_surface_forms(normalized_surface, surface);

CREATE INDEX idx_dictionary_surface_forms_lexeme
    ON dictionary_surface_forms(lexeme_id);

CREATE TABLE dictionary_lexeme_embeddings (
    lexeme_id BIGINT NOT NULL REFERENCES dictionary_lexemes(id) ON DELETE CASCADE,
    model_id TEXT NOT NULL,
    source_text TEXT NOT NULL,
    dimensions INTEGER NOT NULL,
    embedding VECTOR NOT NULL,
    frequency_rank INTEGER,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (lexeme_id, model_id)
);

CREATE INDEX idx_dictionary_lexeme_embeddings_model_id
    ON dictionary_lexeme_embeddings(model_id);

CREATE TRIGGER update_dictionary_lexeme_embeddings_updated_at
    BEFORE UPDATE ON dictionary_lexeme_embeddings
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();
