-- Batch N1: persisted provider/routing preferences (G1).  The routing
-- profile (FreeOnly/FreeFirst/LocalFirst/QualityFirst/PaidAllowed/Offline)
-- and the preferred model were previously hardcoded per call site with no
-- user control and no persistence.  Global row: id='global'.
CREATE TABLE IF NOT EXISTS provider_preferences (
    id TEXT PRIMARY KEY,
    routing_profile TEXT NOT NULL DEFAULT 'LocalFirst',
    preferred_model TEXT NOT NULL DEFAULT '',
    updated_at_ms INTEGER NOT NULL
);
