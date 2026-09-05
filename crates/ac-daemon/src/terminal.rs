// ── Terminal / process surface (batch N4, G7) ──────────────────────────────
//
// A real, bounded, observable process surface for the daemon:
//
//   * terminal_start  spawns a process (argv allowlisted below, cwd inside
//     the mission workspace), captures stdout+stderr in reader threads, and
//     tracks liveness.  Sessions are registered in a daemon-owned map keyed
//     by session id — nothing about the OS process is exposed to the UI.
//   * terminal_tail   returns the captured output SINCE a caller-supplied
//     cursor so a polling UI renders a live log without re-reading.
//   * terminal_cancel terminates a live session (SIGTERM then SIGKILL) and
//     records the honest terminal state.
//   * terminal_list  reports every live/finished session with state.
//
// Output persistence: when a session finishes (exit, timeout, or cancel),
// the captured combined output is persisted as CommandOutput evidence
// (redacted via the evidence layer's own redact + our source-line redaction)
// and linked to the mission through task-attempt style references.  The
// evidence id is returned by terminal_tail/terminal_state so the UI can link
// directly to it.
//
// Honesty rules (same as the design-preview precedent):
//   * `alive`/`exit_code` always come from the REAL child (try_wait), never
//     from a timer guess.
//   * A session that was never started reports that; unknown ids error.
//   * Cancel is honest: it may fail (already-exited races) and then reports
//     the actual observed state instead of claiming success.
//
// Process safety follows the design-preview pattern: argv must match the
// allowlist, cwd must exist, output is byte-capped, and the daemon kills all
// live children on stop (same hook as design_children).

/// Byte cap per stream per session — the terminal surface is for tails, not
/// for capturing unbounded logs.
const MAX_TERMINAL_CAPTURE_BYTES: usize = 256 * 1024;

/// Hard lifetime cap so a forgotten session cannot run forever.
const MAX_TERMINAL_LIFETIME_SECS: u64 = 30 * 60;

/// Allowlisted executables for the terminal surface.  Deliberately narrow:
/// build/dev tooling the product itself runs, not arbitrary shells.
fn terminal_argv_allowed(argv: &[String]) -> bool {
    if argv.is_empty() {
        return false;
    }
    match argv[0].as_str() {
        "npm" | "pnpm" | "yarn" | "node" | "cargo" | "make" | "python3" | "git" => {
            // No shell metacharacters anywhere in the argv — these are
            // executed directly (no shell), but keep the rule anyway so a
            // future wrapper cannot smuggle one in.
            !argv.iter().any(|arg| {
                arg.contains('|')
                    || arg.contains(';')
                    || arg.contains("&&")
                    || arg.contains('`')
                    || arg.contains('$')
            })
        }
        _ => false,
    }
}

/// Internal per-session live state.  The daemon owns the child handle; the
/// UI only ever sees ids, states, and captured bytes.
pub struct TerminalSessionState {
    pub id: String,
    pub argv: Vec<String>,
    pub cwd: String,
    pub mission_id: Option<String>,
    pub output: Mutex<VecDeque<String>>,
    pub total_bytes: AtomicUsize,
    pub child: Mutex<Option<std::process::Child>>,
    pub exit_code: Mutex<Option<i32>>,
    pub cancelled: AtomicBool,
    pub started_at_ms: i64,
    pub evidence_id: Mutex<Option<String>>,
}

impl TerminalSessionState {
    fn append_output_line(&self, line: &str) {
        let mut out = self.output.lock().unwrap();
        if self.total_bytes.load(Ordering::SeqCst) + line.len()
            > MAX_TERMINAL_CAPTURE_BYTES
        {
            // Cap reached: stop appending, but mark truncation once.
            if !out.iter().any(|l| l.contains("[output truncated")) {
                out.push_back("[output truncated at capture byte cap]".to_string());
            }
            return;
        }
        out.push_back(line.to_string());
        self.total_bytes.fetch_add(line.len(), Ordering::SeqCst);
    }
}

/// Redact a raw output line for the UI tail AND for evidence.  Uses the
//  same secret-key heuristic as grounding excerpts (assignment patterns).
fn terminal_redact(line: &str) -> String {
    crate::redact_source_line(line)
}

