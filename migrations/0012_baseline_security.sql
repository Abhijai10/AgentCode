CREATE TABLE IF NOT EXISTS security_threat_models (
  id TEXT PRIMARY KEY,
  repository_id TEXT NOT NULL,
  commit_ref TEXT NOT NULL,
  entry_points TEXT NOT NULL,
  auth_boundaries TEXT NOT NULL,
  data_stores TEXT NOT NULL,
  admin_operations TEXT NOT NULL,
  cloud_configuration TEXT NOT NULL,
  sensitive_assets TEXT NOT NULL,
  evidence_refs TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS security_scan_reports (
  id TEXT PRIMARY KEY,
  repository_id TEXT NOT NULL,
  commit_ref TEXT NOT NULL,
  adapters_run TEXT NOT NULL,
  missing_adapters TEXT NOT NULL,
  threat_model_id TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS security_finding_instances (
  id TEXT PRIMARY KEY,
  scan_id TEXT NOT NULL,
  adapter TEXT NOT NULL,
  rule_id TEXT NOT NULL,
  severity TEXT NOT NULL,
  confidence INTEGER NOT NULL,
  proof_level TEXT NOT NULL,
  file_path TEXT NOT NULL,
  line INTEGER NOT NULL,
  fingerprint TEXT NOT NULL,
  redacted_evidence TEXT NOT NULL,
  raw_evidence_ref TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_security_instances_scan ON security_finding_instances(scan_id);

CREATE TABLE IF NOT EXISTS security_findings (
  id TEXT PRIMARY KEY,
  scan_id TEXT NOT NULL,
  root_cause TEXT NOT NULL,
  severity TEXT NOT NULL,
  confidence INTEGER NOT NULL,
  exploitability INTEGER NOT NULL,
  status TEXT NOT NULL,
  affected_code TEXT NOT NULL,
  evidence_refs TEXT NOT NULL,
  remediation TEXT NOT NULL,
  instance_ids TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_security_findings_scan ON security_findings(scan_id);

CREATE TABLE IF NOT EXISTS security_reports (
  scan_id TEXT PRIMARY KEY,
  markdown TEXT NOT NULL,
  json_report TEXT NOT NULL,
  sarif TEXT NOT NULL
);
