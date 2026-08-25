use std::collections::BTreeMap;
use std::fs;
use std::io::{BufRead, BufReader, Read, Write};
use std::path::Path;
use std::process::{Child, ChildStdin, Command, Stdio};
use std::time::{Duration, Instant};

use ac_common::{AcError, AcResult, StableId, TimestampMillis};
use ac_sandbox::{ExecRequest, IsolationLevel, SandboxManager, SandboxPolicy};
use ac_security::{Capability, CapabilityPolicy};
use serde_json::{json, Value};
use tree_sitter::{Language, Node, Parser, Point, Tree};

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
            "javascript" | "typescript" | "tsx" => Some(Self::TypeScript),
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
    Available,
    Ready,
    Running,
    Failed,
    Unavailable,
    Restarting,
    Degraded,
    Stopped,
}

impl LspSessionState {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Starting => "starting",
            Self::Available => "available",
            Self::Ready => "ready",
            Self::Running => "running",
            Self::Failed => "failed",
            Self::Unavailable => "unavailable",
            Self::Restarting => "restarting",
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
pub struct LspTextEdit {
    pub file_path: String,
    pub range: SourceRange,
    pub start_character: u32,
    pub end_character: u32,
    pub new_text: String,
    pub provenance: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LspWorkspaceEdit {
    pub server: LspServerKind,
    pub edits: Vec<LspTextEdit>,
}

pub struct LspClient {
    server: LspServerKind,
    workspace_root: String,
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<std::process::ChildStdout>,
    next_id: u64,
    document_versions: BTreeMap<String, i32>,
}

impl LspClient {
    pub fn start(server: LspServerKind, workspace_root: impl Into<String>) -> AcResult<Self> {
        let workspace_root = workspace_root.into();
        let executable = discover_lsp_executable(server).ok_or_else(|| {
            AcError::new(
                "CODEINTEL-LANGUAGE_SERVER_UNAVAILABLE",
                format!("{} language server is not installed", server.as_str()),
                ac_common::ErrorKind::Unavailable,
                ac_common::Retryability::NotRetryable,
            )
        })?;
        let cwd = Path::new(&workspace_root)
            .canonicalize()
            .map_err(|error| AcError::validation("CODEINTEL-LSP_WORKSPACE", error.to_string()))?;
        let sandbox = SandboxManager::new(SandboxPolicy {
            capability_policy: CapabilityPolicy::new()
                .allow(Capability::ProcessExec("*".to_string())),
            required_isolation: IsolationLevel::ProcessRestricted,
            ..SandboxPolicy::new(vec![cwd.clone()])
        });
        let plan = sandbox.prepare_execution(ExecRequest {
            argv: vec![executable],
            cwd,
            env: toolchain_env(),
            network: false,
            timeout_ms: 30_000,
        })?;
        let mut command = Command::new(&plan.backend_argv[0]);
        command
            .args(&plan.backend_argv[1..])
            .current_dir(&plan.cwd)
            .env_clear()
            .envs(&plan.allowed_env)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null());
        let mut child = command.spawn().map_err(|error| {
            AcError::new(
                "CODEINTEL-LANGUAGE_SERVER_STARTUP_FAILED",
                error.to_string(),
                ac_common::ErrorKind::Unavailable,
                ac_common::Retryability::Retryable,
            )
        })?;
        let stdin = child.stdin.take().ok_or_else(|| {
            AcError::validation("CODEINTEL-LSP_STDIN", "language server stdin unavailable")
        })?;
        let stdout = child.stdout.take().ok_or_else(|| {
            AcError::validation("CODEINTEL-LSP_STDOUT", "language server stdout unavailable")
        })?;
        let mut client = Self {
            server,
            workspace_root,
            child,
            stdin,
            stdout: BufReader::new(stdout),
            next_id: 1,
            document_versions: BTreeMap::new(),
        };
        client.initialize()?;
        Ok(client)
    }

    pub fn initialize(&mut self) -> AcResult<()> {
        let root_uri = file_uri(Path::new(&self.workspace_root))?;
        let id = self.request(
            "initialize",
            json!({
                "processId": null,
                "rootUri": root_uri,
                "capabilities": {
                    "textDocument": {
                        "definition": {},
                        "references": {},
                        "rename": { "prepareSupport": true },
                        "publishDiagnostics": {}
                    },
                    "workspace": { "workspaceEdit": { "documentChanges": true } }
                }
            }),
        )?;
        let _ = self.wait_response(id, Duration::from_secs(10))?;
        self.notify("initialized", json!({}))?;
        Ok(())
    }

