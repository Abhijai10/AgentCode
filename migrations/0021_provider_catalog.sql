CREATE TABLE IF NOT EXISTS provider_catalog (
  id TEXT PRIMARY KEY,
  display_name TEXT NOT NULL,
  description TEXT NOT NULL DEFAULT '',
  website_url TEXT NOT NULL DEFAULT '',
  logo_url TEXT NOT NULL DEFAULT '',
  credential_url TEXT NOT NULL DEFAULT '',
  pricing_classification TEXT NOT NULL DEFAULT 'unknown',
  capabilities TEXT NOT NULL DEFAULT '[]',
  created_at_ms INTEGER NOT NULL,
  updated_at_ms INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS provider_accounts (
  id TEXT PRIMARY KEY,
  provider_id TEXT NOT NULL REFERENCES provider_catalog(id),
  label TEXT NOT NULL,
  credential_ref TEXT NOT NULL,
  credential_region TEXT NOT NULL DEFAULT '',
  organization TEXT NOT NULL DEFAULT '',
  project TEXT NOT NULL DEFAULT '',
  workspace TEXT NOT NULL DEFAULT '',
  enabled INTEGER NOT NULL DEFAULT 1,
  health_state TEXT NOT NULL DEFAULT 'unknown',
  quota_rate_limit INTEGER,
  quota_remaining INTEGER,
  quota_reset_at_ms INTEGER,
  last_success_at_ms INTEGER,
  last_failure_at_ms INTEGER,
  failure_reason TEXT NOT NULL DEFAULT '',
  created_at_ms INTEGER NOT NULL,
  updated_at_ms INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS provider_health_observations (
  id TEXT PRIMARY KEY,
  account_id TEXT NOT NULL REFERENCES provider_accounts(id),
  success INTEGER NOT NULL,
  latency_ms INTEGER NOT NULL DEFAULT 0,
  failure_code TEXT NOT NULL DEFAULT '',
  failure_message TEXT NOT NULL DEFAULT '',
  observed_at_ms INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_provider_accounts_provider ON provider_accounts(provider_id);
CREATE INDEX IF NOT EXISTS idx_provider_health_account ON provider_health_observations(account_id);
CREATE INDEX IF NOT EXISTS idx_provider_health_observed ON provider_health_observations(observed_at_ms);