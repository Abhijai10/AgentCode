-- Security Mode: durable per-conversation security workspace state.
-- Security Mode reuses the existing single SQLite control plane — there is no
-- second database.  Rows are keyed by conversation_id so SECURITY conversations
-- are project-isolated exactly like DISCUSS/DESIGN conversations.  The daemon
-- (Kernel-owned execution authority) is the only writer; the UI reads projected
-- state through the daemon IPC.

-- One security session per SECURITY conversation.  Holds the explicit scope,
-- authorization state, threat model, audit status and the source commit the
-- current state is pinned to.
CREATE TABLE IF NOT EXISTS security_mode_sessions (
  conversation_id TEXT PRIMARY KEY,
  project_path TEXT NOT NULL,
  scope_json TEXT NOT NULL DEFAULT '{}',
  threat_model_json TEXT NOT NULL DEFAULT '{}',
  audit_status TEXT NOT NULL DEFAULT 'SCOPE_REQUIRED',
  final_status TEXT NOT NULL DEFAULT 'SECURITY_VALIDATION_INCOMPLETE',
  source_commit TEXT NOT NULL DEFAULT 'unknown',
  baseline_commit TEXT,
  baseline_roots TEXT NOT NULL DEFAULT '[]',
  baseline_attack_paths INTEGER NOT NULL DEFAULT 0,
  baseline_accepted_risk INTEGER NOT NULL DEFAULT 0,
  scanners_unavailable INTEGER NOT NULL DEFAULT 0,
  available_scanners TEXT NOT NULL DEFAULT '[]',
  unavailable_scanners TEXT NOT NULL DEFAULT '[]',
  created_at_ms INTEGER NOT NULL,
  updated_at_ms INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_secmode_sessions_project
  ON security_mode_sessions(project_path, updated_at_ms);

-- Normalized Security Mode findings with an independent, controlled lifecycle.
-- severity, confidence and exploitability are separate columns by design.
CREATE TABLE IF NOT EXISTS security_mode_findings (
  id TEXT PRIMARY KEY,
  conversation_id TEXT NOT NULL,
  fingerprint TEXT NOT NULL,
  root_cause TEXT NOT NULL,
  category TEXT NOT NULL,
  severity TEXT NOT NULL,
  confidence INTEGER NOT NULL,
  exploitability INTEGER NOT NULL,
  state TEXT NOT NULL,
  affected_code TEXT NOT NULL,
  affected_asset TEXT NOT NULL,
  entry_point TEXT,
  attack_path_refs TEXT NOT NULL DEFAULT '[]',
  evidence_refs TEXT NOT NULL DEFAULT '[]',
  scanner_refs TEXT NOT NULL DEFAULT '[]',
  remediation TEXT NOT NULL DEFAULT '',
  regression_refs TEXT NOT NULL DEFAULT '[]',
  source_commit TEXT NOT NULL,
  environment TEXT NOT NULL DEFAULT 'workspace',
  scope_ref TEXT NOT NULL DEFAULT '',
  mission_ref TEXT,
  created_at_ms INTEGER NOT NULL,
  updated_at_ms INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_secmode_findings_conv
  ON security_mode_findings(conversation_id, state);

-- Composable attack paths per conversation (chain of findings + entry point).
CREATE TABLE IF NOT EXISTS security_mode_attack_paths (
  id TEXT PRIMARY KEY,
  conversation_id TEXT NOT NULL,
  path_json TEXT NOT NULL,
  created_at_ms INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_secmode_paths_conv
  ON security_mode_attack_paths(conversation_id);

-- Safe-validation outcomes (synthetic canary retrievals, minimum-proof runs).
CREATE TABLE IF NOT EXISTS security_mode_validations (
  id TEXT PRIMARY KEY,
  conversation_id TEXT NOT NULL,
  finding_id TEXT NOT NULL,
  plan_id TEXT NOT NULL,
  state TEXT NOT NULL,
  detail TEXT NOT NULL,
  evidence_ref TEXT NOT NULL,
  created_at_ms INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_secmode_validations_conv
  ON security_mode_validations(conversation_id, finding_id);

-- Regression obligations protecting closed findings.
CREATE TABLE IF NOT EXISTS security_mode_regressions (
  id TEXT PRIMARY KEY,
  conversation_id TEXT NOT NULL,
  finding_id TEXT NOT NULL,
  regression_type TEXT NOT NULL,
  target_refs TEXT NOT NULL,
  evidence_ref TEXT NOT NULL,
  last_verified_commit TEXT NOT NULL,
  state TEXT NOT NULL,
  created_at_ms INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_secmode_regressions_conv
  ON security_mode_regressions(conversation_id, finding_id);

-- Suppressions (not dismissals): scope-bounded, expiring, revocable.
CREATE TABLE IF NOT EXISTS security_mode_suppressions (
  id TEXT PRIMARY KEY,
  conversation_id TEXT NOT NULL,
  finding_id TEXT NOT NULL,
  scope_ref TEXT NOT NULL,
  reason TEXT NOT NULL,
  source_actor TEXT NOT NULL,
  created_at_ms INTEGER NOT NULL,
  expires_at_ms INTEGER,
  state TEXT NOT NULL,
  applicability TEXT NOT NULL DEFAULT '',
  compensating_controls TEXT NOT NULL DEFAULT '',
  evidence_ref TEXT NOT NULL DEFAULT ''
);

CREATE INDEX IF NOT EXISTS idx_secmode_suppressions_conv
  ON security_mode_suppressions(conversation_id, finding_id);

-- Risk acceptance: only an authorized human/project-policy path may accept
-- blocking security risk.  The row records that external decision.
CREATE TABLE IF NOT EXISTS security_mode_risk_acceptances (
  id TEXT PRIMARY KEY,
  conversation_id TEXT NOT NULL,
  finding_id TEXT NOT NULL,
  scope_ref TEXT NOT NULL,
  severity TEXT NOT NULL,
  rationale TEXT NOT NULL,
  approver TEXT NOT NULL,
  accepted_at_ms INTEGER NOT NULL,
  review_at_ms INTEGER,
  expires_at_ms INTEGER,
  completion_allowed INTEGER NOT NULL DEFAULT 0,
  evidence_ref TEXT NOT NULL DEFAULT '',
  state TEXT NOT NULL,
  created_at_ms INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_secmode_risks_conv
  ON security_mode_risk_acceptances(conversation_id, finding_id);

-- Canonical Security Report markdown + status per conversation.
CREATE TABLE IF NOT EXISTS security_mode_reports (
  id TEXT PRIMARY KEY,
  conversation_id TEXT NOT NULL,
  report_markdown TEXT NOT NULL,
  final_status TEXT NOT NULL,
  created_at_ms INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_secmode_reports_conv
  ON security_mode_reports(conversation_id, created_at_ms);