    pub fn did_open(&mut self, path: &Path, language: SourceLanguage, text: &str) -> AcResult<i32> {
        let version = self
            .document_versions
            .get(&path.display().to_string())
            .copied()
            .unwrap_or(0)
            + 1;
        self.document_versions
            .insert(path.display().to_string(), version);
        self.notify(
            "textDocument/didOpen",
            json!({
                "textDocument": {
                    "uri": file_uri(path)?,
                    "languageId": language.lsp_language_id(),
                    "version": version,
                    "text": text
                }
            }),
        )?;
        Ok(version)
    }

    pub fn definition(&mut self, path: &Path, line: u32, character: u32) -> AcResult<Value> {
        self.position_request("textDocument/definition", path, line, character)
    }

    pub fn references(&mut self, path: &Path, line: u32, character: u32) -> AcResult<Value> {
        let id = self.request(
            "textDocument/references",
            json!({
                "textDocument": { "uri": file_uri(path)? },
                "position": { "line": line, "character": character },
                "context": { "includeDeclaration": true }
            }),
        )?;
        self.wait_response(id, Duration::from_secs(10))
    }

    pub fn rename(
        &mut self,
        path: &Path,
        line: u32,
        character: u32,
        new_name: &str,
    ) -> AcResult<LspWorkspaceEdit> {
        let id = self.request(
            "textDocument/rename",
            json!({
                "textDocument": { "uri": file_uri(path)? },
                "position": { "line": line, "character": character },
                "newName": new_name
            }),
        )?;
        let response = self.wait_response(id, Duration::from_secs(15))?;
        workspace_edit_from_lsp(self.server, &response)
    }

    pub fn shutdown(mut self) -> AcResult<()> {
        let id = self.request("shutdown", Value::Null)?;
        let _ = self.wait_response(id, Duration::from_secs(5));
        let _ = self.notify("exit", Value::Null);
        let _ = self.child.wait();
        Ok(())
    }

    fn position_request(
        &mut self,
        method: &str,
        path: &Path,
        line: u32,
        character: u32,
    ) -> AcResult<Value> {
        let id = self.request(
            method,
            json!({
                "textDocument": { "uri": file_uri(path)? },
                "position": { "line": line, "character": character }
            }),
        )?;
        self.wait_response(id, Duration::from_secs(10))
    }

    fn request(&mut self, method: &str, params: Value) -> AcResult<u64> {
        let id = self.next_id;
        self.next_id += 1;
        self.write_message(json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params
        }))?;
        Ok(id)
    }

    fn notify(&mut self, method: &str, params: Value) -> AcResult<()> {
        self.write_message(json!({
            "jsonrpc": "2.0",
            "method": method,
            "params": params
        }))
    }

    fn write_message(&mut self, message: Value) -> AcResult<()> {
        let body = serde_json::to_vec(&message)
            .map_err(|error| AcError::validation("CODEINTEL-LSP_JSON", error.to_string()))?;
        write!(self.stdin, "Content-Length: {}\r\n\r\n", body.len())
            .and_then(|_| self.stdin.write_all(&body))
            .and_then(|_| self.stdin.flush())
            .map_err(|error| AcError::validation("CODEINTEL-LSP_WRITE", error.to_string()))
    }

    fn wait_response(&mut self, id: u64, timeout: Duration) -> AcResult<Value> {
        let deadline = Instant::now() + timeout;
        while Instant::now() < deadline {
            let message = self.read_message()?;
            if let (Some(method), Some(request_id)) = (
                message.get("method").and_then(Value::as_str),
                message.get("id"),
            ) {
                self.respond_to_server_request(method, request_id.clone())?;
                continue;
            }
            if message.get("id").and_then(Value::as_u64) != Some(id) {
                continue;
            }
            if let Some(error) = message.get("error") {
                return Err(AcError::validation(
                    "CODEINTEL-LSP_RESPONSE_ERROR",
                    error.to_string(),
                ));
            }
            return Ok(message.get("result").cloned().unwrap_or(Value::Null));
        }
        Err(AcError::new(
            "CODEINTEL-LANGUAGE_SERVER_TIMEOUT",
            "language server request timed out",
            ac_common::ErrorKind::Unavailable,
            ac_common::Retryability::Retryable,
        ))
    }

    fn respond_to_server_request(&mut self, method: &str, request_id: Value) -> AcResult<()> {
        self.write_message(server_request_response(method, request_id))
    }

    fn read_message(&mut self) -> AcResult<Value> {
        let mut content_length = None;
        loop {
            let mut line = String::new();
            let read = self
                .stdout
                .read_line(&mut line)
                .map_err(|error| AcError::validation("CODEINTEL-LSP_READ", error.to_string()))?;
            if read == 0 {
                return Err(AcError::new(
                    "CODEINTEL-LANGUAGE_SERVER_CRASHED",
                    "language server stdout closed",
                    ac_common::ErrorKind::Unavailable,
                    ac_common::Retryability::Retryable,
                ));
            }
            let trimmed = line.trim_end();
            if trimmed.is_empty() {
                break;
            }
            if let Some(value) = trimmed.strip_prefix("Content-Length:") {
                content_length = Some(value.trim().parse::<usize>().map_err(|error| {
                    AcError::validation("CODEINTEL-LSP_FRAME", error.to_string())
                })?);
            }
        }
        let length = content_length.ok_or_else(|| {
            AcError::validation("CODEINTEL-LSP_FRAME", "missing Content-Length header")
        })?;
        let mut body = vec![0; length];
        self.stdout
            .read_exact(&mut body)
            .map_err(|error| AcError::validation("CODEINTEL-LSP_READ", error.to_string()))?;
        serde_json::from_slice(&body).map_err(|error| {
            AcError::validation("CODEINTEL-INVALID_LSP_RESPONSE", error.to_string())
        })
    }
}

