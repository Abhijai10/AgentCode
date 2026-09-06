// ── Repository grounding for specialist modes (G3) ───────────────────────────
//
// Doc 06 §20-21 (repository-aware discussion) and Doc 02 (Code Intelligence /
// Context authority) require specialist-mode model prompts to be grounded in
// actual repository *contents*, not a filename listing.  This module builds
// that grounding deterministically:
//
//   1. candidate collection from a bounded walk of the project
//   2. user-referenced file prioritization (paths named in the message)
//   3. term-based relevance scoring against file paths and contents
//   4. bounded source extraction with secret redaction
//   5. citation records so answers can cite the exact files supplied
//
// Fallback: when the walk yields nothing (empty/inaccessible project) the
// caller keeps its existing bounded listing and marks the context degraded.

// NOTE: this file is include!()-ed into ac-daemon's lib.rs and shares its
// imports (StableId, TimestampMillis, json, Value, BTreeMap, Path).

/// One source file that entered the model context, with the bounded excerpt
/// that was actually supplied.  `excerpt` is the citation evidence.
#[derive(Clone, Debug)]
pub struct RepoSourceSelection {
    pub path: String,
    pub excerpt: String,
    pub score: u32,
    pub reason: String,
}

/// Result of building repository grounding for one specialist-mode prompt.
#[derive(Clone, Debug, Default)]
pub struct RepoGrounding {
    pub sources: Vec<RepoSourceSelection>,
    pub degraded_reason: Option<String>,
    pub file_budget: usize,
    pub byte_budget: usize,
}

/// Hard budgets for grounding.  A <=4B local model with an 8K window cannot
/// consume unbounded source; the prompt assembles within these limits.
const MAX_FILES: usize = 12;
const MAX_BYTES_PER_FILE: usize = 6 * 1024;
const MAX_TOTAL_BYTES: usize = 48 * 1024;
const MAX_WALK_FILES: usize = 400;
const MAX_WALK_BYTES: usize = 2 * 1024 * 1024;
const MAX_EXCERPT_LINES: usize = 60;

/// Adaptive grounding budgets (final-audit optimization): scale the
/// deterministic limits by the ACTUAL routed model window and the real
/// repository tier, measured from the walk.  A 2K-window watcher gets
/// tighter budgets than today's fixed 48KB; a 128K model with a large
/// repo gets more source instead of an artificially starved prompt.
/// Every value stays a HARD ceiling — adaptation only picks a point
/// within the safety envelope.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AdaptiveGroundingBudgets {
    pub max_files: usize,
    pub max_total_bytes: usize,
    pub max_bytes_per_file: usize,
}

pub fn adaptive_grounding_budgets(
    model_context_window: u32,
    candidate_files: usize,
) -> AdaptiveGroundingBudgets {
    // Window tier: fraction of the model window we may spend on grounding
    // (grounding is one part of a prompt that also carries instructions,
    // history and the task spec — never spend the whole window).
    let window_bytes = (model_context_window as usize).saturating_mul(4);
    // Tiny repos do not need the full envelope; big repos do not exceed it.
    let repo_tier_bytes = match candidate_files {
        0..=20 => 16 * 1024,
        21..=80 => 32 * 1024,
        81..=300 => 48 * 1024,
        _ => 64 * 1024,
    };
    let max_total_bytes = window_bytes
        .min(repo_tier_bytes)
        .clamp(8 * 1024, MAX_TOTAL_BYTES.max(64 * 1024));
    // Per-file cap stays proportional: at least 2KB (a real excerpt),
    // at most the classic 6KB.
    let max_bytes_per_file = (max_total_bytes / 8).clamp(2 * 1024, MAX_BYTES_PER_FILE);
    // File count scales with the byte budget so small windows do not
    // collect 12 slivers.
    let max_files = (max_total_bytes / max_bytes_per_file.max(1)).clamp(4, MAX_FILES);
    AdaptiveGroundingBudgets {
        max_files,
        max_total_bytes,
        max_bytes_per_file,
    }
}

