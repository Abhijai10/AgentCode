use std::collections::BTreeMap;

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
            self.extract_symbols(&file.relative_path, &content);
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
                    },
                    "fn hidden() {}".to_string(),
                )],
            )
            .unwrap();
        assert!(receipt.degraded);
        assert!(service.query_symbols("hidden").is_empty());
    }
}