fn server_request_response(method: &str, request_id: Value) -> Value {
    if method == "workspace/configuration" {
        json!({ "jsonrpc": "2.0", "id": request_id, "result": [] })
    } else {
        json!({
            "jsonrpc": "2.0",
            "id": request_id,
            "error": { "code": -32601, "message": "client method not supported" }
        })
    }
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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ParseStatus {
    Parsed,
    SyntaxErrors,
    Unsupported,
    Failed,
}

impl ParseStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Parsed => "Parsed",
            Self::SyntaxErrors => "SyntaxErrors",
            Self::Unsupported => "Unsupported",
            Self::Failed => "Failed",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ByteRange {
    pub start_byte: usize,
    pub end_byte: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AstNode {
    pub kind: String,
    pub text: String,
    pub range: SourceRange,
    pub byte_range: ByteRange,
    pub name: Option<String>,
    pub role: String,
    pub provenance: String,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ParseResult {
    pub language: String,
    pub nodes: Vec<AstNode>,
    pub errors: Vec<String>,
    pub status: ParseStatus,
    pub provenance: String,
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
                    byte_range: ByteRange {
                        start_byte: 0,
                        end_byte: t.len(),
                    },
                    name: None,
                    role: "text-fallback-declaration".to_string(),
                    provenance: "TextFallback".to_string(),
                });
            }
        }
        ParseResult {
            language: language.into(),
            nodes,
            errors,
            status: ParseStatus::Parsed,
            provenance: "TextFallback".to_string(),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SourceLanguage {
    Rust,
    Python,
    JavaScript,
    TypeScript,
    Tsx,
    Go,
    Unknown,
}

impl SourceLanguage {
    pub fn from_path(path: &str) -> Self {
        match path.rsplit('.').next().unwrap_or("") {
            "rs" => Self::Rust,
            "py" => Self::Python,
            "js" | "jsx" => Self::JavaScript,
            "ts" => Self::TypeScript,
            "tsx" => Self::Tsx,
            "go" => Self::Go,
            _ => Self::Unknown,
        }
    }

    pub fn from_name(language: &str) -> Self {
        match language {
            "rust" => Self::Rust,
            "python" => Self::Python,
            "javascript" => Self::JavaScript,
            "typescript" => Self::TypeScript,
            "tsx" => Self::Tsx,
            "go" => Self::Go,
            _ => Self::Unknown,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Rust => "rust",
            Self::Python => "python",
            Self::JavaScript => "javascript",
            Self::TypeScript => "typescript",
            Self::Tsx => "tsx",
            Self::Go => "go",
            Self::Unknown => "unknown",
        }
    }

    fn tree_sitter_language(self) -> Option<Language> {
        match self {
            Self::Rust => Some(tree_sitter_rust::LANGUAGE.into()),
            Self::Python => Some(tree_sitter_python::LANGUAGE.into()),
            Self::JavaScript => Some(tree_sitter_javascript::LANGUAGE.into()),
            Self::TypeScript => Some(tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into()),
            Self::Tsx => Some(tree_sitter_typescript::LANGUAGE_TSX.into()),
            Self::Go => Some(tree_sitter_go::LANGUAGE.into()),
            Self::Unknown => None,
        }
    }

    fn lsp_language_id(self) -> &'static str {
        match self {
            Self::Rust => "rust",
            Self::Python => "python",
            Self::JavaScript => "javascript",
            Self::TypeScript => "typescript",
            Self::Tsx => "typescriptreact",
            Self::Go => "go",
            Self::Unknown => "plaintext",
        }
    }
}

