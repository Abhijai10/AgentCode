CREATE TABLE IF NOT EXISTS lsp_server_sessions (
    id TEXT PRIMARY KEY,
    repository_id TEXT NOT NULL,
    server_type TEXT NOT NULL,
    workspace_root TEXT NOT NULL,
    pid INTEGER,
    state TEXT NOT NULL,
    restart_count INTEGER NOT NULL,
    last_activity_ms INTEGER NOT NULL,
    degraded_reason TEXT
);

CREATE INDEX IF NOT EXISTS idx_lsp_server_sessions_repo
    ON lsp_server_sessions(repository_id, server_type, workspace_root);

CREATE TABLE IF NOT EXISTS semantic_edges (
    id TEXT PRIMARY KEY,
    repository_id TEXT NOT NULL,
    edge_kind TEXT NOT NULL,
    from_path TEXT NOT NULL,
    from_symbol TEXT NOT NULL,
    to_path TEXT NOT NULL,
    to_symbol TEXT NOT NULL,
    provenance_source TEXT NOT NULL,
    confidence INTEGER NOT NULL,
    freshness TEXT NOT NULL,
    commit_ref TEXT NOT NULL,
    worktree_id TEXT NOT NULL,
    created_at_ms INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_semantic_edges_repo_kind
    ON semantic_edges(repository_id, edge_kind);
CREATE INDEX IF NOT EXISTS idx_semantic_edges_from
    ON semantic_edges(repository_id, from_path, from_symbol);
CREATE INDEX IF NOT EXISTS idx_semantic_edges_to
    ON semantic_edges(repository_id, to_path, to_symbol);

CREATE TABLE IF NOT EXISTS semantic_diagnostics (
    id TEXT PRIMARY KEY,
    repository_id TEXT NOT NULL,
    path TEXT NOT NULL,
    line INTEGER NOT NULL,
    severity TEXT NOT NULL,
    message TEXT NOT NULL,
    provenance_source TEXT NOT NULL,
    confidence INTEGER NOT NULL,
    freshness TEXT NOT NULL,
    commit_ref TEXT NOT NULL,
    worktree_id TEXT NOT NULL,
    created_at_ms INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_semantic_diagnostics_repo_path
    ON semantic_diagnostics(repository_id, path);

CREATE TABLE IF NOT EXISTS workspace_boundaries (
    repository_id TEXT NOT NULL,
    kind TEXT NOT NULL,
    root_path TEXT NOT NULL,
    package_name TEXT,
    evidence_path TEXT NOT NULL,
    PRIMARY KEY(repository_id, kind, root_path, evidence_path)
);

CREATE TABLE IF NOT EXISTS optional_index_decisions (
    repository_id TEXT NOT NULL,
    engine TEXT NOT NULL,
    enabled INTEGER NOT NULL,
    reason TEXT NOT NULL,
    measured_files INTEGER NOT NULL,
    measured_edges INTEGER NOT NULL,
    decided_at_ms INTEGER NOT NULL,
    PRIMARY KEY(repository_id, engine)
);
