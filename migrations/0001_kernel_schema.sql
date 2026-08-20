CREATE TABLE IF NOT EXISTS kernel_events (
    id TEXT PRIMARY KEY,
    decision_kind TEXT NOT NULL,
    subject_id TEXT NOT NULL,
    evidence_refs TEXT NOT NULL,
    created_at_ms INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS missions (
    id TEXT PRIMARY KEY,
    original_goal TEXT NOT NULL,
    state TEXT NOT NULL,
    created_at_ms INTEGER NOT NULL,
    updated_at_ms INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS evidence_records (
    id TEXT PRIMARY KEY,
    kind TEXT NOT NULL,
    provenance_json TEXT NOT NULL,
    artifact_uri TEXT NOT NULL,
    content_hash TEXT NOT NULL,
    created_at_ms INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS changesets (
    id TEXT PRIMARY KEY,
    state TEXT NOT NULL,
    rollback_json TEXT,
    created_at_ms INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS worktrees (
    id TEXT PRIMARY KEY,
    repository_id TEXT NOT NULL,
    owner_mission_id TEXT NOT NULL,
    owner_worker_id TEXT NOT NULL,
    path TEXT NOT NULL,
    branch TEXT NOT NULL,
    base_commit TEXT NOT NULL,
    status TEXT NOT NULL,
    created_at_ms INTEGER NOT NULL
);
