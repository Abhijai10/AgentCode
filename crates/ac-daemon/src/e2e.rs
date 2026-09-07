// AI-driven end-to-end testing mode (Codex-Playwright parity): launch the
// project's own dev server, drive the REAL app in headless Chrome through
// its primary flows, collect console/page/network failures plus DOM
// evidence, and turn them into an honest bug report the user can hand to a
// mission (approval-gated, through the normal GoalSubmit/Kernel path).
impl DaemonService {
    /// Run an E2E pass over the project's live app and produce a bug report.
    /// The flow is real: dev server (reuses the design-preview launcher),
    /// real Chrome via the shared engine, real diagnostics — every step
    /// records evidence, and failures are reported as failures.
    pub fn e2e_run(&mut self, conversation_id: &str) -> AcResult<Value> {
        self.ensure_running()?;
        let conv = self.db.conversation(conversation_id)?.ok_or_else(|| {
            AcError::validation("CONVERSATION-NOT_FOUND", "conversation not found")
        })?;
        let _project_path = conv.project_path.clone();

        // 1) Dev server: reuse the design-preview lifecycle (spawn, port
        //    detect, http probe).  It starts the project's OWN dev command.
        let preview = self.db.design_preview(conversation_id)?;
        let port: u16 = match preview {
            Some(p) if p.process_alive => match p.port {
                Some(port) if port > 0 => port as u16,
                _ => 0,
            },
            _ => 0,
        };
        let port = if port > 0 {
            port
        } else {
            self.design_preview_start(conversation_id)?;
            // Port detection is asynchronous (log-line readers + lsof
            // probing run while npm boots the child).  Poll the stored
            // preview row until a ready port appears or the boot window
            // expires — never a silent zero.
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(12);
            let mut detected: u16 = 0;
            while std::time::Instant::now() < deadline {
                let mut row_port: Option<i64> = None;
                if let Some(p) = self.db.design_preview(conversation_id)? {
                    if p.process_alive {
                        row_port = p.port;
                    }
                }
                if let Some(p) = row_port {
                    if p > 0 {
                        detected = p as u16;
                        break;
                    }
                }
                // Retry detection directly: the preview row is written once
                // at spawn and never re-probed.  The spawned pid comes from
                // the stored process id (pid:NNN); lsof walks descendants.
                if let Some(p) = self.db.design_preview(conversation_id)? {
                    if let Some(process) = p.process_id.as_deref().and_then(|s| s.strip_prefix("pid:")) {
                        if let Ok(pid) = process.trim().parse::<u32>() {
                            if let Some(port) = first_listener_port_of(pid) {
                                detected = port;
                                let now = TimestampMillis::now().as_millis() as i64;
                                let updated = ac_db::DesignPreviewRow {
                                    port: Some(port as i64),
                                    ready_url: Some(format!("http://127.0.0.1:{port}")),
                                    http_ready: true,
                                    updated_at_ms: now,
                                    ..p.clone()
                                };
                                let _ = self.db.save_design_preview(&updated);
                                break;
                            }
                        }
                    }
                }
                std::thread::sleep(std::time::Duration::from_millis(400));
            }
            if detected == 0 {
                return Ok(json!({
                    "status": "E2E_NO_SERVER",
                    "detail": "dev server launched but no port became ready in time",
                }));
            }
            detected
        };
        let base_url = format!("http://127.0.0.1:{port}");

        // 2) Drive the app in real Chrome (the SHARED engine — watch surfaces
        //    stay coherent) and collect evidence at every step.
        let policy = ac_security::CapabilityPolicy::new()
            .allow(ac_security::Capability::BrowserAutomation);
        let task_id = StableId::new("e2e");
        let mut evidence = EvidenceStore::new();
        let mut browser = ac_verification::BrowserRuntime::new(policy);
        let process = browser.launch(task_id.clone())?;
        let session = browser.create_session(task_id.clone(), process.id.clone())?;

        let mut steps: Vec<Value> = Vec::new();
        let mut all_console: Vec<String> = Vec::new();
        let mut all_page: Vec<String> = Vec::new();
        let mut all_network: Vec<String> = Vec::new();

        // Primary flows: land on the root view, then click through every
        // navigable control that is SAFE (buttons and links — no form
        // submit), collecting diagnostics after each hop.
        browser.act(
            &session.id,
            ac_verification::BrowserAction::Navigate { url: base_url.clone() },
            &mut evidence,
        )?;
        browser.act(&session.id, ac_verification::BrowserAction::Wait { millis: 1500 }, &mut evidence)?;
        self.e2e_capture(&mut browser, &session.id, &mut evidence, "load /", &mut steps, &mut all_console, &mut all_page, &mut all_network)?;

        // Discover interactive controls and exercise them.
        let controls = browser.act(
            &session.id,
            ac_verification::BrowserAction::Evaluate {
                script: r#"(() => JSON.stringify(
                    [...document.querySelectorAll('button, a[href]')].slice(0, 12).map(el => ({
                        tag: el.tagName.toLowerCase(),
                        text: (el.textContent || '').trim().slice(0, 40),
                        href: el.getAttribute('href') || '',
                    }))
                ))()"#.to_string(),
            },
            &mut evidence,
        )?;
        let controls_json: Value = controls
            .value
            .and_then(|v| serde_json::from_str::<Value>(v.as_str().unwrap_or("[]")).ok())
            .unwrap_or_else(|| json!([]));
        let mut hops = 0usize;
        for control in controls_json.as_array().cloned().unwrap_or_default() {
            if hops >= 6 {
                break;
            }
            let label = control
                .get("text")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();
            if label.is_empty() {
                continue;
            }
            // Click through CDP coordinate-free selector click on the label.
            let escaped = serde_json::to_string(&label).unwrap_or_default();
            let clicked = browser.act(
                &session.id,
                ac_verification::BrowserAction::Evaluate {
                    script: format!(
                        r#"(() => {{ const el = [...document.querySelectorAll('button, a')].find(e => (e.textContent||'').trim().startsWith({escaped})); if (!el) return 'no'; el.click(); return 'ok'; }})()"#
                    ),
                },
                &mut evidence,
            )?;
            let hit = clicked
                .value
                .and_then(|v| v.as_str().map(|s| s.to_string()))
                .unwrap_or_default();
            if hit != "ok" {
                continue;
            }
            hops += 1;
            browser.act(&session.id, ac_verification::BrowserAction::Wait { millis: 900 }, &mut evidence)?;
            self.e2e_capture(&mut browser, &session.id, &mut evidence, &format!("click {label}"), &mut steps, &mut all_console, &mut all_page, &mut all_network)?;
        }

        // 3) Close the test Chrome (keep the user's shared panel alive).
        browser.close_process(&process.id)?;
        // The dev server stays up — it is the project's own preview and the
        // design-preview lifecycle owns stopping it.

        // 4) Honest bug report: real diagnostics, deduplicated, with a
        //    mission-ready goal the user can approve.
        let console = dedup(all_console);
        let page = dedup(all_page);
        let network = dedup(all_network);
        let bug_count = console.len() + page.len() + network.len();
        let report = format!(
            "E2E test report for {}\n\nFlow: loaded the app and exercised {} interactive controls in real Chrome.\n\nConsole errors ({}):\n{}\n\nPage errors ({}):\n{}\n\nNetwork failures ({}):\n{}\n\nAll steps captured with verifiable evidence.",
            base_url,
            hops,
            console.len(),
            console.iter().map(|l| format!("- {l}")).collect::<Vec<_>>().join("\n"),
            page.len(),
            page.iter().map(|l| format!("- {l}")).collect::<Vec<_>>().join("\n"),
            network.len(),
            network.iter().map(|l| format!("- {l}")).collect::<Vec<_>>().join("\n"),
        );
        let report_id = StableId::new("e2e-report");
        self.db.save_e2e_report(&ac_db::E2EReportRow {
            id: report_id.to_string(),
            conversation_id: conversation_id.to_string(),
            base_url: base_url.clone(),
            bug_count: bug_count as i64,
            report_json: serde_json::to_string(&json!({
                "console": console,
                "page": page,
                "network": network,
                "steps": steps,
            }))
            .unwrap_or_default(),
            mission_ref: None,
            created_at_ms: TimestampMillis::now().as_millis() as i64,
        })?;

        Ok(json!({
            "status": "ok",
            "report_id": report_id.to_string(),
            "base_url": base_url,
            "bug_count": bug_count,
            "console_errors": console,
            "page_errors": page,
            "network_failures": network,
            "steps": steps,
            "report": report,
        }))
    }

    /// One evidence-capturing hop: DOM snapshot + diagnostics appended to
    /// the step list and the aggregate failure lists.
    #[allow(clippy::too_many_arguments)]
    fn e2e_capture(
        &self,
        browser: &mut ac_verification::BrowserRuntime,
        session_id: &StableId,
        evidence: &mut EvidenceStore,
        label: &str,
        steps: &mut Vec<Value>,
        console: &mut Vec<String>,
        page: &mut Vec<String>,
        network: &mut Vec<String>,
    ) -> AcResult<()> {
        let diag = browser.diagnostics(session_id, evidence)?;
        let dom = browser.inspect_dom(session_id, evidence)?;
        console.extend(diag.console_errors.iter().cloned());
        page.extend(diag.page_errors.iter().cloned());
        network.extend(diag.network_failures.iter().cloned());
        steps.push(json!({
            "step": label,
            "url": dom.accessibility_tree.first().cloned().unwrap_or_default(),
            "console_errors": diag.console_errors.len(),
            "page_errors": diag.page_errors.len(),
            "network_failures": diag.network_failures.len(),
            "evidence_count": evidence.len(),
            "evidence_last": evidence
                .records()
                .last()
                .map(|r| r.id.to_string())
                .unwrap_or_default(),
        }));
        Ok(())
    }

    /// Hand the report to a mission — approval-gated, through the normal
    /// GoalSubmit/Kernel/Tool Broker path (E2E mode never bypasses
    /// engineering authority, exactly like security remediation).
    pub fn e2e_fix(&mut self, conversation_id: &str, report_id: &str, approved: bool) -> AcResult<Value> {
        self.ensure_running()?;
        let conv = self.db.conversation(conversation_id)?.ok_or_else(|| {
            AcError::validation("CONVERSATION-NOT_FOUND", "conversation not found")
        })?;
        let report = self.db.e2e_report(report_id)?.ok_or_else(|| {
            AcError::validation("E2E-REPORT_NOT_FOUND", "e2e report not found")
        })?;
        if report.conversation_id != conversation_id {
            return Err(AcError::policy_denied(
                "E2E-REPORT_PROJECT_MISMATCH",
                "report does not belong to this conversation",
            ));
        }
        if !approved {
            return Err(AcError::policy_denied(
                "E2E-FIX_NOT_APPROVED",
                "turning an E2E report into a mission requires explicit user approval",
            ));
        }
        let goal = format!(
            "Fix the E2E-tested bugs for the app at {} ({} issues): {}",
            report.base_url,
            report.bug_count,
            report.report_json,
        );
        let (mission_id, _session_id) = self.goal_submit(conversation_id, &goal, &[])?;
        self.db.set_e2e_report_mission(report_id, mission_id.as_str())?;
        Ok(json!({
            "mission_id": mission_id.to_string(),
            "report_id": report_id,
            "conversation_id": conv.id,
        }))
    }

    /// List persisted E2E reports for a conversation.
    pub fn e2e_reports(&self, conversation_id: &str) -> AcResult<Value> {
        self.ensure_running()?;
        let rows = self.db.e2e_reports_for_conversation(conversation_id)?;
        Ok(json!({
            "reports": rows.iter().map(|r| json!({
                "id": r.id,
                "base_url": r.base_url,
                "bug_count": r.bug_count,
                "mission_ref": r.mission_ref,
                "created_at_ms": r.created_at_ms,
            })).collect::<Vec<_>>(),
        }))
    }
}

