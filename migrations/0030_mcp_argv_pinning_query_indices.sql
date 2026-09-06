-- 0030: MCP argv pinning + hot-path query indices.
--
-- MCP supply-chain hardening (final-audit recommendation): the reviewed
-- server manifest pins each stdio server's argv hash; the column persists
-- that pin so a swapped binary cannot silently ride an admitted name
-- across daemon restarts.
ALTER TABLE mcp_servers ADD COLUMN expected_argv_hash TEXT;

-- Query-pattern indices for the observability hot paths (final-audit
-- optimization).  Mission-scoped reads run on every UI refresh; each
-- index matches an existing WHERE + ORDER shape exactly:
--   tasks WHERE mission_id = ? ORDER BY id
--      -> tasks(mission_id, id)
--   kernel_events WHERE subject_id IN (mission + changesets)
--   ORDER BY created_at_ms DESC
--      -> kernel_events(subject_id, created_at_ms)
--   evidence_records WHERE id IN (...) ORDER BY created_at_ms ASC, id ASC
--      -> evidence_records(id, created_at_ms)  [id is PK; created_at_ms
--         covered via the covering index on (id, created_at_ms) is
--         redundant — the IN-lookup is PK-driven, so index the ORDER
--         column only where a scan actually happens: none needed here]
--   conversation_messages is already indexed by (conversation_id,
--   created_at_ms) in 0023; provider_model_records(mission_id) and
--   (conversation_id) by 0024.
CREATE INDEX IF NOT EXISTS idx_tasks_mission ON tasks(mission_id, id);
CREATE INDEX IF NOT EXISTS idx_kernel_events_subject
  ON kernel_events(subject_id, created_at_ms);