#[derive(Default)]
pub struct TreeSitterParser;

impl ParserEngine for TreeSitterParser {
    fn parse(&self, language: &str, source: &str) -> ParseResult {
        let source_language = SourceLanguage::from_name(language);
        parse_tree_sitter(source_language, source)
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
        Ok(TreeSitterParser
            .parse(language, source)
            .nodes
            .into_iter()
            .filter(|n| structural_pattern_matches(n, pattern))
            .collect())
    }
}

pub fn structural_rewrite(
    language: &str,
    source: &str,
    pattern: &str,
    rewrite: &str,
) -> AcResult<String> {
    if pattern.trim().is_empty() {
        return Err(AcError::validation(
            "CODEINTEL-INVALID_STRUCTURAL_PATTERN",
            "structural pattern cannot be empty",
        ));
    }
    let parsed = TreeSitterParser.parse(language, source);
    if parsed.status == ParseStatus::Unsupported {
        return Err(AcError::validation(
            "CODEINTEL-UNSUPPORTED_LANGUAGE",
            format!("no Tree-sitter grammar is registered for {language}"),
        ));
    }
    if parsed.status == ParseStatus::Failed || parsed.status == ParseStatus::SyntaxErrors {
        return Err(AcError::validation(
            "CODEINTEL-PARSE_FAILED",
            parsed.errors.join("; "),
        ));
    }
    let mut matches = parsed
        .nodes
        .into_iter()
        .filter(|node| structural_pattern_matches(node, pattern))
        .collect::<Vec<_>>();
    if matches.is_empty() {
        return Err(AcError::conflict(
            "CODEINTEL-STRUCTURAL_MATCH_NOT_FOUND",
            "structural rewrite found no AST matches",
        ));
    }
    matches.sort_by_key(|node| std::cmp::Reverse(node.byte_range.start_byte));
    let mut output = source.to_string();
    for node in matches {
        output.replace_range(
            node.byte_range.start_byte..node.byte_range.end_byte,
            rewrite,
        );
    }
    Ok(output)
}

fn parse_tree_sitter(language: SourceLanguage, source: &str) -> ParseResult {
    let Some(ts_language) = language.tree_sitter_language() else {
        return ParseResult {
            language: language.as_str().to_string(),
            nodes: Vec::new(),
            errors: vec![format!("unsupported language {}", language.as_str())],
            status: ParseStatus::Unsupported,
            provenance: "Unsupported".to_string(),
        };
    };
    let mut parser = Parser::new();
    if let Err(error) = parser.set_language(&ts_language) {
        return ParseResult {
            language: language.as_str().to_string(),
            nodes: Vec::new(),
            errors: vec![error.to_string()],
            status: ParseStatus::Failed,
            provenance: "TreeSitter".to_string(),
        };
    }
    let Some(tree) = parser.parse(source, None) else {
        return ParseResult {
            language: language.as_str().to_string(),
            nodes: Vec::new(),
            errors: vec!["parser returned no tree".to_string()],
            status: ParseStatus::Failed,
            provenance: "TreeSitter".to_string(),
        };
    };
    let mut nodes = Vec::new();
    let mut errors = Vec::new();
    collect_tree_sitter_nodes(
        language,
        source,
        &tree,
        tree.root_node(),
        &mut nodes,
        &mut errors,
    );
    ParseResult {
        language: language.as_str().to_string(),
        nodes,
        errors,
        status: if tree.root_node().has_error() {
            ParseStatus::SyntaxErrors
        } else {
            ParseStatus::Parsed
        },
        provenance: "TreeSitter".to_string(),
    }
}