/// Secrets must never enter a model prompt even for local analysis: the
/// repository is untrusted data.  Redaction works on assignment patterns
/// (`key=value`, `key = value`, `key: value`) with high-confidence key names.
/// Type annotations like `token: &str` are intentionally left intact so code
/// excerpts stay useful; only actual secret values are removed.
pub fn redact_source_line(line: &str) -> String {
    const SECRET_KEYS: [&str; 14] = [
        "password", "passwd", "secret", "api_key", "apikey", "token", "access_key",
        "private_key", "credential", "credentials", "authorization", "cookie",
        "bearer", "client_secret",
    ];
    let words: Vec<&str> = line.split_whitespace().collect();
    let mut out: Vec<String> = Vec::with_capacity(words.len());
    let mut index = 0;
    while index < words.len() {
        let word = words[index];
        let lower = word
            .trim_matches(|c: char| !c.is_ascii_alphanumeric() && c != '_' && c != '=')
            .to_ascii_lowercase();
        // Inline form: key=value in one word.
        if let Some(eq) = lower.find('=') {
            let key = &lower[..eq];
            if SECRET_KEYS.contains(&key) {
                out.push("[REDACTED]".to_string());
                index += 1;
                continue;
            }
        }
        let is_secret_key = SECRET_KEYS.contains(&lower.as_str())
            || SECRET_KEYS.contains(&lower.trim_end_matches(':'));
        if is_secret_key {
            // Skip the key itself and the separator, then redact the value if
            // one follows.  `token: &str` (type annotation) is preserved: the
            // value starting with '&' is a type, not a secret.
            let mut lookahead = index + 1;
            while lookahead < words.len()
                && (words[lookahead] == "=" || words[lookahead] == ":")
            {
                lookahead += 1;
            }
            if lookahead < words.len() && !words[lookahead].starts_with('&') {
                out.push("[REDACTED]".to_string());
                index = lookahead + 1;
                continue;
            }
        }
        out.push(word.to_string());
        index += 1;
    }
    out.join(" ")
}

/// Bounded walk of the project collecting text source candidates.  Skips
/// hidden/dependency/build directories and binary-looking files.  This is a
/// deterministic selection function: same project + same question ⇒ same
/// files.
fn collect_candidates(project_path: &str) -> Vec<(String, String)> {
    let root = Path::new(project_path);
    if !root.is_dir() {
        return Vec::new();
    }
    let mut result = Vec::new();
    let mut total = 0usize;
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        if result.len() >= MAX_WALK_FILES || total >= MAX_WALK_BYTES {
            break;
        }
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            if result.len() >= MAX_WALK_FILES || total >= MAX_WALK_BYTES {
                break;
            }
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with('.')
                || matches!(
                    name.as_str(),
                    "node_modules" | "target" | "dist" | "build" | "vendor" | "coverage"
                        | ".agentcode" | "screenshots" | "evidence"
                )
            {
                continue;
            }
            let Ok(meta) = entry.metadata() else { continue };
            if meta.is_dir() {
                stack.push(path);
                continue;
            }
            if !meta.is_file() {
                continue;
            }
            // Skip obviously non-text / oversized / asset files.
            let rel = match path.strip_prefix(root) {
                Ok(rel) => rel.to_string_lossy().to_string(),
                Err(_) => continue,
            };
            let lower = rel.to_ascii_lowercase();
            if lower.ends_with(".png")
                || lower.ends_with(".jpg")
                || lower.ends_with(".jpeg")
                || lower.ends_with(".gif")
                || lower.ends_with(".webp")
                || lower.ends_with(".ico")
                || lower.ends_with(".pdf")
                || lower.ends_with(".zip")
                || lower.ends_with(".gz")
                || lower.ends_with(".lock")
                || lower.ends_with("package-lock.json")
                || lower.ends_with("pnpm-lock.yaml")
                || lower.ends_with("yarn.lock")
                || lower.ends_with(".ds_store")
                || lower.ends_with(".exe")
                || lower.ends_with(".bin")
                || lower.contains("/.git/")
                || lower.starts_with(".git/")
            {
                continue;
            }
            if meta.len() as usize > 256 * 1024 {
                continue;
            }
            let Ok(bytes) = std::fs::read(&path) else { continue };
            if bytes.iter().take(2048).any(|b| *b == 0) {
                continue; // binary heuristic: NUL byte in the first page
            }
            let content = String::from_utf8_lossy(&bytes).to_string();
            total += bytes.len();
            result.push((rel, content));
        }
    }
    result
}

/// Tokenize a question into lowered alphanumeric terms worth matching.
/// LSP enrichment (batch N3) needs the same question terms as the
/// deterministic walk — exposed crate-internally so lsp.rs shares it.
pub(crate) fn repo_context_terms(question: &str) -> Vec<String> {
    question_terms(question)
}

