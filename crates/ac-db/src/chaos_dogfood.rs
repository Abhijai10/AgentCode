impl ControlPlaneDb {
    pub fn save_chaos_experiment(&self, row: &ChaosExperimentRow) -> AcResult<()> {
        self.connection
            .execute(
                "INSERT INTO chaos_experiments VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)
                 ON CONFLICT(id) DO UPDATE SET runs=excluded.runs, passes=excluded.passes,
                 final_result=excluded.final_result, state_equivalent=excluded.state_equivalent,
                 unresolved_failures=excluded.unresolved_failures",
                params![
                    row.id,
                    row.gate_id,
                    row.test_id,
                    row.mission_id,
                    row.fault_kind,
                    row.expected_recovery,
                    row.seed,
                    row.runs,
                    row.passes,
                    row.final_result,
                    row.state_equivalent,
                    row.unresolved_failures,
                    row.created_at_ms
                ],
            )
            .map_err(db_error)?;
        Ok(())
    }

    pub fn chaos_experiments(&self) -> AcResult<Vec<ChaosExperimentRow>> {
        let mut stmt = self
            .connection
            .prepare(
                "SELECT id, gate_id, test_id, mission_id, fault_kind, expected_recovery, seed,
                 runs, passes, final_result, state_equivalent, unresolved_failures, created_at_ms
                 FROM chaos_experiments ORDER BY created_at_ms ASC",
            )
            .map_err(db_error)?;
        let rows = stmt
            .query_map([], |row| {
                Ok(ChaosExperimentRow {
                    id: row.get(0)?,
                    gate_id: row.get(1)?,
                    test_id: row.get(2)?,
                    mission_id: row.get(3)?,
                    fault_kind: row.get(4)?,
                    expected_recovery: row.get(5)?,
                    seed: row.get(6)?,
                    runs: row.get(7)?,
                    passes: row.get(8)?,
                    final_result: row.get(9)?,
                    state_equivalent: row.get(10)?,
                    unresolved_failures: row.get(11)?,
                    created_at_ms: row.get(12)?,
                })
            })
            .map_err(db_error)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(db_error)
    }

    pub fn save_chaos_recovery_event(&self, row: &ChaosRecoveryEventRow) -> AcResult<()> {
        self.connection
            .execute(
                "INSERT INTO chaos_recovery_events VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![
                    row.id,
                    row.experiment_id,
                    row.sequence_no,
                    row.phase,
                    row.observed_behavior,
                    row.recovery_action,
                    row.evidence_ref,
                    row.created_at_ms
                ],
            )
            .map_err(db_error)?;
        Ok(())
    }

    pub fn chaos_recovery_events(
        &self,
        experiment_id: &str,
    ) -> AcResult<Vec<ChaosRecoveryEventRow>> {
        let mut stmt = self
            .connection
            .prepare(
                "SELECT id, experiment_id, sequence_no, phase, observed_behavior, recovery_action,
                 evidence_ref, created_at_ms FROM chaos_recovery_events
                 WHERE experiment_id=?1 ORDER BY sequence_no ASC",
            )
            .map_err(db_error)?;
        let rows = stmt
            .query_map([experiment_id], |row| {
                Ok(ChaosRecoveryEventRow {
                    id: row.get(0)?,
                    experiment_id: row.get(1)?,
                    sequence_no: row.get(2)?,
                    phase: row.get(3)?,
                    observed_behavior: row.get(4)?,
                    recovery_action: row.get(5)?,
                    evidence_ref: row.get(6)?,
                    created_at_ms: row.get(7)?,
                })
            })
            .map_err(db_error)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(db_error)
    }

    pub fn save_chaos_report(&self, row: &ChaosReportRow) -> AcResult<()> {
        self.connection
            .execute(
                "INSERT INTO chaos_reports VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![
                    row.id,
                    row.scope,
                    row.experiments,
                    row.recovered,
                    row.recovery_percent,
                    row.unresolved_failures,
                    row.regression_list,
                    row.created_at_ms
                ],
            )
            .map_err(db_error)?;
        Ok(())
    }

    pub fn chaos_reports(&self) -> AcResult<Vec<ChaosReportRow>> {
        let mut stmt = self
            .connection
            .prepare(
                "SELECT id, scope, experiments, recovered, recovery_percent, unresolved_failures,
                 regression_list, created_at_ms FROM chaos_reports ORDER BY created_at_ms ASC",
            )
            .map_err(db_error)?;
        let rows = stmt
            .query_map([], |row| {
                Ok(ChaosReportRow {
                    id: row.get(0)?,
                    scope: row.get(1)?,
                    experiments: row.get(2)?,
                    recovered: row.get(3)?,
                    recovery_percent: row.get(4)?,
                    unresolved_failures: row.get(5)?,
                    regression_list: row.get(6)?,
                    created_at_ms: row.get(7)?,
                })
            })
            .map_err(db_error)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(db_error)
    }

    pub fn save_dogfood_mission(&self, row: &DogfoodMissionRow) -> AcResult<()> {
        self.connection
            .execute(
                "INSERT INTO dogfood_missions VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18)
                 ON CONFLICT(id) DO UPDATE SET status=excluded.status, change_set_id=excluded.change_set_id,
                 verification_report_id=excluded.verification_report_id, evidence_refs=excluded.evidence_refs",
                params![
                    row.id,
                    row.repository_id,
                    row.repository_path,
                    row.mission_kind,
                    row.objective,
                    row.status,
                    row.change_set_id,
                    row.verification_report_id,
                    row.evidence_refs,
                    row.human_interventions,
                    row.provider_switches,
                    row.worker_replacements,
                    row.context_compactions,
                    row.verifier_rejections,
                    row.token_total,
                    row.paid_cost_micros,
                    row.wall_time_ms,
                    row.created_at_ms
                ],
            )
            .map_err(db_error)?;
        Ok(())
    }

    pub fn dogfood_missions(&self) -> AcResult<Vec<DogfoodMissionRow>> {
        let mut stmt = self
            .connection
            .prepare(
                "SELECT id, repository_id, repository_path, mission_kind, objective, status,
                 change_set_id, verification_report_id, evidence_refs, human_interventions,
                 provider_switches, worker_replacements, context_compactions, verifier_rejections,
                 token_total, paid_cost_micros, wall_time_ms, created_at_ms
                 FROM dogfood_missions ORDER BY created_at_ms ASC",
            )
            .map_err(db_error)?;
        let rows = stmt
            .query_map([], |row| {
                Ok(DogfoodMissionRow {
                    id: row.get(0)?,
                    repository_id: row.get(1)?,
                    repository_path: row.get(2)?,
                    mission_kind: row.get(3)?,
                    objective: row.get(4)?,
                    status: row.get(5)?,
                    change_set_id: row.get(6)?,
                    verification_report_id: row.get(7)?,
                    evidence_refs: row.get(8)?,
                    human_interventions: row.get(9)?,
                    provider_switches: row.get(10)?,
                    worker_replacements: row.get(11)?,
                    context_compactions: row.get(12)?,
                    verifier_rejections: row.get(13)?,
                    token_total: row.get(14)?,
                    paid_cost_micros: row.get(15)?,
                    wall_time_ms: row.get(16)?,
                    created_at_ms: row.get(17)?,
                })
            })
            .map_err(db_error)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(db_error)
    }

    pub fn save_dogfood_finding(&self, row: &DogfoodFindingRow) -> AcResult<()> {
        self.connection
            .execute(
                "INSERT INTO dogfood_findings VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    row.id,
                    row.mission_id,
                    row.severity,
                    row.title,
                    row.evidence_refs,
                    row.status
                ],
            )
            .map_err(db_error)?;
        Ok(())
    }

    pub fn dogfood_findings(&self, mission_id: &str) -> AcResult<Vec<DogfoodFindingRow>> {
        let mut stmt = self
            .connection
            .prepare(
                "SELECT id, mission_id, severity, title, evidence_refs, status
                 FROM dogfood_findings WHERE mission_id=?1 ORDER BY id ASC",
            )
            .map_err(db_error)?;
        let rows = stmt
            .query_map([mission_id], |row| {
                Ok(DogfoodFindingRow {
                    id: row.get(0)?,
                    mission_id: row.get(1)?,
                    severity: row.get(2)?,
                    title: row.get(3)?,
                    evidence_refs: row.get(4)?,
                    status: row.get(5)?,
                })
            })
            .map_err(db_error)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(db_error)
    }

    pub fn save_dogfood_proposal(&self, row: &DogfoodProposalRow) -> AcResult<()> {
        self.connection
            .execute(
                "INSERT INTO dogfood_proposals VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![
                    row.id,
                    row.mission_id,
                    row.finding_id,
                    row.summary,
                    row.affected_files,
                    row.change_set_id,
                    row.decision,
                    row.reason
                ],
            )
            .map_err(db_error)?;
        Ok(())
    }

    pub fn dogfood_proposals(&self, mission_id: &str) -> AcResult<Vec<DogfoodProposalRow>> {
        let mut stmt = self
            .connection
            .prepare(
                "SELECT id, mission_id, finding_id, summary, affected_files, change_set_id,
                 decision, reason FROM dogfood_proposals WHERE mission_id=?1 ORDER BY id ASC",
            )
            .map_err(db_error)?;
        let rows = stmt
            .query_map([mission_id], |row| {
                Ok(DogfoodProposalRow {
                    id: row.get(0)?,
                    mission_id: row.get(1)?,
                    finding_id: row.get(2)?,
                    summary: row.get(3)?,
                    affected_files: row.get(4)?,
                    change_set_id: row.get(5)?,
                    decision: row.get(6)?,
                    reason: row.get(7)?,
                })
            })
            .map_err(db_error)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(db_error)
    }

    pub fn save_dogfood_report(&self, row: &DogfoodReportRow) -> AcResult<()> {
        self.connection
            .execute(
                "INSERT INTO dogfood_reports VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                params![
                    row.id,
                    row.scope,
                    row.missions_executed,
                    row.findings,
                    row.accepted_improvements,
                    row.rejected_proposals,
                    row.regressions,
                    row.recommendations,
                    row.created_at_ms
                ],
            )
            .map_err(db_error)?;
        Ok(())
    }

    pub fn dogfood_reports(&self) -> AcResult<Vec<DogfoodReportRow>> {
        let mut stmt = self
            .connection
            .prepare(
                "SELECT id, scope, missions_executed, findings, accepted_improvements,
                 rejected_proposals, regressions, recommendations, created_at_ms
                 FROM dogfood_reports ORDER BY created_at_ms ASC",
            )
            .map_err(db_error)?;
        let rows = stmt
            .query_map([], |row| {
                Ok(DogfoodReportRow {
                    id: row.get(0)?,
                    scope: row.get(1)?,
                    missions_executed: row.get(2)?,
                    findings: row.get(3)?,
                    accepted_improvements: row.get(4)?,
                    rejected_proposals: row.get(5)?,
                    regressions: row.get(6)?,
                    recommendations: row.get(7)?,
                    created_at_ms: row.get(8)?,
                })
            })
            .map_err(db_error)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(db_error)
    }
}