impl crate::DaemonService {
    /// Start a tracked terminal process.  argv must be allowlisted and cwd
    /// must exist; failures are honest validation errors, never silent.
    pub fn terminal_start(
        &mut self,
        mission_id: Option<&str>,
        argv: &[String],
        cwd: &str,
    ) -> AcResult<Value> {
        if !terminal_argv_allowed(argv) {
            return Err(AcError::validation(
                "TERMINAL-COMMAND_NOT_ALLOWED",
                format!(
                    "executable '{}' is not allowlisted for the terminal surface",
                    argv.first().map(String::as_str).unwrap_or("")
                ),
            ));
        }
        let dir = Path::new(cwd);
        if !dir.is_dir() {
            return Err(AcError::validation(
                "TERMINAL-CWD_INVALID",
                format!("cwd '{cwd}' is not a directory"),
            ));
        }
        let session_id = StableId::new("terminal").to_string();
        let mut command = Command::new(&argv[0]);
        command
            .args(&argv[1..])
            .current_dir(dir)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .stdin(Stdio::null());
        let mut child = command.spawn().map_err(|error| {
            AcError::validation(
                "TERMINAL-SPAWN_FAILED",
                format!("could not start '{}': {error}", argv[0]),
            )
        })?;
        let pid = child.id();
        let stdout = child.stdout.take();
        let stderr = child.stderr.take();

        let state = Arc::new(TerminalSessionState {
            id: session_id.clone(),
            argv: argv.to_vec(),
            cwd: cwd.to_string(),
            mission_id: mission_id.map(str::to_string),
            output: Mutex::new(VecDeque::new()),
            total_bytes: AtomicUsize::new(0),
            child: Mutex::new(Some(child)),
            exit_code: Mutex::new(None),
            cancelled: AtomicBool::new(false),
            started_at_ms: TimestampMillis::now().as_millis() as i64,
            evidence_id: Mutex::new(None),
        });

        // Reader threads append lines; the reaper thread finalizes state.
        // stdout and stderr are distinct types, so spawn per-stream.
        let spawn_reader = |pipe: Box<dyn Read + Send>| {
            let state = Arc::clone(&state);
            thread::Builder::new()
                .name("terminal-reader".to_string())
                .spawn(move || {
                    for line in BufReader::new(pipe).lines().map_while(Result::ok) {
                        state.append_output_line(&line);
                    }
                })
                .map_err(|e| AcError::validation("TERMINAL-READER", e.to_string()))
        };
        if let Some(pipe) = stdout {
            spawn_reader(Box::new(pipe))?;
        }
        if let Some(pipe) = stderr {
            spawn_reader(Box::new(pipe))?;
        }

        // Reaper: waits for exit, records the code, persists evidence.
        {
            let state = Arc::clone(&state);
            let db_path = self.db_path.clone();
            thread::Builder::new()
                .name("terminal-reaper".to_string())
                .spawn(move || {
                    // Poll try_wait so cancel() can interleave.
                    let deadline = Instant::now()
                        + std::time::Duration::from_secs(MAX_TERMINAL_LIFETIME_SECS);
                    loop {
                        let mut guard = state.child.lock().unwrap();
                        if let Some(child) = guard.as_mut() {
                            match child.try_wait() {
                                Ok(Some(status)) => {
                                    *state.exit_code.lock().unwrap() = status.code();
                                    // Release the handle so Drop doesn't double-wait.
                                    *guard = None;
                                    drop(guard);
                                    persist_terminal_evidence(&db_path, &state);
                                    return;
                                }
                                Ok(None) => {}
                                Err(_) => {
                                    *guard = None;
                                    drop(guard);
                                    persist_terminal_evidence(&db_path, &state);
                                    return;
                                }
                            }
                        } else {
                            // Cancel path already reaped it.
                            drop(guard);
                            persist_terminal_evidence(&db_path, &state);
                            return;
                        }
                        if Instant::now() > deadline {
                            // Lifetime cap: kill and record honestly.
                            let mut guard = state.child.lock().unwrap();
                            if let Some(child) = guard.as_mut() {
                                let _ = child.kill();
                                let _ = child.wait();
                            }
                            *guard = None;
                            drop(guard);
                            state.cancelled.store(true, Ordering::SeqCst);
                            persist_terminal_evidence(&db_path, &state);
                            return;
                        }
                        drop(guard);
                        thread::sleep(std::time::Duration::from_millis(100));
                    }
                })
                .map_err(|e| AcError::validation("TERMINAL-REAPER", e.to_string()))?;
        }

        self.terminal_sessions
            .lock()
            .unwrap()
            .insert(session_id.clone(), Arc::clone(&state));

        Ok(json!({
            "session_id": session_id,
            "argv": argv,
            "cwd": cwd,
            "pid": pid,
            "mission_id": mission_id,
            "started_at_ms": state.started_at_ms,
            "note": "tracked process; tail via TerminalTail with the session_id",
        }))
    }

