CREATE TABLE IF NOT EXISTS memory_facts (
    id TEXT PRIMARY KEY,
    repository_id TEXT NOT NULL,
    mission_id TEXT,
    task_id TEXT,
    branch TEXT,
    statement TEXT NOT NULL,
    fact_type TEXT NOT NULL,
    source TEXT NOT NULL,
    confidence INTEGER NOT NULL,
    freshness TEXT NOT NULL,
    memory_class TEXT NOT NULL,
    observed_commit TEXT NOT NULL,
    conflict_set_id TEXT,
    valid_from_ms INTEGER NOT NULL,
    valid_until_ms INTEGER,
    superseded_by TEXT,
    last_validation_ms INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_memory_facts_repo_freshness
    ON memory_facts(repository_id, freshness);
CREATE INDEX IF NOT EXISTS idx_memory_facts_task
    ON memory_facts(task_id);

CREATE TABLE IF NOT EXISTS memory_fact_evidence (
    fact_id TEXT NOT NULL,
    evidence_ref TEXT NOT NULL,
    file_path TEXT,
    symbol TEXT,
    content_hash TEXT,
    PRIMARY KEY(fact_id, evidence_ref),
    FOREIGN KEY(fact_id) REFERENCES memory_facts(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_memory_fact_evidence_path
    ON memory_fact_evidence(file_path, symbol);

CREATE TABLE IF NOT EXISTS memory_decisions (
    id TEXT PRIMARY KEY,
    repository_id TEXT NOT NULL,
    mission_id TEXT,
    task_id TEXT,
    branch TEXT,
    decision TEXT NOT NULL,
    rationale TEXT NOT NULL,
    authority_refs TEXT NOT NULL,
    supersedes TEXT,
    created_at_ms INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_memory_decisions_repo
    ON memory_decisions(repository_id, created_at_ms);

CREATE TABLE IF NOT EXISTS task_memory (
    id TEXT PRIMARY KEY,
    task_id TEXT NOT NULL,
    summary TEXT NOT NULL,
    evidence_refs TEXT NOT NULL,
    created_at_ms INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_task_memory_task
    ON task_memory(task_id, created_at_ms);

CREATE TABLE IF NOT EXISTS context_snapshots (
    id TEXT PRIMARY KEY,
    repository_id TEXT NOT NULL,
    mission_id TEXT,
    task_id TEXT,
    branch TEXT,
    reason TEXT NOT NULL,
    content TEXT NOT NULL,
    source_fact_ids TEXT NOT NULL,
    decision_refs TEXT NOT NULL,
    created_at_ms INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_context_snapshots_repo
    ON context_snapshots(repository_id, created_at_ms);
