// ── LSP production wiring for repository grounding (batch N3, G3) ─────────
//
// `ac-code-intel` owns the language-server client; this module wires it into
// the daemon's Discuss grounding so the model sees symbol-adjacent source the
// deterministic walk cannot see (definition/references of the symbols in the
// already-selected files).
//
// Contract:
//   1. LSP is strictly ENRICHMENT on top of the deterministic grounding —
//      it can only ADD sources under the same file/byte budgets, never
//      replace or reorder the deterministic selection.
//   2. Capability detection is honest: no installed language server for a
//      language ⇒ that language is skipped and the fact is recorded in
//      `lsp_status`, never fabricated.
//   3. Every failure path degrades gracefully — a broken server, a timeout,
//      or a workspace the server rejects leaves the original grounding
//      intact with a recorded reason.  The discuss answer still works.
//   4. Only workspace-internal locations are accepted: LSP can return
//      paths outside the project (stdlib, dependency sources); those are
//      rejected so grounding stays honest about what the project contains.
//
// Why a bounded single-shot client per question: the LSP session is started
// only when a supported language is present among the selected sources, used
// for at most MAX_LSP_QUERIES definition/references queries, and shut down.
// Sessions never outlive the request.

// NOTE: this module is `include!`d into lib.rs alongside repo_context.rs, so
// grounding items (RepoGrounding, RepoSourceSelection, build_repo_grounding,
// render_source_block, citations_json, redact_source_line, ...) and the
// common prelude (AcResult, json, Value, Path, ...) are already in scope.
// Only ac_code_intel items are new to this crate.
use ac_code_intel::{LspClient, LspServerKind, SourceLanguage};

/// How many definition/references queries one grounding may spend.  Each
/// query can return many locations; the *result* budget is capped separately
/// so a single pathological answer cannot consume the whole source block.
const MAX_LSP_QUERIES: usize = 24;

/// Upper bound of extra files LSP may contribute, shared with the grounding's
/// own file budget (MAX_FILES = 12).
const MAX_LSP_SOURCES: usize = 4;

/// Byte budget for the LSP contribution to the excerpt pool.
const MAX_LSP_BYTES: usize = 8 * 1024;

/// Map a file extension to the language-server kind that can serve it.  Only
/// languages with a *single* well-supported server are wired; others degrade.
fn server_kind_for_language(language: SourceLanguage) -> Option<LspServerKind> {
    match language {
        SourceLanguage::Rust => Some(LspServerKind::Rust),
        SourceLanguage::TypeScript | SourceLanguage::Tsx | SourceLanguage::JavaScript => {
            Some(LspServerKind::TypeScript)
        }
        SourceLanguage::Python => Some(LspServerKind::Python),
        SourceLanguage::Go => Some(LspServerKind::Go),
        SourceLanguage::Unknown => None,
    }
}

fn language_for_extension(path: &str) -> SourceLanguage {
    let ext = Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    match ext.as_str() {
        "rs" => SourceLanguage::Rust,
        "ts" => SourceLanguage::TypeScript,
        "tsx" => SourceLanguage::Tsx,
        "js" | "jsx" | "mjs" | "cjs" => SourceLanguage::JavaScript,
        "py" => SourceLanguage::Python,
        "go" => SourceLanguage::Go,
        _ => SourceLanguage::Unknown,
    }
}

/// Pick the server kind for the project by majority language among the
/// already-selected grounding sources (deterministic: first kind in a fixed
/// preference order that has any file among the sources AND an installed
/// server).  Starting one server keeps cost bounded; mixed-language projects
/// still get deterministic grounding for the other files.
fn select_server_kind(
    sources: &[String],
) -> Option<(LspServerKind, Vec<(String, SourceLanguage)>)> {
    // Fixed preference order for determinism.
    let order = [
        LspServerKind::Rust,
        LspServerKind::TypeScript,
        LspServerKind::Python,
        LspServerKind::Go,
    ];
    let mut supported: Vec<(String, SourceLanguage)> = Vec::new();
    for path in sources {
        let language = language_for_extension(path);
        if language != SourceLanguage::Unknown {
            supported.push((path.clone(), language));
        }
    }
    for kind in order {
        let matches: Vec<(String, SourceLanguage)> = supported
            .iter()
            .filter(|(_, language)| server_kind_for_language(*language) == Some(kind))
            .cloned()
            .collect();
        if matches.is_empty() {
            continue;
        }
        if ac_code_intel::discover_lsp_executable(kind).is_some() {
            return Some((kind, matches));
        }
    }
    None
}

