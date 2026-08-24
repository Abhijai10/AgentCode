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

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum IndexReadiness {
    BaseReady,
    StructuralReady,
    Degraded,
    Rebuilding,
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
            for prefix in ["fn ", "struct ", "enum ", "trait "] {
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
}