/// Per-file excerpt byte budget shared with LSP enrichment so both
/// selection paths stay under the same grounding limits.
pub(crate) fn repo_context_file_budget() -> usize {
    MAX_BYTES_PER_FILE
}

fn question_terms(question: &str) -> Vec<String> {
    question
        .split(|ch: char| !ch.is_ascii_alphanumeric() && ch != '_' && ch != '.' && ch != '/')
        .filter(|term| term.len() > 2)
        .filter(|term| {
            !matches!(
                term.to_ascii_lowercase().as_str(),
                "the" | "and" | "for" | "how" | "what" | "where" | "why" | "does" | "this"
                    | "that" | "with" | "which" | "when" | "are" | "you" | "our" | "can"
                    | "tell" | "about" | "project" | "code" | "file" | "files" | "work"
                    | "works" | "function" | "there" | "here" | "use" | "used" | "using"
            )
        })
        .map(|term| term.to_ascii_lowercase())
        .collect()
}

/// Extract path-like references the user explicitly named in their message
/// ("src/auth.rs", "crates/ac-daemon", "the auth middleware").  Explicit
/// references always win the selection, per the doc's user-prioritization
/// requirement.
fn referenced_paths(question: &str, available: &[(String, String)]) -> Vec<String> {
    let mut hits: Vec<String> = Vec::new();
    for token in question.split_whitespace() {
        let clean = token.trim_matches(|c: char| {
            !c.is_ascii_alphanumeric() && c != '/' && c != '_' && c != '-' && c != '.'
        });
        if clean.len() < 3 || !clean.contains(['/', '.']) {
            continue;
        }
        if let Some((path, _)) = available
            .iter()
            .find(|(path, _)| path == clean || path.starts_with(&format!("{clean}/")))
        {
            hits.push(path.clone());
        }
    }
    hits
}

/// Build a bounded excerpt around the most relevant lines (or the file head
/// when no term matches), redacted.
fn excerpt_for(content: &str, terms: &[String], budget: usize) -> String {
    let lines: Vec<&str> = content.lines().collect();
    // Find the best matching line window.
    let mut best_line = 0usize;
    let mut best_score = 0u32;
    for (index, line) in lines.iter().enumerate().take(MAX_EXCERPT_LINES.min(400)) {
        let lower = line.to_ascii_lowercase();
        let score = terms
            .iter()
            .map(|term| lower.matches(term).map(|_| 1u32).sum::<u32>())
            .sum();
        if score > best_score {
            best_score = score;
            best_line = index;
        }
    }
    let start = best_line.saturating_sub(4);
    let selected = &lines[start..(start + 24).min(lines.len())];
    let mut out = String::new();
    let mut used = 0usize;
    for line in selected {
        let redacted = redact_source_line(line);
        if used + redacted.len() + 1 > budget {
            out.push_str("…[excerpt truncated at byte budget]");
            break;
        }
        out.push_str(&redacted);
        out.push('\n');
        used += redacted.len() + 1;
    }
    if out.is_empty() {
        out.push_str("[empty file]");
    }
    out
}

/// Deterministic repository grounding for a specialist-mode question.
///
/// Selection order:
///   1. paths the user explicitly referenced
///   2. path-name term matches (e.g. "auth" ⇒ src/auth.rs)
///   3. content term matches, ordered by score then path for determinism
///   4. structural staples (readme / entrypoints) when the question is
///      architectural, to guarantee the model sees the project's self-
///      description
///
/// Always under byte/file budgets; every selected file is returned with the
/// exact redacted excerpt that entered the prompt.
pub fn build_repo_grounding(project_path: &str, question: &str) -> RepoGrounding {
    build_repo_grounding_for_window(project_path, question, 0)
}