fn collect_tree_sitter_nodes(
    language: SourceLanguage,
    source: &str,
    _tree: &Tree,
    node: Node<'_>,
    output: &mut Vec<AstNode>,
    errors: &mut Vec<String>,
) {
    if node.is_error() || node.is_missing() {
        errors.push(format!(
            "{} node at line {}",
            node.kind(),
            node.start_position().row + 1
        ));
    }
    if let Some((role, name_node)) = structural_node(language, node) {
        let text = node.utf8_text(source.as_bytes()).unwrap_or("").to_string();
        let name = name_node
            .utf8_text(source.as_bytes())
            .ok()
            .map(ToString::to_string);
        output.push(AstNode {
            kind: node.kind().to_string(),
            text,
            range: source_range(node.start_position(), node.end_position()),
            byte_range: ByteRange {
                start_byte: node.start_byte(),
                end_byte: node.end_byte(),
            },
            name,
            role: role.to_string(),
            provenance: "TreeSitter".to_string(),
        });
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_tree_sitter_nodes(language, source, _tree, child, output, errors);
    }
}

fn structural_node<'a>(
    language: SourceLanguage,
    node: Node<'a>,
) -> Option<(&'static str, Node<'a>)> {
    let kind = node.kind();
    let name = node.child_by_field_name("name");
    match language {
        SourceLanguage::Rust => match kind {
            "function_item" => name.map(|node| ("function", node)),
            "struct_item" => name.map(|node| ("struct", node)),
            "enum_item" => name.map(|node| ("enum", node)),
            "trait_item" => name.map(|node| ("trait", node)),
            "mod_item" => name.map(|node| ("module", node)),
            "const_item" => name.map(|node| ("constant", node)),
            "static_item" => name.map(|node| ("static", node)),
            "use_declaration" => Some(("import", node)),
            "call_expression" => node.child(0).map(|node| ("call", node)),
            _ => None,
        },
        SourceLanguage::Python => match kind {
            "function_definition" => name.map(|node| ("function", node)),
            "class_definition" => name.map(|node| ("class", node)),
            "import_statement" | "import_from_statement" => Some(("import", node)),
            "call" => node.child(0).map(|node| ("call", node)),
            _ => None,
        },
        SourceLanguage::JavaScript | SourceLanguage::TypeScript | SourceLanguage::Tsx => match kind
        {
            "function_declaration" => name.map(|node| ("function", node)),
            "method_definition" => name.map(|node| ("method", node)),
            "class_declaration" => name.map(|node| ("class", node)),
            "interface_declaration" => name.map(|node| ("interface", node)),
            "lexical_declaration" | "variable_declaration" => Some(("declaration", node)),
            "import_statement" | "export_statement" => Some(("import", node)),
            "call_expression" => node
                .child_by_field_name("function")
                .map(|node| ("call", node)),
            _ => None,
        },
        SourceLanguage::Go => match kind {
            "function_declaration" | "method_declaration" => name.map(|node| ("function", node)),
            "type_declaration" => Some(("type", node)),
            "import_declaration" => Some(("import", node)),
            "call_expression" => node
                .child_by_field_name("function")
                .map(|node| ("call", node)),
            _ => None,
        },
        SourceLanguage::Unknown => None,
    }
}

fn source_range(start: Point, end: Point) -> SourceRange {
    SourceRange {
        start_line: (start.row + 1) as u32,
        end_line: (end.row + 1) as u32,
    }
}

fn structural_pattern_matches(node: &AstNode, pattern: &str) -> bool {
    let pattern = pattern.trim();
    if let Some((role, name)) = pattern.split_once(':') {
        return node.role == role
            && node
                .name
                .as_deref()
                .map(|candidate| candidate == name || name == "*")
                .unwrap_or_else(|| name == "*" || node.text.contains(name));
    }
    node.role == pattern
        || node.kind == pattern
        || node.name.as_deref() == Some(pattern)
        || node.text.trim() == pattern
}

pub fn discover_lsp_executable(server: LspServerKind) -> Option<String> {
    let env_key = match server {
        LspServerKind::Rust => "AGENTCODE_RUST_ANALYZER",
        LspServerKind::TypeScript => "AGENTCODE_TYPESCRIPT_LANGUAGE_SERVER",
        LspServerKind::Python => "AGENTCODE_PYRIGHT",
        LspServerKind::Go => "AGENTCODE_GOPLS",
    };
    if let Ok(path) = std::env::var(env_key) {
        if Path::new(&path).is_file() && lsp_executable_usable(&path) {
            return Some(path);
        }
    }
    let candidates: &[&str] = match server {
        LspServerKind::Rust => &["rust-analyzer"],
        LspServerKind::TypeScript => &["typescript-language-server"],
        LspServerKind::Python => &["pyright-langserver", "basedpyright-langserver", "pyright"],
        LspServerKind::Go => &["gopls"],
    };
    let path = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&path) {
        for candidate in candidates {
            let executable = dir.join(candidate);
            if executable.is_file() && lsp_executable_usable(&executable.display().to_string()) {
                return Some(executable.display().to_string());
            }
        }
    }
    None
}

