CREATE TABLE ai_provider_profiles (
    id BIGSERIAL PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    base_url TEXT NOT NULL,
    api_key TEXT NOT NULL DEFAULT '',
    model_ids TEXT[] NOT NULL DEFAULT '{}',
    is_default BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE UNIQUE INDEX uq_ai_provider_profiles_default
    ON ai_provider_profiles(is_default)
    WHERE is_default;

CREATE TRIGGER update_ai_provider_profiles_updated_at
    BEFORE UPDATE ON ai_provider_profiles
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TABLE ai_task_model_settings (
    task_key TEXT PRIMARY KEY,
    provider_id BIGINT REFERENCES ai_provider_profiles(id) ON DELETE SET NULL,
    model_id TEXT,
    inherit_task_key TEXT,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT ck_ai_task_model_settings_mode
        CHECK (
            (inherit_task_key IS NULL AND provider_id IS NOT NULL AND model_id IS NOT NULL AND model_id <> '')
            OR
            (inherit_task_key IS NOT NULL AND provider_id IS NULL AND model_id IS NULL)
        )
);

CREATE TRIGGER update_ai_task_model_settings_updated_at
    BEFORE UPDATE ON ai_task_model_settings
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();
