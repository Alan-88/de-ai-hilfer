CREATE TABLE agent_batches (
    id TEXT PRIMARY KEY,
    idempotency_key TEXT NOT NULL UNIQUE,
    request JSONB NOT NULL,
    status TEXT NOT NULL DEFAULT 'queued',
    steps JSONB NOT NULL,
    error TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX agent_batches_pending ON agent_batches(created_at) WHERE status IN ('queued', 'running');

-- Commit a business mutation and its receipt in the same transaction.
CREATE TABLE agent_operation_receipts (
    operation_key TEXT PRIMARY KEY,
    result JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE analysis_checkpoints (
    cache_key TEXT PRIMARY KEY,
    stage TEXT NOT NULL,
    value JSONB NOT NULL,
    expires_at TIMESTAMPTZ NOT NULL
);
