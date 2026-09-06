impl ControlPlaneDb {
    pub fn save_code_index(
        &self,
        repository_id: &str,
        root: &str,
        commit: &str,
        files: &[(String, String, String)],
        symbols: &[(String, String, String, u32)],
        imports: &[(String, String, u32)],
    ) -> AcResult<()> {
        self.connection.execute("INSERT INTO code_repositories (id, root, commit_ref, indexed_at_ms) VALUES (?1, ?2, ?3, ?4) ON CONFLICT(id) DO UPDATE SET root=excluded.root, commit_ref=excluded.commit_ref, indexed_at_ms=excluded.indexed_at_ms", params![repository_id, root, commit, millis(TimestampMillis::now())]).map_err(db_error)?;
        self.connection
            .execute(
                "DELETE FROM code_files WHERE repository_id=?1",
                params![repository_id],
            )
            .map_err(db_error)?;
        self.connection
            .execute(
                "DELETE FROM code_symbols WHERE repository_id=?1",
                params![repository_id],
            )
            .map_err(db_error)?;
        self.connection
            .execute(
                "DELETE FROM code_imports WHERE repository_id=?1",
                params![repository_id],
            )
            .map_err(db_error)?;
        for (path, hash, language) in files {
            self.connection
                .execute(
                    "INSERT INTO code_files VALUES (?1, ?2, ?3, ?4, '{}')",
                    params![repository_id, path, hash, language],
                )
                .map_err(db_error)?;
        }
        for (path, name, kind, line) in symbols {
            self.connection
                .execute(
                    "INSERT INTO code_symbols VALUES (?1, ?2, ?3, ?4, ?5)",
                    params![repository_id, path, name, kind, line],
                )
                .map_err(db_error)?;
        }
        for (path, target, line) in imports {
            self.connection
                .execute(
                    "INSERT INTO code_imports VALUES (?1, ?2, ?3, ?4)",
                    params![repository_id, path, target, line],
                )
                .map_err(db_error)?;
        }
        Ok(())
    }

    pub fn code_symbol_count(&self, repository_id: &str) -> AcResult<u64> {
        self.connection
            .query_row(
                "SELECT COUNT(*) FROM code_symbols WHERE repository_id=?1",
                params![repository_id],
                |row| row.get(0),
            )
            .map_err(db_error)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn save_semantic_index(
        &self,
        repository_id: &str,
        commit: &str,
        worktree_id: &str,
        lsp_sessions: &[LspSessionRow],
        edges: &[SemanticEdgeRow],
        diagnostics: &[SemanticDiagnosticRow],
        workspace_boundaries: &[WorkspaceBoundaryRow],
        optional_indexes: &[OptionalIndexDecisionRow],
    ) -> AcResult<()> {
        for table in [
            "lsp_server_sessions",
            "semantic_edges",
            "semantic_diagnostics",
            "workspace_boundaries",
            "optional_index_decisions",
        ] {
            self.connection
                .execute(
                    &format!("DELETE FROM {table} WHERE repository_id=?1"),
                    params![repository_id],
                )
                .map_err(db_error)?;
        }
        let now = millis(TimestampMillis::now());
        for (id, server, root, pid, state, restart_count) in lsp_sessions {
            self.connection
                .execute(
                    "INSERT INTO lsp_server_sessions (
                        id, repository_id, server_type, workspace_root, pid, state,
                        restart_count, last_activity_ms, degraded_reason
                     ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, NULL)",
                    params![
                        id,
                        repository_id,
                        server,
                        root,
                        pid,
                        state,
                        restart_count,
                        now
                    ],
                )
                .map_err(db_error)?;
        }
        for (kind, from_path, from_symbol, to_path, to_symbol, source, confidence) in edges {
            self.connection
                .execute(
                    "INSERT INTO semantic_edges (
                        id, repository_id, edge_kind, from_path, from_symbol, to_path, to_symbol,
                        provenance_source, confidence, freshness, commit_ref, worktree_id, created_at_ms
                     ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, 'fresh', ?10, ?11, ?12)",
                    params![
                        StableId::new("edge").to_string(),
                        repository_id,
                        kind,
                        from_path,
                        from_symbol,
                        to_path,
                        to_symbol,
                        source,
                        confidence,
                        commit,
                        worktree_id,
                        now
                    ],
                )
                .map_err(db_error)?;
        }
        for (path, line, severity, message, confidence) in diagnostics {
            self.connection
                .execute(
                    "INSERT INTO semantic_diagnostics (
                        id, repository_id, path, line, severity, message, provenance_source,
                        confidence, freshness, commit_ref, worktree_id, created_at_ms
                     ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'lsp-diagnostics-normalized', ?7, 'fresh', ?8, ?9, ?10)",
                    params![
                        StableId::new("diag").to_string(),
                        repository_id,
                        path,
                        line,
                        severity,
                        message,
                        confidence,
                        commit,
                        worktree_id,
                        now
                    ],
                )
                .map_err(db_error)?;
        }
        for (kind, root_path, package_name, evidence_path) in workspace_boundaries {
            self.connection
                .execute(
                    "INSERT INTO workspace_boundaries VALUES (?1, ?2, ?3, ?4, ?5)",
                    params![repository_id, kind, root_path, package_name, evidence_path],
                )
                .map_err(db_error)?;
        }
        for (engine, enabled, reason, measured_files, measured_edges) in optional_indexes {
            self.connection
                .execute(
                    "INSERT INTO optional_index_decisions VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                    params![
                        repository_id,
                        engine,
                        if *enabled { 1_i64 } else { 0_i64 },
                        reason,
                        *measured_files as i64,
                        *measured_edges as i64,
                        now
                    ],
                )
                .map_err(db_error)?;
        }
        Ok(())
    }

    pub fn semantic_edge_count(&self, repository_id: &str) -> AcResult<u64> {
        self.connection
            .query_row(
                "SELECT COUNT(*) FROM semantic_edges WHERE repository_id=?1",
                params![repository_id],
                |row| row.get(0),
            )
            .map_err(db_error)
    }

    pub fn workspace_boundary_count(&self, repository_id: &str) -> AcResult<u64> {
        self.connection
            .query_row(
                "SELECT COUNT(*) FROM workspace_boundaries WHERE repository_id=?1",
                params![repository_id],
                |row| row.get(0),
            )
            .map_err(db_error)
    }

    pub fn optional_index_enabled(
        &self,
        repository_id: &str,
        engine: &str,
    ) -> AcResult<Option<bool>> {
        self.connection
            .query_row(
                "SELECT enabled FROM optional_index_decisions WHERE repository_id=?1 AND engine=?2",
                params![repository_id, engine],
                |row| row.get::<_, i64>(0),
            )
            .optional()
            .map(|value| value.map(|enabled| enabled != 0))
            .map_err(db_error)
    }

    pub fn save_memory_fact(
        &self,
        fact: &MemoryFactRow,
        evidence: &[MemoryEvidenceRow],
    ) -> AcResult<()> {
        if fact.statement.trim().is_empty() || evidence.is_empty() {
            return Err(AcError::validation(
                "DB-MEMORY_FACT_INVALID",
                "memory facts require statement and evidence",
            ));
        }
        self.connection
            .execute(
                "INSERT INTO memory_facts (
                    id, repository_id, mission_id, task_id, branch, statement, fact_type, source,
                    confidence, freshness, memory_class, observed_commit, conflict_set_id,
                    valid_from_ms, valid_until_ms, superseded_by, last_validation_ms
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17)
                 ON CONFLICT(id) DO UPDATE SET
                    statement=excluded.statement,
                    confidence=excluded.confidence,
                    freshness=excluded.freshness,
                    conflict_set_id=excluded.conflict_set_id,
                    valid_until_ms=excluded.valid_until_ms,
                    superseded_by=excluded.superseded_by,
                    last_validation_ms=excluded.last_validation_ms",
                params![
                    fact.id,
                    fact.repository_id,
                    fact.mission_id,
                    fact.task_id,
                    fact.branch,
                    fact.statement,
                    fact.fact_type,
                    fact.source,
                    fact.confidence,
                    fact.freshness,
                    fact.memory_class,
                    fact.observed_commit,
                    fact.conflict_set_id,
                    fact.valid_from_ms,
                    fact.valid_until_ms,
                    fact.superseded_by,
                    fact.last_validation_ms
                ],
            )
            .map_err(db_error)?;
        self.connection
            .execute(
                "DELETE FROM memory_fact_evidence WHERE fact_id=?1",
                params![fact.id],
            )
            .map_err(db_error)?;
        for row in evidence {
            self.connection
                .execute(
                    "INSERT INTO memory_fact_evidence VALUES (?1, ?2, ?3, ?4, ?5)",
                    params![
                        row.fact_id,
                        row.evidence_ref,
                        row.file_path,
                        row.symbol,
                        row.content_hash
                    ],
                )
                .map_err(db_error)?;
        }
        Ok(())
    }

    /// Repository-scoped live (not superseded) memory facts, newest first
    /// (F1: hydration for agents + cross-mode reads).
    pub fn memory_facts_for(
        &self,
        repository_id: &str,
        limit: usize,
    ) -> AcResult<Vec<MemoryFactRow>> {
        let mut statement = self
            .connection
            .prepare(
                "SELECT id, repository_id, mission_id, task_id, branch, statement, fact_type,
                        source, confidence, freshness, memory_class, observed_commit,
                        conflict_set_id, valid_from_ms, valid_until_ms, superseded_by,
                        last_validation_ms
                 FROM memory_facts WHERE repository_id=?1
                 ORDER BY last_validation_ms DESC LIMIT ?2",
            )
            .map_err(db_error)?;
        let rows = statement
            .query_map(params![repository_id, limit as i64], memory_fact_from_row)
            .map_err(db_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(db_error)?;
        Ok(rows)
    }

    /// Repository-scoped task memories, newest first (F1 hydration).
    pub fn task_memories_newest(&self, limit: usize) -> AcResult<Vec<TaskMemoryRow>> {
        let mut statement = self
            .connection
            .prepare(
                "SELECT id, task_id, summary, evidence_refs, created_at_ms
                 FROM task_memory ORDER BY created_at_ms DESC LIMIT ?1",
            )
            .map_err(db_error)?;
        let rows = statement
            .query_map(params![limit as i64], |row| {
                Ok(TaskMemoryRow {
                    id: row.get(0)?,
                    task_id: row.get(1)?,
                    summary: row.get(2)?,
                    evidence_refs: row.get(3)?,
                    created_at_ms: row.get(4)?,
                })
            })
            .map_err(db_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(db_error)?;
        Ok(rows)
    }

    pub fn memory_fact(&self, id: &str) -> AcResult<Option<MemoryFactRow>> {
        self.connection
            .query_row(
                "SELECT id, repository_id, mission_id, task_id, branch, statement, fact_type,
                        source, confidence, freshness, memory_class, observed_commit,
                        conflict_set_id, valid_from_ms, valid_until_ms, superseded_by,
                        last_validation_ms
                 FROM memory_facts WHERE id=?1",
                params![id],
                memory_fact_from_row,
            )
            .optional()
            .map_err(db_error)
    }

    pub fn memory_fact_evidence(&self, fact_id: &str) -> AcResult<Vec<MemoryEvidenceRow>> {
        let mut stmt = self
            .connection
            .prepare(
                "SELECT fact_id, evidence_ref, file_path, symbol, content_hash
                 FROM memory_fact_evidence WHERE fact_id=?1 ORDER BY evidence_ref ASC",
            )
            .map_err(db_error)?;
        let rows = stmt
            .query_map(params![fact_id], |row| {
                Ok(MemoryEvidenceRow {
                    fact_id: row.get(0)?,
                    evidence_ref: row.get(1)?,
                    file_path: row.get(2)?,
                    symbol: row.get(3)?,
                    content_hash: row.get(4)?,
                })
            })
            .map_err(db_error)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(db_error)
    }

    pub fn mark_memory_for_source_change(
        &self,
        file_path: &str,
        symbol: Option<&str>,
        deleted: bool,
    ) -> AcResult<u64> {
        let freshness = if deleted { "INVALID" } else { "POSSIBLY_STALE" };
        let changed = if let Some(symbol) = symbol {
            self.connection.execute(
                "UPDATE memory_facts
                 SET freshness=?3, last_validation_ms=?4
                 WHERE id IN (
                    SELECT fact_id FROM memory_fact_evidence
                    WHERE file_path=?1 AND (symbol IS NULL OR symbol=?2)
                 )",
                params![file_path, symbol, freshness, millis(TimestampMillis::now())],
            )
        } else {
            self.connection.execute(
                "UPDATE memory_facts
                 SET freshness=?2, last_validation_ms=?3
                 WHERE id IN (
                    SELECT fact_id FROM memory_fact_evidence WHERE file_path=?1
                 )",
                params![file_path, freshness, millis(TimestampMillis::now())],
            )
        }
        .map_err(db_error)?;
        Ok(changed as u64)
    }

    pub fn save_semantic_chunk(&self, chunk: &SemanticChunkRow) -> AcResult<()> {
        if chunk.dimension == 0 || chunk.vector.len() != chunk.dimension as usize {
            return Err(AcError::validation("DB-SEMANTIC_VECTOR_INVALID", "semantic vector dimension does not match its payload"));
        }
        let vector = serde_json::to_string(&chunk.vector)
            .map_err(|error| AcError::validation("DB-SEMANTIC_VECTOR_SERIALIZE", error.to_string()))?;
        self.connection.execute(
            "INSERT INTO semantic_chunks (id, repository_id, fact_id, content, content_hash, model_id, dimension, vector_json, freshness, created_at_ms)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
             ON CONFLICT(repository_id, fact_id, content_hash, model_id) DO UPDATE SET vector_json=excluded.vector_json, dimension=excluded.dimension, freshness=excluded.freshness, created_at_ms=excluded.created_at_ms",
            params![chunk.id, chunk.repository_id, chunk.fact_id, chunk.content, chunk.content_hash, chunk.model_id, chunk.dimension, vector, chunk.freshness, chunk.created_at_ms],
        ).map_err(db_error)?;
        Ok(())
    }

    pub fn semantic_chunks(&self, repository_id: &str, model_id: &str) -> AcResult<Vec<SemanticChunkRow>> {
        let mut statement = self.connection.prepare(
            "SELECT id, repository_id, fact_id, content, content_hash, model_id, dimension, vector_json, freshness, created_at_ms
             FROM semantic_chunks WHERE repository_id=?1 AND model_id=?2 ORDER BY created_at_ms ASC",
        ).map_err(db_error)?;
        let rows = statement.query_map(params![repository_id, model_id], |row| {
            let vector_json: String = row.get(7)?;
            let vector = serde_json::from_str::<Vec<f32>>(&vector_json).map_err(|error| rusqlite::Error::FromSqlConversionFailure(7, rusqlite::types::Type::Text, Box::new(error)))?;
            Ok(SemanticChunkRow { id: row.get(0)?, repository_id: row.get(1)?, fact_id: row.get(2)?, content: row.get(3)?, content_hash: row.get(4)?, model_id: row.get(5)?, dimension: row.get::<_, u32>(6)?, vector, freshness: row.get(8)?, created_at_ms: row.get(9)? })
        }).map_err(db_error)?;
        let mut chunks = Vec::new();
        for chunk in rows.flatten() {
            if chunk.vector.len() == chunk.dimension as usize { chunks.push(chunk); }
        }
        Ok(chunks)
    }

    pub fn save_memory_decision(&self, decision: &MemoryDecisionRow) -> AcResult<()> {
        if decision.decision.trim().is_empty() || decision.authority_refs.trim().is_empty() {
            return Err(AcError::validation(
                "DB-MEMORY_DECISION_INVALID",
                "decisions require text and authority refs",
            ));
        }
        self.connection
            .execute(
                "INSERT INTO memory_decisions VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                params![
                    decision.id,
                    decision.repository_id,
                    decision.mission_id,
                    decision.task_id,
                    decision.branch,
                    decision.decision,
                    decision.rationale,
                    decision.authority_refs,
                    decision.supersedes,
                    decision.created_at_ms
                ],
            )
            .map_err(db_error)?;
        Ok(())
    }

    pub fn memory_decision_count(&self, repository_id: &str) -> AcResult<u64> {
        self.connection
            .query_row(
                "SELECT COUNT(*) FROM memory_decisions WHERE repository_id=?1",
                params![repository_id],
                |row| row.get(0),
            )
            .map_err(db_error)
    }

    /// Read-only listing of accepted project decisions for a repository
    /// identity — used by Discuss/Design memory inheritance.  Never used
    /// as a write path (single-writer rule).
    pub fn memory_decisions_for(
        &self,
        repository_id: &str,
        limit: usize,
    ) -> AcResult<Vec<MemoryDecisionRow>> {
        let mut statement = self
            .connection
            .prepare(
                "SELECT id, repository_id, mission_id, task_id, branch, decision, rationale,
                 authority_refs, supersedes, created_at_ms
                 FROM memory_decisions WHERE repository_id=?1
                 ORDER BY created_at_ms DESC LIMIT ?2",
            )
            .map_err(db_error)?;
        let rows = statement
            .query_map(params![repository_id, limit as i64], |row| {
                Ok(MemoryDecisionRow {
                    id: row.get(0)?,
                    repository_id: row.get(1)?,
                    mission_id: row.get(2)?,
                    task_id: row.get(3)?,
                    branch: row.get(4)?,
                    decision: row.get(5)?,
                    rationale: row.get(6)?,
                    authority_refs: row.get(7)?,
                    supersedes: row.get(8)?,
                    created_at_ms: row.get(9)?,
                })
            })
            .map_err(db_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(db_error)?;
        Ok(rows)
    }

    pub fn save_task_memory(&self, memory: &TaskMemoryRow) -> AcResult<()> {
        self.connection
            .execute(
                "INSERT INTO task_memory VALUES (?1, ?2, ?3, ?4, ?5)",
                params![
                    memory.id,
                    memory.task_id,
                    memory.summary,
                    memory.evidence_refs,
                    memory.created_at_ms
                ],
            )
            .map_err(db_error)?;
        Ok(())
    }

    pub fn task_memory_count(&self, task_id: &str) -> AcResult<u64> {
        self.connection
            .query_row(
                "SELECT COUNT(*) FROM task_memory WHERE task_id=?1",
                params![task_id],
                |row| row.get(0),
            )
            .map_err(db_error)
    }

    pub fn save_context_snapshot(&self, snapshot: &ContextSnapshotRow) -> AcResult<()> {
        self.connection
            .execute(
                "INSERT INTO context_snapshots VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                params![
                    snapshot.id,
                    snapshot.repository_id,
                    snapshot.mission_id,
                    snapshot.task_id,
                    snapshot.branch,
                    snapshot.reason,
                    snapshot.content,
                    snapshot.source_fact_ids,
                    snapshot.decision_refs,
                    snapshot.created_at_ms
                ],
            )
            .map_err(db_error)?;
        Ok(())
    }

    pub fn context_snapshot(&self, id: &str) -> AcResult<Option<ContextSnapshotRow>> {
        self.connection
            .query_row(
                "SELECT id, repository_id, mission_id, task_id, branch, reason, content,
                        source_fact_ids, decision_refs, created_at_ms
                 FROM context_snapshots WHERE id=?1",
                params![id],
                |row| {
                    Ok(ContextSnapshotRow {
                        id: row.get(0)?,
                        repository_id: row.get(1)?,
                        mission_id: row.get(2)?,
                        task_id: row.get(3)?,
                        branch: row.get(4)?,
                        reason: row.get(5)?,
                        content: row.get(6)?,
                        source_fact_ids: row.get(7)?,
                        decision_refs: row.get(8)?,
                        created_at_ms: row.get(9)?,
                    })
                },
            )
            .optional()
            .map_err(db_error)
    }

    pub fn save_context_pack_manifest(&self, manifest: &ContextPackManifestRow) -> AcResult<()> {
        if manifest.source_fragment_ids.trim().is_empty() {
            return Err(AcError::validation(
                "DB-CONTEXT_MANIFEST_INVALID",
                "context manifest requires source fragment provenance",
            ));
        }
        self.connection
            .execute(
                "INSERT INTO context_pack_manifests VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
                params![
                    manifest.id,
                    manifest.pack_id,
                    manifest.task_id,
                    manifest.role,
                    manifest.profile,
                    manifest.source_fragment_ids,
                    manifest.omitted_fragment_ids,
                    manifest.raw_evidence_refs,
                    manifest.cache_keys,
                    manifest.score_trace,
                    manifest.total_input_tokens,
                    manifest.hard_ceiling,
                    manifest.created_at_ms
                ],
            )
            .map_err(db_error)?;
        Ok(())
    }

    pub fn context_pack_manifest(&self, id: &str) -> AcResult<Option<ContextPackManifestRow>> {
        self.connection
            .query_row(
                "SELECT id, pack_id, task_id, role, profile, source_fragment_ids,
                        omitted_fragment_ids, raw_evidence_refs, cache_keys, score_trace,
                        total_input_tokens, hard_ceiling, created_at_ms
                 FROM context_pack_manifests WHERE id=?1",
                params![id],
                |row| {
                    Ok(ContextPackManifestRow {
                        id: row.get(0)?,
                        pack_id: row.get(1)?,
                        task_id: row.get(2)?,
                        role: row.get(3)?,
                        profile: row.get(4)?,
                        source_fragment_ids: row.get(5)?,
                        omitted_fragment_ids: row.get(6)?,
                        raw_evidence_refs: row.get(7)?,
                        cache_keys: row.get(8)?,
                        score_trace: row.get(9)?,
                        total_input_tokens: row.get(10)?,
                        hard_ceiling: row.get(11)?,
                        created_at_ms: row.get(12)?,
                    })
                },
            )
            .optional()
            .map_err(db_error)
    }

    pub fn save_context_compression_receipt(
        &self,
        receipt: &ContextCompressionReceiptRow,
    ) -> AcResult<()> {
        self.connection
            .execute(
                "INSERT INTO context_compression_receipts VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                params![
                    receipt.id,
                    receipt.raw_evidence_ref,
                    receipt.command_class,
                    receipt.compressor_id,
                    receipt.raw_hash,
                    receipt.compressed_output,
                    receipt.raw_token_estimate,
                    receipt.compressed_token_estimate,
                    receipt.omitted_lines,
                    receipt.created_at_ms
                ],
            )
            .map_err(db_error)?;
        Ok(())
    }

    pub fn context_compression_receipt(
        &self,
        id: &str,
    ) -> AcResult<Option<ContextCompressionReceiptRow>> {
        self.connection
            .query_row(
                "SELECT id, raw_evidence_ref, command_class, compressor_id, raw_hash,
                        compressed_output, raw_token_estimate, compressed_token_estimate,
                        omitted_lines, created_at_ms
                 FROM context_compression_receipts WHERE id=?1",
                params![id],
                |row| {
                    Ok(ContextCompressionReceiptRow {
                        id: row.get(0)?,
                        raw_evidence_ref: row.get(1)?,
                        command_class: row.get(2)?,
                        compressor_id: row.get(3)?,
                        raw_hash: row.get(4)?,
                        compressed_output: row.get(5)?,
                        raw_token_estimate: row.get(6)?,
                        compressed_token_estimate: row.get(7)?,
                        omitted_lines: row.get(8)?,
                        created_at_ms: row.get(9)?,
                    })
                },
            )
            .optional()
            .map_err(db_error)
    }

    pub fn save_context_cache_entry(&self, entry: &ContextCacheEntryRow) -> AcResult<()> {
        self.connection
            .execute(
                "INSERT INTO context_cache_entries VALUES (?1, ?2, ?3, ?4, ?5)
                 ON CONFLICT(cache_key) DO UPDATE SET
                    content_hash=excluded.content_hash,
                    token_estimate=excluded.token_estimate,
                    source_ref=excluded.source_ref,
                    created_at_ms=excluded.created_at_ms",
                params![
                    entry.cache_key,
                    entry.content_hash,
                    entry.token_estimate,
                    entry.source_ref,
                    entry.created_at_ms
                ],
            )
            .map_err(db_error)?;
        Ok(())
    }

    pub fn context_cache_entry(&self, cache_key: &str) -> AcResult<Option<ContextCacheEntryRow>> {
        self.connection
            .query_row(
                "SELECT cache_key, content_hash, token_estimate, source_ref, created_at_ms
                 FROM context_cache_entries WHERE cache_key=?1",
                params![cache_key],
                |row| {
                    Ok(ContextCacheEntryRow {
                        cache_key: row.get(0)?,
                        content_hash: row.get(1)?,
                        token_estimate: row.get(2)?,
                        source_ref: row.get(3)?,
                        created_at_ms: row.get(4)?,
                    })
                },
            )
            .optional()
            .map_err(db_error)
    }

    pub fn save_context_retrieval_record(
        &self,
        record: &ContextRetrievalRecordRow,
    ) -> AcResult<()> {
        self.connection
            .execute(
                "INSERT INTO context_retrieval_records VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                params![
                    record.id,
                    record.pack_id,
                    record.need,
                    record.reason,
                    record.query,
                    record.result_fragment_ids,
                    record.added_tokens,
                    if record.degraded { 1_i64 } else { 0_i64 },
                    record.created_at_ms
                ],
            )
            .map_err(db_error)?;
        Ok(())
    }

    pub fn context_retrieval_records(
        &self,
        pack_id: &str,
    ) -> AcResult<Vec<ContextRetrievalRecordRow>> {
        let mut stmt = self
            .connection
            .prepare(
                "SELECT id, pack_id, need, reason, query, result_fragment_ids, added_tokens,
                        degraded, created_at_ms
                 FROM context_retrieval_records WHERE pack_id=?1 ORDER BY created_at_ms ASC",
            )
            .map_err(db_error)?;
        let rows = stmt
            .query_map(params![pack_id], |row| {
                Ok(ContextRetrievalRecordRow {
                    id: row.get(0)?,
                    pack_id: row.get(1)?,
                    need: row.get(2)?,
                    reason: row.get(3)?,
                    query: row.get(4)?,
                    result_fragment_ids: row.get(5)?,
                    added_tokens: row.get(6)?,
                    degraded: row.get::<_, i64>(7)? != 0,
                    created_at_ms: row.get(8)?,
                })
            })
            .map_err(db_error)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(db_error)
    }

    pub fn save_context_pack_metrics(&self, metrics: &ContextPackMetricsRow) -> AcResult<()> {
        self.connection
            .execute(
                "INSERT INTO context_pack_metrics VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
                params![
                    metrics.id,
                    metrics.pack_id,
                    metrics.role,
                    metrics.selected_fragments as i64,
                    metrics.omitted_fragments as i64,
                    metrics.total_input_tokens,
                    metrics.budget_target,
                    metrics.hard_ceiling,
                    metrics.deduped_fragments as i64,
                    metrics.redacted_fragments as i64,
                    metrics.retrieval_steps as i64,
                    metrics.cache_hits as i64,
                    metrics.created_at_ms
                ],
            )
            .map_err(db_error)?;
        Ok(())
    }

    pub fn context_pack_metrics(&self, pack_id: &str) -> AcResult<Option<ContextPackMetricsRow>> {
        self.connection
            .query_row(
                "SELECT id, pack_id, role, selected_fragments, omitted_fragments,
                        total_input_tokens, budget_target, hard_ceiling, deduped_fragments,
                        redacted_fragments, retrieval_steps, cache_hits, created_at_ms
                 FROM context_pack_metrics WHERE pack_id=?1",
                params![pack_id],
                |row| {
                    Ok(ContextPackMetricsRow {
                        id: row.get(0)?,
                        pack_id: row.get(1)?,
                        role: row.get(2)?,
                        selected_fragments: row.get::<_, i64>(3)? as usize,
                        omitted_fragments: row.get::<_, i64>(4)? as usize,
                        total_input_tokens: row.get(5)?,
                        budget_target: row.get(6)?,
                        hard_ceiling: row.get(7)?,
                        deduped_fragments: row.get::<_, i64>(8)? as usize,
                        redacted_fragments: row.get::<_, i64>(9)? as usize,
                        retrieval_steps: row.get::<_, i64>(10)? as usize,
                        cache_hits: row.get::<_, i64>(11)? as usize,
                        created_at_ms: row.get(12)?,
                    })
                },
            )
            .optional()
            .map_err(db_error)
    }

    pub fn save_context_benchmark_result(
        &self,
        result: &ContextBenchmarkResultRow,
    ) -> AcResult<()> {
        self.connection
            .execute(
                "INSERT INTO context_benchmark_results VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                params![
                    result.id,
                    result.task_name,
                    result.broad_tokens,
                    result.targeted_tokens,
                    if result.broad_success { 1_i64 } else { 0_i64 },
                    if result.targeted_success { 1_i64 } else { 0_i64 },
                    result.retry_delta,
                    result.latency_delta_ms,
                    if result.passed { 1_i64 } else { 0_i64 },
                    result.created_at_ms
                ],
            )
            .map_err(db_error)?;
        Ok(())
    }

    pub fn context_benchmark_result(
        &self,
        id: &str,
    ) -> AcResult<Option<ContextBenchmarkResultRow>> {
        self.connection
            .query_row(
                "SELECT id, task_name, broad_tokens, targeted_tokens, broad_success,
                        targeted_success, retry_delta, latency_delta_ms, passed, created_at_ms
                 FROM context_benchmark_results WHERE id=?1",
                params![id],
                |row| {
                    Ok(ContextBenchmarkResultRow {
                        id: row.get(0)?,
                        task_name: row.get(1)?,
                        broad_tokens: row.get(2)?,
                        targeted_tokens: row.get(3)?,
                        broad_success: row.get::<_, i64>(4)? != 0,
                        targeted_success: row.get::<_, i64>(5)? != 0,
                        retry_delta: row.get(6)?,
                        latency_delta_ms: row.get(7)?,
                        passed: row.get::<_, i64>(8)? != 0,
                        created_at_ms: row.get(9)?,
                    })
                },
            )
            .optional()
            .map_err(db_error)
    }
}