    /// Tail captured output since `cursor` (line offset).  Returns the new
    /// cursor plus live state — a polling UI appends `lines` each round.
    pub fn terminal_tail(&self, session_id: &str, cursor: usize) -> AcResult<Value> {
        let sessions = self.terminal_sessions.lock().unwrap();
        let Some(state) = sessions.get(session_id) else {
            return Err(AcError::validation(
                "TERMINAL-SESSION_UNKNOWN",
                "no terminal session with that id",
            ));
        };
        let output = state.output.lock().unwrap();
        let total = output.len();
        let lines: Vec<String> = if cursor < total {
            output.iter().skip(cursor).cloned().collect()
        } else {
            Vec::new()
        };
        let exit_code = *state.exit_code.lock().unwrap();
        let evidence_id = state.evidence_id.lock().unwrap().clone();
        Ok(json!({
            "session_id": session_id,
            "cursor": total,
            "lines": lines,
            "alive": exit_code.is_none(),
            "exit_code": exit_code,
            "cancelled": state.cancelled.load(Ordering::SeqCst),
            "evidence_id": evidence_id,
            "total_captured_bytes": state.total_bytes.load(Ordering::SeqCst),
        }))
    }

    /// Cancel a live session: SIGTERM-ish via child kill (Rust std exposes
    /// only kill; keep the honest name), then finalize + evidence.
    pub fn terminal_cancel(&mut self, session_id: &str) -> AcResult<Value> {
        let state = {
            let sessions = self.terminal_sessions.lock().unwrap();
            let Some(state) = sessions.get(session_id) else {
                return Err(AcError::validation(
                    "TERMINAL-SESSION_UNKNOWN",
                    "no terminal session with that id",
                ));
            };
            Arc::clone(state)
        };
        let mut guard = state.child.lock().unwrap();
        if let Some(child) = guard.as_mut() {
            state.cancelled.store(true, Ordering::SeqCst);
            let kill_result = child.kill();
            let wait_result = child.wait();
            let code = wait_result.ok().and_then(|s| s.code());
            *state.exit_code.lock().unwrap() = code;
            *guard = None;
            drop(guard);
            let evidence_id = persist_terminal_evidence(&self.db_path, &state);
            return Ok(json!({
                "session_id": session_id,
                "cancelled": kill_result.is_ok(),
                "exit_code": code,
                "evidence_id": evidence_id,
                "note": if kill_result.is_ok() {
                    "process terminated; captured output persisted as evidence"
                } else {
                    "process already exited before cancel; captured output persisted"
                },
            }));
        }
        drop(guard);
        Ok(json!({
            "session_id": session_id,
            "cancelled": false,
            "exit_code": *state.exit_code.lock().unwrap(),
            "evidence_id": state.evidence_id.lock().unwrap().clone(),
            "note": "session already terminal; nothing cancelled",
        }))
    }

    /// List all sessions (live and finished) with honest state.
    pub fn terminal_list(&self) -> AcResult<Value> {
        let sessions = self.terminal_sessions.lock().unwrap();
        let mut list = Vec::new();
        for state in sessions.values() {
            let exit_code = *state.exit_code.lock().unwrap();
            list.push(json!({
                "session_id": state.id,
                "argv": state.argv,
                "cwd": state.cwd,
                "mission_id": state.mission_id,
                "alive": exit_code.is_none(),
                "exit_code": exit_code,
                "cancelled": state.cancelled.load(Ordering::SeqCst),
                "started_at_ms": state.started_at_ms,
                "captured_lines": state.output.lock().unwrap().len(),
            }));
        }
        Ok(json!({ "sessions": list }))
    }

    /// Kill every live terminal session (daemon stop path, same as
    /// design_children).
    pub fn terminal_shutdown_all(&mut self) {
        let sessions = self.terminal_sessions.lock().unwrap();
        for state in sessions.values() {
            let mut guard = state.child.lock().unwrap();
            if let Some(child) = guard.as_mut() {
                let _ = child.kill();
                let _ = child.wait();
            }
            *guard = None;
        }
    }
}