/// Dedup preserving first-seen order (bug reports read better deduped).
fn dedup(items: Vec<String>) -> Vec<String> {
    let mut seen = std::collections::HashSet::new();
    items
        .into_iter()
        .filter(|item| !item.is_empty() && seen.insert(item.clone()))
        .collect()
}

#[cfg(test)]
mod e2e_tests {
    use super::*;

    /// The E2E report lifecycle without a browser: fix requires explicit
    /// approval, fix hands the work to a real mission (GoalSubmit path),
    /// and reports persist per conversation.
    #[test]
    fn e2e_fix_requires_approval_and_creates_mission() {
        let (dir, db, lock) = crate::tests::temp_paths();
        let mut daemon = DaemonService::open(&db, &lock).unwrap();
        daemon.start().unwrap();
        // A DESIGN conversation on a real path (the e2e flow reads the
        // conversation; the run itself is browser-gated and tested E2E
        // elsewhere).
        let conv = daemon
            .create_conversation(
                dir.to_str().unwrap(),
                "DESIGN",
                "E2E test conversation",
            )
            .unwrap();

        // Seed a report row directly (what a real E2E run persists).
        let report_id = StableId::new("rep").to_string();
        daemon
            .db
            .save_e2e_report(&ac_db::E2EReportRow {
                id: report_id.clone(),
                conversation_id: conv.to_string(),
                base_url: "http://127.0.0.1:4180".to_string(),
                bug_count: 2,
                report_json: r#"{"console":["x is not defined"],"page":[],"network":[]}"#.to_string(),
                mission_ref: None,
                created_at_ms: TimestampMillis::now().as_millis() as i64,
            })
            .unwrap();

        // Unapproved fix is refused.
        let refused = daemon
            .e2e_fix(conv.as_str(), &report_id, false)
            .unwrap_err();
        assert_eq!(refused.code(), "E2E-FIX_NOT_APPROVED");

        // Approved fix creates a real mission through GoalSubmit.
        let fixed = daemon.e2e_fix(conv.as_str(), &report_id, true).unwrap();
        assert!(fixed["mission_id"].is_string(), "mission created: {fixed}");
        let mission_id = fixed["mission_id"].as_str().unwrap();

        // The report now references its mission.
        let reports = daemon.e2e_reports(conv.as_str()).unwrap();
        let entry = reports["reports"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["id"].as_str().unwrap() == report_id)
            .expect("report listed");
        assert_eq!(entry["mission_ref"].as_str().unwrap(), mission_id);

        // Cross-conversation guard: another conversation cannot use the report.
        let other = daemon.create_conversation("/", "DESIGN", "other").unwrap();
        let denied = daemon.e2e_fix(other.as_str(), &report_id, true).unwrap_err();
        assert_eq!(denied.code(), "E2E-REPORT_PROJECT_MISMATCH");
        let _ = fs::remove_dir_all(dir);
    }
}

