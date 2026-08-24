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
    operations_json TEXT NOT NULL DEFAULT '',
    metadata_json TEXT,
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
    current_commit TEXT NOT NULL DEFAULT '',
    status TEXT NOT NULL,
    created_at_ms INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS agent_sessions (
    id TEXT PRIMARY KEY,
    mission_id TEXT NOT NULL,
    state TEXT NOT NULL,
    updated_at_ms INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS agent_checkpoints (
    id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL,
    next_step INTEGER NOT NULL,
    state TEXT NOT NULL,
    created_at_ms INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS worktree_checkpoints (
    id TEXT PRIMARY KEY,
    worktree_id TEXT NOT NULL,
    commit_ref TEXT NOT NULL,
    reason TEXT NOT NULL,
    created_at_ms INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS provider_routing_decisions (
    id TEXT PRIMARY KEY,
    task_id TEXT NOT NULL,
    candidates_json TEXT NOT NULL,
    selected_json TEXT,
    rejected_json TEXT NOT NULL,
    fallback_reason TEXT,
    latency_ms INTEGER NOT NULL,
    input_tokens INTEGER NOT NULL,
    output_tokens INTEGER NOT NULL,
    estimated_cost_micros INTEGER NOT NULL,
    created_at_ms INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS tool_execution_records (
    id TEXT PRIMARY KEY,
    tool_call_id TEXT NOT NULL,
    tool_id TEXT NOT NULL,
    status TEXT NOT NULL,
    manifest_json TEXT NOT NULL,
    raw_output TEXT NOT NULL,
    evidence_ref TEXT NOT NULL,
    created_at_ms INTEGER NOT NULL
);
