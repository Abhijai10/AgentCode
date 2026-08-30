pub const CURRENT_SCHEMA_VERSION: u32 = 22;

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
        if current_version > CURRENT_SCHEMA_VERSION {
            return Err(AcError::conflict(
                "DB-FUTURE_VERSION",
                format!(
                    "database user_version {current_version} is newer than supported version {CURRENT_SCHEMA_VERSION}"
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
        if current_version < 13 {
            tx.execute_batch(include_str!(
                "../../../migrations/0013_advanced_ai_security.sql"
            ))
            .map_err(db_error)?;
        }
        if current_version < 14 {
            tx.execute_batch(include_str!(
                "../../../migrations/0014_discuss_design_modes.sql"
            ))
            .map_err(db_error)?;
        }
        if current_version < 15 {
            tx.execute_batch(include_str!(
                "../../../migrations/0015_desktop_optimization.sql"
            ))
            .map_err(db_error)?;
        }
        if current_version < 16 {
            tx.execute_batch(include_str!(
                "../../../migrations/0016_chaos_dogfood.sql"
            ))
            .map_err(db_error)?;
        }
        if current_version < 17 {
            tx.execute_batch(include_str!(
                "../../../migrations/0017_security_release.sql"
            ))
            .map_err(db_error)?;
        }
        if current_version < 18 {
            tx.execute_batch(include_str!(
                "../../../migrations/0018_release_candidate_v1.sql"
            ))
            .map_err(db_error)?;
        }
        if current_version < 19 {
            tx.execute_batch(include_str!("../../../migrations/0019_daemon_semantic_memory.sql"))
                .map_err(db_error)?;
        }
        if current_version < 20 {
            tx.execute_batch(include_str!("../../../migrations/0020_task_acceptance_criteria.sql"))
                .map_err(db_error)?;
        }
        if current_version < 21 {
            add_column_if_missing(
                &tx,
                "tasks",
                "acceptance_criteria_json",
                "TEXT NOT NULL DEFAULT '[]'",
            )?;
            add_column_if_missing(&tx, "evidence_records", "raw_content", "TEXT")?;
            add_column_if_missing(&tx, "evidence_records", "model_summary", "TEXT")?;
            add_column_if_missing(
                &tx,
                "evidence_records",
                "sensitive",
                "INTEGER NOT NULL DEFAULT 0",
            )?;
        }
        if current_version < 22 {
            tx.execute_batch(include_str!(
                "../../../migrations/0021_provider_catalog.sql"
            ))
            .map_err(db_error)?;
        }
        tx.pragma_update(None, "user_version", CURRENT_SCHEMA_VERSION)
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

fn add_column_if_missing(
    tx: &rusqlite::Transaction<'_>,
    table: &str,
    column: &str,
    definition: &str,
) -> AcResult<()> {
    if table_has_column(tx, table, column)? {
        return Ok(());
    }
    tx.execute_batch(&format!("ALTER TABLE {table} ADD COLUMN {column} {definition}"))
        .map_err(db_error)
}

fn table_has_column(
    tx: &rusqlite::Transaction<'_>,
    table: &str,
    column: &str,
) -> AcResult<bool> {
    let mut stmt = tx
        .prepare(&format!("PRAGMA table_info({table})"))
        .map_err(db_error)?;
    let columns = stmt
        .query_map([], |row| row.get::<_, String>(1))
        .map_err(db_error)?;
    for candidate in columns {
        if candidate.map_err(db_error)? == column {
            return Ok(true);
        }
    }
    Ok(false)
}
