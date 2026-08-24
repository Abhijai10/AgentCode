impl ControlPlaneDb {
    pub fn open(path: impl AsRef<Path>) -> AcResult<Self> {
        let connection = Connection::open(path).map_err(db_error)?;
        let db = Self { connection };
        db.configure()?;
        Ok(db)
    }

    pub fn open_memory() -> AcResult<Self> {
        let connection = Connection::open_in_memory().map_err(db_error)?;
        let db = Self { connection };
        db.configure()?;
        Ok(db)
    }

    pub fn migrate(&mut self) -> AcResult<()> {
        let current_version = self.user_version()?;
        if current_version > 12 {
            return Err(AcError::conflict(
                "DB-FUTURE_VERSION",
                format!(
                    "database user_version {current_version} is newer than supported version 12"
                ),
            ));
        }
        let tx = self.connection.transaction().map_err(db_error)?;
        tx.execute_batch(include_str!("../../../migrations/0001_kernel_schema.sql"))
            .map_err(db_error)?;
        if current_version < 2 {
            tx.execute_batch(include_str!(
                "../../../migrations/0002_git_worktree_hardening.sql"
            ))
            .map_err(db_error)?;
        }
        if current_version < 3 {
            tx.execute_batch(include_str!(
                "../../../migrations/0003_code_intelligence.sql"
            ))
            .map_err(db_error)?;
        }
        if current_version < 4 {
            tx.execute_batch(include_str!(
                "../../../migrations/0004_semantic_repository_graph.sql"
            ))
            .map_err(db_error)?;
        }
        if current_version < 5 {
            tx.execute_batch(include_str!(
                "../../../migrations/0005_persistent_memory.sql"
            ))
            .map_err(db_error)?;
        }
        if current_version < 6 {
            tx.execute_batch(include_str!("../../../migrations/0006_context_engine.sql"))
                .map_err(db_error)?;
        }
        if current_version < 7 {
            tx.execute_batch(include_str!(
                "../../../migrations/0007_full_autonomy_kernel.sql"
            ))
            .map_err(db_error)?;
        }
        if current_version < 8 {
            tx.execute_batch(include_str!(
                "../../../migrations/0008_advanced_edit_engine.sql"
            ))
            .map_err(db_error)?;
        }
        if current_version < 9 {
            tx.execute_batch(include_str!(
                "../../../migrations/0009_verification_evidence_engine.sql"
            ))
            .map_err(db_error)?;
        }
        if current_version < 10 {
            tx.execute_batch(include_str!("../../../migrations/0010_browser_runtime.sql"))
                .map_err(db_error)?;
        }
        if current_version < 11 {
            tx.execute_batch(include_str!(
                "../../../migrations/0011_extensions_skills_hooks_mcp.sql"
            ))
            .map_err(db_error)?;
        }
        if current_version < 12 {
            tx.execute_batch(include_str!(
                "../../../migrations/0012_baseline_security.sql"
            ))
            .map_err(db_error)?;
        }
        tx.pragma_update(None, "user_version", 12)
            .map_err(db_error)?;
        tx.commit().map_err(db_error)?;
        Ok(())
    }

    pub fn user_version(&self) -> AcResult<u32> {
        self.connection
            .pragma_query_value(None, "user_version", |row| row.get(0))
            .map_err(db_error)
    }
}
