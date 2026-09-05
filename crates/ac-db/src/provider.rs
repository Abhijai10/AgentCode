impl ControlPlaneDb {
    pub fn save_provider_catalog_entry(&self, row: &ProviderCatalogRow) -> AcResult<()> {
        self.connection
            .execute(
                "INSERT INTO provider_catalog VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
                 ON CONFLICT(id) DO UPDATE SET display_name=excluded.display_name,
                 description=excluded.description, website_url=excluded.website_url,
                 logo_url=excluded.logo_url, credential_url=excluded.credential_url,
                 pricing_classification=excluded.pricing_classification,
                 capabilities=excluded.capabilities, updated_at_ms=excluded.updated_at_ms",
                params![
                    row.id,
                    row.display_name,
                    row.description,
                    row.website_url,
                    row.logo_url,
                    row.credential_url,
                    row.pricing_classification,
                    row.capabilities,
                    row.created_at_ms,
                    row.updated_at_ms
                ],
            )
            .map_err(db_error)?;
        Ok(())
    }

    pub fn provider_catalog_entry(&self, id: &str) -> AcResult<Option<ProviderCatalogRow>> {
        self.connection
            .query_row(
                "SELECT id, display_name, description, website_url, logo_url, credential_url,
                 pricing_classification, capabilities, created_at_ms, updated_at_ms
                 FROM provider_catalog WHERE id=?1",
                [id],
                |row| {
                    Ok(ProviderCatalogRow {
                        id: row.get(0)?,
                        display_name: row.get(1)?,
                        description: row.get(2)?,
                        website_url: row.get(3)?,
                        logo_url: row.get(4)?,
                        credential_url: row.get(5)?,
                        pricing_classification: row.get(6)?,
                        capabilities: row.get(7)?,
                        created_at_ms: row.get(8)?,
                        updated_at_ms: row.get(9)?,
                    })
                },
            )
            .optional()
            .map_err(db_error)
    }

    pub fn provider_catalog_entries(&self) -> AcResult<Vec<ProviderCatalogRow>> {
        let mut stmt = self
            .connection
            .prepare(
                "SELECT id, display_name, description, website_url, logo_url, credential_url,
                 pricing_classification, capabilities, created_at_ms, updated_at_ms
                 FROM provider_catalog ORDER BY display_name ASC",
            )
            .map_err(db_error)?;
        let rows = stmt
            .query_map([], |row| {
                Ok(ProviderCatalogRow {
                    id: row.get(0)?,
                    display_name: row.get(1)?,
                    description: row.get(2)?,
                    website_url: row.get(3)?,
                    logo_url: row.get(4)?,
                    credential_url: row.get(5)?,
                    pricing_classification: row.get(6)?,
                    capabilities: row.get(7)?,
                    created_at_ms: row.get(8)?,
                    updated_at_ms: row.get(9)?,
                })
            })
            .map_err(db_error)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(db_error)
    }

    pub fn save_provider_account(&self, row: &ProviderAccountRow) -> AcResult<()> {
        self.connection
            .execute(
                "INSERT INTO provider_accounts VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18)
                 ON CONFLICT(id) DO UPDATE SET label=excluded.label,
                 credential_ref=excluded.credential_ref, credential_region=excluded.credential_region,
                 organization=excluded.organization, project=excluded.project,
                 workspace=excluded.workspace, enabled=excluded.enabled,
                 health_state=excluded.health_state, quota_rate_limit=excluded.quota_rate_limit,
                 quota_remaining=excluded.quota_remaining, quota_reset_at_ms=excluded.quota_reset_at_ms,
                 last_success_at_ms=excluded.last_success_at_ms, last_failure_at_ms=excluded.last_failure_at_ms,
                 failure_reason=excluded.failure_reason, updated_at_ms=excluded.updated_at_ms",
                params![
                    row.id,
                    row.provider_id,
                    row.label,
                    row.credential_ref,
                    row.credential_region,
                    row.organization,
                    row.project,
                    row.workspace,
                    row.enabled as i64,
                    row.health_state,
                    row.quota_rate_limit,
                    row.quota_remaining,
                    row.quota_reset_at_ms,
                    row.last_success_at_ms,
                    row.last_failure_at_ms,
                    row.failure_reason,
                    row.created_at_ms,
                    row.updated_at_ms
                ],
            )
            .map_err(db_error)?;
        Ok(())
    }

    pub fn provider_account(&self, id: &str) -> AcResult<Option<ProviderAccountRow>> {
        self.connection
            .query_row(
                "SELECT id, provider_id, label, credential_ref, credential_region, organization,
                 project, workspace, enabled, health_state, quota_rate_limit, quota_remaining,
                 quota_reset_at_ms, last_success_at_ms, last_failure_at_ms, failure_reason,
                 created_at_ms, updated_at_ms
                 FROM provider_accounts WHERE id=?1",
                [id],
                |row| {
                    Ok(ProviderAccountRow {
                        id: row.get(0)?,
                        provider_id: row.get(1)?,
                        label: row.get(2)?,
                        credential_ref: row.get(3)?,
                        credential_region: row.get(4)?,
                        organization: row.get(5)?,
                        project: row.get(6)?,
                        workspace: row.get(7)?,
                        enabled: row.get::<_, i64>(8)? != 0,
                        health_state: row.get(9)?,
                        quota_rate_limit: row.get(10)?,
                        quota_remaining: row.get(11)?,
                        quota_reset_at_ms: row.get(12)?,
                        last_success_at_ms: row.get(13)?,
                        last_failure_at_ms: row.get(14)?,
                        failure_reason: row.get(15)?,
                        created_at_ms: row.get(16)?,
                        updated_at_ms: row.get(17)?,
                    })
                },
            )
            .optional()
            .map_err(db_error)
    }

    pub fn provider_accounts(&self, provider_id: &str) -> AcResult<Vec<ProviderAccountRow>> {
        let mut stmt = self
            .connection
            .prepare(
                "SELECT id, provider_id, label, credential_ref, credential_region, organization,
                 project, workspace, enabled, health_state, quota_rate_limit, quota_remaining,
                 quota_reset_at_ms, last_success_at_ms, last_failure_at_ms, failure_reason,
                 created_at_ms, updated_at_ms
                 FROM provider_accounts WHERE provider_id=?1 ORDER BY label ASC",
            )
            .map_err(db_error)?;
        let rows = stmt
            .query_map([provider_id], |row| {
                Ok(ProviderAccountRow {
                    id: row.get(0)?,
                    provider_id: row.get(1)?,
                    label: row.get(2)?,
                    credential_ref: row.get(3)?,
                    credential_region: row.get(4)?,
                    organization: row.get(5)?,
                    project: row.get(6)?,
                    workspace: row.get(7)?,
                    enabled: row.get::<_, i64>(8)? != 0,
                    health_state: row.get(9)?,
                    quota_rate_limit: row.get(10)?,
                    quota_remaining: row.get(11)?,
                    quota_reset_at_ms: row.get(12)?,
                    last_success_at_ms: row.get(13)?,
                    last_failure_at_ms: row.get(14)?,
                    failure_reason: row.get(15)?,
                    created_at_ms: row.get(16)?,
                    updated_at_ms: row.get(17)?,
                })
            })
            .map_err(db_error)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(db_error)
    }

    pub fn delete_provider_account(&self, id: &str) -> AcResult<()> {
        self.connection
            .execute(
                "DELETE FROM provider_health_observations WHERE account_id=?1",
                [id],
            )
            .map_err(db_error)?;
        self.connection
            .execute("DELETE FROM provider_accounts WHERE id=?1", [id])
            .map_err(db_error)?;
        Ok(())
    }

    pub fn provider_health_observations(
        &self,
        account_id: &str,
        limit: usize,
    ) -> AcResult<Vec<ProviderHealthObservationRow>> {
        let mut stmt = self
            .connection
            .prepare(
                "SELECT id, account_id, success, latency_ms, failure_code, failure_message, observed_at_ms
                 FROM provider_health_observations WHERE account_id=?1
                 ORDER BY observed_at_ms DESC LIMIT ?2",
            )
            .map_err(db_error)?;
        let rows = stmt
            .query_map(params![account_id, limit as i64], |row| {
                Ok(ProviderHealthObservationRow {
                    id: row.get(0)?,
                    account_id: row.get(1)?,
                    success: row.get::<_, i64>(2)? != 0,
                    latency_ms: row.get::<_, i64>(3)? as u64,
                    failure_code: row.get(4)?,
                    failure_message: row.get(5)?,
                    observed_at_ms: row.get(6)?,
                })
            })
            .map_err(db_error)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(db_error)
    }

    /// Batch N1: read the persisted provider/routing preference row
    /// (conventionally id 'global').  None means "defaults apply" — the
    /// daemon then uses the per-call-site fallback profile.
    pub fn provider_preference(&self, id: &str) -> AcResult<Option<ProviderPreferenceRow>> {
        self.connection
            .query_row(
                "SELECT id, routing_profile, preferred_model, updated_at_ms
                 FROM provider_preferences WHERE id=?1",
                [id],
                |row| {
                    Ok(ProviderPreferenceRow {
                        id: row.get(0)?,
                        routing_profile: row.get(1)?,
                        preferred_model: row.get(2)?,
                        updated_at_ms: row.get(3)?,
                    })
                },
            )
            .optional()
            .map_err(db_error)
    }

    /// Batch N1: persist (upsert) the provider/routing preference row.
    /// The daemon is the single writer; the UI only sends the request.
    pub fn save_provider_preference(&self, row: &ProviderPreferenceRow) -> AcResult<()> {
        self.connection
            .execute(
                "INSERT INTO provider_preferences VALUES (?1, ?2, ?3, ?4)
                 ON CONFLICT(id) DO UPDATE SET routing_profile=excluded.routing_profile,
                 preferred_model=excluded.preferred_model,
                 updated_at_ms=excluded.updated_at_ms",
                params![
                    row.id,
                    row.routing_profile,
                    row.preferred_model,
                    row.updated_at_ms
                ],
            )
            .map_err(db_error)?;
        Ok(())
    }

    pub fn append_provider_health_observation(
        &self,
        row: &ProviderHealthObservationRow,
    ) -> AcResult<()> {
        self.connection
            .execute(
                "INSERT INTO provider_health_observations VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![
                    row.id,
                    row.account_id,
                    row.success as i64,
                    row.latency_ms as i64,
                    row.failure_code,
                    row.failure_message,
                    row.observed_at_ms
                ],
            )
            .map_err(db_error)?;
        Ok(())
    }
}