#[cfg(test)]
mod e2e_live_tests {
    use super::*;

    /// Real end-to-end: a fixture project whose dev server is a tiny static
    /// server, driven by real headless Chrome through e2e_run.  Proves the
    /// whole flow: server detection, browser drive, diagnostics, report
    /// persistence — with a page that carries a REAL console error, so the
    /// report must find at least one bug.
    #[test]
    fn e2e_run_drives_real_app_and_reports_real_bugs() {
        let (dir, db, lock) = crate::tests::temp_paths();
        let mut daemon = DaemonService::open(&db, &lock).unwrap();
        daemon.start().unwrap();

        // Fixture: package.json with `npm run dev` -> python static server.
        let page = r#"<!doctype html><html><head><script>setTimeout(() => { throw new Error("real bug: boom"); }, 50);</script></head><body><button id="b1" onclick="document.title='clicked'">One</button><button onclick="undefinedFn()">Two</button></body></html>"#;
        fs::write(dir.join("index.html"), page).unwrap();
        fs::write(
            dir.join("package.json"),
            r#"{"name":"fixture","scripts":{"dev":"python3 -m http.server 8099"}}"#,
        )
        .unwrap();
        let project_path = dir.to_str().unwrap().to_string();
        let conv = daemon
            .create_conversation(&project_path, "DESIGN", "live e2e")
            .unwrap();

        // Kill any stale server on the fixture port first.
        let _ = std::process::Command::new("sh")
            .args(["-c", "lsof -ti :8099 | xargs kill -9 2>/dev/null || true"])
            .status();

        let result = daemon.e2e_run(conv.as_str());
        let _ = std::process::Command::new("sh")
            .args(["-c", "lsof -ti :8099 | xargs kill -9 2>/dev/null || true"])
            .status();
        let report = result.expect("e2e run completes");
        assert_eq!(report["status"], "ok", "run result: {report}");
        assert!(report["base_url"].as_str().unwrap().contains("8099"));

        // The fixture page throws a REAL console error on load, so the run
        // must report at least one bug and persist the report.
        let bugs = report["bug_count"].as_i64().unwrap();
        assert!(bugs >= 1, "real console error must be reported: {report}");
        assert!(report["report_id"].is_string());
        let persisted = daemon
            .db
            .e2e_report(report["report_id"].as_str().unwrap())
            .unwrap()
            .expect("report persisted");
        assert_eq!(persisted.bug_count, bugs);
        let _ = fs::remove_dir_all(dir);
    }
}
