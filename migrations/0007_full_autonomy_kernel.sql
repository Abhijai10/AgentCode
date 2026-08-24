CREATE TABLE IF NOT EXISTS mission_contract_revisions (
    id TEXT PRIMARY KEY,
    mission_id TEXT NOT NULL,
    revision INTEGER NOT NULL,
    original_goal TEXT NOT NULL,
    reason TEXT NOT NULL,
    created_at_ms INTEGER NOT NULL,
    UNIQUE(mission_id, revision)
);

CREATE TABLE IF NOT EXISTS requirement_matrix_entries (
    id TEXT PRIMARY KEY,
    mission_id TEXT NOT NULL,
    contract_revision INTEGER NOT NULL,
    description TEXT NOT NULL,
    requirement_type TEXT NOT NULL,
    priority INTEGER NOT NULL,
    source TEXT NOT NULL,
    verification_strategy TEXT NOT NULL,
    blocking INTEGER NOT NULL,
    implementation_status TEXT NOT NULL,
    verification_status TEXT NOT NULL,
    evidence_refs TEXT NOT NULL,
    linked_task_ids TEXT NOT NULL,
    created_at_ms INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_requirement_matrix_mission
    ON requirement_matrix_entries(mission_id, contract_revision);

CREATE TABLE IF NOT EXISTS task_leases (
    task_id TEXT PRIMARY KEY,
    worker_id TEXT NOT NULL,
    lease_epoch INTEGER NOT NULL,
    expires_at_ms INTEGER NOT NULL,
    heartbeat_interval_ms INTEGER NOT NULL,
    state TEXT NOT NULL,
    updated_at_ms INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_task_leases_expiry
    ON task_leases(expires_at_ms, state);

CREATE TABLE IF NOT EXISTS autonomy_mailbox_messages (
    id TEXT PRIMARY KEY,
    mission_id TEXT NOT NULL,
    sender_worker_id TEXT NOT NULL,
    recipient_worker_id TEXT,
    message_type TEXT NOT NULL,
    subject_id TEXT,
    payload TEXT NOT NULL,
    delivered INTEGER NOT NULL,
    created_at_ms INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_autonomy_mailbox_mission
    ON autonomy_mailbox_messages(mission_id, delivered, created_at_ms);

CREATE TABLE IF NOT EXISTS autonomy_records (
    id TEXT PRIMARY KEY,
    mission_id TEXT NOT NULL,
    category TEXT NOT NULL,
    subject_id TEXT,
    payload TEXT NOT NULL,
    created_at_ms INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_autonomy_records_mission_category
    ON autonomy_records(mission_id, category, created_at_ms);