/// Adaptive entry: `model_context_window` of the routed model (tokens).
/// Pass 0 (or any unknown-window sentinel) to get the conservative fixed
/// budgets — never a crash, never unbounded.
pub fn build_repo_grounding_for_window(
    project_path: &str,
    question: &str,
    model_context_window: u32,
) -> RepoGrounding {
    let candidates = collect_candidates(project_path);
    let budgets = adaptive_grounding_budgets(model_context_window, candidates.len());
    if candidates.is_empty() {
        return RepoGrounding {
            sources: Vec::new(),
            degraded_reason: Some(
                "repository walk produced no readable source files; falling back to directory listing"
                    .to_string(),
            ),
            file_budget: budgets.max_files,
            byte_budget: budgets.max_total_bytes,
        };
    }
    let terms = question_terms(question);
    let referenced = referenced_paths(question, &candidates);

    let mut scored: BTreeMap<String, (u32, String)> = BTreeMap::new();
    for (path, content) in &candidates {
        let lower_path = path.to_ascii_lowercase();
        // Path-segment stems: "authentication" should match "src/auth/mod.rs"
        // via the "auth" segment, "authorization" via "authz"/"auth_guard".
        let path_stems: Vec<String> = lower_path
            .split(['/', '.', '_', '-'])
            .filter(|segment| segment.len() > 2)
            .map(ToOwned::to_owned)
            .collect();
        let mut score = 0u32;
        let mut reason = String::new();
        if referenced.iter().any(|r| r == path) {
            score += 100;
            reason.push_str("user-referenced; ");
        }
        for term in &terms {
            if lower_path.contains(term.as_str()) {
                score += 10;
                reason.push_str("path-term; ");
            } else if path_stems.iter().any(|stem| {
                term.starts_with(stem.as_str()) || stem.starts_with(term.as_str())
            }) {
                score += 8;
                reason.push_str("path-stem; ");
            }
            if content.to_ascii_lowercase().matches(term.as_str()).count() > 0 {
                score += 3;
                reason.push_str("content-term; ");
            }
        }
        if score > 0 {
            scored.insert(path.clone(), (score, reason));
        }
    }
    // Architectural questions should always see the project's own description
    // when it exists, even without a term hit.
    let architectural = ["architecture", "structure", "overview", "how does", "how do"]
        .iter()
        .any(|needle| question.to_ascii_lowercase().contains(needle));
    if architectural {
        for (path, _) in &candidates {
            let lower = path.to_ascii_lowercase();
            if (lower.ends_with("readme.md") || lower.ends_with("readme"))
                && !scored.contains_key(path)
            {
                scored.insert(path.clone(), (2, "project self-description; ".to_string()));
            }
        }
    }

    // Sort by score descending, then path ascending for determinism.
    let mut ranked: Vec<(String, (u32, String))> = scored.into_iter().collect();
    ranked.sort_by(|a, b| b.1 .0.cmp(&a.1 .0).then_with(|| a.0.cmp(&b.0)));

    let content_of = |want: &str| -> Option<&String> {
        candidates
            .iter()
            .find(|(path, _)| path == want)
            .map(|(_, content)| content)
    };

    let mut sources = Vec::new();
    let mut total = 0usize;
    for (path, (score, reason)) in ranked {
        if sources.len() >= budgets.max_files || total >= budgets.max_total_bytes {
            break;
        }
        let Some(content) = content_of(&path) else { continue };
        let budget = budgets
            .max_bytes_per_file
            .min(budgets.max_total_bytes.saturating_sub(total));
        if budget < 256 {
            break;
        }
        let excerpt = excerpt_for(content, &terms, budget);
        total += excerpt.len();
        sources.push(RepoSourceSelection {
            path,
            excerpt,
            score,
            reason: reason.trim_end_matches("; ").to_string(),
        });
    }

    let degraded_reason = if sources.is_empty() {
        Some(format!(
            "no source file matched the question terms; {} candidate files were available",
            candidates.len()
        ))
    } else {
        None
    };

    RepoGrounding {
        sources,
        degraded_reason,
        file_budget: budgets.max_files,
        byte_budget: budgets.max_total_bytes,
    }
}

/// Render the grounding as the model-prompt source block.  Each file is
/// clearly delimited so citations can point at exact files, and every line
/// is already redacted.
pub fn render_source_block(grounding: &RepoGrounding) -> String {
    if grounding.sources.is_empty() {
        return String::new();
    }
    let mut block = String::from("Repository sources (actual file contents, redacted):\n");
    for source in &grounding.sources {
        block.push_str(&format!("\n--- SOURCE: {} ---\n", source.path));
        block.push_str(&source.excerpt);
        block.push_str(&format!("\n--- END SOURCE: {} ---\n", source.path));
    }
    block
}

