impl ControlPlaneDb {
    pub fn save_discuss_session(&self, row: &DiscussSessionRow) -> AcResult<()> {
        self.connection
            .execute(
                "INSERT INTO discuss_sessions VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
                 ON CONFLICT(id) DO UPDATE SET state=excluded.state,
                 context_manifest_refs=excluded.context_manifest_refs,
                 accepted_decision_refs=excluded.accepted_decision_refs,
                 updated_at_ms=excluded.updated_at_ms",
                params![
                    row.id,
                    row.repository_id,
                    row.title,
                    row.state,
                    row.context_manifest_refs,
                    row.accepted_decision_refs,
                    row.created_at_ms,
                    row.updated_at_ms
                ],
            )
            .map_err(db_error)?;
        Ok(())
    }

    pub fn discuss_session(&self, id: &str) -> AcResult<Option<DiscussSessionRow>> {
        self.connection
            .query_row(
                "SELECT id, repository_id, title, state, context_manifest_refs,
                 accepted_decision_refs, created_at_ms, updated_at_ms
                 FROM discuss_sessions WHERE id=?1",
                [id],
                |row| {
                    Ok(DiscussSessionRow {
                        id: row.get(0)?,
                        repository_id: row.get(1)?,
                        title: row.get(2)?,
                        state: row.get(3)?,
                        context_manifest_refs: row.get(4)?,
                        accepted_decision_refs: row.get(5)?,
                        created_at_ms: row.get(6)?,
                        updated_at_ms: row.get(7)?,
                    })
                },
            )
            .optional()
            .map_err(db_error)
    }

