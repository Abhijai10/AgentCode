CREATE TABLE IF NOT EXISTS browser_processes (
    id TEXT PRIMARY KEY,
    task_id TEXT NOT NULL,
    mode TEXT NOT NULL,
    state TEXT NOT NULL,
    profile_dir TEXT NOT NULL,
    created_at_ms INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS browser_sessions (
    id TEXT PRIMARY KEY,
    task_id TEXT NOT NULL,
    process_id TEXT NOT NULL,
    current_url TEXT,
    profile TEXT NOT NULL,
    storage_state_ref TEXT,
    sensitive INTEGER NOT NULL,
    stale_evidence_refs TEXT NOT NULL,
    created_at_ms INTEGER NOT NULL,
    updated_at_ms INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_browser_sessions_task
    ON browser_sessions(task_id, updated_at_ms);

CREATE TABLE IF NOT EXISTS browser_dev_servers (
    id TEXT PRIMARY KEY,
    task_id TEXT NOT NULL,
    command TEXT NOT NULL,
    port INTEGER NOT NULL,
    ready_url TEXT NOT NULL,
    process_alive INTEGER NOT NULL,
    http_ready INTEGER NOT NULL,
    route_loadable INTEGER NOT NULL,
    retained INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS browser_screenshots (
    id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL,
    task_id TEXT NOT NULL,
    commit_ref TEXT NOT NULL,
    viewport TEXT NOT NULL,
    url TEXT NOT NULL,
    artifact_uri TEXT NOT NULL,
    sensitive INTEGER NOT NULL,
    evidence_ref TEXT NOT NULL,
    captured_at_ms INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_browser_screenshots_session
    ON browser_screenshots(session_id, viewport);

CREATE TABLE IF NOT EXISTS browser_visual_qa (
    id TEXT PRIMARY KEY,
    screenshot_ref TEXT NOT NULL,
    passed INTEGER NOT NULL,
    findings TEXT NOT NULL,
    adapter TEXT NOT NULL,
    evidence_ref TEXT NOT NULL
);
