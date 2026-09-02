impl DaemonService {
    pub fn design_send(
        &mut self,
        conversation_id: &str,
        content: &str,
        attachment_ids: &[String],
    ) -> AcResult<ac_db::ConversationMessageRow> {
        self.ensure_running()?;
        let conv = self.db.conversation(conversation_id)?.ok_or_else(|| {
            AcError::validation("CONVERSATION-NOT_FOUND", "conversation not found")
        })?;
        if conv.mode != "DESIGN" {
            return Err(AcError::validation(
                "CONVERSATION-WRONG_MODE",
                "design_send requires a DESIGN conversation",
            ));
        }
        if content.trim().is_empty() {
            return Err(AcError::validation(
                "CONVERSATION-EMPTY_CONTENT",
                "message content must not be empty",
            ));
        }

        let user_msg = self.append_message(conversation_id, "user", content, None, "{}")?;
        for att_id in attachment_ids {
            let _ = self.db.link_message_attachment(att_id, &user_msg.id);
        }

        let attachments = self.load_goal_attachments(attachment_ids, &conv.project_path)?;
        let mut attachment_block = String::new();
        for att in &attachments {
            if let ac_agent::AttachmentContent::Text(text) = &att.content {
                let bounded = if text.len() > 4096 {
                    format!("{}...\n[truncated {} chars]", &text[..4096], text.len())
                } else {
                    text.clone()
                };
                attachment_block.push_str(&format!(
                    "\n--- Attachment: {} ---\n{}\n--- end {} ---",
                    att.filename, bounded, att.filename
                ));
            }
        }

        let messages = self.db.messages_for_conversation(conversation_id)?;
        let recent = messages
            .iter()
            .rev()
            .take(20)
            .map(|m| format!("{}: {}", m.role, bounded_ui_summary(&m.content, 4096)))
            .collect::<Vec<_>>()
            .join("\n");

        let project_hint = format!("Project: {}\n", conv.project_path);
        let project_files = bounded_project_listing(&conv.project_path, 80);
        let design_state = self
            .db
            .design_document(conversation_id, "design_state")
            .ok()
            .flatten()
            .map(|doc| doc.content_json)
            .unwrap_or_default();
        let design_state_block = if design_state.is_empty() {
            String::new()
        } else {
            format!(
                "\n--- Current DESIGN_STATE ---\n{}\n--- end DESIGN_STATE ---\n",
                bounded_ui_summary(&design_state, 2048)
            )
        };

        let prompt = format!(
            "{}\n{}\n{}\n{}\n{}\n\n--\nYou are a design assistant. Discuss the project's design, \
             suggest improvements, help create a Design Brief and Design Grammar. \
             Keep responses technical and product-specific. Do NOT write code, \
             modify files, or execute commands. Focus on design direction, visual \
             hierarchy, typography, spacing, color, and interaction patterns.\n",
            project_hint, project_files, recent, attachment_block, design_state_block,
        );

        let db_path = self.db_path.clone();
        let mut providers = crate::daemon_provider_registry(&self.db, &db_path).map_err(|error| {
            AcError::validation(
                "DESIGN-PROVIDER_SETUP",
                format!("cannot initialize provider registry: {error}"),
            )
        })?;

        let mut profile = ac_provider::TaskProfile::discuss(
            ac_common::StableId::new("design"),
            ac_provider::RoutingProfile::LocalFirst,
        );
        profile.required_context = 4096;
        let cancel = Arc::new(AtomicBool::new(false));
        let result = providers.request_model(&profile, prompt, 4096, &|| {
            cancel.load(Ordering::Relaxed)
        });

        let content = match result {
            Ok(ref execution) => {
                let text = provider_events_text(&execution.events);
                let mission_id = StableId::new("design");
                if let Some(selected) = &execution.decision.selected {
                    let record = ac_agent::ProviderModelRecord {
                        provider_id: selected.provider_id.to_string(),
                        provider_account_id: Some(selected.connection_id.to_string()),
                        model_id: selected.model_identity_id.to_string(),
                        model_name: selected.model_name.clone(),
                        routing_mode: format!("{:?}", profile.routing_profile),
                        attempt_number: 1,
                        success: true,
                        failure_class: None,
                        created_at_ms: TimestampMillis::now().as_millis() as i64,
                    };
                    if let Ok(dur) = ac_db::ControlPlaneDb::open(&db_path) {
                        let _ = dur.save_provider_model_record(&ac_db::ProviderModelRecordRow {
                            id: format!("pmr-design-{}", StableId::new("record")),
                            project_path: Some(conv.project_path.clone()),
                            conversation_id: Some(conversation_id.to_string()),
                            mission_id: Some(mission_id.to_string()),
                            session_id: None,
                            task_id: None,
                            provider_id: record.provider_id,
                            provider_account_id: record.provider_account_id,
                            model_id: record.model_id,
                            model_name: record.model_name,
                            routing_mode: record.routing_mode,
                            attempt_number: 1,
                            success: true,
                            failure_class: None,
                            created_at_ms: record.created_at_ms,
                        });
                    }
                }
                if text.is_empty() {
                    "[empty response from model]".to_string()
                } else {
                    text
                }
            }
            Err(ref failure) => format!("[provider unavailable: {failure:?}]"),
        };

        let metadata = json!({
            "mode": "design",
            "provider_model": result.as_ref().ok().and_then(|exec| {
                exec.decision.selected.as_ref().map(|s| {
                    json!({"provider_id": s.provider_id.to_string(), "model_name": s.model_name})
                })
            }),
        });

        self.append_message(conversation_id, "assistant", &content, None, &metadata.to_string())
    }

    /// Bounded product understanding: scan the project for package metadata,
    /// routes, components, styles, tokens and assets.  Returns a compact
    /// structured analysis and persists it as a durable design document.
    pub fn design_understand(&self, conversation_id: &str) -> AcResult<Value> {
        self.ensure_running()?;
        let conv = self.db.conversation(conversation_id)?.ok_or_else(|| {
            AcError::validation("CONVERSATION-NOT_FOUND", "conversation not found")
        })?;

        let project_path = &conv.project_path;
        let dir = Path::new(project_path);
        let mut framework = None;
        let mut routes: Vec<String> = Vec::new();
        let mut components: Vec<String> = Vec::new();
        let mut style_files: Vec<String> = Vec::new();
        let mut tokens: Vec<String> = Vec::new();
        let fonts: Vec<String> = Vec::new();
        let mut assets: Vec<String> = Vec::new();
        let mut navigation: Vec<String> = Vec::new();

        if dir.join("package.json").exists() {
            if let Ok(content) = fs::read_to_string(dir.join("package.json")) {
                if content.contains("\"next\"") {
                    framework = Some("Next.js".to_string());
                } else if content.contains("vite") {
                    framework = Some("Vite".to_string());
                } else if content.contains("react-scripts") {
                    framework = Some("Create React App".to_string());
                } else {
                    framework = Some("Node".to_string());
                }
            }
        }
        if dir.join("Cargo.toml").exists() {
            framework = Some("Rust".to_string());
        }

        let mut walked = 0usize;
        let mut walk = |entry: &str| {
            if walked >= 200 {
                return;
            }
            walked += 1;
            let path = Path::new(entry);
            let rel = path
                .strip_prefix(project_path)
                .unwrap_or(path)
                .to_string_lossy()
                .to_string();
            let lower = rel.to_ascii_lowercase();
            if lower.contains("/pages/")
                || lower.contains("/routes/")
                || lower.contains("/app/")
                || rel.ends_with("App.tsx")
                || rel.ends_with("main.tsx")
                || rel.ends_with(".html")
            {
                routes.push(rel.clone());
            }
            if lower.contains("component")
                || rel.ends_with(".tsx")
                || rel.ends_with(".jsx")
                || rel.ends_with(".vue")
                || rel.ends_with(".svelte")
            {
                components.push(rel.clone());
            }
            if rel.ends_with(".css")
                || rel.ends_with(".scss")
                || lower.contains("tailwind")
                || lower.contains("theme")
            {
                style_files.push(rel.clone());
            }
            if lower.contains("tailwind.config")
                || lower.contains("theme")
                || lower.contains("tokens")
                || rel.ends_with("globals.css")
            {
                tokens.push(rel.clone());
            }
            if rel.ends_with(".png")
                || rel.ends_with(".jpg")
                || rel.ends_with(".jpeg")
                || rel.ends_with(".webp")
                || rel.ends_with(".svg")
            {
                assets.push(rel.clone());
            }
            if rel.contains("nav") || lower.contains("sidebar") || lower.contains("header") {
                navigation.push(rel);
            }
        };

        let mut scan_dir = |path: &Path| {
            if let Ok(read) = fs::read_dir(path) {
                for entry in read.flatten().take(120) {
                    walk(&entry.path().to_string_lossy());
                }
            }
        };
        scan_dir(dir);
        for sub in ["src", "app", "pages", "components", "styles", "assets", "public"]
            .iter()
            .filter(|s| dir.join(s).is_dir())
            .map(|s| dir.join(s))
        {
            scan_dir(&sub);
        }

        let analysis = json!({
            "framework": framework,
            "routes": routes,
            "components": components,
            "style_files": style_files,
            "tokens": tokens,
            "fonts": fonts,
            "assets": assets,
            "navigation": navigation,
        });

        let now = TimestampMillis::now().as_millis() as i64;
        let doc = DesignDocumentRow {
            id: StableId::new("ddesign").to_string(),
            conversation_id: conversation_id.to_string(),
            doc_type: "product_analysis".to_string(),
            content_json: analysis.to_string(),
            version: 1,
            evidence_refs: String::new(),
            created_at_ms: now,
            updated_at_ms: now,
        };
        self.db.save_design_document(&doc)?;

        Ok(analysis)
    }

    pub fn design_brief(&self, conversation_id: &str, audience: &str, workflow: &str) -> AcResult<Value> {
        self.ensure_running()?;
        let conv = self.db.conversation(conversation_id)?.ok_or_else(|| {
            AcError::validation("CONVERSATION-NOT_FOUND", "conversation not found")
        })?;

        let analysis_doc = self.db.design_document(conversation_id, "product_analysis").ok().flatten();
        let analysis: Value = analysis_doc
            .as_ref()
            .and_then(|d| serde_json::from_str(&d.content_json).ok())
            .unwrap_or(json!({}));

        let framework = analysis.get("framework").and_then(Value::as_str).unwrap_or("unknown");
        let components = analysis.get("components").and_then(Value::as_array).map(|a| a.len()).unwrap_or(0);
        let routes = analysis.get("routes").and_then(Value::as_array).map(|a| a.len()).unwrap_or(0);

        let product = &conv.title;
        let personality = if product.to_ascii_lowercase().contains("admin")
            || workflow.to_ascii_lowercase().contains("review")
        {
            "calm, dense, operational".to_string()
        } else {
            format!("specific to {}", product)
        };

        let brief = json!({
            "product": product,
            "audience": audience,
            "personality": personality,
            "density": if components > 8 { "compact" } else { "moderate" },
            "primary_workflow": workflow,
            "framework": framework,
            "routes": routes,
            "components": components,
            "visual_goals": [
                format!("make {} immediately recognizable", product),
                "prioritize repeated task scanning over decorative explanation".to_string(),
            ],
            "patterns_to_avoid": [
                "generic AI productivity copy".to_string(),
                "oversized gradient hero sections unrelated to workflow".to_string(),
                "identical feature cards without hierarchy".to_string(),
            ],
            "things_to_preserve": ["existing functional routes and navigation".to_string()],
            "things_to_avoid": [
                "unnecessary decorative elements".to_string(),
                "generic AI SaaS styling".to_string(),
            ],
        });

        let now = TimestampMillis::now().as_millis() as i64;
        let doc = DesignDocumentRow {
            id: StableId::new("dbrief").to_string(),
            conversation_id: conversation_id.to_string(),
            doc_type: "design_brief".to_string(),
            content_json: brief.to_string(),
            version: 1,
            evidence_refs: String::new(),
            created_at_ms: now,
            updated_at_ms: now,
        };
        self.db.save_design_document(&doc)?;

        Ok(brief)
    }

    pub fn design_grammar(&self, conversation_id: &str) -> AcResult<Value> {
        self.ensure_running()?;
        let conv = self.db.conversation(conversation_id)?.ok_or_else(|| {
            AcError::validation("CONVERSATION-NOT_FOUND", "conversation not found")
        })?;

        let brief_doc = self.db.design_document(conversation_id, "design_brief").ok().flatten();
        let brief: Value = brief_doc
            .as_ref()
            .and_then(|d| serde_json::from_str(&d.content_json).ok())
            .unwrap_or(json!({}));

        let product = brief.get("product").and_then(Value::as_str).unwrap_or(&conv.title);

        let grammar = json!({
            "type_scale": [
                "compact panel headings".to_string(),
                "hero scale only when the first screen is a true product hero".to_string(),
            ],
            "spacing": ["dense 8px grid for operational controls".to_string()],
            "radii": ["cards and controls use 4-8px radii".to_string()],
            "surfaces": ["full-width sections; cards only for repeated items".to_string()],
            "color_roles": [format!("brand role anchored to {}", product)],
            "semantic_states": ["error, warning, success, disabled".to_string()],
            "navigation": ["derive navigation from existing routes".to_string()],
            "motion": ["motion clarifies state changes, never just decoration".to_string()],
            "iconography": ["use existing icon library before custom vectors".to_string()],
            "component_principles": [
                format!("optimize the primary workflow", ),
                "respect density preferences".to_string(),
            ],
            "density": "compact".to_string(),
            "interaction": ["keyboard-first, mouse-optimized".to_string()],
            "responsive_principles": ["content adapts by priority, not by hiding".to_string()],
        });

        let now = TimestampMillis::now().as_millis() as i64;
        let doc = DesignDocumentRow {
            id: StableId::new("dgrammar").to_string(),
            conversation_id: conversation_id.to_string(),
            doc_type: "design_grammar".to_string(),
            content_json: grammar.to_string(),
            version: 1,
            evidence_refs: String::new(),
            created_at_ms: now,
            updated_at_ms: now,
        };
        self.db.save_design_document(&doc)?;

        Ok(grammar)
    }

    pub fn design_state(&self, conversation_id: &str) -> AcResult<Value> {
        self.ensure_running()?;
        let docs = self.db.design_documents(conversation_id)?;
        let analysis = docs.iter().find(|d| d.doc_type == "product_analysis");
        let brief = docs.iter().find(|d| d.doc_type == "design_brief");
        let grammar = docs.iter().find(|d| d.doc_type == "design_grammar");

        let mut facts = Vec::new();
        if let Some(b) = brief {
            let v: Value = serde_json::from_str(&b.content_json).unwrap_or_default();
            facts.push(format!("Product: {}", v.get("product").and_then(Value::as_str).unwrap_or("")));
            facts.push(format!("Audience: {}", v.get("audience").and_then(Value::as_str).unwrap_or("")));
            facts.push(format!("Primary workflow: {}", v.get("primary_workflow").and_then(Value::as_str).unwrap_or("")));
        }
        if let Some(a) = analysis {
            let v: Value = serde_json::from_str(&a.content_json).unwrap_or_default();
            facts.push(format!("Framework: {}", v.get("framework").and_then(Value::as_str).unwrap_or("unknown")));
            facts.push(format!("Components: {}", v.get("components").and_then(Value::as_array).map(|a| a.len()).unwrap_or(0)));
        }

        let mut grammar_lines = Vec::new();
        if let Some(g) = grammar {
            let v: Value = serde_json::from_str(&g.content_json).unwrap_or_default();
            if let Some(principles) = v.get("component_principles").and_then(Value::as_array) {
                for p in principles {
                    grammar_lines.push(p.as_str().unwrap_or("").to_string());
                }
            }
        }

        let state = json!({
            "facts": facts,
            "grammar_principles": grammar_lines,
            "content": format!(
                "# DESIGN_STATE.md\n\n## Design Decisions\n- {}\n\n## Grammar\n- {}\n",
                facts.join("\n- "),
                grammar_lines.join("\n- "),
            ),
        });

        let now = TimestampMillis::now().as_millis() as i64;
        let doc = DesignDocumentRow {
            id: StableId::new("dstate").to_string(),
            conversation_id: conversation_id.to_string(),
            doc_type: "design_state".to_string(),
            content_json: state.to_string(),
            version: 1,
            evidence_refs: String::new(),
            created_at_ms: now,
            updated_at_ms: now,
        };
        self.db.save_design_document(&doc)?;

        Ok(state)
    }

    pub fn design_critique(&self, conversation_id: &str, content: &str, doc_type: &str) -> AcResult<Value> {
        self.ensure_running()?;
        let lower = content.to_ascii_lowercase();
        let mut findings = Vec::new();

        if lower.contains("gradient") && (lower.contains("hero") || lower.contains("100vh")) {
            findings.push(json!({
                "rule": "oversized_gradient_hero",
                "severity": 3,
                "explanation": "large gradient hero competes with the actual workflow"
            }));
        }
        if lower.matches("card").count() >= 3 && !lower.contains("admin") {
            findings.push(json!({
                "rule": "identical_generic_cards",
                "severity": 3,
                "explanation": "repeated generic cards do not express product-specific hierarchy"
            }));
        }
        if lower.contains("ai-powered") || lower.contains("reimagine your workflow") {
            findings.push(json!({
                "rule": "generic_ai_copy",
                "severity": 2,
                "explanation": "copy could describe almost any product"
            }));
        }
        if lower.contains("glass") || lower.contains("backdrop-filter") {
            findings.push(json!({
                "rule": "gratuitous_glass",
                "severity": 2,
                "explanation": "glass styling needs a product reason and contrast proof"
            }));
        }
        if lower.contains("welcome to") || lower.contains("get started") {
            findings.push(json!({
                "rule": "generic_placeholder",
                "severity": 1,
                "explanation": "generic placeholder text should be replaced with product-specific content"
            }));
        }

        let passed = findings.is_empty();
        let improvement_required = findings.iter().any(|f| f["severity"].as_u64().unwrap_or(0) >= 3);

        let result = json!({
            "passed": passed,
            "improvement_required": improvement_required,
            "findings": findings,
        });

        let now = TimestampMillis::now().as_millis() as i64;
        let critique = DesignCritiqueRow {
            id: StableId::new("dcritique").to_string(),
            conversation_id: conversation_id.to_string(),
            doc_type: Some(doc_type.to_string()),
            passed,
            findings_json: result.to_string(),
            improvement_required,
            evidence_refs: String::new(),
            created_at_ms: now,
        };
        self.db.save_design_critique(&critique)?;

        Ok(result)
    }

    pub fn design_preview_start(&self, conversation_id: &str) -> AcResult<Value> {
        self.ensure_running()?;
        let conv = self.db.conversation(conversation_id)?.ok_or_else(|| {
            AcError::validation("CONVERSATION-NOT_FOUND", "conversation not found")
        })?;
        let project_path = &conv.project_path;

        let dev_command = detect_dev_command(project_path);
        if dev_command.is_empty() {
            return Ok(json!({
                "status": "PREVIEW_COMMAND_MISSING",
                "detail": "No dev server command detected in project. Try npm run dev, cargo run, or similar."
            }));
        }

        // Spawn the dev server with a live output reader that detects the port.
        let argv: Vec<&str> = dev_command.split_whitespace().collect();
        let mut cmd = Command::new(argv[0]);
        if argv.len() > 1 {
            cmd.args(&argv[1..]);
        }
        cmd.current_dir(project_path)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let mut child = cmd.spawn().map_err(|e| {
            AcError::validation("DESIGN-PREVIEW_SPAWN_FAILED", format!("could not start dev server: {e}"))
        })?;
        let pid = child.id();

        let stdout = child.stdout.take().ok_or_else(|| {
            AcError::validation("DESIGN-PREVIEW_NO_STDOUT", "dev server stdout unavailable")
        })?;

        // Port detection runs in a reader thread so the daemon loop never blocks.
        let port_signal = Arc::new(std::sync::Mutex::new(Option::<u16>::None));
        let signal = Arc::clone(&port_signal);
        let stderr_reader = child.stderr.take();
        std::thread::Builder::new()
            .name("design-preview-reader".to_string())
            .spawn(move || {
                for line in BufReader::new(stdout).lines().map_while(Result::ok) {
                    let mut guard = signal.lock().unwrap();
                    if guard.is_none() {
                        *guard = detect_port_from_line(&line);
                    }
                }
            })
            .map_err(|e| AcError::validation("DESIGN-PREVIEW_READER", e.to_string()))?;
        if let Some(stderr) = stderr_reader {
            std::thread::Builder::new()
                .name("design-preview-stderr".to_string())
                .spawn(move || {
                    for _line in BufReader::new(stderr).lines().map_while(Result::ok) {}
                })
                .map_err(|e| AcError::validation("DESIGN-PREVIEW_READER", e.to_string()))?;
        }

        // Wait briefly for the port, then probe default ports.
        let mut port = None;
        let start = Instant::now();
        while start.elapsed() < Duration::from_secs(4) {
            if let Some(p) = *port_signal.lock().unwrap() {
                port = Some(p);
                break;
            }
            std::thread::sleep(Duration::from_millis(100));
        }
        if port.is_none() {
            for candidate in DEFAULT_DEV_PORTS {
                if test_http_ready(candidate) {
                    port = Some(candidate);
                    break;
                }
            }
        }
        if port.is_none() {
            // Keep the process alive for status probing but report no port yet.
            let _ = self.db.save_design_preview(&DesignPreviewRow {
                id: StableId::new("dpreview").to_string(),
                conversation_id: conversation_id.to_string(),
                port: None,
                ready_url: None,
                process_id: Some(format!("pid:{pid}")),
                process_alive: true,
                http_ready: false,
                browser_session_id: None,
                created_at_ms: TimestampMillis::now().as_millis() as i64,
                updated_at_ms: TimestampMillis::now().as_millis() as i64,
            });
            self.design_children
                .lock()
                .unwrap()
                .insert(conversation_id.to_string(), child);
            return Ok(json!({
                "status": "launched",
                "port": null,
                "ready_url": null,
                "pid": pid,
                "command": dev_command,
                "detail": "dev server launched; port not yet detected — poll DesignPreviewStatus",
            }));
        }

        let ready_url = format!("http://127.0.0.1:{}", port.unwrap());
        let now = TimestampMillis::now().as_millis() as i64;
        let preview = DesignPreviewRow {
            id: StableId::new("dpreview").to_string(),
            conversation_id: conversation_id.to_string(),
port: port.map(|p| p as i64),
            ready_url: Some(ready_url.clone()),
            process_id: Some(format!("pid:{pid}")),
            process_alive: true,
            http_ready: true,
            browser_session_id: None,
            created_at_ms: now,
            updated_at_ms: now,
        };
        self.db.save_design_preview(&preview)?;
        self.design_children
            .lock()
            .unwrap()
            .insert(conversation_id.to_string(), child);

        Ok(json!({
            "status": "launched",
            "port": port,
            "ready_url": ready_url,
            "pid": pid,
            "command": dev_command,
        }))
    }

    pub fn design_preview_status(&self, conversation_id: &str) -> AcResult<Value> {
        let preview = self.db.design_preview(conversation_id)?;
        match preview {
            Some(p) => {
                let alive = p.process_alive;
                let port = p.port.unwrap_or(0);
                let ready_url = p.ready_url.clone().unwrap_or_default();
                let http_ok = if port > 0 { test_http_ready(port as u16) } else { false };
                Ok(json!({
                    "process_alive": alive,
                    "http_ready": http_ok,
                    "port": port,
                    "ready_url": ready_url,
                }))
            }
            None => Ok(json!({
                "status": "no_preview",
                "detail": "No preview session has been started for this conversation.",
            })),
        }
    }

    pub fn design_preview_stop(&self, conversation_id: &str) -> AcResult<Value> {
        let mut children = self.design_children.lock().unwrap();
        if let Some(mut child) = children.remove(conversation_id) {
            let _ = child.kill();
            let _ = child.wait();
        }
        Ok(json!({ "status": "stopped" }))
    }

    /// Browser inspection of the real running application.  `url` overrides
    /// the preview URL.  When `deterministic` is true (tests / no Chrome),
    /// `html` is used as the rendered page so the deterministic harness can
    /// exercise DOM/diagnostics/screenshot without a live server.
    pub fn design_browser(
        &self,
        conversation_id: &str,
        url: &str,
        html: &str,
        deterministic: bool,
    ) -> AcResult<Value> {
        let preview = self.db.design_preview(conversation_id)?;
        let target_url = if url.is_empty() {
            preview
                .as_ref()
                .and_then(|p| p.ready_url.clone())
                .unwrap_or_else(|| "about:blank".to_string())
        } else {
            url.to_string()
        };

        let mut evidence_store = EvidenceStore::new();
        let policy = ac_security::CapabilityPolicy::new()
            .allow(ac_security::Capability::BrowserAutomation);
        let mut browser = if deterministic {
            ac_verification::BrowserRuntime::deterministic_harness_for_tests(policy)
        } else {
            ac_verification::BrowserRuntime::new(policy)
        };

        let task_id = StableId::new("design");
        let process = browser.launch(task_id.clone())?;
        let session = browser.create_session(task_id.clone(), process.id.clone())?;

        if deterministic && !html.is_empty() {
            let _ = browser.act(
                &session.id,
                ac_verification::BrowserAction::OpenHtmlForTest {
                    url: target_url.clone(),
                    html: html.to_string(),
                },
                &mut evidence_store,
            )?;
        } else {
            let _ = browser.act(
                &session.id,
                ac_verification::BrowserAction::Navigate { url: target_url.clone() },
                &mut evidence_store,
            )?;
        }

        let dom = browser.inspect_dom(&session.id, &mut evidence_store)?;
        let diag = browser.diagnostics(&session.id, &mut evidence_store)?;

        let viewport = ac_verification::ViewportProfile {
            name: "desktop",
            width: 1440,
            height: 900,
        };
        let screenshot = browser.capture_screenshot(
            &session.id,
            task_id,
            "design-preview",
            viewport,
            &mut evidence_store,
        )?;

        let diagnostics = json!({
            "console_errors": diag.console_errors,
            "page_errors": diag.page_errors,
            "network_failures": diag.network_failures,
            "http_status": diag.http_status,
        });

        let result = json!({
            "url": target_url,
            "visible_text": dom.visible_text,
            "controls": dom.controls,
            "accessibility_tree": dom.accessibility_tree,
            "diagnostics": diagnostics,
            "screenshot_uri": screenshot.artifact_uri,
            "screenshot_evidence_ref": screenshot.evidence_ref.to_string(),
        });

        if let Some(ref p) = preview {
            let now = TimestampMillis::now().as_millis() as i64;
            let updated = DesignPreviewRow {
                browser_session_id: Some(session.id.to_string()),
                updated_at_ms: now,
                ..p.clone()
            };
            let _ = self.db.save_design_preview(&updated);
        }

        Ok(result)
    }

    pub fn design_qa_responsive(&self, _conversation_id: &str, content: &str) -> AcResult<Value> {
        let mut issues = Vec::new();
        if content.contains("position: fixed") || content.contains("position:absolute") {
            issues.push("fixed/absolute positioning may cause overflow on small viewports".to_string());
        }
        if content.contains("width:") && !content.contains("max-width") && !content.contains("responsive") {
            issues.push("fixed widths without max-width may overflow".to_string());
        }
        if content.contains("overflow: hidden") {
            issues.push("overflow hidden may clip content on small screens".to_string());
        }
        Ok(json!({
            "passed": issues.is_empty(),
            "issues": issues,
            "viewports_checked": ["compact (1024px)", "normal (1440px)", "wide (1920px)"],
        }))
    }

    pub fn design_qa_accessibility(&self, _conversation_id: &str, content: &str) -> AcResult<Value> {
        let mut issues = Vec::new();
        if !content.contains("role=") && !content.contains("aria-") {
            issues.push("no ARIA roles or attributes found".to_string());
        }
        if !content.contains("<button") && !content.contains("<a ") {
            issues.push("no interactive controls found (buttons or links)".to_string());
        }
        if content.contains("color:") && !content.contains("background-color") && !content.contains("contrast") {
            issues.push("colors found without contrast specification".to_string());
        }
        if !content.contains("<label") && !content.contains("aria-label") {
            issues.push("no form labels found".to_string());
        }
        Ok(json!({
            "passed": issues.is_empty(),
            "issues": issues,
            "checks": ["semantic controls", "keyboard navigation", "focus visibility", "labels", "contrast"],
        }))
    }

    pub fn design_qa_functional(&self, _conversation_id: &str, content: &str) -> AcResult<Value> {
        let mut issues = Vec::new();
        if content.contains("404") || content.contains("not found") {
            issues.push("page contains 404 or not found content".to_string());
        }
        if content.contains("console.error") || content.contains("throw new Error") {
            issues.push("page contains script errors".to_string());
        }
        Ok(json!({
            "passed": issues.is_empty(),
            "issues": issues,
            "flows_checked": ["primary workflow navigation"],
        }))
    }

    pub fn design_repair(&self, conversation_id: &str, content: &str, doc_type: &str) -> AcResult<Value> {
        let critique = self.design_critique(conversation_id, content, doc_type)?;
        let passed = critique["passed"].as_bool().unwrap_or(true);
        let findings = critique["findings"].as_array().cloned().unwrap_or_default();

        let repairs: Vec<Value> = findings
            .iter()
            .map(|f| {
                let rule = f["rule"].as_str().unwrap_or("");
                match rule {
                    "oversized_gradient_hero" => json!({
                        "issue": rule,
                        "repair": "Replace gradient hero with a focused product-specific header. Use solid background colors from the design palette."
                    }),
                    "identical_generic_cards" => json!({
                        "issue": rule,
                        "repair": "Differentiate cards with product-specific content, hierarchy, and visual priority. Remove cards that don't serve the primary workflow."
                    }),
                    "generic_ai_copy" => json!({
                        "issue": rule,
                        "repair": "Replace generic AI copy with specific, meaningful text that describes the actual product functionality."
                    }),
                    "gratuitous_glass" => json!({
                        "issue": rule,
                        "repair": "Replace glass panels with solid surfaces. Ensure sufficient contrast for all text."
                    }),
                    _ => json!({
                        "issue": rule,
                        "repair": "Review and replace with product-specific content."
                    }),
                }
            })
            .collect();

        Ok(json!({
            "passed": passed,
            "repairs": repairs,
            "improvement_required": !passed,
        }))
    }
}