    pub fn save_discuss_message(&self, row: &DiscussMessageRow) -> AcResult<()> {
        self.connection
            .execute(
                "INSERT INTO discuss_messages VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![
                    row.id,
                    row.session_id,
                    row.role,
                    row.content,
                    row.context_ref,
                    row.evidence_refs,
                    row.created_at_ms
                ],
            )
            .map_err(db_error)?;
        Ok(())
    }

    pub fn discuss_messages(&self, session_id: &str) -> AcResult<Vec<DiscussMessageRow>> {
        let mut stmt = self
            .connection
            .prepare(
                "SELECT id, session_id, role, content, context_ref, evidence_refs, created_at_ms
                 FROM discuss_messages WHERE session_id=?1 ORDER BY created_at_ms ASC",
            )
            .map_err(db_error)?;
        let rows = stmt
            .query_map([session_id], |row| {
                Ok(DiscussMessageRow {
                    id: row.get(0)?,
                    session_id: row.get(1)?,
                    role: row.get(2)?,
                    content: row.get(3)?,
                    context_ref: row.get(4)?,
                    evidence_refs: row.get(5)?,
                    created_at_ms: row.get(6)?,
                })
            })
            .map_err(db_error)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(db_error)
    }

    pub fn save_discuss_decision_candidate(
        &self,
        row: &DiscussDecisionCandidateRow,
    ) -> AcResult<()> {
        self.connection
            .execute(
                "INSERT INTO discuss_decision_candidates VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
                 ON CONFLICT(id) DO UPDATE SET accepted_decision_ref=excluded.accepted_decision_ref",
                params![
                    row.id,
                    row.session_id,
                    row.decision,
                    row.rationale,
                    row.evidence_refs,
                    row.accepted_decision_ref,
                    row.created_at_ms
                ],
            )
            .map_err(db_error)?;
        Ok(())
    }

    pub fn save_discuss_plan(&self, row: &DiscussPlanRow) -> AcResult<()> {
        self.connection
            .execute(
                "INSERT INTO discuss_plans VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
                 ON CONFLICT(id) DO UPDATE SET promoted_mission_id=excluded.promoted_mission_id",
                params![
                    row.id,
                    row.session_id,
                    row.requirements,
                    row.tasks,
                    row.constraints_json,
                    row.open_questions,
                    row.accepted_decision_refs,
                    row.promoted_mission_id,
                    row.created_at_ms
                ],
            )
            .map_err(db_error)?;
        Ok(())
    }

    pub fn discuss_plan(&self, id: &str) -> AcResult<Option<DiscussPlanRow>> {
        self.connection
            .query_row(
                "SELECT id, session_id, requirements, tasks, constraints_json, open_questions,
                 accepted_decision_refs, promoted_mission_id, created_at_ms
                 FROM discuss_plans WHERE id=?1",
                [id],
                |row| {
                    Ok(DiscussPlanRow {
                        id: row.get(0)?,
                        session_id: row.get(1)?,
                        requirements: row.get(2)?,
                        tasks: row.get(3)?,
                        constraints_json: row.get(4)?,
                        open_questions: row.get(5)?,
                        accepted_decision_refs: row.get(6)?,
                        promoted_mission_id: row.get(7)?,
                        created_at_ms: row.get(8)?,
                    })
                },
            )
            .optional()
            .map_err(db_error)
    }

    pub fn save_design_session(&self, row: &DesignSessionRow) -> AcResult<()> {
        self.connection
            .execute(
                "INSERT INTO design_sessions VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
                 ON CONFLICT(id) DO UPDATE SET state=excluded.state,
                 hard_constraints=excluded.hard_constraints, updated_at_ms=excluded.updated_at_ms",
                params![
                    row.id,
                    row.repository_id,
                    row.product,
                    row.state,
                    row.hard_constraints,
                    row.created_at_ms,
                    row.updated_at_ms
                ],
            )
            .map_err(db_error)?;
        Ok(())
    }

    pub fn design_session(&self, id: &str) -> AcResult<Option<DesignSessionRow>> {
        self.connection
            .query_row(
                "SELECT id, repository_id, product, state, hard_constraints, created_at_ms,
                 updated_at_ms FROM design_sessions WHERE id=?1",
                [id],
                |row| {
                    Ok(DesignSessionRow {
                        id: row.get(0)?,
                        repository_id: row.get(1)?,
                        product: row.get(2)?,
                        state: row.get(3)?,
                        hard_constraints: row.get(4)?,
                        created_at_ms: row.get(5)?,
                        updated_at_ms: row.get(6)?,
                    })
                },
            )
            .optional()
            .map_err(db_error)
    }

    pub fn save_design_artifact(&self, row: &DesignArtifactRow) -> AcResult<()> {
        self.connection
            .execute(
                "INSERT INTO design_artifacts VALUES (?1, ?2, ?3, ?4, ?5, ?6)
                 ON CONFLICT(id) DO UPDATE SET current_version=excluded.current_version",
                params![
                    row.id,
                    row.session_id,
                    row.name,
                    row.artifact_type,
                    row.current_version,
                    row.created_at_ms
                ],
            )
            .map_err(db_error)?;
        Ok(())
    }

    pub fn save_design_artifact_version(&self, row: &DesignArtifactVersionRow) -> AcResult<()> {
        self.connection
            .execute(
                "INSERT INTO design_artifact_versions VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![
                    row.id,
                    row.artifact_id,
                    row.version,
                    row.summary,
                    row.content_hash,
                    row.evidence_refs,
                    row.created_at_ms
                ],
            )
            .map_err(db_error)?;
        Ok(())
    }

    pub fn design_artifact_versions(
        &self,
        artifact_id: &str,
    ) -> AcResult<Vec<DesignArtifactVersionRow>> {
        let mut stmt = self
            .connection
            .prepare(
                "SELECT id, artifact_id, version, summary, content_hash, evidence_refs,
                 created_at_ms FROM design_artifact_versions
                 WHERE artifact_id=?1 ORDER BY version ASC",
            )
            .map_err(db_error)?;
        let rows = stmt
            .query_map([artifact_id], |row| {
                Ok(DesignArtifactVersionRow {
                    id: row.get(0)?,
                    artifact_id: row.get(1)?,
                    version: row.get(2)?,
                    summary: row.get(3)?,
                    content_hash: row.get(4)?,
                    evidence_refs: row.get(5)?,
                    created_at_ms: row.get(6)?,
                })
            })
            .map_err(db_error)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(db_error)
    }

    pub fn save_design_visual_evaluation(
        &self,
        row: &DesignVisualEvaluationRow,
    ) -> AcResult<()> {
        self.connection
            .execute(
                "INSERT INTO design_visual_evaluations VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                params![
                    row.id,
                    row.artifact_version_id,
                    row.passed,
                    row.findings,
                    row.responsive_viewports,
                    row.accessibility_checks,
                    row.functional_flows,
                    row.evidence_refs,
                    row.created_at_ms
                ],
            )
            .map_err(db_error)?;
        Ok(())
    }

    pub fn design_visual_evaluations(
        &self,
        artifact_version_id: &str,
    ) -> AcResult<Vec<DesignVisualEvaluationRow>> {
        let mut stmt = self
            .connection
            .prepare(
                "SELECT id, artifact_version_id, passed, findings, responsive_viewports,
                 accessibility_checks, functional_flows, evidence_refs, created_at_ms
                 FROM design_visual_evaluations
                 WHERE artifact_version_id=?1 ORDER BY created_at_ms ASC",
            )
            .map_err(db_error)?;
        let rows = stmt
            .query_map([artifact_version_id], |row| {
                Ok(DesignVisualEvaluationRow {
                    id: row.get(0)?,
                    artifact_version_id: row.get(1)?,
                    passed: row.get(2)?,
                    findings: row.get(3)?,
                    responsive_viewports: row.get(4)?,
                    accessibility_checks: row.get(5)?,
                    functional_flows: row.get(6)?,
                    evidence_refs: row.get(7)?,
                    created_at_ms: row.get(8)?,
                })
            })
            .map_err(db_error)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(db_error)
    }
}