/// Extract (path, line) locations from a definition/references LSP result.
/// Accepts single-location objects, arrays, and `{uri, range}` shapes.
fn locations_from_result(result: &Value) -> Vec<(String, u32)> {
    let mut out: Vec<(String, u32)> = Vec::new();
    let items: Vec<&Value> = match result {
        Value::Array(items) => items.iter().collect(),
        Value::Object(_) => vec![result],
        _ => return out,
    };
    for item in items {
        // definition may return { uri, range } or LocationLink { targetUri, targetSelectionRange }
        let uri = item
            .get("uri")
            .or_else(|| item.get("targetUri"))
            .and_then(Value::as_str);
        let range = item
            .get("range")
            .or_else(|| item.get("targetSelectionRange"))
            .or_else(|| item.get("targetRange"));
        let Some(uri) = uri else { continue };
        let line = range
            .and_then(|r| r.get("start"))
            .and_then(|s| s.get("line"))
            .and_then(Value::as_u64)
            .unwrap_or(0) as u32;
        out.push((uri.to_string(), line));
    }
    out
}

/// Convert an LSP `file://` URI to a workspace-relative path when it is
/// inside the project root; None otherwise (stdlib/dependency sources are
/// rejected so grounding never claims project content it does not have).
fn relative_path_in_workspace(uri: &str, project_root: &Path) -> Option<String> {
    let path = uri.strip_prefix("file://")?;
    let path = path.replace("%20", " ");
    let absolute = Path::new(&path);
    // Canonicalize the project root once for prefix comparison.  Failure to
    // canonicalize (e.g. the daemon runs where the root is not resolvable)
    // degrades to no LSP sources — never an error for the caller.
    let root = project_root.canonicalize().ok()?;
    let absolute = absolute.canonicalize().unwrap_or_else(|_| absolute.to_path_buf());
    let relative = absolute.strip_prefix(&root).ok()?;
    let text = relative.to_string_lossy().to_string();
    if text.is_empty() || text == "." {
        None
    } else {
        Some(text)
    }
}

