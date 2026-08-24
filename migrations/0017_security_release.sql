CREATE TABLE IF NOT EXISTS dependency_audit_records (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    version TEXT NOT NULL,
    license TEXT NOT NULL,
    source TEXT NOT NULL,
    checksum TEXT NOT NULL,
    security_status TEXT NOT NULL,
    vulnerability_refs TEXT NOT NULL,
    release_blocking INTEGER NOT NULL,
    created_at_ms INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS hardening_reports (
    id TEXT PRIMARY KEY,
    report_type TEXT NOT NULL,
    findings TEXT NOT NULL,
    mitigations TEXT NOT NULL,
    unresolved_risks TEXT NOT NULL,
    accepted_limitations TEXT NOT NULL,
    release_blocked INTEGER NOT NULL,
    created_at_ms INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS release_artifacts (
    id TEXT PRIMARY KEY,
    version TEXT NOT NULL,
    platform TEXT NOT NULL,
    artifact_kind TEXT NOT NULL,
    build_hash TEXT NOT NULL,
    integrity_hash TEXT NOT NULL,
    source_commit TEXT NOT NULL,
    created_at_ms INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS release_builds (
    id TEXT PRIMARY KEY,
    version TEXT NOT NULL,
    commit_ref TEXT NOT NULL,
    build_profile TEXT NOT NULL,
    environment TEXT NOT NULL,
    reproducible INTEGER NOT NULL,
    created_at_ms INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS update_records (
    id TEXT PRIMARY KEY,
    current_version TEXT NOT NULL,
    available_version TEXT NOT NULL,
    decision TEXT NOT NULL,
    verified INTEGER NOT NULL,
    rollback_ref TEXT,
    recovery_action TEXT NOT NULL,
    created_at_ms INTEGER NOT NULL
);
