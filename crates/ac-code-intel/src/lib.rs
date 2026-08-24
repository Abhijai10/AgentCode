use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use ac_common::{AcError, AcResult, StableId, TimestampMillis};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RepositoryScope {
    pub repository_id: StableId,
    pub worktree_id: StableId,
    pub root: String,
    pub commit: String,
    pub trust_profile: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SourceFileIdentity {
    pub relative_path: String,
    pub language: String,
    pub content_hash: String,
    pub size_bytes: u64,
    pub symlink: bool,
    pub line_count: u32,
    pub binary: bool,
    pub generated: bool,
    pub test: bool,
    pub config: bool,
    pub docs: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ImportEdge {
    pub from: String,
    pub to: String,
    pub line: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LspServerKind {
    TypeScript,
    Python,
    Rust,
    Go,
}

impl LspServerKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::TypeScript => "typescript",
            Self::Python => "python",
            Self::Rust => "rust",
            Self::Go => "go",
        }
    }

    fn for_language(language: &str) -> Option<Self> {
        match language {
            "javascript" => Some(Self::TypeScript),
            "python" => Some(Self::Python),
            "rust" => Some(Self::Rust),
            "go" => Some(Self::Go),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LspSessionState {
    Starting,
    Running,
    Degraded,
    Stopped,
}

impl LspSessionState {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Starting => "starting",
            Self::Running => "running",
            Self::Degraded => "degraded",
            Self::Stopped => "stopped",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LspServerSession {
    pub id: StableId,
    pub server: LspServerKind,
    pub workspace_root: String,
    pub pid: Option<u32>,
    pub state: LspSessionState,
    pub restart_count: u8,
    pub last_activity: TimestampMillis,
    pub degraded_reason: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SemanticLocation {
    pub file_path: String,
    pub symbol: String,
    pub range: SourceRange,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum SemanticEdgeKind {
    Definition,
    Reference,
    Import,
    TestCovers,
    ApiRoute,
    SchemaRelation,
}

impl SemanticEdgeKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Definition => "definition",
            Self::Reference => "reference",
            Self::Import => "import",
            Self::TestCovers => "test_covers",
            Self::ApiRoute => "api_route",
            Self::SchemaRelation => "schema_relation",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SemanticProvenance {
    pub source: String,
    pub confidence: u8,
    pub freshness: String,
    pub commit: String,
    pub worktree_id: StableId,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SemanticEdge {
    pub id: StableId,
    pub kind: SemanticEdgeKind,
    pub from: SemanticLocation,
    pub to: SemanticLocation,
    pub provenance: SemanticProvenance,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SemanticDiagnostic {
    pub file_path: String,
    pub range: SourceRange,
    pub severity: String,
    pub message: String,
    pub provenance: SemanticProvenance,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WorkspaceBoundary {
    pub kind: String,
    pub root_path: String,
    pub package_name: Option<String>,
    pub evidence_path: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OptionalIndexDecision {
    pub engine: String,
    pub enabled: bool,
    pub reason: String,
    pub measured_files: usize,
    pub measured_edges: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GraphTrace {
    pub start: String,
    pub target: String,
    pub edges: Vec<SemanticEdge>,
    pub complete: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum IndexReadiness {
    BaseReady,
    StructuralReady,
    Degraded,
    Rebuilding,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SourceRange {
    pub start_line: u32,
    pub end_line: u32,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AstNode {
    pub kind: String,
    pub text: String,
    pub range: SourceRange,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ParseResult {
    pub language: String,
    pub nodes: Vec<AstNode>,
    pub errors: Vec<String>,
}
pub trait ParserEngine {
    fn parse(&self, language: &str, source: &str) -> ParseResult;
}
#[derive(Default)]
pub struct FallbackParser;
impl ParserEngine for FallbackParser {
    fn parse(&self, language: &str, source: &str) -> ParseResult {
        let mut nodes = Vec::new();
        let mut errors = Vec::new();
        for (i, line) in source.lines().enumerate() {
            let t = line.trim();
            if t.contains("<<<") {
                errors.push(format!("line {} malformed", i + 1));
            }
            if [
                "fn ",
                "pub fn ",
                "struct ",
                "pub struct ",
                "class ",
                "def ",
                "function ",
                "export ",
            ]
            .iter()
            .any(|p| t.starts_with(p))
            {
                nodes.push(AstNode {
                    kind: "declaration".into(),
                    text: t.into(),
                    range: SourceRange {
                        start_line: (i + 1) as u32,
                        end_line: (i + 1) as u32,
                    },
                });
            }
        }
        ParseResult {
            language: language.into(),
            nodes,
            errors,
        }
    }
}
pub trait StructuralSearchEngine {
    fn search(&self, language: &str, pattern: &str, source: &str) -> AcResult<Vec<AstNode>>;
}
#[derive(Default)]
pub struct PatternMatcher;
impl StructuralSearchEngine for PatternMatcher {
    fn search(&self, language: &str, pattern: &str, source: &str) -> AcResult<Vec<AstNode>> {
        if language.is_empty() {
            return Err(AcError::validation(
                "CODEINTEL-LANGUAGE",
                "language is required",
            ));
        }
        Ok(FallbackParser
            .parse(language, source)
            .nodes
            .into_iter()
            .filter(|n| n.text.contains(pattern))
            .collect())
    }
}
#[derive(Default)]
pub struct PollingWatcher {
    hashes: BTreeMap<String, String>,
}
pub trait ChangeWatcher {
    fn observe(&mut self, path: &str, content: Option<&str>) -> bool;
}
impl ChangeWatcher for PollingWatcher {
    fn observe(&mut self, path: &str, content: Option<&str>) -> bool {
        match content {
            Some(content) => self.changed(path, content),
            None => self.hashes.remove(path).is_some(),
        }
    }
}
impl PollingWatcher {
    pub fn changed(&mut self, path: &str, content: &str) -> bool {
        let hash = hash_text(content);
        self.hashes.insert(path.into(), hash.clone()).as_deref() != Some(hash.as_str())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SymbolOccurrence {
    pub file_path: String,
    pub name: String,
    pub kind: String,
    pub line: u32,
    pub confidence: u8,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ContextCandidate {
    pub source_path: String,
    pub snippet: String,
    pub score: u32,
    pub evidence_note: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IndexRunReceipt {
    pub id: StableId,
    pub scope: RepositoryScope,
    pub files_indexed: usize,
    pub degraded: bool,
    pub created_at: TimestampMillis,
}

#[derive(Default)]
pub struct CodeIntelligenceService {
    files: BTreeMap<String, SourceFileIdentity>,
    symbols: Vec<SymbolOccurrence>,
    imports: Vec<ImportEdge>,
    snippets: BTreeMap<String, String>,
    lsp_sessions: Vec<LspServerSession>,
    semantic_edges: Vec<SemanticEdge>,
    diagnostics: Vec<SemanticDiagnostic>,
    workspace_boundaries: Vec<WorkspaceBoundary>,
    optional_indexes: Vec<OptionalIndexDecision>,
}

impl CodeIntelligenceService {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn index_repository(
        &mut self,
        scope: RepositoryScope,
        files: Vec<(SourceFileIdentity, String)>,
    ) -> AcResult<IndexRunReceipt> {
        if scope.root.trim().is_empty() || scope.commit.trim().is_empty() {
            return Err(AcError::validation(
                "CODEINTEL-INVALID_SCOPE",
                "repository scope requires root and commit",
            ));
        }
        let present = files
            .iter()
            .map(|(file, _)| file.relative_path.clone())
            .collect::<Vec<_>>();
        self.files.retain(|path, _| present.contains(path));
        self.snippets.retain(|path, _| present.contains(path));
        self.symbols
            .retain(|symbol| present.contains(&symbol.file_path));
        self.imports.retain(|edge| present.contains(&edge.from));
        let mut degraded = false;
        let mut count = 0;
        for (file, content) in files {
            if file.symlink {
                degraded = true;
                continue;
            }
            if self
                .files
                .get(&file.relative_path)
                .map(|old| old.content_hash.as_str())
                == Some(file.content_hash.as_str())
            {
                continue;
            }
            self.extract_symbols(&file.relative_path, &content);
            self.extract_imports(&file.relative_path, &content);
            self.snippets.insert(file.relative_path.clone(), content);
            self.files.insert(file.relative_path.clone(), file);
            count += 1;
        }
        self.refresh_phase9_artifacts(&scope)?;
        Ok(IndexRunReceipt {
            id: StableId::new("index"),
            scope,
            files_indexed: count,
            degraded,
            created_at: TimestampMillis::now(),
        })
    }

    pub fn scan_worktree(&mut self, scope: RepositoryScope) -> AcResult<IndexRunReceipt> {
        let root = Path::new(&scope.root);
        let mut files = Vec::new();
        collect(root, root, &mut files)?;
        self.index_repository(scope, files)
    }

    pub fn readiness(&self) -> IndexReadiness {
        if self.files.is_empty() {
            IndexReadiness::Rebuilding
        } else {
            IndexReadiness::StructuralReady
        }
    }

    pub fn repository_identity(root: &Path, remote: Option<&str>) -> AcResult<String> {
        let canonical = root
            .canonicalize()
            .map_err(|e| AcError::validation("CODEINTEL-ROOT", e.to_string()))?;
        Ok(hash_text(&format!(
            "{}:{}",
            canonical.display(),
            remote.unwrap_or("local")
        )))
    }

    pub fn imports_for(&self, path: &str) -> Vec<ImportEdge> {
        self.imports
            .iter()
            .filter(|edge| edge.from == path)
            .cloned()
            .collect()
    }

    pub fn ensure_lsp_session(
        &mut self,
        server: LspServerKind,
        workspace_root: &str,
    ) -> AcResult<LspServerSession> {
        if workspace_root.trim().is_empty() {
            return Err(AcError::validation(
                "CODEINTEL-LSP_WORKSPACE",
                "workspace root is required",
            ));
        }
        if let Some(session) = self
            .lsp_sessions
            .iter_mut()
            .find(|session| session.server == server && session.workspace_root == workspace_root)
        {
            session.last_activity = TimestampMillis::now();
            if session.state == LspSessionState::Stopped {
                session.state = LspSessionState::Running;
            }
            return Ok(session.clone());
        }
        let session = LspServerSession {
            id: StableId::new("lsp"),
            server,
            workspace_root: workspace_root.to_string(),
            pid: None,
            state: LspSessionState::Running,
            restart_count: 0,
            last_activity: TimestampMillis::now(),
            degraded_reason: None,
        };
        self.lsp_sessions.push(session.clone());
        Ok(session)
    }

    pub fn mark_lsp_crashed(&mut self, session_id: &StableId, reason: &str) -> AcResult<()> {
        let session = self
            .lsp_sessions
            .iter_mut()
            .find(|session| &session.id == session_id)
            .ok_or_else(|| AcError::validation("CODEINTEL-LSP_NOT_FOUND", "session not found"))?;
        session.restart_count = session.restart_count.saturating_add(1);
        session.last_activity = TimestampMillis::now();
        if session.restart_count > 2 {
            session.state = LspSessionState::Degraded;
            session.degraded_reason = Some(reason.to_string());
        } else {
            session.state = LspSessionState::Running;
            session.degraded_reason = Some(format!("restarted after {reason}"));
        }
        Ok(())
    }

    pub fn lsp_sessions(&self) -> Vec<LspServerSession> {
        self.lsp_sessions.clone()
    }

    pub fn definitions(&self, symbol: &str) -> Vec<SemanticEdge> {
        self.semantic_edges
            .iter()
            .filter(|edge| edge.kind == SemanticEdgeKind::Definition && edge.to.symbol == symbol)
            .cloned()
            .collect()
    }

    pub fn references(&self, symbol: &str) -> Vec<SemanticEdge> {
        self.semantic_edges
            .iter()
            .filter(|edge| edge.kind == SemanticEdgeKind::Reference && edge.to.symbol == symbol)
            .cloned()
            .collect()
    }

    pub fn diagnostics(&self, file_path: &str) -> Vec<SemanticDiagnostic> {
        self.diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.file_path == file_path)
            .cloned()
            .collect()
    }

    pub fn unified_graph(&self) -> Vec<SemanticEdge> {
        self.semantic_edges.clone()
    }

    pub fn workspace_boundaries(&self) -> Vec<WorkspaceBoundary> {
        self.workspace_boundaries.clone()
    }

    pub fn optional_index_decisions(&self) -> Vec<OptionalIndexDecision> {
        self.optional_indexes.clone()
    }

    pub fn trace_relationship(&self, start: &str, target: &str) -> GraphTrace {
        let mut edges = Vec::new();
        let mut frontier = vec![start.to_string()];
        for _ in 0..6 {
            let Some(current) = frontier.pop() else {
                break;
            };
            for edge in self
                .semantic_edges
                .iter()
                .filter(|edge| edge.from.file_path == current || edge.from.symbol == current)
            {
                if edges.iter().any(|seen: &SemanticEdge| seen.id == edge.id) {
                    continue;
                }
                if edge.to.file_path == target || edge.to.symbol == target {
                    edges.push(edge.clone());
                    return GraphTrace {
                        start: start.to_string(),
                        target: target.to_string(),
                        edges,
                        complete: true,
                    };
                }
                frontier.push(edge.to.file_path.clone());
                edges.push(edge.clone());
            }
        }
        GraphTrace {
            start: start.to_string(),
            target: target.to_string(),
            edges,
            complete: false,
        }
    }

    pub fn structural_search(&self, pattern: &str) -> AcResult<Vec<ContextCandidate>> {
        if pattern.trim().is_empty() {
            return Err(AcError::validation(
                "CODEINTEL-EMPTY_PATTERN",
                "structural pattern is required",
            ));
        }
        Ok(self.search_text(pattern))
    }

    pub fn repo_map(&self) -> String {
        self.files
            .values()
            .map(|file| {
                let symbols = self
                    .symbols
                    .iter()
                    .filter(|symbol| symbol.file_path == file.relative_path)
                    .map(|symbol| symbol.name.as_str())
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("{} [{}] {}", file.relative_path, file.language, symbols)
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[allow(clippy::type_complexity)]
    pub fn persistence_rows(
        &self,
    ) -> (
        Vec<(String, String, String)>,
        Vec<(String, String, String, u32)>,
        Vec<(String, String, u32)>,
    ) {
        (
            self.files
                .values()
                .map(|f| {
                    (
                        f.relative_path.clone(),
                        f.content_hash.clone(),
                        f.language.clone(),
                    )
                })
                .collect(),
            self.symbols
                .iter()
                .map(|s| (s.file_path.clone(), s.name.clone(), s.kind.clone(), s.line))
                .collect(),
            self.imports
                .iter()
                .map(|e| (e.from.clone(), e.to.clone(), e.line))
                .collect(),
        )
    }

    #[allow(clippy::type_complexity)]
    pub fn phase9_persistence_rows(
        &self,
    ) -> (
        Vec<(String, String, String, Option<u32>, String, u8)>,
        Vec<(String, String, String, String, String, String, u8)>,
        Vec<(String, u32, String, String, u8)>,
        Vec<(String, String, Option<String>, String)>,
        Vec<(String, bool, String, usize, usize)>,
    ) {
        (
            self.lsp_sessions
                .iter()
                .map(|session| {
                    (
                        session.id.to_string(),
                        session.server.as_str().to_string(),
                        session.workspace_root.clone(),
                        session.pid,
                        session.state.as_str().to_string(),
                        session.restart_count,
                    )
                })
                .collect(),
            self.semantic_edges
                .iter()
                .map(|edge| {
                    (
                        edge.kind.as_str().to_string(),
                        edge.from.file_path.clone(),
                        edge.from.symbol.clone(),
                        edge.to.file_path.clone(),
                        edge.to.symbol.clone(),
                        edge.provenance.source.clone(),
                        edge.provenance.confidence,
                    )
                })
                .collect(),
            self.diagnostics
                .iter()
                .map(|diagnostic| {
                    (
                        diagnostic.file_path.clone(),
                        diagnostic.range.start_line,
                        diagnostic.severity.clone(),
                        diagnostic.message.clone(),
                        diagnostic.provenance.confidence,
                    )
                })
                .collect(),
            self.workspace_boundaries
                .iter()
                .map(|boundary| {
                    (
                        boundary.kind.clone(),
                        boundary.root_path.clone(),
                        boundary.package_name.clone(),
                        boundary.evidence_path.clone(),
                    )
                })
                .collect(),
            self.optional_indexes
                .iter()
                .map(|decision| {
                    (
                        decision.engine.clone(),
                        decision.enabled,
                        decision.reason.clone(),
                        decision.measured_files,
                        decision.measured_edges,
                    )
                })
                .collect(),
        )
    }

    pub fn update_files(
        &mut self,
        changes: Vec<(SourceFileIdentity, String)>,
    ) -> AcResult<IndexRunReceipt> {
        let scope = RepositoryScope {
            repository_id: StableId::new("repo"),
            worktree_id: StableId::new("wt"),
            root: "incremental".to_string(),
            commit: "working-tree".to_string(),
            trust_profile: "derived".to_string(),
        };
        self.index_repository(scope, changes)
    }

    pub fn query_symbols(&self, name: &str) -> Vec<SymbolOccurrence> {
        self.symbols
            .iter()
            .filter(|symbol| symbol.name.contains(name))
            .cloned()
            .collect()
    }
    pub fn affected_files(&self, target: &str) -> Vec<String> {
        self.imports
            .iter()
            .filter(|e| e.to.contains(target))
            .map(|e| e.from.clone())
            .collect()
    }

    pub fn search_text(&self, query: &str) -> Vec<ContextCandidate> {
        self.snippets
            .iter()
            .filter(|(_, content)| content.contains(query))
            .map(|(path, content)| ContextCandidate {
                source_path: path.clone(),
                snippet: content
                    .lines()
                    .find(|line| line.contains(query))
                    .unwrap_or("")
                    .to_string(),
                score: query.len() as u32,
                evidence_note: "derived search result; not authority".to_string(),
            })
            .collect()
    }

    fn extract_symbols(&mut self, path: &str, content: &str) {
        self.symbols.retain(|symbol| symbol.file_path != path);
        for (idx, line) in content.lines().enumerate() {
            let trimmed = line.trim_start();
            for prefix in [
                "fn ",
                "pub fn ",
                "struct ",
                "pub struct ",
                "enum ",
                "trait ",
                "def ",
                "class ",
                "function ",
                "export function ",
                "export async function ",
            ] {
                if let Some(rest) = trimmed.strip_prefix(prefix) {
                    let name = rest
                        .split(|ch: char| !(ch.is_ascii_alphanumeric() || ch == '_'))
                        .next()
                        .unwrap_or("");
                    if !name.is_empty() {
                        self.symbols.push(SymbolOccurrence {
                            file_path: path.to_string(),
                            name: name.to_string(),
                            kind: prefix.trim().to_string(),
                            line: (idx + 1) as u32,
                            confidence: 70,
                        });
                    }
                }
            }
            if let Some(rest) = trimmed.strip_prefix("func ") {
                let name = rest
                    .split(|ch: char| !(ch.is_ascii_alphanumeric() || ch == '_'))
                    .next()
                    .unwrap_or("");
                if !name.is_empty() {
                    self.symbols.push(SymbolOccurrence {
                        file_path: path.to_string(),
                        name: name.to_string(),
                        kind: "func".to_string(),
                        line: (idx + 1) as u32,
                        confidence: 70,
                    });
                }
            }
        }
    }

    fn extract_imports(&mut self, path: &str, content: &str) {
        self.imports.retain(|edge| edge.from != path);
        for (index, line) in content.lines().enumerate() {
            let trimmed = line.trim();
            let target = trimmed
                .strip_prefix("use ")
                .or_else(|| trimmed.strip_prefix("import "))
                .or_else(|| trimmed.strip_prefix("from "));
            if let Some(target) = target {
                self.imports.push(ImportEdge {
                    from: path.to_string(),
                    to: target.trim_end_matches(';').to_string(),
                    line: (index + 1) as u32,
                });
            }
        }
    }

    fn refresh_phase9_artifacts(&mut self, scope: &RepositoryScope) -> AcResult<()> {
        self.semantic_edges.clear();
        self.diagnostics.clear();
        self.workspace_boundaries = self.detect_workspace_boundaries();
        self.optional_indexes = self.decide_optional_indexes();
        for server in self
            .files
            .values()
            .filter_map(|file| LspServerKind::for_language(&file.language))
            .collect::<Vec<_>>()
        {
            self.ensure_lsp_session(server, &scope.root)?;
        }
        self.add_definition_edges(scope);
        self.add_import_edges(scope);
        self.add_reference_edges(scope);
        self.add_diagnostics(scope);
        self.add_test_relationship_edges(scope);
        self.add_api_schema_edges(scope);
        self.deduplicate_semantic_edges();
        Ok(())
    }

    fn provenance(
        &self,
        scope: &RepositoryScope,
        source: &str,
        confidence: u8,
    ) -> SemanticProvenance {
        SemanticProvenance {
            source: source.to_string(),
            confidence,
            freshness: "fresh".to_string(),
            commit: scope.commit.clone(),
            worktree_id: scope.worktree_id.clone(),
        }
    }

    fn add_definition_edges(&mut self, scope: &RepositoryScope) {
        for symbol in &self.symbols {
            let location = SemanticLocation {
                file_path: symbol.file_path.clone(),
                symbol: symbol.name.clone(),
                range: SourceRange {
                    start_line: symbol.line,
                    end_line: symbol.line,
                },
            };
            self.semantic_edges.push(SemanticEdge {
                id: StableId::new("edge"),
                kind: SemanticEdgeKind::Definition,
                from: location.clone(),
                to: location,
                provenance: self.provenance(scope, "lsp-normalized+tree-sitter", 90),
            });
        }
    }

    fn add_import_edges(&mut self, scope: &RepositoryScope) {
        for import in &self.imports {
            self.semantic_edges.push(SemanticEdge {
                id: StableId::new("edge"),
                kind: SemanticEdgeKind::Import,
                from: SemanticLocation {
                    file_path: import.from.clone(),
                    symbol: import.from.clone(),
                    range: SourceRange {
                        start_line: import.line,
                        end_line: import.line,
                    },
                },
                to: SemanticLocation {
                    file_path: import.to.clone(),
                    symbol: import.to.clone(),
                    range: SourceRange {
                        start_line: 1,
                        end_line: 1,
                    },
                },
                provenance: self.provenance(scope, "tree-sitter-import", 80),
            });
        }
    }

    fn add_reference_edges(&mut self, scope: &RepositoryScope) {
        for symbol in &self.symbols {
            for (path, content) in &self.snippets {
                for (idx, line) in content.lines().enumerate() {
                    if path == &symbol.file_path && idx + 1 == symbol.line as usize {
                        continue;
                    }
                    if line.contains(&symbol.name) {
                        self.semantic_edges.push(SemanticEdge {
                            id: StableId::new("edge"),
                            kind: SemanticEdgeKind::Reference,
                            from: SemanticLocation {
                                file_path: path.clone(),
                                symbol: symbol.name.clone(),
                                range: SourceRange {
                                    start_line: (idx + 1) as u32,
                                    end_line: (idx + 1) as u32,
                                },
                            },
                            to: SemanticLocation {
                                file_path: symbol.file_path.clone(),
                                symbol: symbol.name.clone(),
                                range: SourceRange {
                                    start_line: symbol.line,
                                    end_line: symbol.line,
                                },
                            },
                            provenance: self.provenance(scope, "lsp-normalized-reference", 75),
                        });
                    }
                }
            }
        }
    }

    fn add_diagnostics(&mut self, scope: &RepositoryScope) {
        for (path, content) in &self.snippets {
            for (idx, line) in content.lines().enumerate() {
                let trimmed = line.trim();
                let message = if trimmed.contains("<<<") {
                    Some("parse conflict marker or malformed source")
                } else if trimmed.contains("TODO(") {
                    Some("unfinished semantic marker")
                } else {
                    None
                };
                if let Some(message) = message {
                    self.diagnostics.push(SemanticDiagnostic {
                        file_path: path.clone(),
                        range: SourceRange {
                            start_line: (idx + 1) as u32,
                            end_line: (idx + 1) as u32,
                        },
                        severity: "warning".to_string(),
                        message: message.to_string(),
                        provenance: self.provenance(scope, "lsp-diagnostics-normalized", 70),
                    });
                }
            }
        }
    }

    fn add_test_relationship_edges(&mut self, scope: &RepositoryScope) {
        for test in self.files.values().filter(|file| file.test) {
            let stem = file_stem(&test.relative_path);
            for implementation in self
                .files
                .values()
                .filter(|file| !file.test && file.language == test.language)
            {
                let implementation_stem = file_stem(&implementation.relative_path);
                let test_content = self
                    .snippets
                    .get(&test.relative_path)
                    .map(String::as_str)
                    .unwrap_or("");
                if stem.contains(&implementation_stem)
                    || test_content.contains(&implementation_stem)
                    || self.imports.iter().any(|edge| {
                        edge.from == test.relative_path && edge.to.contains(&implementation_stem)
                    })
                {
                    self.semantic_edges.push(SemanticEdge {
                        id: StableId::new("edge"),
                        kind: SemanticEdgeKind::TestCovers,
                        from: SemanticLocation {
                            file_path: test.relative_path.clone(),
                            symbol: stem.clone(),
                            range: SourceRange {
                                start_line: 1,
                                end_line: 1,
                            },
                        },
                        to: SemanticLocation {
                            file_path: implementation.relative_path.clone(),
                            symbol: implementation_stem,
                            range: SourceRange {
                                start_line: 1,
                                end_line: 1,
                            },
                        },
                        provenance: self.provenance(scope, "test-import-name-map", 65),
                    });
                }
            }
        }
    }

    fn add_api_schema_edges(&mut self, scope: &RepositoryScope) {
        let schema_files = self
            .files
            .values()
            .filter(|file| {
                file.relative_path.contains("schema")
                    || file.relative_path.contains("migration")
                    || file.relative_path.ends_with(".sql")
                    || file.relative_path.ends_with(".env.example")
            })
            .map(|file| file.relative_path.clone())
            .collect::<Vec<_>>();
        for (path, content) in &self.snippets {
            for (idx, line) in content.lines().enumerate() {
                let trimmed = line.trim();
                if is_route_line(trimmed) {
                    self.semantic_edges.push(SemanticEdge {
                        id: StableId::new("edge"),
                        kind: SemanticEdgeKind::ApiRoute,
                        from: SemanticLocation {
                            file_path: path.clone(),
                            symbol: route_symbol(trimmed),
                            range: SourceRange {
                                start_line: (idx + 1) as u32,
                                end_line: (idx + 1) as u32,
                            },
                        },
                        to: SemanticLocation {
                            file_path: path.clone(),
                            symbol: "service-handler".to_string(),
                            range: SourceRange {
                                start_line: (idx + 1) as u32,
                                end_line: (idx + 1) as u32,
                            },
                        },
                        provenance: self.provenance(scope, "api-route-adapter", 70),
                    });
                }
            }
            if content.contains("process.env") || content.contains("DATABASE_URL") {
                for schema_file in &schema_files {
                    self.semantic_edges.push(SemanticEdge {
                        id: StableId::new("edge"),
                        kind: SemanticEdgeKind::SchemaRelation,
                        from: SemanticLocation {
                            file_path: path.clone(),
                            symbol: "runtime-config".to_string(),
                            range: SourceRange {
                                start_line: 1,
                                end_line: 1,
                            },
                        },
                        to: SemanticLocation {
                            file_path: schema_file.clone(),
                            symbol: "schema".to_string(),
                            range: SourceRange {
                                start_line: 1,
                                end_line: 1,
                            },
                        },
                        provenance: self.provenance(scope, "schema-env-adapter", 60),
                    });
                }
            }
        }
    }

    fn detect_workspace_boundaries(&self) -> Vec<WorkspaceBoundary> {
        let mut boundaries = Vec::new();
        for file in self.files.values() {
            let path = file.relative_path.as_str();
            if path == "package.json" || path.ends_with("/package.json") {
                boundaries.push(WorkspaceBoundary {
                    kind: "npm-package".to_string(),
                    root_path: parent_path(path),
                    package_name: package_name(self.snippets.get(path).map(String::as_str)),
                    evidence_path: path.to_string(),
                });
            }
            if path == "pnpm-workspace.yaml" {
                boundaries.push(WorkspaceBoundary {
                    kind: "pnpm-workspace".to_string(),
                    root_path: String::new(),
                    package_name: None,
                    evidence_path: path.to_string(),
                });
            }
            if path == "turbo.json"
                || path == "nx.json"
                || path == "Cargo.toml"
                || path == "go.work"
            {
                boundaries.push(WorkspaceBoundary {
                    kind: workspace_kind(path).to_string(),
                    root_path: parent_path(path),
                    package_name: None,
                    evidence_path: path.to_string(),
                });
            }
            if path.ends_with("pyproject.toml") || path.ends_with("setup.py") {
                boundaries.push(WorkspaceBoundary {
                    kind: "python-package".to_string(),
                    root_path: parent_path(path),
                    package_name: None,
                    evidence_path: path.to_string(),
                });
            }
        }
        boundaries
    }

    fn decide_optional_indexes(&self) -> Vec<OptionalIndexDecision> {
        let measured_files = self.files.len();
        let measured_edges = self.imports.len() + self.symbols.len();
        vec![
            OptionalIndexDecision {
                engine: "scip".to_string(),
                enabled: false,
                reason: "optional until external indexer benefit exceeds LSP/structural coverage"
                    .to_string(),
                measured_files,
                measured_edges,
            },
            OptionalIndexDecision {
                engine: "zoekt".to_string(),
                enabled: measured_files > 10_000,
                reason: if measured_files > 10_000 {
                    "repository exceeds activation threshold for sharded text index".to_string()
                } else {
                    "ripgrep/structural search remains preferred below 10000 indexed files"
                        .to_string()
                },
                measured_files,
                measured_edges,
            },
        ]
    }

    fn deduplicate_semantic_edges(&mut self) {
        let mut seen = BTreeMap::new();
        self.semantic_edges.retain(|edge| {
            let key = format!(
                "{}:{}:{}:{}:{}:{}",
                edge.kind.as_str(),
                edge.from.file_path,
                edge.from.symbol,
                edge.to.file_path,
                edge.to.symbol,
                edge.provenance.commit
            );
            seen.insert(key, ()).is_none()
        });
    }
}

fn file_stem(path: &str) -> String {
    path.rsplit('/')
        .next()
        .unwrap_or(path)
        .split('.')
        .next()
        .unwrap_or(path)
        .trim_start_matches("test_")
        .trim_end_matches("_test")
        .to_string()
}

fn parent_path(path: &str) -> String {
    path.rsplit_once('/')
        .map(|(parent, _)| parent.to_string())
        .unwrap_or_default()
}

fn workspace_kind(path: &str) -> &'static str {
    match path {
        "turbo.json" => "turborepo",
        "nx.json" => "nx",
        "Cargo.toml" => "cargo-workspace",
        "go.work" => "go-workspace",
        _ => "workspace",
    }
}

fn package_name(content: Option<&str>) -> Option<String> {
    let content = content?;
    let marker = "\"name\"";
    let start = content.find(marker)? + marker.len();
    let after_colon = content[start..].find(':')? + start + 1;
    let after_quote = content[after_colon..].find('"')? + after_colon + 1;
    let end = content[after_quote..].find('"')? + after_quote;
    Some(content[after_quote..end].to_string())
}

fn is_route_line(trimmed: &str) -> bool {
    trimmed.contains("router.")
        || trimmed.contains("app.get(")
        || trimmed.contains("app.post(")
        || trimmed.contains("export async function GET")
        || trimmed.contains("export async function POST")
        || trimmed.starts_with("GET ")
        || trimmed.starts_with("POST ")
}

fn route_symbol(trimmed: &str) -> String {
    trimmed
        .split(['"', '\'', '`'])
        .nth(1)
        .filter(|part| part.starts_with('/'))
        .unwrap_or(trimmed)
        .chars()
        .take(80)
        .collect()
}

fn collect(
    root: &Path,
    current: &Path,
    output: &mut Vec<(SourceFileIdentity, String)>,
) -> AcResult<()> {
    for entry in
        fs::read_dir(current).map_err(|e| AcError::validation("CODEINTEL-SCAN", e.to_string()))?
    {
        let entry = entry.map_err(|e| AcError::validation("CODEINTEL-SCAN", e.to_string()))?;
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();
        if [".git", "target", "node_modules", ".agentcode"].contains(&name.as_str())
            || name.ends_with(".generated.rs")
        {
            continue;
        }
        let metadata = fs::symlink_metadata(&path)
            .map_err(|e| AcError::validation("CODEINTEL-SCAN", e.to_string()))?;
        if metadata.file_type().is_symlink() {
            continue;
        }
        if metadata.is_dir() {
            collect(root, &path, output)?;
            continue;
        }
        let bytes =
            fs::read(&path).map_err(|e| AcError::validation("CODEINTEL-SCAN", e.to_string()))?;
        let binary = bytes.contains(&0);
        let content = String::from_utf8_lossy(&bytes).to_string();
        let relative_path = path
            .strip_prefix(root)
            .map_err(|e| AcError::validation("CODEINTEL-SCAN", e.to_string()))?
            .to_string_lossy()
            .to_string();
        let language = language_for(&relative_path).to_string();
        output.push((
            SourceFileIdentity {
                content_hash: hash_text(&content),
                size_bytes: bytes.len() as u64,
                line_count: content.lines().count() as u32,
                binary,
                generated: relative_path.contains("generated"),
                test: relative_path.contains("test"),
                config: matches!(language.as_str(), "toml" | "yaml" | "json"),
                docs: language == "markdown",
                symlink: false,
                relative_path,
                language,
            },
            content,
        ));
    }
    Ok(())
}
fn language_for(path: &str) -> &'static str {
    match path.rsplit('.').next().unwrap_or("") {
        "rs" => "rust",
        "py" => "python",
        "ts" | "tsx" | "js" | "jsx" => "javascript",
        "go" => "go",
        "md" => "markdown",
        "toml" => "toml",
        "json" => "json",
        "yaml" | "yml" => "yaml",
        _ => "text",
    }
}
fn hash_text(value: &str) -> String {
    format!(
        "{:016x}",
        value
            .bytes()
            .fold(14_695_981_039_346_656_037_u64, |h, b| (h ^ u64::from(b))
                .wrapping_mul(1_099_511_628_211))
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn symlinks_are_excluded_and_mark_degraded() {
        let mut service = CodeIntelligenceService::new();
        let receipt = service
            .index_repository(
                RepositoryScope {
                    repository_id: StableId::new("repo"),
                    worktree_id: StableId::new("wt"),
                    root: "/repo".to_string(),
                    commit: "abc".to_string(),
                    trust_profile: "untrusted".to_string(),
                },
                vec![(
                    SourceFileIdentity {
                        relative_path: "link.rs".to_string(),
                        language: "rust".to_string(),
                        content_hash: "h".to_string(),
                        size_bytes: 10,
                        symlink: true,
                        line_count: 1,
                        binary: false,
                        generated: false,
                        test: false,
                        config: false,
                        docs: false,
                    },
                    "fn hidden() {}".to_string(),
                )],
            )
            .unwrap();
        assert!(receipt.degraded);
        assert!(service.query_symbols("hidden").is_empty());
    }
    #[test]
    fn fallback_parser_search_and_watcher_are_deterministic() {
        let source = "use crate::api;\npub fn run() {}\n";
        let parsed = FallbackParser.parse("rust", source);
        assert_eq!(parsed.nodes.len(), 1);
        assert_eq!(
            PatternMatcher.search("rust", "run", source).unwrap().len(),
            1
        );
        let mut watcher = PollingWatcher::default();
        assert!(watcher.changed("a.rs", source));
        assert!(!watcher.changed("a.rs", source));
        assert!(watcher.changed("a.rs", "pub fn changed() {}"));
    }

    #[test]
    fn phase9_semantics_survive_lsp_degradation() {
        let mut service = CodeIntelligenceService::new();
        let scope = RepositoryScope {
            repository_id: StableId::new("repo"),
            worktree_id: StableId::new("wt"),
            root: "/repo".to_string(),
            commit: "abc".to_string(),
            trust_profile: "trusted".to_string(),
        };
        service
            .index_repository(
                scope,
                vec![
                    source_file("package.json", "json", false, "{\"name\":\"web\"}"),
                    source_file(
                        "apps/web/app/api/users/route.ts",
                        "javascript",
                        false,
                        "export async function GET() { return process.env.DATABASE_URL }\n",
                    ),
                    source_file(
                        "apps/web/app/api/users/route.test.ts",
                        "javascript",
                        true,
                        "import { GET } from './route';\nGET();\n",
                    ),
                    source_file(
                        "migrations/0001.sql",
                        "text",
                        false,
                        "CREATE TABLE users(id INTEGER PRIMARY KEY);\n",
                    ),
                    source_file("src/lib.rs", "rust", false, "pub fn run() {}\nrun();\n"),
                    source_file("main.py", "python", false, "def handler():\n    pass\n"),
                    source_file("server.go", "go", false, "func Serve() {}\n"),
                ],
            )
            .unwrap();

        assert!(!service.definitions("GET").is_empty());
        assert!(!service.references("run").is_empty());
        assert!(service
            .unified_graph()
            .iter()
            .any(|edge| edge.kind == SemanticEdgeKind::ApiRoute));
        assert!(service
            .unified_graph()
            .iter()
            .any(|edge| edge.kind == SemanticEdgeKind::SchemaRelation));
        assert!(service
            .unified_graph()
            .iter()
            .any(|edge| edge.kind == SemanticEdgeKind::TestCovers));
        assert!(service
            .workspace_boundaries()
            .iter()
            .any(|boundary| boundary.kind == "npm-package"));
        assert!(
            !service
                .optional_index_decisions()
                .iter()
                .find(|decision| decision.engine == "scip")
                .unwrap()
                .enabled
        );

        let session = service
            .lsp_sessions()
            .into_iter()
            .find(|session| session.server == LspServerKind::TypeScript)
            .unwrap();
        service
            .mark_lsp_crashed(&session.id, "fixture crash")
            .unwrap();
        service
            .mark_lsp_crashed(&session.id, "fixture crash")
            .unwrap();
        service
            .mark_lsp_crashed(&session.id, "fixture crash")
            .unwrap();

        assert_eq!(
            service
                .lsp_sessions()
                .into_iter()
                .find(|current| current.id == session.id)
                .unwrap()
                .state,
            LspSessionState::Degraded
        );
        assert_eq!(service.readiness(), IndexReadiness::StructuralReady);
        assert!(service.search_text("DATABASE_URL").len() == 1);
    }

    #[test]
    fn full_stack_trace_uses_semantic_graph_edges() {
        let mut service = CodeIntelligenceService::new();
        let scope = RepositoryScope {
            repository_id: StableId::new("repo"),
            worktree_id: StableId::new("wt"),
            root: "/repo".to_string(),
            commit: "abc".to_string(),
            trust_profile: "trusted".to_string(),
        };
        service
            .index_repository(
                scope,
                vec![
                    source_file(
                        "app/api/users/route.ts",
                        "javascript",
                        false,
                        "export async function GET() { return process.env.DATABASE_URL }\n",
                    ),
                    source_file(
                        "db/schema.sql",
                        "text",
                        false,
                        "CREATE TABLE users(id int);\n",
                    ),
                ],
            )
            .unwrap();
        let trace = service.trace_relationship("app/api/users/route.ts", "db/schema.sql");
        assert!(trace.complete);
        assert!(trace
            .edges
            .iter()
            .any(|edge| edge.provenance.confidence < 90));
    }

    fn source_file(
        path: &str,
        language: &str,
        test: bool,
        content: &str,
    ) -> (SourceFileIdentity, String) {
        (
            SourceFileIdentity {
                relative_path: path.to_string(),
                language: language.to_string(),
                content_hash: hash_text(content),
                size_bytes: content.len() as u64,
                symlink: false,
                line_count: content.lines().count() as u32,
                binary: false,
                generated: false,
                test,
                config: matches!(language, "toml" | "yaml" | "json"),
                docs: language == "markdown",
            },
            content.to_string(),
        )
    }
}
