CREATE TABLE IF NOT EXISTS semantic_chunks (
    id TEXT PRIMARY KEY,
    repository_id TEXT NOT NULL,
    fact_id TEXT NOT NULL,
    content TEXT NOT NULL,
    content_hash TEXT NOT NULL,
    model_id TEXT NOT NULL,
    dimension INTEGER NOT NULL,
    vector_json TEXT NOT NULL,
    freshness TEXT NOT NULL,
    created_at_ms INTEGER NOT NULL,
    FOREIGN KEY(fact_id) REFERENCES memory_facts(id) ON DELETE CASCADE,
    UNIQUE(repository_id, fact_id, content_hash, model_id)
);
CREATE INDEX IF NOT EXISTS idx_semantic_chunks_lookup
    ON semantic_chunks(repository_id, model_id, freshness, created_at_ms);