/// Bounded, redacted excerpt around an LSP-reported line — mirrors the
/// deterministic grounding's excerpt shape (redacted lines, byte cap).
fn lsp_excerpt_for(content: &str, line: u32, budget: usize) -> String {
    let lines: Vec<&str> = content.lines().collect();
    let anchor = (line as usize).min(lines.len().saturating_sub(1));
    let start = anchor.saturating_sub(2);
    let end = (anchor + 10).min(lines.len());
    let mut out = String::new();
    let mut used = 0usize;
    for line in &lines[start..end] {
        let redacted = redact_source_line(line);
        if used + redacted.len() + 1 > budget {
            out.push_str("…[lsp excerpt truncated at byte budget]");
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


/// Identifier runs that are never worth an LSP query: keywords and
/// primitive/common type names.  Real declarations beat these in practice.
const SYMBOL_SKIP: [&str; 24] = [
    "pub", "fn", "use", "mod", "struct", "enum", "impl", "trait", "const",
    "let", "mut", "return", "crate", "super", "self", "Self", "match",
    "str", "bool", "String", "Vec", "u32", "i32", "true",
];

/// Find up to `limit` distinct symbol positions worth querying in a file:
/// identifier runs (length > 3) that are not keywords, ordered so runs on
/// lines containing a question term come first (deterministic: document
/// order within each class).  Querying several distinct symbols per file is
/// what lets a single source surface cross-file declarations the term walk
/// cannot see — the first symbol may be declared locally (no new source),
/// while a later one resolves into the enrichment.
fn symbol_positions_for_terms(
    content: &str,
    terms: &[String],
    limit: usize,
) -> Vec<(u32, u32)> {
    let mut term_adjacent: Vec<(u32, u32)> = Vec::new();
    let mut others: Vec<(u32, u32)> = Vec::new();
    let mut seen: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    for (index, line) in content.lines().enumerate() {
        let lower = line.to_ascii_lowercase();
        let line_has_term = terms.iter().any(|term| lower.contains(term.as_str()));
        let mut run: Option<(usize, usize)> = None; // (start_byte, start_char)
        let mut char_no = 0usize;
        let bytes = line.as_bytes();
        for (byte_no, ch) in line.char_indices() {
            if ch.is_alphanumeric() || ch == '_' {
                if run.is_none() {
                    run = Some((byte_no, char_no));
                }
            } else if let Some((start_byte, start_char)) = run.take() {
                let text = &line[start_byte..byte_no];
                if text.len() > 3
                    && !SYMBOL_SKIP.contains(&text)
                    && seen.insert(text.to_string())
                {
                    let pos = (index as u32, start_char as u32);
                    if line_has_term {
                        term_adjacent.push(pos);
                    } else {
                        others.push(pos);
                    }
                }
            }
            char_no += ch.len_utf8();
            let _ = bytes; // silence unused; remove in next pass
        }
        // trailing run at end of line
        if let Some((start_byte, start_char)) = run {
            let text = &line[start_byte..];
            if text.len() > 3
                && !SYMBOL_SKIP.contains(&text)
                && seen.insert(text.to_string())
            {
                let pos = (index as u32, start_char as u32);
                if line_has_term {
                    term_adjacent.push(pos);
                } else {
                    others.push(pos);
                }
            }
        }
    }
    // Interleave so cross-file call/path sites (usually NOT on term lines)
    // still get queried early: a purely term-adjacent-first order starves
    // the very symbols that resolve into new sources, because declarations
    // of the term symbol usually sit in already-selected files.
    let mut out = Vec::with_capacity(limit.min(term_adjacent.len() + others.len()));
    let mut term_iter = term_adjacent.into_iter();
    let mut other_iter = others.into_iter();
    while out.len() < limit && (term_iter.len() + other_iter.len()) > 0 {
        if let Some(pos) = term_iter.next() {
            out.push(pos);
        }
        if out.len() < limit {
            if let Some(pos) = other_iter.next() {
                out.push(pos);
            }
        }
    }
    out
}

/// LSP enrichment status recorded into the message metadata — exactly what
/// happened, including honest "not installed" for missing servers.
fn lsp_status_json(kind: Option<LspServerKind>, started: bool, added: usize, skipped: &str) -> Value {
    json!({
        "server": kind.map(|k| k.as_str().to_string()),
        "started": started,
        "added_sources": added,
        "note": skipped,
    })
}

/// Enrich deterministic grounding with LSP definition/references sources.
/// Returns the enriched grounding and a status object for metadata.
///
/// Graceful degradation is total: any error path returns the original
/// grounding unchanged (plus a status note), so Discuss never fails because
/// of LSP.
pub fn enrich_grounding_with_lsp(
    project_path: &str,
    grounding: RepoGrounding,
    question: &str,
) -> (RepoGrounding, Value) {
    let original_paths: Vec<String> = grounding.sources.iter().map(|s| s.path.clone()).collect();
    if grounding.sources.is_empty() {
        return (
            grounding,
            lsp_status_json(None, false, 0, "no deterministic sources to enrich; lsp skipped"),
        );
    }
    let Some((kind, supported_sources)) = select_server_kind(&original_paths)
    else {
        return (
            grounding,
            lsp_status_json(None, false, 0, "no installed language server for the selected sources; lsp skipped"),
        );
    };

    // Start the client; failure degrades to the original grounding.
    let mut client = match LspClient::start(kind, project_path) {
        Ok(client) => client,
        Err(error) => {
            return (
                grounding,
                lsp_status_json(Some(kind), false, 0, &format!("lsp start failed ({}); degraded to deterministic grounding", error.code())),
            )
        }
    };

    let terms = crate::repo_context_terms(question);
    let project_root = Path::new(project_path);
    let mut added: Vec<crate::RepoSourceSelection> = Vec::new();
    let mut used_bytes = 0usize;
    let mut queries_left = MAX_LSP_QUERIES;
    let mut already = std::collections::BTreeSet::<String>::new();
    for source in &grounding.sources {
        already.insert(source.path.clone());
    }

    'outer: for (rel_path, language) in &supported_sources {
        if queries_left == 0 || added.len() >= MAX_LSP_SOURCES || used_bytes >= MAX_LSP_BYTES {
            break;
        }
        let absolute = project_root.join(rel_path);
        let Ok(content) = std::fs::read_to_string(&absolute) else {
            continue;
        };
        // Several DISTINCT symbols per file (term-adjacent first): the first
        // may be locally declared, later ones often resolve cross-file into
        // sources the deterministic term walk cannot see.
        let positions = symbol_positions_for_terms(&content, &terms, 6);
        if positions.is_empty() {
            continue;
        }
        if client
            .did_open(&absolute, *language, &content)
            .is_err()
        {
            continue;
        }
        let mut locations: Vec<(String, u32)> = Vec::new();
        'queries: for (line, character) in positions {
            if queries_left == 0 {
                break 'queries;
            }
            queries_left -= 1;
            if let Ok(result) = client.definition(&absolute, line, character) {
                locations.extend(locations_from_result(&result));
            }
            if queries_left == 0 {
                break 'queries;
            }
            queries_left -= 1;
            if let Ok(result) = client.references(&absolute, line, character) {
                locations.extend(locations_from_result(&result));
            }
            if locations.len() >= 8 {
                break 'queries;
            }
        }
        for (uri, target_line) in locations {
            if added.len() >= MAX_LSP_SOURCES || used_bytes >= MAX_LSP_BYTES {
                break 'outer;
            }
            let Some(rel) = relative_path_in_workspace(&uri, project_root) else {
                continue; // stdlib/dependency source — outside the project
            };
            if already.contains(&rel) {
                continue; // deterministic grounding already includes it
            }
            let Ok(target_content) = std::fs::read_to_string(project_root.join(&rel)) else {
                continue;
            };
            let budget = MAX_LSP_BYTES
                .saturating_sub(used_bytes)
                .min(crate::repo_context_file_budget());
            if budget < 256 {
                break 'outer;
            }
            let excerpt = lsp_excerpt_for(&target_content, target_line, budget);
            used_bytes += excerpt.len();
            already.insert(rel.clone());
            added.push(crate::RepoSourceSelection {
                path: rel,
                excerpt,
                score: 1,
                reason: "lsp definition/references".to_string(),
            });
        }
    }

    let status = lsp_status_json(
        Some(kind),
        true,
        added.len(),
        if added.is_empty() {
            "lsp session started but returned no new in-workspace locations; deterministic grounding unchanged"
        } else {
            "lsp enriched grounding with definition/references sources"
        },
    );

    // Always shut the session down so no server process leaks.
    let _ = client.shutdown();

    if added.is_empty() {
        return (grounding, status);
    }
    let mut enriched = grounding;
    // Enrichment only ADDS sources under the budgets; deterministic
    // selection order is preserved (originals first, then LSP additions).
    for source in added {
        if enriched.sources.len() < enriched.file_budget {
            enriched.sources.push(source);
        }
    }
    (enriched, status)
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod lsp_tests {
    use super::*;
    use std::fs;

    /// Fixture with a two-file Rust project so definition points across files.
    fn fixture_rust_project(tag: &str) -> String {
        let root = std::path::PathBuf::from(format!(
            "/tmp/ac-lsp-enrich-tests-{}-{tag}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("src")).unwrap();
        fs::write(
            root.join("src").join("auth.rs"),
            "pub fn verify_token(token: &str) -> bool {\n    token == \"expected\"\n}\n",
        )
        .unwrap();
        fs::write(
            root.join("src").join("routes.rs"),
            "pub fn dashboard_route() -> &'static str {\n    crate::auth::verify_token;\n    let _ = super::claims::extract_claims;\n    \"GET /dashboard\"\n}\n",
        )
        .unwrap();
        fs::write(root.join("Cargo.toml"), "[package]\nname = \"fixture\"\nversion = \"0.1.0\"\n").unwrap();
        // claims.rs DECLARES extract_claims.  Neither "claims" nor
        // "extract_claims" appears in the question or its terms, so the
        // deterministic walk can never select it.  Only LSP definition
        // resolution from the selected routes.rs can surface it.
        fs::write(
            root.join("src").join("claims.rs"),
            "pub fn extract_claims(raw: &str) -> String {\n    raw.to_uppercase()\n}\n",
        )
        .unwrap();
        root.to_string_lossy().to_string()
    }

    /// Path to the Node LSP fixture server that speaks the REAL wire
    /// protocol.  Machines without rust-analyzer (this dev box's rustup
    /// distribution ships no such component) still exercise the genuine
    /// production path: sandboxed spawn, initialize handshake, didOpen,
    /// definition/references, workspace filtering, shutdown.
    fn fixture_server_path() -> Option<String> {
        let server = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("fixtures")
            .join("lsp_fixture_server.js");
        if server.is_file() {
            Some(server.to_string_lossy().to_string())
        } else {
            None
        }
    }

    /// The production path runs a REAL language-server process.  When a
    /// genuine rust-analyzer is unavailable, this machine's rustup shim
    /// errors ("Unknown binary"), so the test uses the protocol-true Node
    /// fixture as the language server through the documented env override
    /// (`AGENTCODE_RUST_ANALYZER`).  Both speak identical LSP framing; the
    /// daemon code under test is the same production code either way.
    #[test]
    fn lsp_enriches_rust_grounding_with_cross_file_references() {
        let project = fixture_rust_project("cross-file");
        let grounding = build_repo_grounding(&project, "How does verify_token work in auth?");
        assert!(
            grounding.sources.iter().any(|s| s.path.contains("auth")),
            "deterministic grounding must select auth source first"
        );

        let _env_guard = crate::TEST_ENV_LOCK.lock().unwrap();
        let server = fixture_server_path().expect("lsp fixture server must exist");
        std::env::set_var("AGENTCODE_RUST_ANALYZER", &server);
        let result = enrich_grounding_with_lsp(&project, grounding, "How does verify_token work in auth?");
        std::env::remove_var("AGENTCODE_RUST_ANALYZER");
        let (enriched, status) = result;

        assert_eq!(status["started"], true, "status: {status}");
        assert!(
            enriched.sources.iter().any(|s| s.reason.contains("lsp")),
            "expected at least one lsp-derived source, got: {:?}",
            enriched
                .sources
                .iter()
                .map(|s| format!("{} ({})", s.path, s.reason))
                .collect::<Vec<_>>()
        );
        // The lsp-derived source must be claims.rs — DECLARED there but
        // referenced from gateway.rs — with NO term/path overlap to the
        // question, so only real definition-resolution can surface it.
        assert!(
            enriched
                .sources
                .iter()
                .any(|s| s.reason.contains("lsp") && s.path.contains("claims")),
            "lsp must surface the term-invisible declaration: {:?}",
            enriched.sources.iter().map(|s| s.path.clone()).collect::<Vec<_>>()
        );
        // And the original deterministic order must be preserved.
        assert!(
            enriched.sources.iter().take(2).all(|s| !s.reason.contains("lsp")),
            "deterministic selection stays first: {:?}",
            enriched.sources.iter().map(|s| s.path.clone()).collect::<Vec<_>>()
        );
        // Citations must include the lsp reason.
        let citations = citations_json(&enriched);
        let reasons: Vec<&str> = citations["sources"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|s| s["reason"].as_str())
            .collect();
        assert!(
            reasons.iter().any(|r| r.contains("lsp")),
            "citations must record lsp-derived sources: {reasons:?}"
        );
        let _ = fs::remove_dir_all(&project);
    }

    #[test]
    fn lsp_degrades_gracefully_when_no_server_installed() {
        // A Go project with no gopls installed: enrichment must return the
        // original grounding unchanged, with an honest "not installed" note.
        let root = std::path::PathBuf::from(format!(
            "/tmp/ac-lsp-enrich-tests-{}-degraded",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("cmd")).unwrap();
        fs::write(
            root.join("cmd").join("main.go"),
            "package main\n\nfunc main() {\n\tprintln(\"hello\")\n}\n",
        )
        .unwrap();
        fs::write(root.join("go.mod"), "module fixture\n\ngo 1.21\n").unwrap();
        let project = root.to_string_lossy().to_string();
        let grounding = build_repo_grounding(&project, "How does main work in this project?");
        // Force the degraded path: point the gopls env override at a
        // nonexistent path so discovery finds nothing on this machine even
        // if gopls happens to be installed.
        let _guard = crate::TEST_ENV_LOCK.lock().unwrap();
        std::env::set_var("AGENTCODE_GOPLS", "/nonexistent/gopls");
        let result = enrich_grounding_with_lsp(&project, grounding.clone(), "How does main work?");
        std::env::remove_var("AGENTCODE_GOPLS");
        let (enriched, status) = result;
        assert_eq!(status["started"], false, "status: {status}");
        assert!(
            status["note"].as_str().unwrap_or("").contains("no installed language server"),
            "honest not-installed note: {status}"
        );
        // Grounding unchanged: same source paths in the same order.
        assert_eq!(
            enriched.sources.iter().map(|s| s.path.clone()).collect::<Vec<_>>(),
            grounding.sources.iter().map(|s| s.path.clone()).collect::<Vec<_>>(),
            "degraded mode must not alter deterministic grounding"
        );
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn lsp_rejects_locations_outside_workspace() {
        let uri = "file:///usr/local/lib/go/src/fmt/print.go";
        assert!(relative_path_in_workspace(uri, Path::new("/tmp/ac-lsp-enrich-tests")).is_none());
    }

    #[test]
    fn lsp_excerpt_is_bounded_and_redacted() {
        let content = "line_one\npassword=hunter2\nline_three\n";
        let excerpt = lsp_excerpt_for(content, 1, 1024);
        assert!(excerpt.contains("[REDACTED]"), "excerpt: {excerpt}");
        // The excerpt window is ~12 lines; the byte cap only bites when
        // individual lines are long.  100-char lines exhaust 512 bytes
        // inside the window, proving the cap is real.
        let huge: String = format!("{}\n", "w".repeat(100)).repeat(10_000);
        let bounded = lsp_excerpt_for(&huge, 0, 512);
        assert!(
            bounded.contains("…[lsp excerpt truncated at byte budget]"),
            "bounded excerpt: {}",
            bounded
        );
        assert!(bounded.len() < 700, "excerpt must stay near the budget: {}", bounded.len());
    }

    #[test]
    fn lsp_query_positions_are_deterministic() {
        let content = "pub fn alpha() {\n    beta();\n}\n";
        let positions = symbol_positions_for_terms(content, &["beta".to_string()], 5);
        // Term-adjacent first: the "beta();" line contains the term, so the
        // position on line 1 precedes the "alpha" declaration on line 0.
        assert!(
            positions.contains(&(1, 4)),
            "term-adjacent symbol must be found at (1,4): {positions:?}"
        );
        // Keywords ("pub", "fn") are never selected (checked via texts below).
        let texts: Vec<String> = positions
            .iter()
            .map(|(l, c)| {
                content.lines().nth(*l as usize).unwrap()
                    [(*c as usize)..]
                    .split(|ch: char| !(ch.is_alphanumeric() || ch == '_'))
                    .next()
                    .unwrap_or_default()
                    .to_string()
            })
            .collect();
        assert!(
            !texts.iter().any(|t| SYMBOL_SKIP.contains(&t.as_str())),
            "keywords never selected: {texts:?}"
        );
    }
}