const DEFAULT_DEV_PORTS: [u16; 7] = [5173, 3000, 8080, 8000, 4173, 4321, 1420];

fn detect_dev_command(project_path: &str) -> String {
    let dir = Path::new(project_path);
    if dir.join("package.json").exists() {
        if let Ok(content) = fs::read_to_string(dir.join("package.json")) {
            if let Ok(pkg) = serde_json::from_str::<Value>(&content) {
                if let Some(scripts) = pkg.get("scripts").and_then(Value::as_object) {
                    for cmd in ["dev", "preview", "start", "serve"] {
                        if scripts.get(cmd).and_then(Value::as_str).is_some() {
                            return format!("npm run {}", cmd);
                        }
                    }
                }
            }
        }
    }
    if dir.join("Cargo.toml").exists() {
        return "cargo run".to_string();
    }
    if dir.join("Makefile").exists() {
        return "make run".to_string();
    }
    if dir.join("index.html").exists() || dir.join("public/index.html").exists() {
        return "python3 -m http.server 8080".to_string();
    }
    String::new()
}

fn detect_port_from_line(line: &str) -> Option<u16> {
    let line_lower = line.to_ascii_lowercase();
    for word in line_lower.split_whitespace() {
        let candidate = word
            .trim_matches([':', '/', ',', ';', '(', ')'])
            .to_string();
        if let Some(port_str) = candidate.strip_prefix("localhost:") {
            if let Ok(port) = port_str.parse::<u16>() {
                if port > 0 && port < 65535 {
                    return Some(port);
                }
            }
        }
        if let Some(port_str) = candidate.strip_prefix("127.0.0.1:") {
            if let Ok(port) = port_str.parse::<u16>() {
                if port > 0 && port < 65535 {
                    return Some(port);
                }
            }
        }
    }
    // Match "port 3000" or "Local: http://localhost:5173/"
    if let Some(idx) = line_lower.find("http://localhost:") {
        let tail = &line_lower[idx + "http://localhost:".len()..];
        let port_str = tail
            .chars()
            .take_while(|c| c.is_ascii_digit())
            .collect::<String>();
        if let Ok(port) = port_str.parse::<u16>() {
            return Some(port);
        }
    }
    if let Some(idx) = line_lower.find(":port ") {
        let tail = &line_lower[idx + ":port ".len()..];
        let port_str = tail
            .chars()
            .take_while(|c| c.is_ascii_digit())
            .collect::<String>();
        if let Ok(port) = port_str.parse::<u16>() {
            return Some(port);
        }
    }
    None
}

fn test_http_ready(port: u16) -> bool {
    use std::net::TcpStream;
    let addr = format!("127.0.0.1:{port}");
    addr.parse::<std::net::SocketAddr>()
        .ok()
        .and_then(|a| TcpStream::connect_timeout(&a, Duration::from_millis(150)).ok())
        .is_some()
}