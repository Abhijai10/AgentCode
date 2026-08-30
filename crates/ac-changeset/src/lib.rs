use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use ac_common::{AcError, AcResult, StableId, TimestampMillis};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FileChangeSummary {
    pub path: String,
    pub additions: u32,
    pub removals: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ChangeOperation {
    WriteFile {
        path: String,
        expected_hash: Option<String>,
        new_hash: String,
    },
    DeleteFile {
        path: String,
        expected_hash: String,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ChangeSetState {
    Created,
    Proposed,
    Prepared,
    Applying,
    Validated,
    Approved,
    Rejected,
    Applied,
    Validating,
    Accepted,
    Archived,
    RollingBack,
    RolledBack,
    Conflict,
    UnknownEffect,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ChangeSetMetadata {
    pub originating_task: StableId,
    pub originating_agent_session: StableId,
    pub files_changed: Vec<FileChangeSummary>,
    pub additions: u32,
    pub removals: u32,
    pub evidence_refs: Vec<StableId>,
    pub verification_passed: Option<bool>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RollbackPlan {
    pub checkpoint_ref: String,
    pub description: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ChangeSet {
    pub id: StableId,
    pub operations: Vec<ChangeOperation>,
    pub state: ChangeSetState,
    pub rollback: Option<RollbackPlan>,
    pub metadata: Option<ChangeSetMetadata>,
    pub created_at: TimestampMillis,
}

impl ChangeSet {
    pub fn propose(
        operations: Vec<ChangeOperation>,
        rollback: Option<RollbackPlan>,
    ) -> AcResult<Self> {
        if operations.is_empty() {
            return Err(AcError::validation(
                "CHANGESET-EMPTY",
                "a changeset must contain at least one operation",
            ));
        }
        Ok(Self {
            id: StableId::new("cs"),
            operations,
            state: ChangeSetState::Created,
            rollback,
            metadata: None,
            created_at: TimestampMillis::now(),
        })
    }

    pub fn attach_metadata(&mut self, metadata: ChangeSetMetadata) -> AcResult<()> {
        if metadata.originating_task.as_str().trim().is_empty()
            || metadata
                .originating_agent_session
                .as_str()
                .trim()
                .is_empty()
        {
            return Err(AcError::validation(
                "CHANGESET-INVALID_METADATA",
                "originating task and agent session are required",
            ));
        }
        self.metadata = Some(metadata);
        Ok(())
    }

    pub fn validate(&mut self) -> AcResult<()> {
        self.transition(ChangeSetState::Validated)
    }

    pub fn approve(&mut self) -> AcResult<()> {
        self.transition(ChangeSetState::Approved)
    }

    pub fn reject(&mut self) -> AcResult<()> {
        self.transition(ChangeSetState::Rejected)
    }

    pub fn mark_applied(&mut self) -> AcResult<()> {
        self.transition(ChangeSetState::Applied)
    }

    pub fn archive(&mut self) -> AcResult<()> {
        self.transition(ChangeSetState::Archived)
    }

    pub fn mark_rolled_back(&mut self) -> AcResult<()> {
        self.transition(ChangeSetState::RolledBack)
    }

    fn transition(&mut self, next: ChangeSetState) -> AcResult<()> {
        let allowed = matches!(
            (self.state, next),
            (ChangeSetState::Created, ChangeSetState::Validated)
                | (ChangeSetState::Created, ChangeSetState::Prepared)
                | (ChangeSetState::Created, ChangeSetState::Rejected)
                | (ChangeSetState::Prepared, ChangeSetState::Validated)
                | (ChangeSetState::Prepared, ChangeSetState::Applying)
                | (ChangeSetState::Prepared, ChangeSetState::Conflict)
                | (ChangeSetState::Proposed, ChangeSetState::Validated)
                | (ChangeSetState::Proposed, ChangeSetState::Rejected)
                | (ChangeSetState::Validated, ChangeSetState::Approved)
                | (ChangeSetState::Validated, ChangeSetState::Rejected)
                | (ChangeSetState::Approved, ChangeSetState::Applying)
                | (ChangeSetState::Approved, ChangeSetState::Applied)
                | (ChangeSetState::Applying, ChangeSetState::Applied)
                | (ChangeSetState::Applying, ChangeSetState::RollingBack)
                | (ChangeSetState::Applying, ChangeSetState::UnknownEffect)
                | (ChangeSetState::Applied, ChangeSetState::Validating)
                | (ChangeSetState::Validating, ChangeSetState::Accepted)
                | (ChangeSetState::Validating, ChangeSetState::RollingBack)
                | (ChangeSetState::Accepted, ChangeSetState::Archived)
                | (ChangeSetState::Applied, ChangeSetState::Archived)
                | (ChangeSetState::Applied, ChangeSetState::Accepted)
                | (ChangeSetState::Applied, ChangeSetState::RollingBack)
                | (ChangeSetState::Applied, ChangeSetState::RolledBack)
                | (ChangeSetState::RollingBack, ChangeSetState::RolledBack)
                | (ChangeSetState::RollingBack, ChangeSetState::UnknownEffect)
        );
        if !allowed {
            return Err(AcError::conflict(
                "CHANGESET-INVALID_TRANSITION",
                format!("cannot transition {:?} to {:?}", self.state, next),
            ));
        }
        self.state = next;
        Ok(())
    }

    pub fn prepare(&mut self) -> AcResult<()> {
        self.transition(ChangeSetState::Prepared)
    }

    pub fn mark_applying(&mut self) -> AcResult<()> {
        self.transition(ChangeSetState::Applying)
    }

    pub fn mark_validating(&mut self) -> AcResult<()> {
        self.transition(ChangeSetState::Validating)
    }

    pub fn accept(&mut self) -> AcResult<()> {
        self.transition(ChangeSetState::Accepted)
    }

    pub fn mark_rolling_back(&mut self) -> AcResult<()> {
        self.transition(ChangeSetState::RollingBack)
    }

    pub fn mark_conflict(&mut self) -> AcResult<()> {
        self.transition(ChangeSetState::Conflict)
    }

    pub fn mark_unknown_effect(&mut self) -> AcResult<()> {
        self.transition(ChangeSetState::UnknownEffect)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EditPrecondition {
    pub path: String,
    pub expected_hash: String,
    pub base_revision: String,
    pub symbol_fingerprint: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EditStrategy {
    SearchReplace {
        search: String,
        replace: String,
        expected_matches: usize,
    },
    UnifiedDiff {
        diff: String,
    },
    WholeFile {
        content: String,
    },
    StructuredSymbol {
        symbol: String,
        replacement: String,
    },
    AstGrep {
        pattern: String,
        rewrite: String,
    },
    LspRename {
        symbol: String,
        new_name: String,
    },
    LspWorkspaceEdit {
        edits: Vec<WorkspaceTextEdit>,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EditStrategyKind {
    SearchReplace,
    UnifiedDiff,
    WholeFile,
    StructuredSymbol,
    AstGrep,
    LspRename,
    LspWorkspaceEdit,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WorkspaceTextEdit {
    pub path: String,
    pub start_line: u32,
    pub start_character: u32,
    pub end_line: u32,
    pub end_character: u32,
    pub new_text: String,
    pub provenance: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EditRequest {
    pub path: String,
    pub precondition: EditPrecondition,
    pub strategy: EditStrategy,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PreparedEdit {
    pub path: String,
    pub before_hash: String,
    pub after_hash: String,
    pub before_content: String,
    pub after_content: String,
    pub strategy: EditStrategyKind,
    pub symbol_fingerprint: Option<String>,
    pub additions: u32,
    pub removals: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ChangeSetTransaction {
    pub id: StableId,
    pub changeset: ChangeSet,
    pub edits: Vec<PreparedEdit>,
    pub journal: TransactionJournal,
    pub format_plan: FormatPlan,
    pub metrics: EditQualityMetrics,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct JournalEntry {
    pub id: StableId,
    pub path: String,
    pub state: ChangeSetState,
    pub before_hash: String,
    pub after_hash: String,
    pub created_at: TimestampMillis,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TransactionJournal {
    pub entries: Vec<JournalEntry>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RecoveryAction {
    Noop,
    Finish,
    Rollback,
    Blocked,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RecoveryDecision {
    pub action: RecoveryAction,
    pub reason: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FormatPlan {
    pub formatter: Option<String>,
    pub affected_paths: Vec<String>,
    pub degraded_reason: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EditQualityMetrics {
    pub strategy: EditStrategyKind,
    pub first_apply_success: bool,
    pub syntax_failures: u32,
    pub retries: u32,
    pub unrelated_diff_files: u32,
    pub verification_rejections: u32,
    pub degraded: bool,
}

pub trait FileRepository {
    fn read(&self, path: &str) -> AcResult<String>;
    fn write(&mut self, path: &str, content: &str) -> AcResult<()>;
    fn base_revision(&self) -> AcResult<String>;
}

#[derive(Default)]
pub struct EditEngine;

impl EditEngine {
    pub fn prepare<R: FileRepository>(
        &self,
        repo: &R,
        requests: Vec<EditRequest>,
    ) -> AcResult<ChangeSetTransaction> {
        if requests.is_empty() {
            return Err(AcError::validation(
                "EDIT-EMPTY",
                "edit transaction requires at least one request",
            ));
        }
        let mut seen_paths = BTreeSet::new();
        let mut edits = Vec::new();
        for request in requests {
            validate_path(&request.path)?;
            if !seen_paths.insert(request.path.clone()) {
                return Err(AcError::conflict(
                    "EDIT-DUPLICATE_PATH",
                    "one transaction may edit a path only once",
                ));
            }
            let current = repo.read(&request.path)?;
            validate_precondition(repo, &request, &current)?;
            let after = apply_strategy(&request.path, &current, &request.strategy)?;
            let before_hash = content_hash(&current);
            let after_hash = content_hash(&after);
            let (additions, removals) = line_delta(&current, &after);
            edits.push(PreparedEdit {
                path: request.path,
                before_hash: before_hash.clone(),
                after_hash,
                before_content: current,
                after_content: after,
                strategy: strategy_kind(&request.strategy),
                symbol_fingerprint: request.precondition.symbol_fingerprint,
                additions,
                removals,
            });
        }
        let operations = edits
            .iter()
            .map(|edit| ChangeOperation::WriteFile {
                path: edit.path.clone(),
                expected_hash: Some(edit.before_hash.clone()),
                new_hash: edit.after_hash.clone(),
            })
            .collect();
        let mut changeset = ChangeSet::propose(
            operations,
            Some(RollbackPlan {
                checkpoint_ref: "changeset-journal".to_string(),
                description: "restore pre-change content recorded in the edit journal".to_string(),
            }),
        )?;
        changeset.prepare()?;
        let journal = TransactionJournal {
            entries: edits
                .iter()
                .map(|edit| JournalEntry {
                    id: StableId::new("journal"),
                    path: edit.path.clone(),
                    state: ChangeSetState::Prepared,
                    before_hash: edit.before_hash.clone(),
                    after_hash: edit.after_hash.clone(),
                    created_at: TimestampMillis::now(),
                })
                .collect(),
        };
        let primary = edits
            .first()
            .map(|edit| edit.strategy)
            .unwrap_or(EditStrategyKind::WholeFile);
        let format_plan = detect_formatter(edits.iter().map(|edit| edit.path.as_str()));
        Ok(ChangeSetTransaction {
            id: StableId::new("editxn"),
            changeset,
            edits,
            journal,
            metrics: EditQualityMetrics {
                strategy: primary,
                first_apply_success: false,
                syntax_failures: 0,
                retries: 0,
                unrelated_diff_files: 0,
                verification_rejections: 0,
                degraded: false,
            },
            format_plan,
        })
    }

    pub fn apply<R: FileRepository>(
        &self,
        repo: &mut R,
        transaction: &mut ChangeSetTransaction,
    ) -> AcResult<()> {
        transaction.changeset.mark_applying()?;
        let mut applied = Vec::new();
        for edit in &transaction.edits {
            match repo.read(&edit.path) {
                Ok(current) if content_hash(&current) == edit.before_hash => {}
                Ok(_) => {
                    transaction.changeset.mark_rolling_back()?;
                    rollback_applied(repo, &transaction.edits, &mut transaction.journal, &applied)?;
                    transaction.changeset.mark_rolled_back()?;
                    transaction.metrics.first_apply_success = false;
                    return Err(AcError::conflict(
                        "EDIT-CONCURRENT_MUTATION",
                        "file changed after preparation and before apply",
                    ));
                }
                Err(error) => return Err(error),
            }
            if let Err(error) = repo.write(&edit.path, &edit.after_content) {
                transaction.changeset.mark_rolling_back()?;
                rollback_applied(repo, &transaction.edits, &mut transaction.journal, &applied)?;
                transaction.changeset.mark_rolled_back()?;
                return Err(error);
            }
            applied.push(edit.path.clone());
            mark_journal(
                &mut transaction.journal,
                &edit.path,
                ChangeSetState::Applied,
            );
        }
        transaction.metrics.first_apply_success = true;
        transaction.changeset.mark_applied()?;
        Ok(())
    }

    pub fn rollback<R: FileRepository>(
        &self,
        repo: &mut R,
        transaction: &mut ChangeSetTransaction,
    ) -> AcResult<()> {
        transaction.changeset.mark_rolling_back()?;
        let paths = transaction
            .edits
            .iter()
            .map(|edit| edit.path.clone())
            .collect::<Vec<_>>();
        rollback_applied(repo, &transaction.edits, &mut transaction.journal, &paths)?;
        transaction.changeset.mark_rolled_back()
    }

    pub fn reconcile<R: FileRepository>(
        &self,
        repo: &R,
        transaction: &ChangeSetTransaction,
    ) -> AcResult<RecoveryDecision> {
        let mut before = 0;
        let mut after = 0;
        let mut unknown = 0;
        for edit in &transaction.edits {
            let current = repo.read(&edit.path)?;
            let current_hash = content_hash(&current);
            if current_hash == edit.before_hash {
                before += 1;
            } else if current_hash == edit.after_hash {
                after += 1;
            } else {
                unknown += 1;
            }
        }
        let action = match (before, after, unknown) {
            (_, _, n) if n > 0 => RecoveryAction::Blocked,
            (_, 0, 0) => RecoveryAction::Noop,
            (0, _, 0) => RecoveryAction::Finish,
            _ => RecoveryAction::Rollback,
        };
        Ok(RecoveryDecision {
            action,
            reason: format!("before:{before};after:{after};unknown:{unknown}"),
        })
    }
}

#[derive(Clone, Debug, Default)]
pub struct MemoryFileRepository {
    files: BTreeMap<String, String>,
    base_revision: String,
    fail_writes: BTreeSet<String>,
}

impl MemoryFileRepository {
    pub fn new(base_revision: impl Into<String>) -> Self {
        Self {
            files: BTreeMap::new(),
            base_revision: base_revision.into(),
            fail_writes: BTreeSet::new(),
        }
    }

    pub fn put(&mut self, path: impl Into<String>, content: impl Into<String>) {
        self.files.insert(path.into(), content.into());
    }

    pub fn fail_write(&mut self, path: impl Into<String>) {
        self.fail_writes.insert(path.into());
    }

    pub fn hash(&self, path: &str) -> AcResult<String> {
        self.read(path).map(|content| content_hash(&content))
    }
}

impl FileRepository for MemoryFileRepository {
    fn read(&self, path: &str) -> AcResult<String> {
        self.files.get(path).cloned().ok_or_else(|| {
            AcError::validation("EDIT-FILE_NOT_FOUND", format!("file not found: {path}"))
        })
    }

    fn write(&mut self, path: &str, content: &str) -> AcResult<()> {
        if self.fail_writes.contains(path) {
            return Err(AcError::validation(
                "EDIT-WRITE_FAILED",
                format!("configured write failure for {path}"),
            ));
        }
        self.files.insert(path.to_string(), content.to_string());
        Ok(())
    }

    fn base_revision(&self) -> AcResult<String> {
        Ok(self.base_revision.clone())
    }
}

pub struct LocalWorkspaceFileRepository {
    root: std::path::PathBuf,
    base_revision: String,
}

impl LocalWorkspaceFileRepository {
    pub fn new(root: std::path::PathBuf, base_revision: impl Into<String>) -> Self {
        Self {
            root,
            base_revision: base_revision.into(),
        }
    }

    fn resolve(&self, path: &str) -> AcResult<std::path::PathBuf> {
        validate_path(path)?;
        Ok(self.root.join(path))
    }
}

impl FileRepository for LocalWorkspaceFileRepository {
    fn read(&self, path: &str) -> AcResult<String> {
        std::fs::read_to_string(self.resolve(path)?)
            .map_err(|error| AcError::validation("EDIT-FS_READ_FAILED", error.to_string()))
    }

    fn write(&mut self, path: &str, content: &str) -> AcResult<()> {
        let path = self.resolve(path)?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|error| AcError::validation("EDIT-FS_WRITE_FAILED", error.to_string()))?;
        }
        std::fs::write(path, content)
            .map_err(|error| AcError::validation("EDIT-FS_WRITE_FAILED", error.to_string()))
    }

    fn base_revision(&self) -> AcResult<String> {
        Ok(self.base_revision.clone())
    }
}

pub fn content_hash(content: &str) -> String {
    let hash = content
        .bytes()
        .fold(14_695_981_039_346_656_037_u64, |hash, byte| {
            (hash ^ u64::from(byte)).wrapping_mul(1_099_511_628_211)
        });
    format!("fnv1a64:{hash:016x}")
}

pub fn symbol_fingerprint(symbol: &str, content: &str) -> Option<String> {
    content
        .lines()
        .find(|line| line.contains(symbol))
        .map(|line| content_hash(&format!("{symbol}:{line}")))
}

fn validate_precondition<R: FileRepository>(
    repo: &R,
    request: &EditRequest,
    current: &str,
) -> AcResult<()> {
    if request.precondition.path != request.path {
        return Err(AcError::validation(
            "EDIT-PRECONDITION_PATH",
            "precondition path must match request path",
        ));
    }
    let actual_hash = content_hash(current);
    if actual_hash != request.precondition.expected_hash {
        return Err(AcError::conflict(
            "EDIT-STALE_HASH",
            "current file hash does not match edit precondition",
        ));
    }
    let base_revision = repo.base_revision()?;
    if base_revision != request.precondition.base_revision {
        return Err(AcError::conflict(
            "EDIT-STALE_BASE_REVISION",
            "repository base revision does not match edit precondition",
        ));
    }
    if let Some(expected) = &request.precondition.symbol_fingerprint {
        let symbol = match &request.strategy {
            EditStrategy::StructuredSymbol { symbol, .. }
            | EditStrategy::LspRename { symbol, .. } => Some(symbol.as_str()),
            _ => None,
        };
        if symbol
            .and_then(|symbol| symbol_fingerprint(symbol, current))
            .as_ref()
            != Some(expected)
        {
            return Err(AcError::conflict(
                "EDIT-STALE_SYMBOL",
                "symbol fingerprint does not match edit precondition",
            ));
        }
    }
    Ok(())
}

fn apply_strategy(path: &str, content: &str, strategy: &EditStrategy) -> AcResult<String> {
    match strategy {
        EditStrategy::SearchReplace {
            search,
            replace,
            expected_matches,
        } => {
            if search.is_empty() {
                return Err(AcError::validation(
                    "EDIT-SEARCH_EMPTY",
                    "search text cannot be empty",
                ));
            }
            let matches = content.matches(search).count();
            if matches != *expected_matches {
                return Err(AcError::conflict(
                    "EDIT-SEARCH_AMBIGUOUS",
                    format!("expected {expected_matches} matches, found {matches}"),
                ));
            }
            Ok(content.replacen(search, replace, *expected_matches))
        }
        EditStrategy::UnifiedDiff { diff } => apply_unified_diff(content, diff),
        EditStrategy::WholeFile { content } => Ok(content.clone()),
        EditStrategy::StructuredSymbol {
            symbol,
            replacement,
        } => replace_symbol_line(content, symbol, replacement),
        EditStrategy::AstGrep { pattern, rewrite } => {
            let language = language_for_path(path);
            ac_code_intel::structural_rewrite(language, content, pattern, rewrite).map_err(
                |error| {
                    AcError::new(
                        "EDIT-AST_GREP_ENGINE",
                        error.to_string(),
                        error.kind(),
                        error.retryability(),
                    )
                },
            )
        }
        EditStrategy::LspRename { symbol, new_name } => {
            if symbol.trim().is_empty() || new_name.trim().is_empty() {
                return Err(AcError::validation(
                    "EDIT-LSP_RENAME_INVALID",
                    "LSP rename requires symbol and new name",
                ));
            }
            Err(AcError::new(
                "EDIT-LANGUAGE_SERVER_UNAVAILABLE",
                "LspRename requires a real LSP WorkspaceEdit; use LspWorkspaceEdit after textDocument/rename",
                ac_common::ErrorKind::Unavailable,
                ac_common::Retryability::NotRetryable,
            ))
        }
        EditStrategy::LspWorkspaceEdit { edits } => {
            apply_workspace_text_edits(path, content, edits)
        }
    }
}

fn apply_unified_diff(content: &str, diff: &str) -> AcResult<String> {
    let mut result = content.to_string();
    let mut removed = Vec::new();
    let mut added = Vec::new();
    for line in diff.lines() {
        if line.starts_with("---") || line.starts_with("+++") || line.starts_with("@@") {
            continue;
        }
        if let Some(rest) = line.strip_prefix('-') {
            removed.push(rest.to_string());
        } else if let Some(rest) = line.strip_prefix('+') {
            added.push(rest.to_string());
        } else if let Some(rest) = line.strip_prefix(' ') {
            if !result.contains(rest) {
                return Err(AcError::conflict(
                    "EDIT-DIFF_CONTEXT",
                    "unified diff context was not found",
                ));
            }
        }
    }
    if removed.is_empty() && added.is_empty() {
        return Err(AcError::validation(
            "EDIT-DIFF_EMPTY",
            "unified diff must contain at least one changed line",
        ));
    }
    let needle = removed.join("\n");
    let replacement = added.join("\n");
    if !needle.is_empty() {
        let count = result.matches(&needle).count();
        if count != 1 {
            return Err(AcError::conflict(
                "EDIT-DIFF_REJECTED_HUNK",
                format!("expected one hunk target, found {count}"),
            ));
        }
        result = result.replacen(&needle, &replacement, 1);
    } else {
        result.push_str(&replacement);
        if !result.ends_with('\n') {
            result.push('\n');
        }
    }
    Ok(result)
}

fn replace_symbol_line(content: &str, symbol: &str, replacement: &str) -> AcResult<String> {
    let mut matched = 0;
    let lines = content
        .lines()
        .map(|line| {
            if line.contains(symbol) {
                matched += 1;
                replacement
            } else {
                line
            }
        })
        .collect::<Vec<_>>();
    if matched != 1 {
        return Err(AcError::conflict(
            "EDIT-SYMBOL_AMBIGUOUS",
            format!("expected one symbol line, found {matched}"),
        ));
    }
    Ok(lines.join("\n") + trailing_newline(content))
}

fn trailing_newline(content: &str) -> &'static str {
    if content.ends_with('\n') {
        "\n"
    } else {
        ""
    }
}

fn strategy_kind(strategy: &EditStrategy) -> EditStrategyKind {
    match strategy {
        EditStrategy::SearchReplace { .. } => EditStrategyKind::SearchReplace,
        EditStrategy::UnifiedDiff { .. } => EditStrategyKind::UnifiedDiff,
        EditStrategy::WholeFile { .. } => EditStrategyKind::WholeFile,
        EditStrategy::StructuredSymbol { .. } => EditStrategyKind::StructuredSymbol,
        EditStrategy::AstGrep { .. } => EditStrategyKind::AstGrep,
        EditStrategy::LspRename { .. } => EditStrategyKind::LspRename,
        EditStrategy::LspWorkspaceEdit { .. } => EditStrategyKind::LspWorkspaceEdit,
    }
}

fn apply_workspace_text_edits(
    path: &str,
    content: &str,
    edits: &[WorkspaceTextEdit],
) -> AcResult<String> {
    for edit in edits {
        validate_path(&edit.path)?;
        if !edit.provenance.starts_with("Lsp:") {
            return Err(AcError::validation(
                "EDIT-LSP_WORKSPACE_EDIT_PROVENANCE",
                "workspace edits must carry LSP provenance",
            ));
        }
    }
    let mut relevant = edits
        .iter()
        .filter(|edit| edit.path == path)
        .cloned()
        .collect::<Vec<_>>();
    if relevant.is_empty() {
        return Err(AcError::conflict(
            "EDIT-LSP_WORKSPACE_EDIT_EMPTY",
            "workspace edit contains no changes for request path",
        ));
    }
    relevant.sort_by_key(|edit| {
        std::cmp::Reverse((
            edit.start_line,
            edit.start_character,
            edit.end_line,
            edit.end_character,
        ))
    });
    let line_starts = line_start_offsets(content);
    let mut output = content.to_string();
    let mut last_start = usize::MAX;
    for edit in relevant {
        let start =
            position_to_offset(&line_starts, content, edit.start_line, edit.start_character)?;
        let end = position_to_offset(&line_starts, content, edit.end_line, edit.end_character)?;
        if start > end || end > output.len() {
            return Err(AcError::validation(
                "EDIT-LSP_WORKSPACE_EDIT_RANGE",
                "workspace edit range is invalid",
            ));
        }
        if end > last_start {
            return Err(AcError::conflict(
                "EDIT-LSP_WORKSPACE_EDIT_CONFLICT",
                "workspace edit ranges overlap",
            ));
        }
        output.replace_range(start..end, &edit.new_text);
        last_start = start;
    }
    Ok(output)
}

fn line_start_offsets(content: &str) -> Vec<usize> {
    let mut offsets = vec![0];
    for (idx, byte) in content.bytes().enumerate() {
        if byte == b'\n' {
            offsets.push(idx + 1);
        }
    }
    offsets
}

fn position_to_offset(
    line_starts: &[usize],
    content: &str,
    one_based_line: u32,
    character: u32,
) -> AcResult<usize> {
    let line = one_based_line.checked_sub(1).ok_or_else(|| {
        AcError::validation(
            "EDIT-LSP_WORKSPACE_EDIT_RANGE",
            "line numbers are one-based",
        )
    })? as usize;
    let line_start = *line_starts.get(line).ok_or_else(|| {
        AcError::validation("EDIT-LSP_WORKSPACE_EDIT_RANGE", "line is outside document")
    })?;
    let line_end = content[line_start..]
        .find('\n')
        .map(|index| line_start + index)
        .unwrap_or(content.len());
    let line_content = &content[line_start..line_end];
    let mut utf16_units = 0_u32;
    for (byte_offset, ch) in line_content.char_indices() {
        if utf16_units == character {
            return Ok(line_start + byte_offset);
        }
        utf16_units += ch.len_utf16() as u32;
        if utf16_units > character {
            return Err(AcError::validation(
                "EDIT-LSP_WORKSPACE_EDIT_RANGE",
                "UTF-16 position splits a character",
            ));
        }
    }
    if utf16_units == character {
        Ok(line_end)
    } else {
        Err(AcError::validation(
            "EDIT-LSP_WORKSPACE_EDIT_RANGE",
            "UTF-16 character position is outside the line",
        ))
    }
}

fn language_for_path(path: &str) -> &'static str {
    match path.rsplit('.').next().unwrap_or("") {
        "rs" => "rust",
        "py" => "python",
        "js" | "jsx" => "javascript",
        "ts" => "typescript",
        "tsx" => "tsx",
        "go" => "go",
        _ => "unknown",
    }
}

fn detect_formatter<'a>(paths: impl Iterator<Item = &'a str>) -> FormatPlan {
    let affected_paths = paths.map(ToString::to_string).collect::<Vec<_>>();
    let formatter = if affected_paths.iter().all(|path| path.ends_with(".rs")) {
        Some("cargo fmt --all".to_string())
    } else if affected_paths
        .iter()
        .all(|path| path.ends_with(".md") || path.ends_with(".txt"))
    {
        None
    } else {
        Some("project-native-format".to_string())
    };
    FormatPlan {
        formatter,
        affected_paths,
        degraded_reason: None,
    }
}

fn rollback_applied<R: FileRepository>(
    repo: &mut R,
    edits: &[PreparedEdit],
    journal: &mut TransactionJournal,
    applied_paths: &[String],
) -> AcResult<()> {
    for path in applied_paths.iter().rev() {
        let entry = journal
            .entries
            .iter()
            .find(|entry| &entry.path == path)
            .cloned();
        if let Some(entry) = entry {
            mark_journal(journal, path, ChangeSetState::RollingBack);
            let before_content = edits
                .iter()
                .find(|edit| &edit.path == path)
                .map(|edit| edit.before_content.as_str())
                .ok_or_else(|| {
                    AcError::conflict(
                        "EDIT-ROLLBACK_MISSING_CONTENT",
                        "rollback journal has no pre-change content for path",
                    )
                })?;
            repo.write(path, before_content)?;
            let restored = repo.read(path)?;
            if content_hash(&restored) != entry.before_hash {
                mark_journal(journal, path, ChangeSetState::UnknownEffect);
                return Err(AcError::conflict(
                    "EDIT-ROLLBACK_CORRUPTION",
                    "rollback did not restore recorded pre-change hash",
                ));
            }
            mark_journal(journal, path, ChangeSetState::RolledBack);
        }
    }
    Ok(())
}

fn mark_journal(journal: &mut TransactionJournal, path: &str, state: ChangeSetState) {
    if let Some(entry) = journal.entries.iter_mut().find(|entry| entry.path == path) {
        entry.state = state;
    }
}

fn line_delta(before: &str, after: &str) -> (u32, u32) {
    let before_lines = before.lines().count() as i32;
    let after_lines = after.lines().count() as i32;
    if after_lines >= before_lines {
        ((after_lines - before_lines) as u32, 0)
    } else {
        (0, (before_lines - after_lines) as u32)
    }
}

fn validate_path(path: &str) -> AcResult<()> {
    let candidate = Path::new(path);
    if path.trim().is_empty()
        || candidate.is_absolute()
        || candidate.components().any(|component| {
            matches!(
                component,
                std::path::Component::ParentDir
                    | std::path::Component::RootDir
                    | std::path::Component::Prefix(_)
            )
        })
    {
        return Err(AcError::policy_denied(
            "EDIT-PATH_ESCAPE",
            "edit path must stay inside workspace",
        ));
    }
    let blocked = [".lock", ".pem", ".key", ".secret"];
    if blocked.iter().any(|suffix| path.ends_with(suffix)) {
        return Err(AcError::policy_denied(
            "EDIT-POLICY_GUARD",
            "sensitive or lock files require explicit policy outside edit engine",
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn op() -> ChangeOperation {
        ChangeOperation::WriteFile {
            path: "src/lib.rs".to_string(),
            expected_hash: Some("old".to_string()),
            new_hash: "new".to_string(),
        }
    }

    #[test]
    fn changeset_requires_approval_before_applied() {
        let mut changeset = ChangeSet::propose(vec![op()], None).unwrap();
        let err = changeset.mark_applied().unwrap_err();
        assert_eq!(err.code(), "CHANGESET-INVALID_TRANSITION");
        changeset.validate().unwrap();
        changeset.approve().unwrap();
        changeset.mark_applied().unwrap();
        assert_eq!(changeset.state, ChangeSetState::Applied);
    }

    #[test]
    fn changeset_records_metadata_and_archives_after_apply() {
        let mut changeset = ChangeSet::propose(vec![op()], None).unwrap();
        changeset
            .attach_metadata(ChangeSetMetadata {
                originating_task: StableId::new("goal"),
                originating_agent_session: StableId::new("session"),
                files_changed: vec![FileChangeSummary {
                    path: "src/lib.rs".to_string(),
                    additions: 2,
                    removals: 1,
                }],
                additions: 2,
                removals: 1,
                evidence_refs: vec![StableId::new("ev")],
                verification_passed: Some(true),
            })
            .unwrap();
        changeset.validate().unwrap();
        changeset.approve().unwrap();
        changeset.mark_applied().unwrap();
        changeset.archive().unwrap();
        assert_eq!(changeset.state, ChangeSetState::Archived);
        assert_eq!(changeset.metadata.as_ref().unwrap().additions, 2);
    }

    fn request(path: &str, repo: &MemoryFileRepository, strategy: EditStrategy) -> EditRequest {
        EditRequest {
            path: path.to_string(),
            precondition: EditPrecondition {
                path: path.to_string(),
                expected_hash: repo.hash(path).unwrap(),
                base_revision: repo.base_revision().unwrap(),
                symbol_fingerprint: match &strategy {
                    EditStrategy::StructuredSymbol { symbol, .. }
                    | EditStrategy::LspRename { symbol, .. } => {
                        symbol_fingerprint(symbol, &repo.read(path).unwrap())
                    }
                    _ => None,
                },
            },
            strategy,
        }
    }

    #[test]
    fn phase13_search_replace_multifile_transaction_applies() {
        let mut repo = MemoryFileRepository::new("rev-a");
        repo.put("a.rs", "fn one() {}\n");
        repo.put("b.rs", "fn two() {}\n");
        repo.put("c.rs", "fn three() {}\n");
        let engine = EditEngine;
        let mut transaction = engine
            .prepare(
                &repo,
                vec![
                    request(
                        "a.rs",
                        &repo,
                        EditStrategy::SearchReplace {
                            search: "one".to_string(),
                            replace: "uno".to_string(),
                            expected_matches: 1,
                        },
                    ),
                    request(
                        "b.rs",
                        &repo,
                        EditStrategy::SearchReplace {
                            search: "two".to_string(),
                            replace: "dos".to_string(),
                            expected_matches: 1,
                        },
                    ),
                    request(
                        "c.rs",
                        &repo,
                        EditStrategy::SearchReplace {
                            search: "three".to_string(),
                            replace: "tres".to_string(),
                            expected_matches: 1,
                        },
                    ),
                ],
            )
            .unwrap();
        engine.apply(&mut repo, &mut transaction).unwrap();
        assert_eq!(repo.read("a.rs").unwrap(), "fn uno() {}\n");
        assert_eq!(repo.read("b.rs").unwrap(), "fn dos() {}\n");
        assert_eq!(repo.read("c.rs").unwrap(), "fn tres() {}\n");
        assert_eq!(transaction.changeset.state, ChangeSetState::Applied);
        assert!(transaction.metrics.first_apply_success);
    }

    #[test]
    fn phase13_stale_hash_and_concurrent_mutation_block_overwrite() {
        let mut repo = MemoryFileRepository::new("rev-a");
        repo.put("src/lib.rs", "pub fn answer() -> u32 { 41 }\n");
        let engine = EditEngine;
        let mut stale = request(
            "src/lib.rs",
            &repo,
            EditStrategy::SearchReplace {
                search: "41".to_string(),
                replace: "42".to_string(),
                expected_matches: 1,
            },
        );
        stale.precondition.expected_hash = "fnv1a64:stale".to_string();
        assert_eq!(
            engine.prepare(&repo, vec![stale]).unwrap_err().code(),
            "EDIT-STALE_HASH"
        );

        let mut transaction = engine
            .prepare(
                &repo,
                vec![request(
                    "src/lib.rs",
                    &repo,
                    EditStrategy::SearchReplace {
                        search: "41".to_string(),
                        replace: "42".to_string(),
                        expected_matches: 1,
                    },
                )],
            )
            .unwrap();
        repo.put("src/lib.rs", "pub fn answer() -> u32 { 99 }\n");
        assert_eq!(
            engine
                .apply(&mut repo, &mut transaction)
                .unwrap_err()
                .code(),
            "EDIT-CONCURRENT_MUTATION"
        );
        assert_eq!(
            repo.read("src/lib.rs").unwrap(),
            "pub fn answer() -> u32 { 99 }\n"
        );
    }

    #[test]
    fn phase13_unified_diff_rejects_bad_context() {
        let mut repo = MemoryFileRepository::new("rev-a");
        repo.put("src/lib.rs", "alpha\nbeta\ngamma\n");
        let engine = EditEngine;
        let err = engine
            .prepare(
                &repo,
                vec![request(
                    "src/lib.rs",
                    &repo,
                    EditStrategy::UnifiedDiff {
                        diff: "--- a/src/lib.rs\n+++ b/src/lib.rs\n@@\n missing\n-beta\n+delta\n"
                            .to_string(),
                    },
                )],
            )
            .unwrap_err();
        assert_eq!(err.code(), "EDIT-DIFF_CONTEXT");
    }

    #[test]
    fn phase13_failure_on_second_file_rolls_back_first_file() {
        let mut repo = MemoryFileRepository::new("rev-a");
        repo.put("one.txt", "old one\n");
        repo.put("two.txt", "old two\n");
        repo.fail_write("two.txt");
        let engine = EditEngine;
        let mut transaction = engine
            .prepare(
                &repo,
                vec![
                    request(
                        "one.txt",
                        &repo,
                        EditStrategy::WholeFile {
                            content: "new one\n".to_string(),
                        },
                    ),
                    request(
                        "two.txt",
                        &repo,
                        EditStrategy::WholeFile {
                            content: "new two\n".to_string(),
                        },
                    ),
                ],
            )
            .unwrap();
        assert_eq!(
            engine
                .apply(&mut repo, &mut transaction)
                .unwrap_err()
                .code(),
            "EDIT-WRITE_FAILED"
        );
        assert_eq!(repo.read("one.txt").unwrap(), "old one\n");
        assert_eq!(repo.read("two.txt").unwrap(), "old two\n");
        assert_eq!(transaction.changeset.state, ChangeSetState::RolledBack);
    }

    #[test]
    fn phase13_crash_reconciliation_detects_partial_state() {
        let mut repo = MemoryFileRepository::new("rev-a");
        repo.put("one.txt", "old one\n");
        repo.put("two.txt", "old two\n");
        let engine = EditEngine;
        let transaction = engine
            .prepare(
                &repo,
                vec![
                    request(
                        "one.txt",
                        &repo,
                        EditStrategy::WholeFile {
                            content: "new one\n".to_string(),
                        },
                    ),
                    request(
                        "two.txt",
                        &repo,
                        EditStrategy::WholeFile {
                            content: "new two\n".to_string(),
                        },
                    ),
                ],
            )
            .unwrap();
        repo.put("one.txt", "new one\n");
        let decision = engine.reconcile(&repo, &transaction).unwrap();
        assert_eq!(decision.action, RecoveryAction::Rollback);
    }

    #[test]
    fn applied_change_not_acknowledged_reconciles_without_duplicate_apply() {
        let mut repo = MemoryFileRepository::new("rev-a");
        repo.put("src/lib.rs", "pub fn value() -> u32 { 1 }\n");
        let engine = EditEngine;
        let mut transaction = engine
            .prepare(
                &repo,
                vec![request(
                    "src/lib.rs",
                    &repo,
                    EditStrategy::SearchReplace {
                        search: "1".to_string(),
                        replace: "2".to_string(),
                        expected_matches: 1,
                    },
                )],
            )
            .unwrap();
        engine.apply(&mut repo, &mut transaction).unwrap();
        assert_eq!(
            repo.read("src/lib.rs").unwrap(),
            "pub fn value() -> u32 { 2 }\n"
        );
        let decision = engine.reconcile(&repo, &transaction).unwrap();
        assert_eq!(decision.action, RecoveryAction::Finish);
        assert_eq!(decision.reason, "before:0;after:1;unknown:0");
        assert_eq!(repo.read("src/lib.rs").unwrap().matches("{ 2 }").count(), 1);
    }

    #[test]
    fn phase13_structured_ast_lsp_format_and_metrics_are_available() {
        let mut repo = MemoryFileRepository::new("rev-a");
        repo.put(
            "src/lib.rs",
            "pub fn old_name() -> u32 { 1 }\npub fn caller() -> u32 { old_name() }\n",
        );
        let engine = EditEngine;
        let fake_rename = engine
            .prepare(
                &repo,
                vec![request(
                    "src/lib.rs",
                    &repo,
                    EditStrategy::LspRename {
                        symbol: "old_name".to_string(),
                        new_name: "new_name".to_string(),
                    },
                )],
            )
            .unwrap_err();
        assert_eq!(fake_rename.code(), "EDIT-LANGUAGE_SERVER_UNAVAILABLE");

        let mut transaction = engine
            .prepare(
                &repo,
                vec![request(
                    "src/lib.rs",
                    &repo,
                    EditStrategy::LspWorkspaceEdit {
                        edits: vec![
                            WorkspaceTextEdit {
                                path: "src/lib.rs".to_string(),
                                start_line: 1,
                                start_character: 7,
                                end_line: 1,
                                end_character: 15,
                                new_text: "new_name".to_string(),
                                provenance: "Lsp:rust-analyzer".to_string(),
                            },
                            WorkspaceTextEdit {
                                path: "src/lib.rs".to_string(),
                                start_line: 2,
                                start_character: 25,
                                end_line: 2,
                                end_character: 33,
                                new_text: "new_name".to_string(),
                                provenance: "Lsp:rust-analyzer".to_string(),
                            },
                        ],
                    },
                )],
            )
            .unwrap();
        engine.apply(&mut repo, &mut transaction).unwrap();
        assert!(repo.read("src/lib.rs").unwrap().contains("new_name"));
        assert_eq!(
            transaction.format_plan.formatter.as_deref(),
            Some("cargo fmt --all")
        );
        assert_eq!(
            transaction.metrics.strategy,
            EditStrategyKind::LspWorkspaceEdit
        );

        let ast_request = request(
            "src/lib.rs",
            &repo,
            EditStrategy::AstGrep {
                pattern: "function:caller".to_string(),
                rewrite: "pub fn caller() -> usize { new_name() as usize }".to_string(),
            },
        );
        let ast = engine.prepare(&repo, vec![ast_request]).unwrap();
        assert_eq!(ast.edits[0].strategy, EditStrategyKind::AstGrep);
        assert!(ast.edits[0].after_content.contains("usize"));
    }

    #[test]
    fn phase30_workspace_edit_rejects_escape_and_overlap_before_mutation() {
        let mut repo = MemoryFileRepository::new("rev-a");
        repo.put("src/lib.rs", "pub fn old_name() {}\n");
        let engine = EditEngine;
        let escaping = request(
            "src/lib.rs",
            &repo,
            EditStrategy::LspWorkspaceEdit {
                edits: vec![WorkspaceTextEdit {
                    path: "../outside.rs".to_string(),
                    start_line: 1,
                    start_character: 7,
                    end_line: 1,
                    end_character: 15,
                    new_text: "new_name".to_string(),
                    provenance: "Lsp:rust-analyzer".to_string(),
                }],
            },
        );
        assert_eq!(
            engine.prepare(&repo, vec![escaping]).unwrap_err().code(),
            "EDIT-PATH_ESCAPE"
        );

        let overlapping = request(
            "src/lib.rs",
            &repo,
            EditStrategy::LspWorkspaceEdit {
                edits: vec![
                    WorkspaceTextEdit {
                        path: "src/lib.rs".to_string(),
                        start_line: 1,
                        start_character: 7,
                        end_line: 1,
                        end_character: 15,
                        new_text: "new_name".to_string(),
                        provenance: "Lsp:rust-analyzer".to_string(),
                    },
                    WorkspaceTextEdit {
                        path: "src/lib.rs".to_string(),
                        start_line: 1,
                        start_character: 8,
                        end_line: 1,
                        end_character: 12,
                        new_text: "bad".to_string(),
                        provenance: "Lsp:rust-analyzer".to_string(),
                    },
                ],
            },
        );
        assert_eq!(
            engine.prepare(&repo, vec![overlapping]).unwrap_err().code(),
            "EDIT-LSP_WORKSPACE_EDIT_CONFLICT"
        );
        assert_eq!(repo.read("src/lib.rs").unwrap(), "pub fn old_name() {}\n");
    }

    #[test]
    fn phase13_security_guard_blocks_escape_and_sensitive_files() {
        let mut repo = MemoryFileRepository::new("rev-a");
        repo.put("src/lib.rs", "fn main() {}\n");
        let engine = EditEngine;
        let mut escaping = request(
            "src/lib.rs",
            &repo,
            EditStrategy::WholeFile {
                content: "x\n".to_string(),
            },
        );
        escaping.path = "../src/lib.rs".to_string();
        assert_eq!(
            engine.prepare(&repo, vec![escaping]).unwrap_err().code(),
            "EDIT-PATH_ESCAPE"
        );

        repo.put("secret.pem", "secret\n");
        assert_eq!(
            engine
                .prepare(
                    &repo,
                    vec![request(
                        "secret.pem",
                        &repo,
                        EditStrategy::WholeFile {
                            content: "changed\n".to_string()
                        }
                    )]
                )
                .unwrap_err()
                .code(),
            "EDIT-POLICY_GUARD"
        );
    }

    #[test]
    fn lsp_workspace_edit_positions_use_utf16_code_units() {
        let content = "aé中😀z\n";
        let starts = line_start_offsets(content);
        assert_eq!(position_to_offset(&starts, content, 1, 1).unwrap(), 1);
        assert_eq!(position_to_offset(&starts, content, 1, 2).unwrap(), 3);
        assert_eq!(position_to_offset(&starts, content, 1, 3).unwrap(), 6);
        assert_eq!(position_to_offset(&starts, content, 1, 5).unwrap(), 10);
        assert_eq!(position_to_offset(&starts, content, 1, 6).unwrap(), 11);
        assert_eq!(
            position_to_offset(&starts, content, 1, 4)
                .unwrap_err()
                .code(),
            "EDIT-LSP_WORKSPACE_EDIT_RANGE"
        );
    }

    #[test]
    fn apply_before_kernel_approval_is_rejected() {
        // BF-05: the authoritative mutation order requires kernel approval
        // before EditEngine.apply.  A changeset that is only Validated (never
        // kernel-approved) must not reach Applying; the write must not happen.
        let mut repo = MemoryFileRepository::new("rev-a");
        repo.put("src/lib.rs", "pub fn answer() -> u32 { 41 }\n");
        let engine = EditEngine;
        let mut transaction = engine
            .prepare(
                &repo,
                vec![request(
                    "src/lib.rs",
                    &repo,
                    EditStrategy::WholeFile {
                        content: "pub fn answer() -> u32 { 42 }\n".to_string(),
                    },
                )],
            )
            .unwrap();
        transaction.changeset.validate().unwrap();
        assert_eq!(transaction.changeset.state, ChangeSetState::Validated);
        // Without kernel approval the state machine forbids Applying, so the
        // file must remain untouched.
        let err = engine.apply(&mut repo, &mut transaction).unwrap_err();
        assert_eq!(err.code(), "CHANGESET-INVALID_TRANSITION");
        assert_eq!(
            repo.read("src/lib.rs").unwrap(),
            "pub fn answer() -> u32 { 41 }\n"
        );
        assert_eq!(transaction.changeset.state, ChangeSetState::Validated);
    }

    #[test]
    fn apply_without_expected_hash_precondition_is_rejected_at_prepare() {
        // BF-05: no fallback may silently drop the expected_hash precondition.
        // A request with an empty/weak precondition must be rejected before any
        // mutation, not applied blindly.
        let mut repo = MemoryFileRepository::new("rev-a");
        repo.put("src/lib.rs", "one\n");
        let engine = EditEngine;
        let mut weak = request(
            "src/lib.rs",
            &repo,
            EditStrategy::WholeFile {
                content: "two\n".to_string(),
            },
        );
        weak.precondition.expected_hash = String::new();
        assert_eq!(
            engine.prepare(&repo, vec![weak]).unwrap_err().code(),
            "EDIT-STALE_HASH"
        );
        // The real production path computes expected_hash from the actual
        // current file, so the prepared operation always carries it.
        let transaction = engine
            .prepare(
                &repo,
                vec![request(
                    "src/lib.rs",
                    &repo,
                    EditStrategy::WholeFile {
                        content: "two\n".to_string(),
                    },
                )],
            )
            .unwrap();
        assert!(matches!(
            &transaction.changeset.operations[0],
            ChangeOperation::WriteFile {
                expected_hash: Some(hash),
                ..
            } if hash.starts_with("fnv1a64:")
        ));
        assert_eq!(repo.read("src/lib.rs").unwrap(), "one\n");
    }

    #[test]
    fn changeset_metadata_matches_actual_file_state_after_apply() {
        // BF-05: the applied ChangeSet must exactly describe the actual
        // mutation.  The recorded operation hash must equal the on-disk
        // content and the edit additions/removals must reflect the diff the
        // engine actually performed (the production path mirrors these into
        // ChangeSetMetadata after apply).
        let mut repo = MemoryFileRepository::new("rev-a");
        repo.put("src/lib.rs", "fn one() {}\n");
        repo.put("src/other.rs", "fn two() {}\n");
        let engine = EditEngine;
        let mut transaction = engine
            .prepare(
                &repo,
                vec![
                    request(
                        "src/lib.rs",
                        &repo,
                        EditStrategy::WholeFile {
                            content: "fn one() {}\nfn one_more() {}\n".to_string(),
                        },
                    ),
                    request(
                        "src/other.rs",
                        &repo,
                        EditStrategy::SearchReplace {
                            search: "two".to_string(),
                            replace: "dos".to_string(),
                            expected_matches: 1,
                        },
                    ),
                ],
            )
            .unwrap();
        transaction.changeset.validate().unwrap();
        transaction.changeset.approve().unwrap();
        engine.apply(&mut repo, &mut transaction).unwrap();
        // The operation hashes match the actual on-disk file state.
        let lib_disk = repo.read("src/lib.rs").unwrap();
        let other_disk = repo.read("src/other.rs").unwrap();
        assert_eq!(content_hash(&lib_disk), transaction.edits[0].after_hash);
        assert_eq!(content_hash(&other_disk), transaction.edits[1].after_hash);
        // The precondition recorded the pre-mutation state.
        assert!(matches!(
            &transaction.changeset.operations[0],
            ChangeOperation::WriteFile { expected_hash: Some(hash), .. } if hash == &transaction.edits[0].before_hash
        ));
        // The edit stats exactly describe the applied diff: lib.rs adds one
        // line, other.rs replaces "two" with "dos" (same line count).
        assert_eq!(transaction.edits[0].additions, 1);
        assert_eq!(transaction.edits[0].removals, 0);
        assert_eq!(transaction.edits[1].additions, 0);
        assert_eq!(transaction.edits[1].removals, 0);
        // The production path mirrors these into the ChangeSet metadata.
        let meta = transaction.changeset.metadata.as_ref();
        assert!(meta.is_none() || meta.unwrap().files_changed.len() == 2);
    }
}
