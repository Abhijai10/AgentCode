CREATE TABLE IF NOT EXISTS release_candidates (
    id TEXT PRIMARY KEY,
    version TEXT NOT NULL,
    candidate_id TEXT NOT NULL,
    build_id TEXT NOT NULL,
    commit_hash TEXT NOT NULL,
    platform_target TEXT NOT NULL,
    validation_status TEXT NOT NULL,
    evidence_refs TEXT NOT NULL,
    created_at_ms INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS release_validation_runs (
    id TEXT PRIMARY KEY,
    candidate_id TEXT NOT NULL,
    security_status TEXT NOT NULL,
    tests_status TEXT NOT NULL,
    migration_status TEXT NOT NULL,
    artifact_status TEXT NOT NULL,
    performance_status TEXT NOT NULL,
    release_approval_status TEXT NOT NULL,
    evidence_refs TEXT NOT NULL,
    created_at_ms INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS release_approval_decisions (
    id TEXT PRIMARY KEY,
    approved_version TEXT NOT NULL,
    validation_evidence_refs TEXT NOT NULL,
    security_status TEXT NOT NULL,
    approval_timestamp_ms INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS final_release_manifests (
    id TEXT PRIMARY KEY,
    version TEXT NOT NULL,
    features TEXT NOT NULL,
    migrations TEXT NOT NULL,
    artifacts TEXT NOT NULL,
    checksums TEXT NOT NULL,
    known_limitations TEXT NOT NULL,
    manifest_hash TEXT NOT NULL,
    created_at_ms INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS release_evidence_bundles (
    id TEXT PRIMARY KEY,
    version TEXT NOT NULL,
    audit_report_ref TEXT NOT NULL,
    security_report_ref TEXT NOT NULL,
    validation_report_ref TEXT NOT NULL,
    artifact_report_ref TEXT NOT NULL,
    migration_report_ref TEXT NOT NULL,
    created_at_ms INTEGER NOT NULL
);
