impl ControlPlaneDb {
    pub fn save_design_document(&self, row: &DesignDocumentRow) -> AcResult<()> {
        self.connection
            .execute(
                "INSERT INTO design_documents VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
                 ON CONFLICT(id) DO UPDATE SET content_json=excluded.content_json,
                 version=excluded.version, evidence_refs=excluded.evidence_refs,
                 updated_at_ms=excluded.updated_at_ms",
                params![
                    row.id,
                    row.conversation_id,
                    row.doc_type,
                    row.content_json,
                    row.version,
                    row.evidence_refs,
                    row.created_at_ms,
                    row.updated_at_ms
                ],
            )
            .map_err(db_error)?;
        Ok(())
    }

    pub fn design_document(
        &self,
        conversation_id: &str,
        doc_type: &str,
    ) -> AcResult<Option<DesignDocumentRow>> {
        self.connection
            .query_row(
                "SELECT id, conversation_id, doc_type, content_json, version, evidence_refs,
                 created_at_ms, updated_at_ms
                 FROM design_documents
                 WHERE conversation_id=?1 AND doc_type=?2 ORDER BY updated_at_ms DESC LIMIT 1",
                params![conversation_id, doc_type],
                |row| {
                    Ok(DesignDocumentRow {
                        id: row.get(0)?,
                        conversation_id: row.get(1)?,
                        doc_type: row.get(2)?,
                        content_json: row.get(3)?,
                        version: row.get(4)?,
                        evidence_refs: row.get(5)?,
                        created_at_ms: row.get(6)?,
                        updated_at_ms: row.get(7)?,
                    })
                },
            )
            .optional()
            .map_err(db_error)
    }

    pub fn design_documents(&self, conversation_id: &str) -> AcResult<Vec<DesignDocumentRow>> {
        let mut stmt = self
            .connection
            .prepare(
                "SELECT id, conversation_id, doc_type, content_json, version, evidence_refs,
                 created_at_ms, updated_at_ms
                 FROM design_documents WHERE conversation_id=?1 ORDER BY doc_type ASC",
            )
            .map_err(db_error)?;
        let rows = stmt
            .query_map([conversation_id], |row| {
                Ok(DesignDocumentRow {
                    id: row.get(0)?,
                    conversation_id: row.get(1)?,
                    doc_type: row.get(2)?,
                    content_json: row.get(3)?,
                    version: row.get(4)?,
                    evidence_refs: row.get(5)?,
                    created_at_ms: row.get(6)?,
                    updated_at_ms: row.get(7)?,
                })
            })
            .map_err(db_error)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(db_error)
    }

    /// All design documents with a given doc_type, regardless of
    /// conversation — used for project-scoped design memory (constraints)
    /// keyed by a deterministic per-project doc_type.
    pub fn design_documents_by_type(
        &self,
        doc_type: &str,
    ) -> AcResult<Vec<DesignDocumentRow>> {
        let mut stmt = self
            .connection
            .prepare(
                "SELECT id, conversation_id, doc_type, content_json, version, evidence_refs,
                 created_at_ms, updated_at_ms
                 FROM design_documents WHERE doc_type=?1 ORDER BY updated_at_ms ASC",
            )
            .map_err(db_error)?;
        let rows = stmt
            .query_map([doc_type], |row| {
                Ok(DesignDocumentRow {
                    id: row.get(0)?,
                    conversation_id: row.get(1)?,
                    doc_type: row.get(2)?,
                    content_json: row.get(3)?,
                    version: row.get(4)?,
                    evidence_refs: row.get(5)?,
                    created_at_ms: row.get(6)?,
                    updated_at_ms: row.get(7)?,
                })
            })
            .map_err(db_error)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(db_error)
    }

    pub fn save_design_preview(&self, row: &DesignPreviewRow) -> AcResult<()> {
        self.connection
            .execute(
                "INSERT INTO design_previews VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
                 ON CONFLICT(id) DO UPDATE SET port=excluded.port,
                 ready_url=excluded.ready_url, process_id=excluded.process_id,
                 process_alive=excluded.process_alive, http_ready=excluded.http_ready,
                 browser_session_id=excluded.browser_session_id,
                 updated_at_ms=excluded.updated_at_ms",
                params![
                    row.id,
                    row.conversation_id,
                    row.port,
                    row.ready_url,
                    row.process_id,
                    row.process_alive,
                    row.http_ready,
                    row.browser_session_id,
                    row.created_at_ms,
                    row.updated_at_ms
                ],
            )
            .map_err(db_error)?;
        Ok(())
    }

    pub fn design_preview(
        &self,
        conversation_id: &str,
    ) -> AcResult<Option<DesignPreviewRow>> {
        self.connection
            .query_row(
                "SELECT id, conversation_id, port, ready_url, process_id, process_alive,
                 http_ready, browser_session_id, created_at_ms, updated_at_ms
                 FROM design_previews WHERE conversation_id=?1 ORDER BY updated_at_ms DESC LIMIT 1",
                [conversation_id],
                |row| {
                    Ok(DesignPreviewRow {
                        id: row.get(0)?,
                        conversation_id: row.get(1)?,
                        port: row.get(2)?,
                        ready_url: row.get(3)?,
                        process_id: row.get(4)?,
                        process_alive: row.get(5)?,
                        http_ready: row.get(6)?,
                        browser_session_id: row.get(7)?,
                        created_at_ms: row.get(8)?,
                        updated_at_ms: row.get(9)?,
                    })
                },
            )
            .optional()
            .map_err(db_error)
    }

    pub fn save_design_critique(&self, row: &DesignCritiqueRow) -> AcResult<()> {
        self.connection
            .execute(
                "INSERT INTO design_critiques VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![
                    row.id,
                    row.conversation_id,
                    row.doc_type,
                    row.passed,
                    row.findings_json,
                    row.improvement_required,
                    row.evidence_refs,
                    row.created_at_ms
                ],
            )
            .map_err(db_error)?;
        Ok(())
    }

    pub fn design_critiques(&self, conversation_id: &str) -> AcResult<Vec<DesignCritiqueRow>> {
        let mut stmt = self
            .connection
            .prepare(
                "SELECT id, conversation_id, doc_type, passed, findings_json,
                 improvement_required, evidence_refs, created_at_ms
                 FROM design_critiques WHERE conversation_id=?1 ORDER BY created_at_ms ASC",
            )
            .map_err(db_error)?;
        let rows = stmt
            .query_map([conversation_id], |row| {
                Ok(DesignCritiqueRow {
                    id: row.get(0)?,
                    conversation_id: row.get(1)?,
                    doc_type: row.get(2)?,
                    passed: row.get(3)?,
                    findings_json: row.get(4)?,
                    improvement_required: row.get(5)?,
                    evidence_refs: row.get(6)?,
                    created_at_ms: row.get(7)?,
                })
            })
            .map_err(db_error)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(db_error)
    }
}