fn lsp_executable_usable(executable: &str) -> bool {
    Command::new(executable)
        .arg("--version")
        .env_clear()
        .envs(toolchain_env())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

fn file_uri(path: &Path) -> AcResult<String> {
    let path = path
        .canonicalize()
        .map_err(|error| AcError::validation("CODEINTEL-LSP_URI", error.to_string()))?;
    Ok(format!(
        "file://{}",
        path.display().to_string().replace(' ', "%20")
    ))
}

fn path_from_uri(uri: &str) -> AcResult<String> {
    let path = uri
        .strip_prefix("file://")
        .ok_or_else(|| AcError::validation("CODEINTEL-LSP_URI", "only file:// URIs are supported"))?
        .replace("%20", " ");
    Ok(path)
}

fn workspace_edit_from_lsp(server: LspServerKind, result: &Value) -> AcResult<LspWorkspaceEdit> {
    let mut edits = Vec::new();
    if let Some(changes) = result.get("changes").and_then(Value::as_object) {
        for (uri, file_edits) in changes {
            collect_lsp_text_edits(server, uri, file_edits, &mut edits)?;
        }
    }
    if let Some(changes) = result.get("documentChanges").and_then(Value::as_array) {
        for change in changes {
            if let Some(uri) = change
                .get("textDocument")
                .and_then(|doc| doc.get("uri"))
                .and_then(Value::as_str)
            {
                if let Some(file_edits) = change.get("edits") {
                    collect_lsp_text_edits(server, uri, file_edits, &mut edits)?;
                }
            }
        }
    }
    if edits.is_empty() {
        return Err(AcError::conflict(
            "CODEINTEL-LSP_WORKSPACE_EDIT_EMPTY",
            "language server returned no rename edits",
        ));
    }
    Ok(LspWorkspaceEdit { server, edits })
}

fn collect_lsp_text_edits(
    server: LspServerKind,
    uri: &str,
    value: &Value,
    output: &mut Vec<LspTextEdit>,
) -> AcResult<()> {
    let file_path = path_from_uri(uri)?;
    let edits = value.as_array().ok_or_else(|| {
        AcError::validation("CODEINTEL-INVALID_LSP_RESPONSE", "edits must be an array")
    })?;
    for edit in edits {
        let range = edit.get("range").ok_or_else(|| {
            AcError::validation("CODEINTEL-INVALID_LSP_RESPONSE", "edit range is missing")
        })?;
        let start = range.get("start").ok_or_else(|| {
            AcError::validation("CODEINTEL-INVALID_LSP_RESPONSE", "range start is missing")
        })?;
        let end = range.get("end").ok_or_else(|| {
            AcError::validation("CODEINTEL-INVALID_LSP_RESPONSE", "range end is missing")
        })?;
        let start_position = lsp_types::Position {
            line: start.get("line").and_then(Value::as_u64).unwrap_or(0) as u32,
            character: start.get("character").and_then(Value::as_u64).unwrap_or(0) as u32,
        };
        let end_position = lsp_types::Position {
            line: end.get("line").and_then(Value::as_u64).unwrap_or(0) as u32,
            character: end.get("character").and_then(Value::as_u64).unwrap_or(0) as u32,
        };
        output.push(LspTextEdit {
            file_path: file_path.clone(),
            range: SourceRange {
                start_line: start_position.line + 1,
                end_line: end_position.line + 1,
            },
            start_character: start_position.character,
            end_character: end_position.character,
            new_text: edit
                .get("newText")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string(),
            provenance: format!("Lsp:{}", server.as_str()),
        });
    }
    Ok(())
}

fn toolchain_env() -> BTreeMap<String, String> {
    ["PATH", "HOME", "CARGO_HOME", "RUSTUP_HOME", "RUSTC_WRAPPER"]
        .iter()
        .filter_map(|key| {
            std::env::var(key)
                .ok()
                .map(|value| ((*key).to_string(), value))
        })
        .collect()
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
    parse_statuses: BTreeMap<String, ParseStatus>,
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
        self.parse_statuses.retain(|path, _| present.contains(path));
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
            self.extract_tree_sitter_facts(&file.relative_path, &file.language, &content);
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
            state: if discover_lsp_executable(server).is_some() {
                LspSessionState::Available
            } else {
                LspSessionState::Unavailable
            },
            restart_count: 0,
            last_activity: TimestampMillis::now(),
            degraded_reason: discover_lsp_executable(server)
                .is_none()
                .then(|| "language server executable not found".to_string()),
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
            session.state = LspSessionState::Restarting;
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
        let mut candidates = Vec::new();
        for (path, content) in &self.snippets {
            let language = self
                .files
                .get(path)
                .map(|file| file.language.as_str())
                .unwrap_or("unknown");
            for node in PatternMatcher.search(language, pattern, content)? {
                candidates.push(ContextCandidate {
                    source_path: path.clone(),
                    snippet: node.text.lines().next().unwrap_or("").to_string(),
                    score: 100 + node.text.len() as u32,
                    evidence_note: format!(
                        "TreeSitter structural match; role={}; provenance={}",
                        node.role, node.provenance
                    ),
                });
            }
        }
        Ok(candidates)
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

    pub fn parse_status(&self, path: &str) -> Option<ParseStatus> {
        self.parse_statuses.get(path).copied()
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

    fn extract_tree_sitter_facts(&mut self, path: &str, language: &str, content: &str) {
        self.symbols.retain(|symbol| symbol.file_path != path);
        self.imports.retain(|edge| edge.from != path);
        let parsed = TreeSitterParser.parse(language, content);
        self.parse_statuses.insert(path.to_string(), parsed.status);
        if matches!(
            parsed.status,
            ParseStatus::Unsupported | ParseStatus::Failed
        ) {
            return;
        }
        for node in parsed.nodes {
            if node.role == "import" {
                self.imports.push(ImportEdge {
                    from: path.to_string(),
                    to: normalize_import_target(language, &node.text),
                    line: node.range.start_line,
                });
            } else if let Some(name) = node.name {
                self.symbols.push(SymbolOccurrence {
                    file_path: path.to_string(),
                    name,
                    kind: node.role,
                    line: node.range.start_line,
                    confidence: if parsed.status == ParseStatus::Parsed {
                        95
                    } else {
                        80
                    },
                });
            }
        }
    }

    #[allow(dead_code)]
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
                provenance: self.provenance(scope, "TreeSitter", 95),
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
                            provenance: self.provenance(scope, "TextFallbackReference", 55),
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
                        provenance: self.provenance(scope, "TextFallbackDiagnostic", 55),
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
        "js" | "jsx" => "javascript",
        "ts" => "typescript",
        "tsx" => "tsx",
        "go" => "go",
        "md" => "markdown",
        "toml" => "toml",
        "json" => "json",
        "yaml" | "yml" => "yaml",
        _ => "text",
    }
}

fn normalize_import_target(language: &str, text: &str) -> String {
    let trimmed = text.trim().trim_end_matches(';');
    match SourceLanguage::from_name(language) {
        SourceLanguage::Rust => trimmed
            .strip_prefix("use ")
            .or_else(|| trimmed.strip_prefix("pub use "))
            .or_else(|| trimmed.strip_prefix("mod "))
            .unwrap_or(trimmed)
            .trim()
            .trim_end_matches(';')
            .to_string(),
        SourceLanguage::Python => trimmed
            .strip_prefix("from ")
            .or_else(|| trimmed.strip_prefix("import "))
            .unwrap_or(trimmed)
            .trim()
            .to_string(),
        SourceLanguage::JavaScript | SourceLanguage::TypeScript | SourceLanguage::Tsx => trimmed
            .split(['"', '\'', '`'])
            .nth(1)
            .unwrap_or(trimmed)
            .to_string(),
        SourceLanguage::Go => trimmed.split('"').nth(1).unwrap_or(trimmed).to_string(),
        SourceLanguage::Unknown => trimmed.to_string(),
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

    #[test]
    fn tree_sitter_extracts_symbols_and_imports_for_supported_languages() {
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
                        "src/lib.rs",
                        "rust",
                        false,
                        "use crate::api;\npub struct User;\nimpl User { pub fn name(&self) {} }\npub fn run() {}\n",
                    ),
                    source_file(
                        "main.py",
                        "python",
                        false,
                        "import os\nclass Worker:\n    pass\ndef handle():\n    pass\n",
                    ),
                    source_file(
                        "web.ts",
                        "typescript",
                        false,
                        "import { x } from './x';\ninterface User { id: string }\nclass View {}\nfunction render() {}\n",
                    ),
                    source_file(
                        "server.go",
                        "go",
                        false,
                        "package main\nimport \"fmt\"\ntype User struct{}\nfunc Serve() {}\n",
                    ),
                ],
            )
            .unwrap();

        for path in ["src/lib.rs", "main.py", "web.ts", "server.go"] {
            assert_eq!(service.parse_status(path), Some(ParseStatus::Parsed));
        }
        assert!(!service.query_symbols("run").is_empty());
        assert!(!service.query_symbols("Worker").is_empty());
        assert!(!service.query_symbols("render").is_empty());
        assert!(!service.query_symbols("Serve").is_empty());
        assert!(service
            .imports_for("web.ts")
            .iter()
            .any(|edge| edge.to == "./x"));
    }

    #[test]
    fn structural_search_and_rewrite_are_ast_scoped() {
        let source = "fn target() -> u32 { 1 }\nfn untouched() -> u32 { target() }\n";
        let matches = PatternMatcher
            .search("rust", "function:target", source)
            .unwrap();
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].name.as_deref(), Some("target"));
        assert_eq!(matches[0].provenance, "TreeSitter");

        let rewritten = structural_rewrite(
            "rust",
            source,
            "function:target",
            "fn target() -> usize { 1 }",
        )
        .unwrap();
        assert!(rewritten.contains("fn target() -> usize"));
        assert!(rewritten.contains("fn untouched() -> u32 { target() }"));
        assert_eq!(
            structural_rewrite("rust", source, "function:missing", "fn missing() {}")
                .unwrap_err()
                .code(),
            "CODEINTEL-STRUCTURAL_MATCH_NOT_FOUND"
        );
    }

    #[test]
    fn lsp_discovery_reports_real_installed_status() {
        let mut service = CodeIntelligenceService::new();
        let session = service
            .ensure_lsp_session(LspServerKind::Rust, "/tmp")
            .unwrap();
        if discover_lsp_executable(LspServerKind::Rust).is_some() {
            assert_eq!(session.state, LspSessionState::Available);
        } else {
            assert_eq!(session.state, LspSessionState::Unavailable);
        }
    }

    #[test]
    fn lsp_workspace_edit_normalizes_changes_payload() {
        let payload = json!({
            "changes": {
                "file:///tmp/demo.rs": [
                    {
                        "range": {
                            "start": { "line": 0, "character": 7 },
                            "end": { "line": 0, "character": 10 }
                        },
                        "newText": "new_name"
                    }
                ]
            }
        });
        let edit = workspace_edit_from_lsp(LspServerKind::Rust, &payload).unwrap();
        assert_eq!(edit.edits.len(), 1);
        assert_eq!(edit.edits[0].file_path, "/tmp/demo.rs");
        assert_eq!(edit.edits[0].range.start_line, 1);
        assert_eq!(edit.edits[0].provenance, "Lsp:rust");
    }

    #[test]
    fn rust_analyzer_lsp_lifecycle_runs_when_installed() {
        let root = std::env::temp_dir().join(format!("agentcode-lsp-{}", StableId::new("tmp")));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("src")).unwrap();
        std::fs::write(
            root.join("Cargo.toml"),
            "[package]\nname = \"agentcode_lsp_fixture\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[lib]\npath = \"src/lib.rs\"\n",
        )
        .unwrap();
        let lib = root.join("src/lib.rs");
        std::fs::write(&lib, "pub fn answer() -> u32 { 42 }\n").unwrap();

        if discover_lsp_executable(LspServerKind::Rust).is_none() {
            let err = match LspClient::start(LspServerKind::Rust, root.display().to_string()) {
                Ok(client) => {
                    let _ = client.shutdown();
                    panic!(
                        "rust-analyzer unexpectedly started after discovery returned unavailable"
                    )
                }
                Err(error) => error,
            };
            assert_eq!(err.code(), "CODEINTEL-LANGUAGE_SERVER_UNAVAILABLE");
            let _ = std::fs::remove_dir_all(root);
            return;
        }

        let mut client = LspClient::start(LspServerKind::Rust, root.display().to_string()).unwrap();
        let text = std::fs::read_to_string(&lib).unwrap();
        let version = client.did_open(&lib, SourceLanguage::Rust, &text).unwrap();
        assert_eq!(version, 1);
        client.shutdown().unwrap();
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn lsp_server_requests_receive_json_rpc_responses() {
        let configuration = server_request_response("workspace/configuration", json!(7));
        assert_eq!(configuration["id"], 7);
        assert_eq!(configuration["result"], json!([]));

        let unsupported = server_request_response("window/showMessageRequest", json!(8));
        assert_eq!(unsupported["id"], 8);
        assert_eq!(unsupported["error"]["code"], -32601);
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