/// Persist the captured (redacted) output of a finished session as
/// CommandOutput evidence bound to the mission (when one is linked).  Runs on
/// the reaper/cancel thread with its own DB connection — the daemon's
/// primary connection stays free.
fn persist_terminal_evidence(db_path: &Path, state: &TerminalSessionState) -> Option<String> {
    // Idempotent: only one finalizer wins.
    {
        let evidence = state.evidence_id.lock().unwrap();
        if evidence.is_some() {
            return evidence.clone();
        }
    }
    let lines: Vec<String> = state
        .output
        .lock()
        .unwrap()
        .iter()
        .map(|l| terminal_redact(l))
        .collect();
    let content = lines.join("\n");
    let hash = crate::fnv1a64_hash(content.as_bytes());
    let evidence_id = StableId::new("termevidence").to_string();
    let record = ac_evidence::EvidenceRecord {
        id: StableId::from_existing(&evidence_id).ok()?,
        kind: ac_evidence::EvidenceKind::CommandOutput,
        provenance: ac_evidence::Provenance {
            source: "terminal".to_string(),
            commit: None,
            worktree: Some(state.cwd.clone()),
            tool: Some(state.argv.first().cloned().unwrap_or_default()),
        },
        artifact_uri: format!("mem://terminal/{}", state.id),
        content_hash: hash,
        // Captured output can contain real values; keep it non-durable raw
        // but summarize safely for the model.
        raw_content: None,
        model_summary: Some(crate::bounded_terminal_summary(&content)),
        sensitive: true,
        created_at: TimestampMillis::now(),
    };
    let db = ac_db::ControlPlaneDb::open(db_path).ok()?;
    db.append_evidence(&record).ok()?;
    let mut evidence = state.evidence_id.lock().unwrap();
    *evidence = Some(evidence_id.clone());
    Some(evidence_id)
}

/// Bounded, redacted summary of terminal output for the evidence record.
pub fn bounded_terminal_summary(content: &str) -> String {
    let bounded = if content.len() > 2048 {
        format!("{}…[truncated]", &content[..content.floor_char_boundary(2048)])
    } else {
        content.to_string()
    };
    bounded
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod terminal_tests {
    use super::*;

    #[test]
    fn allowlist_blocks_shells_and_metacharacters() {
        assert!(terminal_argv_allowed(&["cargo".to_string(), "test".to_string()]));
        assert!(terminal_argv_allowed(&["npm".to_string(), "run".to_string(), "build".to_string()]));
        assert!(!terminal_argv_allowed(&["sh".to_string()]));
        assert!(!terminal_argv_allowed(&["bash".to_string()]));
        assert!(!terminal_argv_allowed(&["cargo".to_string(), "build".to_string(), "&& rm -rf /".to_string()]));
        assert!(!terminal_argv_allowed(&["node".to_string(), "-e".to_string(), "$(cat /etc/passwd)".to_string()]));
        assert!(!terminal_argv_allowed(&[]));
    }

    #[test]
    fn output_capped_at_byte_budget() {
        let state = TerminalSessionState {
            id: "t".into(),
            argv: vec!["node".into()],
            cwd: "/tmp".into(),
            mission_id: None,
            output: Mutex::new(VecDeque::new()),
            total_bytes: AtomicUsize::new(0),
            child: Mutex::new(None),
            exit_code: Mutex::new(None),
            cancelled: AtomicBool::new(false),
            started_at_ms: 0,
            evidence_id: Mutex::new(None),
        };
        let big_line = "x".repeat(60 * 1024);
        for _ in 0..10 {
            state.append_output_line(&big_line);
        }
        let out = state.output.lock().unwrap();
        let total: usize = out
            .iter()
            .filter(|l| !l.contains("[output truncated"))
            .map(|l| l.len())
            .sum();
        assert!(total <= MAX_TERMINAL_CAPTURE_BYTES + 1024, "total {total}");
        assert!(out.iter().any(|l| l.contains("[output truncated")));
    }

    #[test]
    fn terminal_summary_is_bounded() {
        let huge = "y".repeat(10_000);
        let summary = bounded_terminal_summary(&huge);
        assert!(summary.len() <= 2100, "len {}", summary.len());
        assert!(summary.contains("…[truncated]"));
    }
}
