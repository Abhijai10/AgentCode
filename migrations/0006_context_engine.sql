CREATE TABLE IF NOT EXISTS context_pack_manifests (
    id TEXT PRIMARY KEY,
    pack_id TEXT NOT NULL,
    task_id TEXT NOT NULL,
    role TEXT NOT NULL,
    profile TEXT NOT NULL,
    source_fragment_ids TEXT NOT NULL,
    omitted_fragment_ids TEXT NOT NULL,
    raw_evidence_refs TEXT NOT NULL,
    cache_keys TEXT NOT NULL,
    score_trace TEXT NOT NULL,
    total_input_tokens INTEGER NOT NULL,
    hard_ceiling INTEGER NOT NULL,
    created_at_ms INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_context_pack_manifests_task
    ON context_pack_manifests(task_id, created_at_ms);

CREATE TABLE IF NOT EXISTS context_compression_receipts (
    id TEXT PRIMARY KEY,
    raw_evidence_ref TEXT NOT NULL,
    command_class TEXT NOT NULL,
    compressor_id TEXT NOT NULL,
    raw_hash TEXT NOT NULL,
    compressed_output TEXT NOT NULL,
    raw_token_estimate INTEGER NOT NULL,
    compressed_token_estimate INTEGER NOT NULL,
    omitted_lines INTEGER NOT NULL,
    created_at_ms INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_context_compression_raw_ref
    ON context_compression_receipts(raw_evidence_ref);

CREATE TABLE IF NOT EXISTS context_cache_entries (
    cache_key TEXT PRIMARY KEY,
    content_hash TEXT NOT NULL,
    token_estimate INTEGER NOT NULL,
    source_ref TEXT NOT NULL,
    created_at_ms INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_context_cache_source
    ON context_cache_entries(source_ref);

CREATE TABLE IF NOT EXISTS context_retrieval_records (
    id TEXT PRIMARY KEY,
    pack_id TEXT NOT NULL,
    need TEXT NOT NULL,
    reason TEXT NOT NULL,
    query TEXT NOT NULL,
    result_fragment_ids TEXT NOT NULL,
    added_tokens INTEGER NOT NULL,
    degraded INTEGER NOT NULL,
    created_at_ms INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_context_retrieval_pack
    ON context_retrieval_records(pack_id, created_at_ms);

CREATE TABLE IF NOT EXISTS context_pack_metrics (
    id TEXT PRIMARY KEY,
    pack_id TEXT NOT NULL,
    role TEXT NOT NULL,
    selected_fragments INTEGER NOT NULL,
    omitted_fragments INTEGER NOT NULL,
    total_input_tokens INTEGER NOT NULL,
    budget_target INTEGER NOT NULL,
    hard_ceiling INTEGER NOT NULL,
    deduped_fragments INTEGER NOT NULL,
    redacted_fragments INTEGER NOT NULL,
    retrieval_steps INTEGER NOT NULL,
    cache_hits INTEGER NOT NULL,
    created_at_ms INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_context_pack_metrics_pack
    ON context_pack_metrics(pack_id);

CREATE TABLE IF NOT EXISTS context_benchmark_results (
    id TEXT PRIMARY KEY,
    task_name TEXT NOT NULL,
    broad_tokens INTEGER NOT NULL,
    targeted_tokens INTEGER NOT NULL,
    broad_success INTEGER NOT NULL,
    targeted_success INTEGER NOT NULL,
    retry_delta INTEGER NOT NULL,
    latency_delta_ms INTEGER NOT NULL,
    passed INTEGER NOT NULL,
    created_at_ms INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_context_benchmark_task
    ON context_benchmark_results(task_name, created_at_ms);
