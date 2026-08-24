CREATE TABLE IF NOT EXISTS chaos_experiments (
    id TEXT PRIMARY KEY,
    gate_id TEXT NOT NULL,
    test_id TEXT NOT NULL,
    mission_id TEXT NOT NULL,
    fault_kind TEXT NOT NULL,
    expected_recovery TEXT NOT NULL,
    seed INTEGER NOT NULL,
    runs INTEGER NOT NULL,
    passes INTEGER NOT NULL,
    final_result TEXT NOT NULL,
    state_equivalent INTEGER NOT NULL,
    unresolved_failures TEXT NOT NULL,
    created_at_ms INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS chaos_recovery_events (
    id TEXT PRIMARY KEY,
    experiment_id TEXT NOT NULL,
    sequence_no INTEGER NOT NULL,
    phase TEXT NOT NULL,
    observed_behavior TEXT NOT NULL,
    recovery_action TEXT NOT NULL,
    evidence_ref TEXT NOT NULL,
    created_at_ms INTEGER NOT NULL,
    FOREIGN KEY(experiment_id) REFERENCES chaos_experiments(id)
);

CREATE TABLE IF NOT EXISTS chaos_reports (
    id TEXT PRIMARY KEY,
    scope TEXT NOT NULL,
    experiments INTEGER NOT NULL,
    recovered INTEGER NOT NULL,
    recovery_percent INTEGER NOT NULL,
    unresolved_failures TEXT NOT NULL,
    regression_list TEXT NOT NULL,
    created_at_ms INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS dogfood_missions (
    id TEXT PRIMARY KEY,
    repository_id TEXT NOT NULL,
    repository_path TEXT NOT NULL,
    mission_kind TEXT NOT NULL,
    objective TEXT NOT NULL,
    status TEXT NOT NULL,
    change_set_id TEXT,
    verification_report_id TEXT,
    evidence_refs TEXT NOT NULL,
    human_interventions INTEGER NOT NULL,
    provider_switches INTEGER NOT NULL,
    worker_replacements INTEGER NOT NULL,
    context_compactions INTEGER NOT NULL,
    verifier_rejections INTEGER NOT NULL,
    token_total INTEGER NOT NULL,
    paid_cost_micros INTEGER NOT NULL,
    wall_time_ms INTEGER NOT NULL,
    created_at_ms INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS dogfood_findings (
    id TEXT PRIMARY KEY,
    mission_id TEXT NOT NULL,
    severity TEXT NOT NULL,
    title TEXT NOT NULL,
    evidence_refs TEXT NOT NULL,
    status TEXT NOT NULL,
    FOREIGN KEY(mission_id) REFERENCES dogfood_missions(id)
);

CREATE TABLE IF NOT EXISTS dogfood_proposals (
    id TEXT PRIMARY KEY,
    mission_id TEXT NOT NULL,
    finding_id TEXT NOT NULL,
    summary TEXT NOT NULL,
    affected_files TEXT NOT NULL,
    change_set_id TEXT NOT NULL,
    decision TEXT NOT NULL,
    reason TEXT NOT NULL,
    FOREIGN KEY(mission_id) REFERENCES dogfood_missions(id),
    FOREIGN KEY(finding_id) REFERENCES dogfood_findings(id)
);

CREATE TABLE IF NOT EXISTS dogfood_reports (
    id TEXT PRIMARY KEY,
    scope TEXT NOT NULL,
    missions_executed INTEGER NOT NULL,
    findings INTEGER NOT NULL,
    accepted_improvements INTEGER NOT NULL,
    rejected_proposals INTEGER NOT NULL,
    regressions TEXT NOT NULL,
    recommendations TEXT NOT NULL,
    created_at_ms INTEGER NOT NULL
);
