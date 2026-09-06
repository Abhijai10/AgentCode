CREATE TABLE IF NOT EXISTS discuss_sessions (
  id TEXT PRIMARY KEY,
  repository_id TEXT NOT NULL,
  title TEXT NOT NULL,
  state TEXT NOT NULL,
  context_manifest_refs TEXT NOT NULL,
  accepted_decision_refs TEXT NOT NULL,
  created_at_ms INTEGER NOT NULL,
  updated_at_ms INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_discuss_sessions_repo ON discuss_sessions(repository_id);

CREATE TABLE IF NOT EXISTS discuss_messages (
  id TEXT PRIMARY KEY,
  session_id TEXT NOT NULL,
  role TEXT NOT NULL,
  content TEXT NOT NULL,
  context_ref TEXT,
  evidence_refs TEXT NOT NULL,
  created_at_ms INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_discuss_messages_session ON discuss_messages(session_id);

CREATE TABLE IF NOT EXISTS discuss_decision_candidates (
  id TEXT PRIMARY KEY,
  session_id TEXT NOT NULL,
  decision TEXT NOT NULL,
  rationale TEXT NOT NULL,
  evidence_refs TEXT NOT NULL,
  accepted_decision_ref TEXT,
  created_at_ms INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_discuss_decision_candidates_session ON discuss_decision_candidates(session_id);

CREATE TABLE IF NOT EXISTS discuss_plans (
  id TEXT PRIMARY KEY,
  session_id TEXT NOT NULL,
  requirements TEXT NOT NULL,
  tasks TEXT NOT NULL,
  constraints_json TEXT NOT NULL,
  open_questions TEXT NOT NULL,
  accepted_decision_refs TEXT NOT NULL,
  promoted_mission_id TEXT,
  created_at_ms INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_discuss_plans_session ON discuss_plans(session_id);

CREATE TABLE IF NOT EXISTS design_sessions (
  id TEXT PRIMARY KEY,
  repository_id TEXT NOT NULL,
  product TEXT NOT NULL,
  state TEXT NOT NULL,
  hard_constraints TEXT NOT NULL,
  created_at_ms INTEGER NOT NULL,
  updated_at_ms INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_design_sessions_repo ON design_sessions(repository_id);

CREATE TABLE IF NOT EXISTS design_artifacts (
  id TEXT PRIMARY KEY,
  session_id TEXT NOT NULL,
  name TEXT NOT NULL,
  artifact_type TEXT NOT NULL,
  current_version INTEGER NOT NULL,
  created_at_ms INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_design_artifacts_session ON design_artifacts(session_id);

CREATE TABLE IF NOT EXISTS design_artifact_versions (
  id TEXT PRIMARY KEY,
  artifact_id TEXT NOT NULL,
  version INTEGER NOT NULL,
  summary TEXT NOT NULL,
  content_hash TEXT NOT NULL,
  evidence_refs TEXT NOT NULL,
  created_at_ms INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_design_artifact_versions_artifact ON design_artifact_versions(artifact_id);

CREATE TABLE IF NOT EXISTS design_visual_evaluations (
  id TEXT PRIMARY KEY,
  artifact_version_id TEXT NOT NULL,
  passed INTEGER NOT NULL,
  findings TEXT NOT NULL,
  responsive_viewports TEXT NOT NULL,
  accessibility_checks TEXT NOT NULL,
  functional_flows TEXT NOT NULL,
  evidence_refs TEXT NOT NULL,
  created_at_ms INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_design_visual_evaluations_version ON design_visual_evaluations(artifact_version_id);
