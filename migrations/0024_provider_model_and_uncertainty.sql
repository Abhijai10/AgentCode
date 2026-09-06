-- Durable provider/model routing records and remaining uncertainty
-- for the conversation activity projection (G2-FIX-01, G2-FIX-02).

-- Provider/model records: factual, persisted routing decisions.
-- Never stores API keys, access tokens, passwords, or secret values.
CREATE TABLE IF NOT EXISTS provider_model_records (
  id TEXT PRIMARY KEY,
  project_path TEXT,
  conversation_id TEXT,
  mission_id TEXT,
  session_id TEXT,
  task_id TEXT,
  provider_id TEXT NOT NULL,
  provider_account_id TEXT,
  model_id TEXT NOT NULL,
  model_name TEXT NOT NULL,
  routing_mode TEXT NOT NULL,
  attempt_number INTEGER NOT NULL DEFAULT 1,
  success INTEGER NOT NULL,
  failure_class TEXT,
  created_at_ms INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_provider_model_records_mission
    ON provider_model_records(mission_id, created_at_ms);
CREATE INDEX IF NOT EXISTS idx_provider_model_records_conversation
    ON provider_model_records(conversation_id, created_at_ms);
CREATE INDEX IF NOT EXISTS idx_provider_model_records_project
    ON provider_model_records(project_path);

-- Add remaining_uncertainty to final_audits so the conversation activity
-- projection can expose unresolved limitations from authoritative backend state.
-- Absence of uncertainty is indistinguishable from "no uncertainty".
ALTER TABLE final_audits ADD COLUMN remaining_uncertainty TEXT NOT NULL DEFAULT '';