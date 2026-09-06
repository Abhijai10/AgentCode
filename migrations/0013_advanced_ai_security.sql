CREATE TABLE IF NOT EXISTS active_security_authorizations (
  id TEXT PRIMARY KEY,
  repository_id TEXT NOT NULL,
  target TEXT NOT NULL,
  environment TEXT NOT NULL,
  allowed_targets TEXT NOT NULL,
  cloud_accounts TEXT NOT NULL,
  credential_ref TEXT,
  rate_limit_per_minute INTEGER NOT NULL,
  concurrency_limit INTEGER NOT NULL,
  forbidden_actions TEXT NOT NULL,
  expires_at_ms INTEGER NOT NULL,
  cleanup_required INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS active_security_reports (
  id TEXT PRIMARY KEY,
  repository_id TEXT NOT NULL,
  commit_ref TEXT NOT NULL,
  authorization_id TEXT NOT NULL,
  environment TEXT NOT NULL,
  adapters TEXT NOT NULL,
  findings_count INTEGER NOT NULL,
  attack_graph_nodes INTEGER NOT NULL,
  attack_graph_edges INTEGER NOT NULL,
  stop_reasons TEXT NOT NULL,
  cleanup_verified INTEGER NOT NULL,
  degraded TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_active_security_reports_repo ON active_security_reports(repository_id);

CREATE TABLE IF NOT EXISTS ai_security_reports (
  id TEXT PRIMARY KEY,
  repository_id TEXT NOT NULL,
  commit_ref TEXT NOT NULL,
  surfaces TEXT NOT NULL,
  harnesses TEXT NOT NULL,
  attack_cases_count INTEGER NOT NULL,
  attack_results_count INTEGER NOT NULL,
  findings_count INTEGER NOT NULL,
  mitigations_verified INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_ai_security_reports_repo ON ai_security_reports(repository_id);

CREATE TABLE IF NOT EXISTS ai_attack_cases (
  id TEXT PRIMARY KEY,
  report_id TEXT NOT NULL,
  category TEXT NOT NULL,
  fixture TEXT NOT NULL,
  expected_policy TEXT NOT NULL,
  synthetic INTEGER NOT NULL,
  blocked INTEGER NOT NULL,
  evidence_ref TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_ai_attack_cases_report ON ai_attack_cases(report_id);