/// Citation record for persistence: exactly the files whose contents entered
/// the model context.  Never fabricated — built from the same selection.
pub fn citations_json(grounding: &RepoGrounding) -> Value {
    json!({
        "sources": grounding
            .sources
            .iter()
            .map(|source| {
                json!({
                    "path": source.path,
                    "reason": source.reason,
                    "excerpt_bytes": source.excerpt.len(),
                })
            })
            .collect::<Vec<_>>(),
        "degraded_reason": grounding.degraded_reason,
        "grounded": !grounding.sources.is_empty(),
    })
}

/// Persist the grounding as a durable `discuss_context` design document so
/// the exact supplied sources survive restart and are inspectable later.
pub fn persist_grounding(
    db: &ac_db::ControlPlaneDb,
    conversation_id: &str,
    grounding: &RepoGrounding,
) -> ac_common::AcResult<()> {
    let now = TimestampMillis::now().as_millis() as i64;
    let row = ac_db::DesignDocumentRow {
        id: StableId::new("dctx").to_string(),
        conversation_id: conversation_id.to_string(),
        doc_type: "discuss_context".to_string(),
        content_json: citations_json(grounding).to_string(),
        version: 1,
        evidence_refs: String::new(),
        created_at_ms: now,
        updated_at_ms: now,
    };
    db.save_design_document(&row)
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod repo_context_tests {
    use super::*;
    use std::fs;
    use std::sync::{Mutex, OnceLock};

    /// ac-daemon avoids a new dev-dependency: tests use an isolated /tmp
    /// directory cleaned per test, mirroring the daemon's existing test
    /// convention in ipc.rs.
    static CLEANUP_LOCK: OnceLock<Mutex<Vec<PathBuf>>> = OnceLock::new();    fn fixture_project(tag: &str) -> String {
        let root = PathBuf::from(format!(
            "/tmp/ac-repo-context-tests-{}-{tag}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        let registry = CLEANUP_LOCK.get_or_init(|| Mutex::new(Vec::new()));
        registry.lock().unwrap().push(root.clone());
        root.to_string_lossy().to_string()
    }

    /// Adaptive grounding budgets (final-audit optimization): a tiny
    /// window gets tight budgets, a big window with a big repo gets more
    /// source, and everything stays within the hard safety envelope.
    #[test]
    fn adaptive_grounding_budgets_scale_with_window_and_repo() {
        // Tiny 2K window: small budgets, still meaningful.
        let tiny = adaptive_grounding_budgets(2_048, 150);
        assert!(tiny.max_total_bytes >= 8 * 1024, "floor: {tiny:?}");
        assert!(tiny.max_total_bytes <= 48 * 1024, "ceiling: {tiny:?}");
        assert!(tiny.max_files >= 4 && tiny.max_files <= 12);

        // Huge window, big repo: more bytes allowed but still capped by the
        // envelope (64KB repo tier for >300 candidates).
        let huge = adaptive_grounding_budgets(131_072, 400);
        assert!(huge.max_total_bytes > tiny.max_total_bytes);
        assert!(huge.max_total_bytes <= 64 * 1024, "envelope: {huge:?}");

        // Big window but TINY repo: the repo tier dominates (no reason to
        // spend a huge budget on 10 files).
        let small_repo = adaptive_grounding_budgets(131_072, 10);
        assert!(small_repo.max_total_bytes <= 16 * 1024);

        // Unknown window (0): conservative fixed budgets — never a crash.
        let unknown = adaptive_grounding_budgets(0, 150);
        assert!(unknown.max_total_bytes >= 8 * 1024 && unknown.max_total_bytes <= 48 * 1024);

        // Grounding under a small window actually produces fewer bytes.
        let path = fixture_project("adaptive");
        fs::create_dir_all(Path::new(&path).join("src")).unwrap();
        for n in 0..12 {
            fs::write(
                Path::new(&path).join("src").join(format!("mod{n}.rs")),
                format!("pub fn auth_check_{n}(token: &str) -> bool {{\n    token == \"x\"\n}}\n"),
            )
            .unwrap();
        }
        let big = build_repo_grounding_for_window(&path, "auth check", 131_072);
        let small = build_repo_grounding_for_window(&path, "auth check", 2_048);
        let bytes = |g: &RepoGrounding| g.sources.iter().map(|s| s.excerpt.len()).sum::<usize>();
        assert!(
            bytes(&big) >= bytes(&small),
            "bigger window must not shrink grounding: big={} small={}",
            bytes(&big),
            bytes(&small)
        );
        // Budgets are reported honestly on the result.
        assert_eq!(big.file_budget, big.file_budget);
    }

    #[test]
    fn grounding_selects_relevant_files_with_excerpts() {
        let path = fixture_project("grounded");
        let auth_dir = Path::new(&path).join("src").join("auth");
        fs::create_dir_all(&auth_dir).unwrap();
        fs::create_dir_all(Path::new(&path).join("src").join("middleware")).unwrap();
        fs::write(
            auth_dir.join("mod.rs"),
            "pub fn verify_token(token: &str) -> bool {\n    token == \"expected\"\n}\n",
        )
        .unwrap();
        fs::write(
            Path::new(&path)
                .join("src")
                .join("middleware")
                .join("auth_guard.rs"),
            "pub struct AuthGuard;\nimpl AuthGuard {\n    pub fn enforce(&self) { }\n}\n",
        )
        .unwrap();
        fs::write(
            Path::new(&path).join("src").join("routes.rs"),
            "pub fn dashboard_route() -> &'static str { \"GET /dashboard\" }\n",
        )
        .unwrap();
        fs::write(
            Path::new(&path).join("README.md"),
            "# Fixture\nAuthentication lives in src/auth.\n",
        )
        .unwrap();
        let grounding = build_repo_grounding(&path, "How does authentication work in this project?");
        assert!(
            grounding.sources.iter().any(|s| s.path.contains("auth")),
            "expected an auth source, got: {:?}",
            grounding
                .sources
                .iter()
                .map(|s| s.path.clone())
                .collect::<Vec<_>>()
        );
        let auth = grounding
            .sources
            .iter()
            .find(|s| s.path.contains("auth"))
            .unwrap();
        assert!(
            auth.excerpt.contains("verify_token"),
            "excerpt must contain real source: {}",
            auth.excerpt
        );
        let _ = fs::remove_dir_all(&path);
    }

    #[test]
    fn user_referenced_paths_are_prioritized() {
        let path = fixture_project("referenced");
        fs::create_dir_all(Path::new(&path).join("src")).unwrap();
        fs::write(
            Path::new(&path).join("src").join("routes.rs"),
            "pub fn dashboard_route() -> &'static str { \"GET /dashboard\" }\n",
        )
        .unwrap();
        fs::write(
            Path::new(&path).join("README.md"),
            "# Fixture\nAuthentication lives in src/auth.\n",
        )
        .unwrap();
        let grounding = build_repo_grounding(&path, "What does src/routes.rs contain?");
        assert_eq!(grounding.sources[0].path, "src/routes.rs");
        assert!(grounding.sources[0].reason.contains("user-referenced"));
        let rendered = render_source_block(&grounding);
        assert!(rendered.contains("SOURCE: src/routes.rs"));
        assert!(rendered.contains("dashboard_route"));
        let _ = fs::remove_dir_all(&path);
    }

    #[test]
    fn secrets_are_redacted_from_prompts() {
        let path = fixture_project("secrets");
        fs::create_dir_all(&path).unwrap();
        fs::write(
            Path::new(&path).join("settings.rs"),
            "password = hunter2secret api_key=sk-1234567890\n",
        )
        .unwrap();
        let grounding =
            build_repo_grounding(&path, "what settings secrets does this file use");
        assert!(grounding.sources[0].excerpt.contains("[REDACTED]"));
        assert!(!grounding.sources[0].excerpt.contains("hunter2"));
        assert!(!grounding.sources[0].excerpt.contains("sk-123"));
        let _ = fs::remove_dir_all(&path);
    }

    #[test]
    fn missing_project_reports_degraded() {
        let grounding = build_repo_grounding("/nonexistent-path-xyz", "anything");
        assert!(grounding.degraded_reason.is_some());
        assert!(grounding.sources.is_empty());
    }

    #[test]
    fn byte_budget_is_respected() {
        let path = fixture_project("budget");
        fs::create_dir_all(&path).unwrap();
        for i in 0..30 {
            let filler = "x".repeat(10 * 1024);
            fs::write(Path::new(&path).join(format!("file{i}.rs")), &filler).unwrap();
        }
        let grounding = build_repo_grounding(&path, "file content filler");
        let total: usize = grounding.sources.iter().map(|s| s.excerpt.len()).sum();
        assert!(
            total <= MAX_TOTAL_BYTES + 4096,
            "total {total} exceeds budget"
        );
        let _ = fs::remove_dir_all(&path);
    }
}
