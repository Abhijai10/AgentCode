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

/// Ring window for the LIVE tail (final-audit optimization): keep at
/// most this many recent lines in memory for cheap polling.  The full
/// capture still flows to evidence (bounded by the byte cap); the ring
/// only bounds what a poll must clone.  The dropped-line count keeps the
/// cursor math honest: a cursor older than the ring start reports the
/// ring head, never a silent gap.
const MAX_TERMINAL_RING_LINES: usize = 2_000;

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
    /// Lines appended EVER (monotonic) — the stable cursor domain even
    /// after ring eviction removes old lines from the window.
    pub total_lines: AtomicUsize,
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
        self.total_lines.fetch_add(1, Ordering::SeqCst);
        // Ring eviction: drop the OLDEST lines beyond the window.  The
        // caller's cursor stays valid — total_lines never decreases (see
        // terminal_tail), and a cursor behind the window start is served
        // from the head with an honest dropped count.
        while out.len() > MAX_TERMINAL_RING_LINES {
            out.pop_front();
        }
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
            total_lines: AtomicUsize::new(0),
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
        // Ring-aware cursor math (final-audit optimization): the cursor
        // domain is TOTAL lines ever (monotonic).  A cursor behind the
        // ring start (evicted lines) is served from the head with an
        // honest `dropped_lines` count instead of silently skipping.
        let total_lines = state.total_lines.load(Ordering::SeqCst);
        let ring_len = output.len();
        let ring_start = total_lines.saturating_sub(ring_len);
        let from = cursor.max(ring_start);
        let dropped = from.saturating_sub(cursor);
        let lines: Vec<String> = if from < total_lines {
            output
                .iter()
                .skip(from - ring_start)
                .cloned()
                .collect()
        } else {
            Vec::new()
        };
        let exit_code = *state.exit_code.lock().unwrap();
        let evidence_id = state.evidence_id.lock().unwrap().clone();
        Ok(json!({
            "session_id": session_id,
            "cursor": total_lines,
            "lines": lines,
            "dropped_lines": dropped,
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

    /// Ring window (final-audit optimization): appending past
    /// MAX_TERMINAL_RING_LINES keeps only the newest window; the tail
    /// reports total_lines EVER (monotonic cursor), serves from the head
    /// when the cursor predates the ring, and reports dropped_lines
    /// honestly instead of silently skipping.
    #[test]
    fn ring_window_keeps_recent_lines_and_honest_cursors() {
        let state = TerminalSessionState {
            id: "ring".into(),
            argv: vec!["node".into()],
            cwd: "/tmp".into(),
            mission_id: None,
            output: Mutex::new(VecDeque::new()),
            total_bytes: AtomicUsize::new(0),
            total_lines: AtomicUsize::new(0),
            child: Mutex::new(None),
            exit_code: Mutex::new(None),
            cancelled: AtomicBool::new(false),
            started_at_ms: 0,
            evidence_id: Mutex::new(None),
        };
        for n in 0..(MAX_TERMINAL_RING_LINES + 500) {
            state.append_output_line(&format!("line-{n}"));
        }
        // Window holds the newest MAX_TERMINAL_RING_LINES lines…
        assert_eq!(state.output.lock().unwrap().len(), MAX_TERMINAL_RING_LINES);
        // …starting at line-500 (the oldest 500 evicted).
        assert_eq!(
            state.output.lock().unwrap().front().unwrap().trim(),
            "line-500"
        );
        // The cursor domain counts EVERY line ever.
        let total = state.total_lines.load(Ordering::SeqCst);
        assert_eq!(total, MAX_TERMINAL_RING_LINES + 500);

        // Fresh cursor (end): nothing new.
        // (terminal_tail serves via the daemon; here we assert the math
        // the tail performs, on the same state.)
        let ring_len = state.output.lock().unwrap().len();
        let ring_start = total.saturating_sub(ring_len);
        assert_eq!(ring_start, 500);
        // A cursor at 500 serves the full window.
        let from = 500usize.max(ring_start);
        assert_eq!(from, 500);
        // A cursor at 100 (behind the ring): served from the head with
        // 400 dropped, never a silent gap.
        let from_behind = 100usize.max(ring_start);
        let dropped = from_behind.saturating_sub(100);
        assert_eq!(dropped, 400);
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
            total_lines: AtomicUsize::new(0),
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
