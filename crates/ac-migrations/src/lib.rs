use std::collections::BTreeSet;

use ac_common::{AcError, AcResult, TimestampMillis};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Migration {
    pub id: &'static str,
    pub description: &'static str,
    pub sql: &'static str,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AppliedMigration {
    pub id: String,
    pub checksum: u64,
    pub applied_at: TimestampMillis,
}

#[derive(Default)]
pub struct MigrationLedger {
    applied: Vec<AppliedMigration>,
}

impl MigrationLedger {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn apply_all(&mut self, migrations: &[Migration]) -> AcResult<Vec<AppliedMigration>> {
        validate_migrations(migrations)?;
        let known: BTreeSet<String> = self.applied.iter().map(|entry| entry.id.clone()).collect();
        let mut newly_applied = Vec::new();
        for migration in migrations {
            if known.contains(migration.id) {
                continue;
            }
            let applied = AppliedMigration {
                id: migration.id.to_string(),
                checksum: checksum(migration.sql),
                applied_at: TimestampMillis::now(),
            };
            self.applied.push(applied.clone());
            newly_applied.push(applied);
        }
        Ok(newly_applied)
    }

    pub fn current_version(&self) -> Option<&str> {
        self.applied.last().map(|entry| entry.id.as_str())
    }
}

pub const KERNEL_SCHEMA_V1: Migration = Migration {
    id: "0001_kernel_schema",
    description: "kernel/evidence/worktree/control-plane base tables",
    sql: include_str!("../../../migrations/0001_kernel_schema.sql"),
};

pub const SEMANTIC_REPOSITORY_GRAPH_V4: Migration = Migration {
    id: "0004_semantic_repository_graph",
    description:
        "semantic repository graph, LSP lifecycle, workspace, and optional index decisions",
    sql: include_str!("../../../migrations/0004_semantic_repository_graph.sql"),
};

pub const PERSISTENT_MEMORY_V5: Migration = Migration {
    id: "0005_persistent_memory",
    description: "persistent knowledge facts, decisions, task memory, and context snapshots",
    sql: include_str!("../../../migrations/0005_persistent_memory.sql"),
};

pub const CONTEXT_ENGINE_V6: Migration = Migration {
    id: "0006_context_engine",
    description:
        "context manifests, compression receipts, cache, retrieval, metrics, and benchmarks",
    sql: include_str!("../../../migrations/0006_context_engine.sql"),
};

pub const FULL_AUTONOMY_KERNEL_V7: Migration = Migration {
    id: "0007_full_autonomy_kernel",
    description: "mission contracts, requirement matrix, leases, mailbox, and autonomy records",
    sql: include_str!("../../../migrations/0007_full_autonomy_kernel.sql"),
};

pub const ADVANCED_EDIT_ENGINE_V8: Migration = Migration {
    id: "0008_advanced_edit_engine",
    description: "advanced edit transactions, operations, journal, and strategy metrics",
    sql: include_str!("../../../migrations/0008_advanced_edit_engine.sql"),
};

pub const VERIFICATION_EVIDENCE_ENGINE_V9: Migration = Migration {
    id: "0009_verification_evidence_engine",
    description: "verification profiles, runs, requirement links, findings, and final audits",
    sql: include_str!("../../../migrations/0009_verification_evidence_engine.sql"),
};

pub fn validate_migrations(migrations: &[Migration]) -> AcResult<()> {
    let mut seen = BTreeSet::new();
    for migration in migrations {
        if migration.id.trim().is_empty()
            || migration.description.trim().is_empty()
            || migration.sql.trim().is_empty()
        {
            return Err(AcError::validation(
                "MIGRATION-INVALID",
                "migration id, description, and sql are required",
            ));
        }
        if !seen.insert(migration.id) {
            return Err(AcError::validation(
                "MIGRATION-DUPLICATE",
                "migration ids must be unique",
            ));
        }
    }
    Ok(())
}

fn checksum(input: &str) -> u64 {
    input
        .bytes()
        .fold(14_695_981_039_346_656_037_u64, |hash, byte| {
            (hash ^ u64::from(byte)).wrapping_mul(1_099_511_628_211)
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migration_ledger_applies_once() {
        let mut ledger = MigrationLedger::new();
        let first = ledger.apply_all(&[KERNEL_SCHEMA_V1]).unwrap();
        let second = ledger.apply_all(&[KERNEL_SCHEMA_V1]).unwrap();
        assert_eq!(first.len(), 1);
        assert!(second.is_empty());
        assert_eq!(ledger.current_version(), Some("0001_kernel_schema"));
    }
}
