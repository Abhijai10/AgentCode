ALTER TABLE tasks ADD COLUMN acceptance_criteria_json TEXT NOT NULL DEFAULT '[]';
ALTER TABLE evidence_records ADD COLUMN raw_content TEXT;
ALTER TABLE evidence_records ADD COLUMN model_summary TEXT;
ALTER TABLE evidence_records ADD COLUMN sensitive INTEGER NOT NULL DEFAULT 0;
