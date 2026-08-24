CREATE TABLE IF NOT EXISTS verification_profiles (
    id TEXT PRIMARY KEY,
    task_id TEXT NOT NULL,
    risk TEXT NOT NULL,
    required_layers TEXT NOT NULL,
    created_at_ms INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS verification_runs (
    id TEXT PRIMARY KEY,
    profile_id TEXT,
    task_id TEXT,
    commit_ref TEXT NOT NULL,
    worktree_id TEXT NOT NULL,
    environment TEXT NOT NULL,
    command TEXT NOT NULL,
    tool_version TEXT NOT NULL,
    normalized_result TEXT NOT NULL,
    raw_artifact TEXT NOT NULL,
    evidence_ref TEXT NOT NULL,
    freshness_dependencies TEXT NOT NULL,
    created_at_ms INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_verification_runs_commit
    ON verification_runs(commit_ref, worktree_id, normalized_result);

CREATE TABLE IF NOT EXISTS requirement_verifications (
    id TEXT PRIMARY KEY,
    requirement_id TEXT NOT NULL,
    verification_run_id TEXT NOT NULL,
    evidence_ref TEXT NOT NULL,
    evidence_kind TEXT NOT NULL,
    verified INTEGER NOT NULL,
    freshness_key TEXT NOT NULL,
    created_at_ms INTEGER NOT NULL,
    FOREIGN KEY(verification_run_id) REFERENCES verification_runs(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_requirement_verifications_requirement
    ON requirement_verifications(requirement_id, verified);

CREATE TABLE IF NOT EXISTS verification_findings (
    id TEXT PRIMARY KEY,
    verification_run_id TEXT,
    audit_id TEXT,
    code TEXT NOT NULL,
    severity TEXT NOT NULL,
    message TEXT NOT NULL,
    created_at_ms INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS final_audits (
    id TEXT PRIMARY KEY,
    mission_id TEXT NOT NULL,
    original_goal TEXT NOT NULL,
    requirements TEXT NOT NULL,
    evidence_refs TEXT NOT NULL,
    passed INTEGER NOT NULL,
    return_to_repair INTEGER NOT NULL,
    completion_allowed INTEGER NOT NULL,
    created_at_ms INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_final_audits_mission
    ON final_audits(mission_id, created_at_ms);
