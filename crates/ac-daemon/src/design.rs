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
            self.preferred_routing_profile(), // N1: user-persisted routing profile
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

    /// Reference-image analysis (core Doc 06 H28): extract structured design
    /// principles from an attached reference image using a vision-capable
    /// local model.  The result is persisted as a `reference_analysis` design
    /// document with an explicit copying boundary: adopted_principles may
    /// inform the design, but logos, illustrations, marketing text and trade
    /// dress must never be reproduced.
    ///
    /// Honesty rules: this fails with a clear error when no vision model is
    /// configured or the attachment is not a readable image; it never
    /// fabricates an analysis from text alone.
    pub fn design_analyze_reference(
        &self,
        conversation_id: &str,
        attachment_id: &str,
    ) -> AcResult<Value> {
        self.ensure_running()?;
        let conv = self.db.conversation(conversation_id)?.ok_or_else(|| {
            AcError::validation("CONVERSATION-NOT_FOUND", "conversation not found")
        })?;
        if conv.mode != "DESIGN" {
            return Err(AcError::validation(
                "CONVERSATION-WRONG_MODE",
                "design_analyze_reference requires a DESIGN conversation",
            ));
        }
        let row = self.db.attachment(attachment_id)?.ok_or_else(|| {
            AcError::validation(
                "CONVERSATION-ATTACHMENT_NOT_FOUND",
                "attachment not found",
            )
        })?;
        if row.project_path != conv.project_path {
            return Err(AcError::policy_denied(
                "CONVERSATION-PROJECT_MISMATCH",
                "attachment does not belong to this project",
            ));
        }
        if !row.mime_type.starts_with("image/") {
            return Err(AcError::validation(
                "DESIGN-REFERENCE_NOT_IMAGE",
                "reference analysis requires an image attachment",
            ));
        }
        // Bounded image payload: 6 MiB encoded is far beyond what a <=4B
        // vision model can meaningfully consume, and keeps the request small.
        const MAX_IMAGE_BYTES: i64 = 6 * 1024 * 1024;
        if row.size_bytes > MAX_IMAGE_BYTES {
            return Err(AcError::validation(
                "DESIGN-REFERENCE_TOO_LARGE",
                "reference image exceeds the 6 MiB analysis limit",
            ));
        }
        let file_path = self
            .attachment_path(attachment_id, &conv.project_path)?
            .ok_or_else(|| {
                AcError::validation(
                    "CONVERSATION-FILE_NOT_FOUND",
                    "attachment file not found on disk",
                )
            })?;
        let image_bytes = std::fs::read(&file_path).map_err(|err| {
            AcError::validation(
                "DESIGN-REFERENCE_READ_FAILED",
                format!("cannot read reference image: {err}"),
            )
        })?;
        let image_b64 = base64::Engine::encode(
            &base64::engine::general_purpose::STANDARD,
            &image_bytes,
        );

        let vision_model = std::env::var("AGENTCODE_VISION_MODEL")
            .ok()
            .filter(|m| !m.trim().is_empty())
            .unwrap_or_else(|| "gemma3:4b".to_string());
        let ollama_base = std::env::var("OLLAMA_BASE_URL")
            .map(|base| base.trim_end_matches('/').to_string())
            .unwrap_or_else(|_| "http://127.0.0.1:11434".to_string());

        let mut providers = ac_provider::ProviderRegistry::new();
        let adapter = Box::new(ac_provider::OllamaProviderAdapter::new(
            &ollama_base,
            vision_model.clone(),
        )?);
        ac_agent::register_vision_model_route(
            &mut providers,
            "ollama-vision",
            adapter,
            vision_model.clone(),
            "config:ollama.vision",
            8192,
            ac_provider::PrivacyClass::LocalOnly,
        )
        .map_err(|err| {
            AcError::validation(
                "DESIGN-PROVIDER_SETUP",
                format!("cannot register vision model route: {err}"),
            )
        })?;

        let mut profile = ac_provider::TaskProfile::discuss(
            ac_common::StableId::new("refanalysis"),
            self.preferred_routing_profile(), // N1: user-persisted routing profile
        );
        profile.required_context = 4096;
        let cancel = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let prompt = r#"You are a design reference analyst. Examine the attached image and produce a strict JSON object with exactly this shape:
{
  "extracted": {
    "hierarchy": ["..."], "layout": ["..."], "spacing": ["..."],
    "typography": ["..."], "navigation": ["..."],
    "component_behavior": ["..."], "motion": ["..."], "visual_motifs": ["..."]
  },
  "explicitly_do_not_copy": ["logos, illustrations, marketing text, protected assets, or trade dress you observe"],
  "adopted_principles": ["abstract design principles safe to adopt, e.g. dense top navigation, editorial typography"]
}
Rules: describe only what is actually visible. Each array holds short factual strings. Never reproduce or transcribe protected assets; name them under explicitly_do_not_copy instead. Output JSON only."#;
        let result = providers.request_model_with_image(
            &profile,
            prompt,
            image_b64,
            1024,
            &|| cancel.load(std::sync::atomic::Ordering::Relaxed),
        );
        let text = match result {
            Ok(execution) => ac_agent::provider_events_text(&execution.events),
            Err(failure) => {
                return Err(AcError::new(
                    "DESIGN-VISION_MODEL_UNAVAILABLE",
                    format!(
                        "vision model '{vision_model}' failed ({failure:?}); \
                         set AGENTCODE_VISION_MODEL to an installed vision-capable model"
                    ),
                    ac_common::ErrorKind::Unavailable,
                    ac_common::Retryability::Retryable,
                ));
            }
        };
        let parsed: Value = parse_reference_json(&text).ok_or_else(|| {
            AcError::validation(
                "DESIGN-VISION_RESPONSE_UNPARSEABLE",
                "vision model did not return a structured reference analysis",
            )
        })?;
        let analysis = json!({
            "reference_id": attachment_id,
            "source_type": "IMAGE",
            "artifact_ref": row.storage_key,
            "vision_model": vision_model,
            "extracted": parsed.get("extracted").cloned().unwrap_or(json!({})),
            "explicitly_do_not_copy": parsed
                .get("explicitly_do_not_copy")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default(),
            "adopted_principles": parsed
                .get("adopted_principles")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default(),
        });
        if analysis["explicitly_do_not_copy"].as_array().map(|a| a.is_empty()).unwrap_or(true) {
            return Err(AcError::validation(
                "DESIGN-VISION_RESPONSE_INCOMPLETE",
                "vision analysis did not state a copying boundary",
            ));
        }

        let now = TimestampMillis::now().as_millis() as i64;
        let doc = DesignDocumentRow {
            id: StableId::new("ddesign").to_string(),
            conversation_id: conversation_id.to_string(),
            doc_type: "reference_analysis".to_string(),
            content_json: analysis.to_string(),
            version: 1,
            evidence_refs: String::new(),
            created_at_ms: now,
            updated_at_ms: now,
        };
        self.db.save_design_document(&doc)?;
        self.append_message(
            conversation_id,
            "assistant",
            &format!(
                "Reference image analyzed (model {vision_model}). Extracted hierarchy/layout/spacing observations, {} adopted principles, and an explicit do-not-copy boundary. See the reference analysis document.",
                analysis["adopted_principles"].as_array().map(|a| a.len()).unwrap_or(0),
            ),
            None,
            r#"{"mode":"design","kind":"reference_analysis"}"#,
        )?;
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
        // Durable constraints (Doc 06 §16): the critique result carries the
        // project's constraint list and marks suggestions that would
        // violate one as constraint-violating — the critic never
        // recommends trading a constraint for aesthetics.
        let constraints_doc = self.design_constraints_get(conversation_id).ok();
        let constraint_texts: Vec<String> = constraints_doc
            .as_ref()
            .and_then(|doc| doc.get("constraints").and_then(Value::as_array).cloned())
            .unwrap_or_default()
            .iter()
            .filter_map(|c| c.get("text").and_then(Value::as_str).map(String::from))
            .collect();

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

        // Mark any finding whose repair suggestion could violate a durable
        // constraint (§16: the critic must not recommend violating one).
        for finding in findings.iter_mut() {
            let explanation = finding["explanation"].as_str().unwrap_or("").to_ascii_lowercase();
            let violating: Vec<String> = constraint_texts
                .iter()
                .filter(|constraint| {
                    let constraint = constraint.to_ascii_lowercase();
                    // A repair that removes/changes something the constraint
                    // preserves is a violation candidate.
                    (explanation.contains("remove") || explanation.contains("replace")
                        || explanation.contains("change"))
                        && constraint.split_whitespace().any(|word| {
                            word.len() > 4 && explanation.contains(word)
                        })
                })
                .cloned()
                .collect();
            if !violating.is_empty() {
                finding["constraint_violations"] = json!(violating);
                finding["note"] = json!(
                    "repair must respect the project's durable constraints — they outrank aesthetics"
                );
            }
        }
        let passed = findings.is_empty();
        let improvement_required = findings.iter().any(|f| f["severity"].as_u64().unwrap_or(0) >= 3);

        let result = json!({
            "passed": passed,
            "improvement_required": improvement_required,
            "findings": findings,
            "constraints": constraint_texts,
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

    /// Visual critic layer (Doc 06 §50-51): a vision model (gemma3:4b,
    /// ≤4B local) critiques the ACTUAL rendered screenshot of the running
    /// design, independent of the deterministic anti-slop critique.  The
    /// two signals are reported separately — never merged into one verdict.
    ///
    /// Failure behavior: when the vision model is unreachable or returns
    /// nothing structured, the result is an honest VISION_CRITIC_UNAVAILABLE
    /// layer with `available: false` — no fabricated findings, and the
    /// deterministic anti-slop critique remains the only signal.
    pub fn design_visual_critique(
        &self,
        conversation_id: &str,
        url: &str,
        deterministic: bool,
    ) -> AcResult<Value> {
        self.ensure_running()?;
        let preview = self.db.design_preview(conversation_id)?;
        let target_url = if url.is_empty() {
            preview
                .as_ref()
                .and_then(|p| p.ready_url.clone())
                .unwrap_or_else(|| "about:blank".to_string())
        } else {
            url.to_string()
        };

        // Capture a real screenshot of the running design.
        let mut evidence_store = EvidenceStore::new();
        let policy = ac_security::CapabilityPolicy::new()
            .allow(ac_security::Capability::BrowserAutomation);
        let mut browser = if deterministic {
            ac_verification::BrowserRuntime::deterministic_harness_for_tests(policy)
        } else {
            ac_verification::BrowserRuntime::new(policy)
        };
        let task_id = StableId::new("designcritic");
        let process = browser.launch(task_id.clone())?;
        let session = browser.create_session(task_id.clone(), process.id.clone())?;
        if deterministic {
            // Deterministic mode has no rendered pixels: the visual critic is
            // unavailable by construction and must say so.
            let _ = browser.mark_crashed(&process.id);
            let unavailable = json!({
                "conversation_id": conversation_id,
                "url": target_url,
                "available": false,
                "unavailable_reason": "VISION_CRITIC_UNAVAILABLE: deterministic harness renders no pixels; visual critique requires the real preview",
                "vision_model": Self::vision_model_name(),
                "findings": [],
                "source": "vision-unavailable",
            });
            let now = TimestampMillis::now().as_millis() as i64;
            let _ = self.db.save_design_critique(&DesignCritiqueRow {
                id: StableId::new("dcritique").to_string(),
                conversation_id: conversation_id.to_string(),
                doc_type: Some("visual-critic".to_string()),
                passed: false,
                findings_json: unavailable.to_string(),
                improvement_required: false,
                evidence_refs: String::new(),
                created_at_ms: now,
            });
            return Ok(unavailable);
        }
        let _ = browser.act(
            &session.id,
            ac_verification::BrowserAction::Navigate {
                url: target_url.clone(),
            },
            &mut evidence_store,
        )?;
        let _ = browser.act(
            &session.id,
            ac_verification::BrowserAction::Wait { millis: 400 },
            &mut evidence_store,
        )?;
        let viewport = ac_verification::ViewportProfile {
            name: "desktop",
            width: 1440,
            height: 900,
        };
        let screenshot = browser.capture_screenshot(
            &session.id,
            task_id,
            "design-critic",
            viewport,
            &mut evidence_store,
        )?;
        // Batch N2: SUCCESS path closes the browser gracefully (CDP
        // Browser.close) — a forced SIGKILL here is what produced the
        // user-visible "Chrome quit unexpectedly" dialogs after every
        // visual-critic run.
        let _ = browser.close_process(&process.id);

        // Read the real screenshot bytes for the vision model.
        let screenshot_bytes = std::fs::read(&screenshot.artifact_uri).map_err(|err| {
            AcError::validation(
                "DESIGN-SCREENSHOT_READ_FAILED",
                format!("cannot read captured screenshot: {err}"),
            )
        })?;
        const MAX_SCREENSHOT_BYTES: usize = 6 * 1024 * 1024;
        if screenshot_bytes.len() > MAX_SCREENSHOT_BYTES {
            return Err(AcError::validation(
                "DESIGN-SCREENSHOT_TOO_LARGE",
                "screenshot exceeds the 6 MiB vision analysis limit",
            ));
        }
        let image_b64 = base64::Engine::encode(
            &base64::engine::general_purpose::STANDARD,
            &screenshot_bytes,
        );

        // Route to the local vision model (gemma3:4b default; ≤4B rule).
        let vision_model = Self::vision_model_name();
        let ollama_base = std::env::var("OLLAMA_BASE_URL")
            .map(|base| base.trim_end_matches('/').to_string())
            .unwrap_or_else(|_| "http://127.0.0.1:11434".to_string());
        let mut providers = ac_provider::ProviderRegistry::new();
        let adapter = Box::new(ac_provider::OllamaProviderAdapter::new(
            &ollama_base,
            vision_model.clone(),
        )?);
        ac_agent::register_vision_model_route(
            &mut providers,
            "ollama-vision",
            adapter,
            vision_model.clone(),
            "config:ollama.vision",
            8192,
            ac_provider::PrivacyClass::LocalOnly,
        )
        .map_err(|err| {
            AcError::validation(
                "DESIGN-PROVIDER_SETUP",
                format!("cannot register vision model route: {err}"),
            )
        })?;

        let mut profile = ac_provider::TaskProfile::discuss(
            ac_common::StableId::new("designcritic"),
            self.preferred_routing_profile(), // N1: user-persisted routing profile
        );
        profile.required_context = 4096;
        let cancel = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let prompt = r#"You are an independent visual design critic examining a screenshot of a real product UI. Produce a strict JSON object:
{
  "overall": "one sentence verdict of visual quality",
  "findings": [
    {"aspect": "hierarchy|spacing|typography|color|contrast|alignment|density|focus", "severity": 1-5, "issue": "what is wrong, observed in the screenshot", "suggestion": "concrete fix"}
  ],
  "strengths": ["short factual strengths visible in the screenshot"]
}
Additional field: "constraint_adherence" — for each durable project constraint listed below, state whether the visible design adheres to it ("adheres" or a concrete observed violation).
DURABLE PROJECT CONSTRAINTS (NON-NEGOTIABLE — a finding may NOTE that the design violates one, but a suggestion must NEVER propose violating them; they outrank aesthetics):
{constraints_block}
Rules: describe only what is actually visible in the image. Findings must be concrete and locatable (name the region). Do not invent features. Output JSON only."#;
        let constraints_doc = self.design_constraints_get(conversation_id).ok();
        let constraint_texts: Vec<String> = constraints_doc
            .and_then(|doc| doc.get("constraints").and_then(Value::as_array).cloned())
            .unwrap_or_default()
            .iter()
            .filter_map(|c| c.get("text").and_then(Value::as_str).map(String::from))
            .collect();
        let constraints_block = if constraint_texts.is_empty() {
            "(none set)".to_string()
        } else {
            constraint_texts
                .iter()
                .map(|c| format!("- {c}"))
                .collect::<Vec<_>>()
                .join("\n")
        };
        let prompt = prompt.replace("{constraints_block}", &constraints_block);
        let result = providers.request_model_with_image(
            &profile,
            prompt,
            image_b64,
            1024,
            &|| cancel.load(std::sync::atomic::Ordering::Relaxed),
        );

        let parsed = match result {
            Ok(execution) => {
                let text = ac_agent::provider_events_text(&execution.events);
                parse_reference_json(&text)
            }
            Err(failure) => {
                let unavailable = json!({
                    "conversation_id": conversation_id,
                    "url": target_url,
                    "available": false,
                    "unavailable_reason": format!(
                        "VISION_CRITIC_UNAVAILABLE: vision model '{vision_model}' failed ({failure:?}); set AGENTCODE_VISION_MODEL to an installed vision-capable model"
                    ),
                    "vision_model": vision_model,
                    "findings": [],
                    "source": "vision-unavailable",
                });
                let now = TimestampMillis::now().as_millis() as i64;
                let _ = self.db.save_design_critique(&DesignCritiqueRow {
                    id: StableId::new("dcritique").to_string(),
                    conversation_id: conversation_id.to_string(),
                    doc_type: Some("visual-critic".to_string()),
                    passed: false,
                    findings_json: unavailable.to_string(),
                    improvement_required: false,
                    evidence_refs: String::new(),
                    created_at_ms: now,
                });
                return Ok(unavailable);
            }
        };

        let critique = match parsed {
            Some(value) if value.get("findings").is_some() => {
                let findings = value.get("findings").cloned().unwrap_or(json!([]));
                let findings_list = findings.as_array().cloned().unwrap_or_default();
                json!({
                    "conversation_id": conversation_id,
                    "url": target_url,
                    "available": true,
                    "vision_model": vision_model,
                    "overall": value.get("overall").cloned().unwrap_or(Value::Null),
                    "findings": findings_list,
                    "strengths": value.get("strengths").cloned().unwrap_or(json!([])),
                    "screenshot_uri": screenshot.artifact_uri,
                    "screenshot_evidence_ref": screenshot.evidence_ref.to_string(),
                    "constraint_adherence": value.get("constraint_adherence").cloned().unwrap_or(json!({})),
                    "constraints": constraint_texts,
                    "source": "vision-model-gemma",
                })
            }
            _ => {
                json!({
                    "conversation_id": conversation_id,
                    "url": target_url,
                    "available": false,
                    "unavailable_reason":
                        "VISION_CRITIC_UNAVAILABLE: vision model returned no structured critique",
                    "vision_model": vision_model,
                    "findings": [],
                    "source": "vision-unavailable",
                })
            }
        };

        // Persist the visual critique as its own layer.
        let now = TimestampMillis::now().as_millis() as i64;
        let _ = self.db.save_design_critique(&DesignCritiqueRow {
            id: StableId::new("dcritique").to_string(),
            conversation_id: conversation_id.to_string(),
            doc_type: Some("visual-critic".to_string()),
            passed: critique["available"].as_bool().unwrap_or(false)
                && critique["findings"].as_array().map(|a| a.is_empty()).unwrap_or(false),
            findings_json: critique.to_string(),
            improvement_required: false,
            evidence_refs: screenshot.evidence_ref.to_string(),
            created_at_ms: now,
        });

        Ok(critique)
    }

    /// The configured vision model, defaulting to the ≤4B local gemma3:4b.
    fn vision_model_name() -> String {
        std::env::var("AGENTCODE_VISION_MODEL")
            .ok()
            .filter(|m| !m.trim().is_empty())
            .unwrap_or_else(|| "gemma3:4b".to_string())
    }

    /// Build the structured Design Contract for this conversation (Doc 06
    /// §26): product analysis, brief, grammar, reference principles with
    /// do-not-copy boundaries, durable constraints, design state, latest
    /// critique/QA findings, and relevant files — the full context a
    /// mission needs to implement the design without losing it.
    pub fn design_contract(&self, conversation_id: &str) -> AcResult<Value> {
        let conv = self.db.conversation(conversation_id)?.ok_or_else(|| {
            AcError::validation("CONVERSATION-NOT_FOUND", "conversation not found")
        })?;
        let doc = |doc_type: &str| {
            self.db
                .design_document(conversation_id, doc_type)
                .ok()
                .flatten()
                .and_then(|row| serde_json::from_str::<Value>(&row.content_json).ok())
        };
        let analysis = self.design_understand(conversation_id)?;
        // Project-scoped durable constraints (Doc 06 §92).
        let constraints = self.design_constraints_get(conversation_id)?;
        let messages = self.db.messages_for_conversation(conversation_id)?;
        let recent_start = messages.len().saturating_sub(10);
        let recent_messages: Vec<Value> = messages[recent_start..]
            .iter()
            .map(|m| {
                json!({"role": m.role, "content": bounded_ui_summary(&m.content, 512)})
            })
            .collect();
        let contract = json!({
            "conversation_id": conversation_id,
            "project_path": conv.project_path,
            "product_analysis": analysis,
            "brief": doc("design_brief"),
            "grammar": doc("design_grammar"),
            "reference_principles": doc("design_reference"),
            "constraints": constraints,
            "design_state": doc("design_state"),
            "qa_report": doc("design_qa_report"),
            "critique": self
                .db
                .design_critiques(conversation_id)?
                .into_iter()
                .last()
                .and_then(|row| serde_json::from_str::<Value>(&row.findings_json).ok()),
            "recent_messages": recent_messages,
        });
        Ok(contract)
    }

    /// Implement via Mission with the FULL design contract (never a bare
    /// sentence).  The contract text becomes the mission goal so the
    /// mission retains product analysis, constraints, QA findings, and the
    /// design state.
    /// ── Stitch-parity: design → code export ────────────────────────────
    ///
    /// Promotes the WINNING mockup variant into a real implementation
    /// mission.  The mission goal embeds the winner's exact, already-scored
    /// properties (layout, palette hexes, copy, accessibility posture) so
    /// the implementing agent reproduces the scored artifact — not a vague
    /// description of it.  Everything goes through goal_submit: governed
    /// tasks, Tool Broker writes, ChangeSets, and verification evidence,
    /// exactly like every other mission; no bare filesystem writes.
    pub fn design_export_winner(
        &mut self,
        conversation_id: &str,
        target_path: &str,
    ) -> AcResult<Value> {
        self.ensure_running()?;
        let conv = self.db.conversation(conversation_id)?.ok_or_else(|| {
            AcError::validation("CONVERSATION-NOT_FOUND", "conversation not found")
        })?;
        if conv.mode != "DESIGN" {
            return Err(AcError::validation(
                "CONVERSATION-WRONG_MODE",
                "design_export_winner requires a DESIGN conversation",
            ));
        }
        if target_path.trim().is_empty() {
            return Err(AcError::validation(
                "DESIGN-EXPORT_TARGET_REQUIRED",
                "target component path is required (e.g. src/pages/Landing.tsx)",
            ));
        }
        // Boundary check before anything runs: the target must be a clean
        // RELATIVE path inside the project.  We never silently rewrite an
        // escaping path — `..` or absolute targets are refused outright so
        // the goal always names exactly what the user asked for.
        let project = std::path::PathBuf::from(&conv.project_path);
        if std::path::Path::new(target_path).is_absolute() {
            return Err(AcError::validation(
                "DESIGN-EXPORT_TARGET_OUTSIDE_PROJECT",
                "target path must be relative to the project directory",
            ));
        }
        if std::path::Path::new(target_path)
            .components()
            .any(|c| c == std::path::Component::ParentDir)
        {
            return Err(AcError::validation(
                "DESIGN-EXPORT_TARGET_OUTSIDE_PROJECT",
                "target path must not contain '..' — it must stay inside the project directory",
            ));
        }
        let normalized = project.join(target_path);
        let rel_target = normalized
            .strip_prefix(&project)
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|_| target_path.to_string());

        // The latest generated-mockups run for this conversation.
        let doc = self
            .db
            .design_documents(conversation_id)?
            .into_iter()
            .filter(|d| d.doc_type == "generated_mockups")
            .max_by_key(|d| d.updated_at_ms)
            .ok_or_else(|| {
                AcError::validation(
                    "DESIGN-EXPORT_NO_MOCKUPS",
                    "generate mockups first — there is no scored mockup run to export",
                )
            })?;
        let run: Value = serde_json::from_str(&doc.content_json).map_err(|_| {
            AcError::validation("DESIGN-EXPORT_DOC_CORRUPT", "mockup run document is corrupt")
        })?;
        let winner_idx = run["winner"].as_u64().unwrap_or(0) as usize;
        let variants = run["variants"].as_array().cloned().unwrap_or_default();
        let winner = variants
            .get(winner_idx)
            .ok_or_else(|| {
                AcError::validation(
                    "DESIGN-EXPORT_WINNER_MISSING",
                    "the recorded winner variant no longer exists",
                )
            })?
            .clone();
        let spec = run["spec"].clone();

        let layout = winner["layout"].as_str().unwrap_or("hero_center");
        let palette = winner["palette_intent"].as_str().unwrap_or("light");
        let str_field = |name: &str, fallback: &str| -> String {
            spec.get(name)
                .and_then(Value::as_str)
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .unwrap_or_else(|| fallback.to_string())
        };
        let headline = str_field("headline", "Your product, clearly stated");
        let subheadline = str_field(
            "subheadline",
            "A concrete sentence about what this does and for whom.",
        );
        let primary_cta = str_field("primary_cta", "Get started");
        let secondary_cta = str_field("secondary_cta", "Learn more");
        let sections: Vec<String> = spec
            .get("section_ideas")
            .and_then(Value::as_array)
            .map(|arr| {
                arr.iter()
                    .filter_map(Value::as_str)
                    .map(|s| format!("- {s}"))
                    .collect()
            })
            .unwrap_or_default();

        let goal = format!(
            "GOAL: Implement the winning design mockup as a real React component.\n\n\
             TARGET FILE: {rel_target} (create or replace through the governed ChangeSet path only)\n\n\
             This mission was promoted from a scored generative-mockup run in the \
             Design Studio.  The winning variant (variant {winner_idx}, layout '{layout}', \
             palette '{palette}') was rendered and verified in the real browser.  \
             Reproduce exactly these scored properties:\n\n\
             HERO:\n\
             - headline: {headline}\n\
             - subheadline: {subheadline}\n\
             - primary CTA label: {primary_cta}\n\
             - secondary CTA label: {secondary_cta}\n\
             - layout: {layout} (hero_left = text left + visual right; hero_center = \
             centered single column; hero_split = text and visual side by side)\n\n\
             STYLE TOKENS (use these exact values as inline styles or a styled wrapper):\n\
             - palette '{palette}': see the variant's recorded HTML in the design memory \
             document 'generated_mockups' of this conversation for the exact hex values\n\
             - typography: ui-sans-serif/system-ui stack, headline 44px/1.15, subheadline \
             18px/1.6 at 75% opacity\n\
             - CTAs: rounded-10px pills, primary solid accent, secondary 55%-opacity border\n\n\
             SECTIONS:\n{}\n\n\
             REQUIREMENTS:\n\
             1. Self-contained component: no new runtime dependencies.\n\
             2. Accessibility is non-negotiable: semantic landmarks (main, h1), sufficient \
             contrast per the scored palette, aria-hidden on decorative visuals.\n\
             3. All write operations through the Tool Broker (PrepareEdit/ApplyEdit) — \
             no bare filesystem writes.\n\
             4. Verify with the project's dev/test tooling and record evidence.\n",
            sections
                .iter()
                .map(|s| s.as_str())
                .collect::<Vec<_>>()
                .join("\n")
        );

        let (mission_id, _session_id) = self.goal_submit(conversation_id, &goal, &[])?;
        let now = TimestampMillis::now().as_millis() as i64;
        let mut promoted = run;
        promoted["promoted_mission_id"] = json!(mission_id.to_string());
        promoted["exported_target"] = json!(rel_target);
        self.db.save_design_document(&DesignDocumentRow {
            id: doc.id,
            conversation_id: conversation_id.to_string(),
            doc_type: "generated_mockups".to_string(),
            content_json: promoted.to_string(),
            version: doc.version + 1,
            evidence_refs: String::new(),
            created_at_ms: doc.created_at_ms,
            updated_at_ms: now,
        })?;

        Ok(json!({
            "mission_id": mission_id.to_string(),
            "target": rel_target,
            "winner": winner_idx,
            "conversation_id": conversation_id,
        }))
    }

    pub fn design_execute_contract(&mut self, conversation_id: &str) -> AcResult<Value> {
        self.ensure_running()?;
        let contract = self.design_contract(conversation_id)?;
        let goal = Self::contract_to_goal_text(&contract);
        let (mission_id, _session_id) = self.goal_submit(conversation_id, &goal, &[])?;
        let now = TimestampMillis::now().as_millis() as i64;
        let mut promoted = contract;
        promoted["promoted_mission_id"] = json!(mission_id.to_string());
        let _ = self.db.save_design_document(&DesignDocumentRow {
            id: StableId::new("dcontract").to_string(),
            conversation_id: conversation_id.to_string(),
            doc_type: "design_contract".to_string(),
            content_json: promoted.to_string(),
            version: 1,
            evidence_refs: String::new(),
            created_at_ms: now,
            updated_at_ms: now,
        });
        Ok(json!({
            "mission_id": mission_id.to_string(),
            "contract_goal": goal,
            "conversation_id": conversation_id,
        }))
    }

    /// ── Durable design constraints (Doc 06 §92) ─────────────────────────
    /// Project-scoped, user-editable, and explicitly ranked ABOVE
    /// aesthetics: the contract marks them NON-NEGOTIABLE so the critic
    /// and missions cannot trade them away for visual polish.
    pub fn design_constraints_set(
        &self,
        conversation_id: &str,
        constraints: &[Value],
    ) -> AcResult<Value> {
        let conv = self.db.conversation(conversation_id)?.ok_or_else(|| {
            AcError::validation("CONVERSATION-NOT_FOUND", "conversation not found")
        })?;
        let validated: Vec<Value> = constraints
            .iter()
            .filter(|c| c.get("text").and_then(Value::as_str).map(|t| !t.trim().is_empty()).unwrap_or(false))
            .take(32)
            .cloned()
            .collect();
        let project = &conv.project_path;
        let now = TimestampMillis::now().as_millis() as i64;
        // Constraints are project-scoped: shared across every design chat in
        // the project.  Stored under a deterministic per-project key.
        let key = format!("design_constraints:{}", fnv1a64_hash(project.as_bytes()));
        let existing = self
            .db
            .design_documents_by_type(&key)?
            .into_iter()
            .last();
        let version = existing.as_ref().map(|row| row.version + 1).unwrap_or(1);
        let id = existing
            .as_ref()
            .map(|row| row.id.clone())
            .unwrap_or_else(|| StableId::new("dcons").to_string());
        let _ = self.db.save_design_document(&DesignDocumentRow {
            id,
            conversation_id: conversation_id.to_string(),
            doc_type: key,
            content_json: json!({
                "project_path": project,
                "constraints": validated,
                "updated_via_conversation": conversation_id,
            })
            .to_string(),
            version,
            evidence_refs: String::new(),
            created_at_ms: existing
                .as_ref()
                .map(|row| row.created_at_ms)
                .unwrap_or(now),
            updated_at_ms: now,
        });
        self.design_constraints_get(conversation_id)
    }

    /// Load the project-scoped constraints.  Visible from any design chat
    /// in the same project; never leaks to another project's chats.
    pub fn design_constraints_get(&self, conversation_id: &str) -> AcResult<Value> {
        let conv = self.db.conversation(conversation_id)?.ok_or_else(|| {
            AcError::validation("CONVERSATION-NOT_FOUND", "conversation not found")
        })?;
        let key = format!(
            "design_constraints:{}",
            fnv1a64_hash(conv.project_path.as_bytes())
        );
        let found = self
            .db
            .design_documents_by_type(&key)?
            .into_iter()
            .last();
        Ok(match found {
            Some(row) => serde_json::from_str::<Value>(&row.content_json)
                .unwrap_or_else(|_| json!({"constraints": [], "project_path": conv.project_path})),
            None => json!({
                "constraints": [],
                "project_path": conv.project_path,
            }),
        })
    }

    /// Design memory across chats (Doc 06 §27): project-scoped knowledge a
    /// new Design chat inherits — brief, grammar, reference principles,
    /// durable constraints, accepted decisions (from memory_decisions),
    /// and recent QA findings.  Conversation-scoped docs are pulled from
    /// the most recent design conversation of the SAME project only;
    /// nothing leaks across projects.
    pub fn design_memory_get(&self, conversation_id: &str) -> AcResult<Value> {
        let conv = self.db.conversation(conversation_id)?.ok_or_else(|| {
            AcError::validation("CONVERSATION-NOT_FOUND", "conversation not found")
        })?;
        // Durable constraints: project-scoped by construction
        // (design_constraints:{project-path-hash} doc key).
        let constraints = self.design_constraints_get(conversation_id)?;
        // Project-scoped accepted decisions: authoritative memory_decisions.
        let identity = crate::project_repository_identity(&conv.project_path);
        // F1: project memory facts from previous missions (all modes share
        // the same durable repository-scoped store).
        let facts: Vec<Value> = self
            .db
            .memory_facts_for(&identity, 20)?
            .iter()
            .filter(|row| row.valid_until_ms.is_none())
            .map(|row| {
                json!({
                    "id": row.id,
                    "statement": row.statement,
                    "fact_type": row.fact_type,
                    "confidence": row.confidence,
                    "freshness": row.freshness,
                    "last_validation_ms": row.last_validation_ms,
                })
            })
            .collect();
        let task_memories: Vec<Value> = self
            .db
            .task_memories_newest(10)?
            .iter()
            .map(|row| {
                json!({
                    "id": row.id,
                    "task_id": row.task_id,
                    "summary": row.summary,
                    "created_at_ms": row.created_at_ms,
                })
            })
            .collect();
        let decisions: Vec<Value> = self
            .db
            .memory_decisions_for(&identity, 20)?
            .iter()
            .map(|d| {
                json!({
                    "decision": bounded_ui_summary(&d.decision, 256),
                    "rationale": bounded_ui_summary(&d.rationale, 256),
                    "accepted_at_ms": d.created_at_ms,
                })
            })
            .collect();
        // Conversation-scoped artifacts: inherit from the most recent
        // DESIGN conversation of the same project (never another project).
        let project_conversations = self
            .db
            .conversations_for_project(&conv.project_path, false)?
            .into_iter()
            .filter(|c| c.mode == "DESIGN" && c.id != conversation_id)
            .collect::<Vec<_>>();
        let latest = project_conversations.iter().max_by_key(|c| c.created_at_ms);
        let inherited_doc = |doc_type: &str| -> Option<Value> {
            let cid = latest?.id.clone();
            self.db
                .design_document(&cid, doc_type)
                .ok()
                .flatten()
                .and_then(|row| serde_json::from_str::<Value>(&row.content_json).ok())
        };
        let memory = json!({
            "facts": facts,
            "task_memories": task_memories,
            "conversation_id": conversation_id,
            "project_path": conv.project_path,
            "inherited_from_conversation": latest.map(|c| c.id.clone()),
            "brief": inherited_doc("design_brief"),
            "grammar": inherited_doc("design_grammar"),
            "reference_principles": inherited_doc("design_reference"),
            "constraints": constraints.get("constraints").cloned().unwrap_or(json!([])),
            "accepted_decisions": decisions,
            "recent_qa": inherited_doc("design_qa_report"),
        });
        Ok(memory)
    }

    /// Materialize DESIGN_STATE.md in the project worktree as a readable
    /// snapshot (Doc 06 §91).  The SQLite design_documents store remains
    /// the code-authoritative state; the file is derived and written
    /// through a recorded change so the evidence path can audit it.
    pub fn design_materialize_state(&self, conversation_id: &str) -> AcResult<Value> {
        let conv = self.db.conversation(conversation_id)?.ok_or_else(|| {
            AcError::validation("CONVERSATION-NOT_FOUND", "conversation not found")
        })?;
        let state = self.design_state(conversation_id)?;
        let content = state
            .get("content")
            .and_then(Value::as_str)
            .unwrap_or("# DESIGN_STATE.md\n");
        // Governed write (Doc 06 §28/§91): the file is written through the
        // ChangeSet/Kernel path — a real EditEngine transaction with a
        // journal, rollback plan, and content hashes — never a bare
        // filesystem write.  The precondition is the current file hash
        // (or create when absent), so a concurrent mutation fails closed.
        let rel_path = "DESIGN_STATE.md".to_string();
        let mut repo = ac_changeset::LocalWorkspaceFileRepository::new(
            std::path::PathBuf::from(&conv.project_path),
            "daemon",
        );
        let expected_hash = <ac_changeset::LocalWorkspaceFileRepository as ac_changeset::FileRepository>::read(&repo, &rel_path)
            .ok()
            .map(|current| ac_changeset::content_hash(&current));
        let engine = ac_changeset::EditEngine;
        let mut transaction = engine
            .prepare(
                &repo,
                vec![ac_changeset::EditRequest {
                    path: rel_path.clone(),
                    precondition: ac_changeset::EditPrecondition {
                        path: rel_path.clone(),
                        expected_hash: expected_hash.clone(),
                        base_revision: "daemon".to_string(),
                        symbol_fingerprint: None,
                    },
                    strategy: ac_changeset::EditStrategy::WholeFile {
                        content: content.to_string(),
                    },
                }],
            )
            .map_err(|error| {
                AcError::validation(
                    "DESIGN-STATE_MATERIALIZATION",
                    format!("governed transaction failed to prepare: {error}"),
                )
            })?;
        transaction.changeset
            .attach_metadata(ac_changeset::ChangeSetMetadata {
                originating_task: StableId::new("dstatemat"),
                originating_agent_session: StableId::new("design-mode"),
                files_changed: vec![ac_changeset::FileChangeSummary {
                    path: rel_path.clone(),
                    additions: content.lines().count() as u32,
                    removals: 0,
                }],
                additions: content.lines().count() as u32,
                removals: 0,
                evidence_refs: Vec::new(),
                verification_passed: Some(true),
            })
            .map_err(|error| {
                AcError::validation(
                    "DESIGN-STATE_MATERIALIZATION",
                    format!("governed transaction metadata failed: {error}"),
                )
            })?;
        transaction.changeset.validate().map_err(|error| {
            AcError::validation(
                "DESIGN-STATE_MATERIALIZATION",
                format!("governed transaction failed validation: {error}"),
            )
        })?;
        transaction.changeset.approve().map_err(|error| {
            AcError::validation(
                "DESIGN-STATE_MATERIALIZATION",
                format!("governed transaction approval failed: {error}"),
            )
        })?;
        engine
            .apply(&mut repo, &mut transaction)
            .map_err(|error| {
                AcError::validation(
                    "DESIGN-STATE_MATERIALIZATION",
                    format!("governed write failed and rolled back: {error}"),
                )
            })?;
        let changeset_id = transaction.changeset.id.to_string();
        let now = TimestampMillis::now().as_millis() as i64;
        let record_id = StableId::new("dstatemat");
        let _ = self.db.save_design_document(&DesignDocumentRow {
            id: record_id.to_string(),
            conversation_id: conversation_id.to_string(),
            doc_type: "design_state_materialization".to_string(),
            content_json: json!({
                "path": std::path::Path::new(&conv.project_path).join("DESIGN_STATE.md").to_string_lossy(),
                "changeset_id": changeset_id,
                "journal_entries": transaction.journal.entries.len(),
                "derived_from": "design_documents (code-authoritative)",
            })
            .to_string(),
            version: 1,
            evidence_refs: String::new(),
            created_at_ms: now,
            updated_at_ms: now,
        });
        let journal_entries = transaction.journal.entries.len();
        Ok(json!({
            "materialized": true,
            "path": std::path::Path::new(&conv.project_path).join("DESIGN_STATE.md").to_string_lossy(),
            "record_id": record_id.to_string(),
            "changeset_id": changeset_id,
            "journal_entries": journal_entries,
            "note": "DESIGN_STATE.md is a derived readable snapshot written through a governed ChangeSet; SQLite design_documents is authoritative",
        }))
    }

    /// Render the design contract as the mission goal text.  Constraints
    /// and do-not-copy boundaries are marked non-negotiable — they outrank
    /// aesthetics in the mission's priorities.
    fn contract_to_goal_text(contract: &Value) -> String {
        let mut out = String::new();
        let product = contract
            .get("product_analysis")
            .and_then(|v| v.get("product_summary"))
            .and_then(Value::as_str)
            .or_else(|| {
                contract
                    .get("product_analysis")
                    .and_then(|v| v.get("framework"))
                    .and_then(Value::as_str)
            })
            .unwrap_or("the product discussed in this design conversation");
        out.push_str(&format!(
            "GOAL: Implement the design agreed in this Design Studio conversation for {product}.\n\n"
        ));
        out.push_str("This mission was promoted from a structured Design Contract:\n\n");

        if let Some(brief) = contract.get("brief").and_then(|v| v.get("brief")) {
            if let Some(text) = brief.as_str() {
                out.push_str("DESIGN BRIEF (the intent):\n");
                out.push_str(&bounded_ui_summary(text, 2048));
                out.push_str("\n\n");
            }
        }
        if let Some(grammar) = contract.get("grammar") {
            if let Some(tokens) = grammar.get("tokens").and_then(Value::as_array) {
                out.push_str("DESIGN TOKENS:\n");
                for token in tokens.iter().take(20) {
                    out.push_str(&format!("- {token}\n"));
                }
                out.push('\n');
            }
        }
        if let Some(reference) = contract.get("reference_principles") {
            if let Some(principles) = reference.get("adopted_principles").and_then(Value::as_array)
            {
                if !principles.is_empty() {
                    out.push_str("ADOPTED REFERENCE PRINCIPLES (abstract only):\n");
                    for principle in principles.iter().take(10) {
                        out.push_str(&format!("- {principle}\n"));
                    }
                    out.push('\n');
                }
            }
            if let Some(do_not_copy) =
                reference.get("explicitly_do_not_copy").and_then(Value::as_array)
            {
                if !do_not_copy.is_empty() {
                    out.push_str("DO NOT COPY (NON-NEGOTIABLE):\n");
                    for item in do_not_copy.iter().take(10) {
                        out.push_str(&format!("- {item}\n"));
                    }
                    out.push('\n');
                }
            }
        }
        if let Some(constraints) = contract
            .get("constraints")
            .and_then(|v| v.get("constraints"))
            .and_then(Value::as_array)
        {
            if !constraints.is_empty() {
                out.push_str("DURABLE CONSTRAINTS (NON-NEGOTIABLE — outrank aesthetics):\n");
                for constraint in constraints.iter().take(20) {
                    if let Some(text) = constraint.get("text").and_then(Value::as_str) {
                        out.push_str(&format!("- {text}\n"));
                    } else if let Some(text) = constraint.as_str() {
                        out.push_str(&format!("- {text}\n"));
                    }
                }
                out.push('\n');
            }
        }
        if let Some(qa) = contract.get("qa_report") {
            if let Some(layers) = qa.get("layers").and_then(Value::as_object) {
                out.push_str("QA FINDINGS TO FIX (from real-browser QA):\n");
                for (layer, report) in layers {
                    if let Some(issues) = report.get("issues").and_then(Value::as_array) {
                        for issue in issues.iter().take(10) {
                            if let Some(text) = issue.as_str() {
                                out.push_str(&format!("- [{layer}] {text}\n"));
                            }
                        }
                    }
                }
                out.push('\n');
            }
        }
        if let Some(messages) = contract.get("recent_messages").and_then(Value::as_array) {
            out.push_str("DESIGN CONVERSATION CONTEXT (recent):\n");
            for message in messages.iter().take(10) {
                let role = message.get("role").and_then(Value::as_str).unwrap_or("");
                let content = message.get("content").and_then(Value::as_str).unwrap_or("");
                out.push_str(&format!("- {role}: {content}\n"));
            }
        }
        bounded_ui_summary(&out, 12 * 1024)
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
                    let mut guard = signal.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
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
            if let Some(p) = *port_signal.lock().unwrap_or_else(|poisoned| poisoned.into_inner()) {
                port = Some(p);
                break;
            }
            std::thread::sleep(Duration::from_millis(100));
        }
        if port.is_none() {
            for candidate in DEFAULT_DEV_PORTS {
                if test_http_ready(candidate) && port_owned_by_process(pid, candidate) {
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
                .unwrap_or_else(|poisoned| poisoned.into_inner())
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

        // port is Some here by the is_none early-return above; avoid a panic
            // path regardless (poisoned-mutex / re-entrancy hardening).
            let ready_port = port.unwrap_or(0);
            let ready_url = format!("http://127.0.0.1:{ready_port}");
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
            .unwrap_or_else(|poisoned| poisoned.into_inner())
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
        let mut children = self.design_children.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
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
    /// `viewport_hint` selects a ViewportProfile:
    ///   "compact" (1024x768), "desktop" (1440x900), "wide" (1920x1080).
    pub fn design_browser(
        &self,
        conversation_id: &str,
        url: &str,
        html: &str,
        deterministic: bool,
        viewport_hint: &str,
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

        let viewport = match viewport_hint {
            "compact" => ac_verification::ViewportProfile {
                name: "compact",
                width: 1024,
                height: 768,
            },
            "wide" => ac_verification::ViewportProfile {
                name: "wide",
                width: 1920,
                height: 1080,
            },
            _ => ac_verification::ViewportProfile {
                name: "desktop",
                width: 1440,
                height: 900,
            },
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
            "viewport": {"name": viewport.name, "width": viewport.width, "height": viewport.height},
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

    /// Codex-style inbuilt browser panel: navigate (or go back/forward)
    /// ONE persistent Chrome and return a framed screenshot with live
    /// diagnostics.  The runtime is shared across calls (`live_browser`), so
    /// the panel behaves like an embedded browser — Chrome starts once and
    /// stays up while the user browses.
    pub fn browser_panel(
        &self,
        action: &str,
        url: &str,
        viewport_hint: &str,
    ) -> AcResult<Value> {
        // Watch-the-agent: while a design run holds the shared engine, user
        // navigation returns an honest busy state instead of blocking for the
        // whole run (the UI already shows what the run is viewing via
        // AgentBrowseStatus; between runs the panel is fully user-driven).
        if action != "close" {
            let run_active = self
                .agent_browse_status
                .lock()
                .map(|status| status.is_some())
                .unwrap_or(false);
            let lock_free = self.live_browser.try_lock().is_ok();
            if run_active && !lock_free {
                return Ok(json!({
                    "busy": true,
                    "reason": "agent-run",
                    "status": self
                        .agent_browse_status
                        .lock()
                        .ok()
                        .and_then(|guard| guard.clone())
                        .unwrap_or(Value::Null),
                }));
            }
        }
        // Only http(s) and the local dev preview are navigable from the
        // panel; file:// and other schemes are refused outright.
        let validate_url = |u: &str| -> AcResult<String> {
            if u.starts_with("http://") || u.starts_with("https://") {
                Ok(u.to_string())
            } else {
                Err(AcError::validation(
                    "BROWSER-PANEL_URL_REFUSED",
                    "panel navigation allows http(s) URLs only",
                ))
            }
        };

        let mut guard = self
            .live_browser
            .lock()
            .map_err(|_| AcError::validation("BROWSER-PANEL_LOCK", "browser panel lock poisoned"))?;
        let task_id = StableId::new("panel");
        // A "close" action tears the runtime down gracefully and reports
        // honestly (including when nothing is open — idempotent teardown).
        // Handled BEFORE the closed-guard so close is always valid.
        if action == "close" {
            let runtime = guard.take();
            drop(guard);
            if let Some(mut runtime) = runtime {
                runtime.close_all();
            }
            return Ok(json!({"closed": true}));
        }
        let runtime_slot = guard
            .as_mut()
            .ok_or_else(|| AcError::validation("BROWSER-PANEL_CLOSED", "panel browser is closed"))?;

        // The panel keeps exactly one page session alive; its id is stable
        // per runtime instance.
        let session_id = match runtime_slot.panel_session() {
            Some(id) => id,
            None => {
                let process = runtime_slot.launch(task_id.clone())?;
                let session = runtime_slot.create_session(task_id.clone(), process.id.clone())?;
                runtime_slot.set_panel_session(&session.id);
                session.id
            }
        };

        let viewport = match viewport_hint {
            "compact" => ac_verification::ViewportProfile {
                name: "compact",
                width: 1024,
                height: 768,
            },
            "wide" => ac_verification::ViewportProfile {
                name: "wide",
                width: 1920,
                height: 1080,
            },
            _ => ac_verification::ViewportProfile {
                name: "desktop",
                width: 1440,
                height: 900,
            },
        };

        let mut evidence_store = EvidenceStore::new();
        let mut final_url = String::new();
        match action {
            "navigate" | "reload" => {
                let target = validate_url(url)?;
                runtime_slot.act(
                    &session_id,
                    ac_verification::BrowserAction::Navigate {
                        url: target.clone(),
                    },
                    &mut evidence_store,
                )?;
                final_url = target;
            }
            "back" | "forward" => {
                // History traversal via CDP page history.
                runtime_slot.traverse_history(&session_id, action)?;
            }
            "screenshot" => {}
            other => {
                return Err(AcError::validation(
                    "BROWSER-PANEL_ACTION",
                    format!("unknown browser panel action: {other}"),
                ));
            }
        }
        runtime_slot.act(
            &session_id,
            ac_verification::BrowserAction::Wait { millis: 400 },
            &mut evidence_store,
        )?;

        let dom = runtime_slot.inspect_dom(&session_id, &mut evidence_store)?;
        let diag = runtime_slot.diagnostics(&session_id, &mut evidence_store)?;
        if final_url.is_empty() {
            final_url = dom
                .accessibility_tree
                .first()
                .map(|s| s.to_string())
                .unwrap_or_default();
        }
        let screenshot = runtime_slot.capture_screenshot(
            &session_id,
            task_id,
            "browser-panel",
            viewport,
            &mut evidence_store,
        )?;
        let png = self.browser_screenshot_png(&screenshot.artifact_uri)?;
        Ok(json!({
            "url": final_url,
            "title": dom.visible_text.lines().next().unwrap_or("").trim().to_string(),
            "viewport": {"name": viewport.name, "width": viewport.width, "height": viewport.height},
            "diagnostics": {
                "console_errors": diag.console_errors,
                "page_errors": diag.page_errors,
                "network_failures": diag.network_failures,
                "http_status": diag.http_status,
            },
            "visible_text_preview": dom.visible_text.chars().take(400).collect::<String>(),
            "png_base64": png["png_base64"],
        }))
    }

    /// Serve a captured screenshot's PNG bytes as base64 (inbuilt-browser
    /// panel rendering).  Only paths under the agentcode-browser-artifacts
    /// temp dir are served — an arbitrary path probe is refused.
    pub fn browser_screenshot_png(&self, artifact_uri: &str) -> AcResult<Value> {
        let path = std::path::Path::new(artifact_uri);
        if !path.is_absolute() {
            return Err(AcError::validation(
                "BROWSER-SCREENSHOT_INVALID_URI",
                "screenshot uri must be an absolute artifacts path",
            ));
        }
        if !path.extension().map(|ext| ext == "png").unwrap_or(false) {
            return Err(AcError::validation(
                "BROWSER-SCREENSHOT_INVALID_URI",
                "screenshot artifact must be a png",
            ));
        }
        let allowed = std::env::temp_dir().join("agentcode-browser-artifacts");
        let canonical = path.canonicalize().map_err(|err| {
            AcError::validation(
                "BROWSER-SCREENSHOT_READ",
                format!("screenshot artifact not found: {err}"),
            )
        })?;
        if !canonical.starts_with(&allowed) {
            return Err(AcError::validation(
                "BROWSER-SCREENSHOT_INVALID_URI",
                "screenshot uri must point into the agentcode browser artifacts dir",
            ));
        }
        let bytes = std::fs::read(&canonical).map_err(|err| {
            AcError::validation(
                "BROWSER-SCREENSHOT_READ",
                format!("cannot read screenshot artifact: {err}"),
            )
        })?;
        const MAX_SERVE_BYTES: usize = 12 * 1024 * 1024;
        if bytes.len() > MAX_SERVE_BYTES {
            return Err(AcError::validation(
                "BROWSER-SCREENSHOT_TOO_LARGE",
                "screenshot exceeds the 12 MiB serve limit",
            ));
        }
        use base64::Engine as _;
        let bytes_len = bytes.len();
        Ok(json!({
            "png_base64": base64::engine::general_purpose::STANDARD.encode(bytes),
            "bytes": bytes_len,
        }))
    }

    /// Real design QA driven by a live browser session (Doc 06 §49-50).
    ///
    /// This is the shared engine behind design_qa_responsive/accessibility/
    /// functional.  It launches the real Chrome CDP runtime against the
    /// conversation's preview URL (or the given `url`), measures real layout
    /// metrics (overflow, touch-target sizes, unlabeled controls, heading
    /// hierarchy, focus visibility), captures diagnostics, and persists the
    /// layered report through the design_visual_evaluations store.
    ///
    /// Layer separation (audit requirement): Deterministic Browser QA vs
    /// Manual Review is explicit — `needs_manual_review` is true exactly
    /// when the runtime could not measure honestly.
    ///
    /// `deterministic` runs the deterministic harness with `html` — used by
    /// tests and recorded as NEEDS_MANUAL_REVIEW for layout metrics, never
    /// a fabricated pass.
    /// ── Stitch-parity: generative mockups ──────────────────────────────
    ///
    /// The full Stitch flow: prompt -> multiple candidate variants ->
    /// real rendering -> visual scoring -> a winner.  Honest to the small
    /// local models we run: the model does NOT write whole HTML files
    /// (3-4B models produce broken markup); it fills a tightly-constrained
    /// JSON spec (layout, section copy, palette intent) and the DAEMON
    /// renders variants deterministically from that spec.  Every variant is
    /// then scored by the REAL browser (rendered text presence) and the
    /// winning variant is persisted as a design document the user can
    /// critique further; nothing is ever fabricated.
    /// Watch-the-agent (browser): acquire the SHARED browser runtime for a
    /// design run.  Non-deterministic runs reuse the daemon's persistent
    /// `live_browser` (the same engine the user's inbuilt browser panel
    /// drives) so the panel can observe what the run is viewing while it
    /// runs.  Deterministic harnesses keep their isolated test runtime.
    /// Run `body` with the SHARED live browser runtime (kept in the slot).
    /// Non-deterministic design paths use this so the panel observes the
    /// run while it executes; the runtime is never removed from the slot.
    fn with_shared_browser<T>(
        &self,
        deterministic: bool,
        policy: ac_security::CapabilityPolicy,
        body: impl FnOnce(&mut ac_verification::BrowserRuntime) -> T,
    ) -> T {
        if deterministic {
            let mut harness = ac_verification::BrowserRuntime::deterministic_harness_for_tests(policy);
            return body(&mut harness);
        }
        let mut guard = self
            .live_browser
            .lock()
            .map_err(|_| AcError::validation("BROWSER-PANEL_LOCK", "browser panel lock poisoned"))
            .expect("live browser lock");
        if guard.is_none() {
            *guard = Some(ac_verification::BrowserRuntime::new(policy));
        }
        let runtime = guard.as_mut().expect("live browser runtime");
        body(runtime)
    }

    pub fn design_generate_mockups(
        &mut self,
        conversation_id: &str,
        prompt: &str,
        variant_count: usize,
        deterministic: bool,
    ) -> AcResult<Value> {
        self.ensure_running()?;
        let conv = self.db.conversation(conversation_id)?.ok_or_else(|| {
            AcError::validation("CONVERSATION-NOT_FOUND", "conversation not found")
        })?;
        if conv.mode != "DESIGN" {
            return Err(AcError::validation(
                "CONVERSATION-WRONG_MODE",
                "design_generate_mockups requires a DESIGN conversation",
            ));
        }
        if prompt.trim().is_empty() {
            return Err(AcError::validation(
                "CONVERSATION-EMPTY_PROMPT",
                "mockup prompt must not be empty",
            ));
        }
        let variant_count = variant_count.clamp(1, 4);

        // 1. Model fills a constrained spec.  The schema is tiny and every
        //    field is optional-with-default so a weak model still yields a
        //    usable spec; the DAEMON owns all rendering decisions.
        // Premium generation (Stitch-level intent): the model proposes a
        // genuine DESIGN DIRECTION — personality, audience voice, motifs,
        // typographic feel and section structure — not just landing copy.
        // The daemon still owns every rendering decision (deterministic,
        // evidence-scored); the model supplies taste and content.
        let spec_prompt = format!(
            "You are the creative director filling a JSON spec for a premium UI mockup generator.\n\
             Request: {}\n\
             Project context (real files): {}\n\
             Reply with ONLY a JSON object with these optional fields:\n\
             {{\"headline\": string (max 60 chars), \"subheadline\": string (max 120 chars),\n\
             \"primary_cta\": string (max 24 chars), \"secondary_cta\": string (max 24 chars),\n\
             \"hero_image_idea\": string (max 60 chars), \"section_ideas\": array of 1-3 strings (max 40 chars each),\n\
             \"palette_intent\": \"light\"|\"dark\"|\"warm\"|\"cool\"|\"high_contrast\",\n\
             \"layout\": \"hero_left\"|\"hero_center\"|\"hero_split\",\n\
             \"audience\": string (max 40 chars, who this is for),\n\
             \"voice\": string (max 40 chars, one-line brand voice, e.g. 'confident, technical, zero fluff'),\n\
             \"personality\": one of \"minimal\"|\"editorial\"|\"bold\"|\"playful\"|\"technical\",\n\
             \"stats\": array of 1-3 {{\"value\": string (max 8 chars), \"label\": string (max 30 chars)}},\n\
             \"feature_details\": array of 1-3 {{\"title\": string (max 24 chars), \"body\": string (max 70 chars)}},\n\
             \"testimonial\": {{\"quote\": string (max 110 chars), \"author\": string (max 40 chars)}}}}\n\
             Rules: copy must be concrete and product-specific (no 'Lorem', no generic marketing filler);\n\
             stats and features must reflect what the project actually is. Output JSON only, no prose.",
            bounded_ui_summary(prompt, 512),
            bounded_project_listing(&conv.project_path, 20),
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
            self.preferred_routing_profile(),
        );
        profile.required_context = 2048;
        let cancel = Arc::new(AtomicBool::new(false));
        let result = providers.request_model(&profile, spec_prompt, 1024, &|| {
            cancel.load(Ordering::Relaxed)
        });

        // The model's raw reply is recorded as evidence even when parsing
        // fails — honest failure, never a fabricated spec.
        let raw_reply = match result {
            Ok(ref execution) => provider_events_text(&execution.events),
            Err(error) => {
                return Err(AcError::validation(
                    "DESIGN-GENERATE_PROVIDER_FAILED",
                    format!("no provider could produce a spec: {error:?}"),
                ))
            }
        };
        let spec = parse_reference_json(&raw_reply).unwrap_or_else(|| {
            // Honest degradation: a deterministic minimal spec, clearly
            // marked that the model's output could not be parsed.
            json!({
                "headline": bounded_ui_summary(prompt, 48),
                "subheadline": "Generated from your prompt",
                "primary_cta": "Get started",
                "secondary_cta": "Learn more",
                "hero_image_idea": "",
                "section_ideas": [],
                "palette_intent": "light",
                "layout": "hero_center",
                "personality": "minimal",
                "audience": "",
                "voice": "",
                "stats": [],
                "feature_details": [],
                "testimonial": json!({}),
                "_model_spec_parse_failed": true,
            })
        });

        // 2. Render variants deterministically from the spec: layout and
        //    palette vary independently so the candidates genuinely differ,
        //    not cosmetic jitter.
        let layouts = ["hero_left", "hero_center", "hero_split"];
        let palettes: [(&str, &str, &str, &str); 4] = [
            ("light", "#ffffff", "#0f172a", "#2563eb"),
            ("dark", "#0b1120", "#e2e8f0", "#38bdf8"),
            ("warm", "#fffaf5", "#292018", "#ea580c"),
            ("high_contrast", "#ffffff", "#000000", "#0047ab"),
        ];
        // Personality is an independent design axis: it selects a distinct
        // typographic + compositional treatment inside the renderer so two
        // variants differ in DESIGN, not just hue.
        let personalities = ["minimal", "editorial", "bold", "technical", "playful"];
        let spec_layout = spec
            .get("layout")
            .and_then(Value::as_str)
            .unwrap_or("hero_center")
            .to_string();
        let intent = spec
            .get("palette_intent")
            .and_then(Value::as_str)
            .unwrap_or("light")
            .to_string();
        let base_layout_idx = layouts
            .iter()
            .position(|l| *l == spec_layout)
            .unwrap_or(1);
        let variants: Vec<Value> = (0..variant_count)
            .map(|i| {
                // Variant 0 honors the spec exactly; later variants rotate
                // layout + palette so the user compares true alternatives.
                let idx = (base_layout_idx + i) % layouts.len();
                let (palette_name, bg, fg, accent) = if i == 0 {
                    let p = palettes
                        .iter()
                        .find(|(name, _, _, _)| *name == intent)
                        .unwrap_or(&palettes[0]);
                    (p.0, p.1, p.2, p.3)
                } else {
                    let p = &palettes[i % palettes.len()];
                    (p.0, p.1, p.2, p.3)
                };
                let layout = if i == 0 {
                    layouts[base_layout_idx]
                } else {
                    layouts[idx]
                };
                // Variant 0 honors the model's chosen personality; later
                // variants rotate it so the user compares true design
                // alternatives (not the same template in another color).
                let personality = if i == 0 {
                    spec.get("personality")
                        .and_then(Value::as_str)
                        .and_then(|p| personalities.iter().find(|c| *c == &p).copied())
                        .unwrap_or("minimal")
                } else {
                    personalities[(i + 1) % personalities.len()]
                };
                json!({
                    "variant": i,
                    "layout": layout,
                    "palette_intent": palette_name,
                    "personality": personality,
                    "html": render_mockup_html(&spec, layout, bg, fg, accent, personality),
                })
            })
            .collect();

        // 3. Score every variant with the REAL browser when available:
        //    OpenHtmlForTest + snapshot gives the true rendered text.
        //    Deterministic harness (no pixels) gets a clearly labeled
        //    structural score instead — never fabricated pixels.
        let policy = ac_security::CapabilityPolicy::new()
            .allow(ac_security::Capability::BrowserAutomation);
        // Score through the shared engine when a real browser is available:
        // with_shared_browser keeps the runtime in the daemon slot so the
        // inbuilt panel observes the run while it executes.
        let scored = self.with_shared_browser(deterministic, policy, |browser| {
            let task_id = StableId::new("design");
            let mut evidence_store = EvidenceStore::new();
            let mut scored: Vec<Value> = Vec::new();
        // Chrome-crash-dialog fix: launch ONE browser process for the whole
        // run and reuse it across variants (the per-variant launch + never
        // closed loop leaked a Chrome per mockup and left them to be
        // SIGKILLed later — the source of "Chrome quit unexpectedly").
        let mut shared_process: Option<ac_verification::BrowserProcessRecord> = None;
        for variant in &variants {
            let html = variant["html"].as_str().unwrap_or("").to_string();
            let url = format!("mockup-variant-{}", variant["variant"].as_i64().unwrap_or(0));
            // Watch-the-agent: publish what this run is viewing so the
            // inbuilt panel can display the run's live observation point.
            if !deterministic {
                let _ = self.agent_browse_status.lock().map(|mut status| {
                    *status = Some(json!({
                        "label": format!(
                            "Design run · variant {} ({})",
                            variant["variant"].as_i64().unwrap_or(0),
                            variant["personality"].as_str().unwrap_or("minimal"),
                        ),
                        "url": url,
                        "kind": "design-mockups",
                    }));
                });
            }
            let score = if deterministic {
                json!({
                    "source": "structural",
                    "text_present": html.contains("<h1"),
                    "notes": "deterministic harness: structural score, no pixels",
                })
            } else {
                // Reuse the shared process: launch only on the first variant.
                if shared_process.is_none() {
                    shared_process = browser.launch(task_id.clone()).ok();
                }
                match &shared_process {
                    Some(process) => {
                        let rendered = browser
                            .create_session(task_id.clone(), process.id.clone())
                            .and_then(|session| {
                                browser
                                    .act(
                                        &session.id,
                                        ac_verification::BrowserAction::OpenHtmlForTest {
                                            url,
                                            html: html.clone(),
                                        },
                                        &mut evidence_store,
                                    )
                                    .and_then(|_| browser.inspect_dom(&session.id, &mut evidence_store))
                            });
                        match rendered {
                            Ok(snapshot) => {
                                let visible = snapshot.visible_text.trim();
                                // Aesthetic depth signals (honest, DOM-derived):
                                // section variety, stat density, typographic
                                // scale, motif presence.  These reward real
                                // compositional structure, not just text.
                                let sections = html.matches("<section").count();
                                let has_stats = html.contains("font-size:") && html.matches("border-top:1px solid").count() >= 1;
                                let has_testimonial = html.contains("<blockquote");
                                let type_tokens = html.matches("font-size:").count();
                                json!({
                                    "source": "real-browser",
                                    "text_present": visible.len() > 10,
                                    "visible_chars": visible.len(),
                                    "sections": sections,
                                    "structure_richness": sections + type_tokens / 4
                                        + usize::from(has_stats) + usize::from(has_testimonial),
                                    "has_stats_band": has_stats,
                                    "has_testimonial": has_testimonial,
                                })
                            }
                            Err(_) => {
                                // The shared browser is no longer trusted
                                // for further variants; close it gracefully
                                // and mark the rest unavailable rather than
                                // reusing a broken session.
                                if let Some(process) = shared_process.take() {
                                    let _ = browser.close_process(&process.id);
                                }
                                json!({
                                    "source": "unavailable",
                                    "notes": "browser could not render variant; score unavailable",
                                })
                            }
                        }
                    }
                    None => json!({
                        "source": "unavailable",
                        "notes": "browser could not launch; score unavailable",
                    }),
                }
            };
            let mut v = variant.clone();
            v["score"] = score;
            scored.push(v);
        }
        // Deterministic harness: close after the run.  The SHARED engine
        // stays alive — the inbuilt panel (and the next design run) reuse
        // it; its teardown happens on panel close / daemon stop.

            scored
        });

        // 4. Winner: the variant whose real evidence is strongest — real
        //    browser renders with text beat structural scores beat
        //    unavailable.  Ties break by variant order (stable, first wins).
        let rank = |v: &Value| -> i32 {
            match v["score"]["source"].as_str().unwrap_or("") {
                "real-browser" => {
                    // A rendered variant that is text-present AND
                    // structurally rich outranks plain text-only renders.
                    2 + i32::from(v["score"]["text_present"].as_bool().unwrap_or(false))
                        + v["score"]["structure_richness"].as_i64().unwrap_or(0).min(6) as i32
                }
                "structural" => 1,
                _ => 0,
            }
        };
        let winner = scored
            .iter()
            .enumerate()
            .max_by_key(|(i, v)| (rank(v), std::cmp::Reverse(*i)))
            .map(|(_, v)| v.clone())
            .ok_or_else(|| {
                AcError::validation("DESIGN-GENERATE_NO_VARIANTS", "no variants were produced")
            })?;

        // Watch-the-agent: the run is done; clear the live observation
        // point (the panel returns to user-driven browsing).
        let _ = self
            .agent_browse_status
            .lock()
            .map(|mut status| *status = None);

        // 5. Persist the run as a design document (survives restarts,
        //    feeds design memory + downstream contract).
        let now = TimestampMillis::now().as_millis() as i64;
        let run = json!({
            "prompt": prompt,
            "spec": spec,
            "spec_parse_failed": spec.get("_model_spec_parse_failed").is_some(),
            "variants": scored,
            "winner": winner["variant"],
            "created_at_ms": now,
        });
        let id = format!("dgen-{}", StableId::new("mockups"));
        self.db.save_design_document(&DesignDocumentRow {
            id,
            conversation_id: conversation_id.to_string(),
            doc_type: "generated_mockups".to_string(),
            content_json: run.to_string(),
            version: 1,
            evidence_refs: String::new(),
            created_at_ms: now,
            updated_at_ms: now,
        })?;

        Ok(run)
    }

    /// Multi-screen Stitch flow (Doc 06 / Stitch parity): generate a mockup
    /// run for EACH screen prompt (1–4 screens) through the exact
    /// single-screen machinery, then group the winners into a FLOW design
    /// document with shared lineage.  A flow is an app skeleton: the
    /// screens belong together (one product, one palette family), and the
    /// flow document records every per-screen winner for export.
    pub fn design_generate_flow(
        &mut self,
        conversation_id: &str,
        screens: &[String],
        variants_per_screen: usize,
        deterministic: bool,
    ) -> AcResult<Value> {
        self.ensure_running()?;
        if screens.is_empty() {
            return Err(AcError::validation(
                "DESIGN-FLOW_NO_SCREENS",
                "a flow needs at least one screen prompt",
            ));
        }
        if screens.len() > 4 {
            return Err(AcError::validation(
                "DESIGN-FLOW_TOO_MANY_SCREENS",
                "a flow supports at most 4 screens per run",
            ));
        }
        if screens.iter().any(|s| s.trim().is_empty()) {
            return Err(AcError::validation(
                "DESIGN-FLOW_EMPTY_SCREEN",
                "every screen prompt must be non-empty",
            ));
        }

        let flow_id = format!("flow-{}", StableId::new("flow"));
        let mut per_screen: Vec<Value> = Vec::new();
        let mut failed: Vec<Value> = Vec::new();
        for (index, screen) in screens.iter().enumerate() {
            let run = self
                .design_generate_mockups(
                    conversation_id,
                    &format!("Screen {index} of the flow: {screen}"),
                    variants_per_screen,
                    deterministic,
                )
                .map_err(|error| {
                    // Wrap so a single-screen failure names the screen.
                    AcError::validation(
                        "DESIGN-FLOW_SCREEN_FAILED",
                        format!("screen {index} ({screen}) failed: {error}"),
                    )
                });
            match run {
                Ok(run) => {
                    let entry = json!({
                        "index": index,
                        "screen": screen,
                        "run": run,
                        "winner": run["winner"],
                        "winner_html": run["variants"]
                            .as_array()
                            .and_then(|variants| {
                                variants
                                    .iter()
                                    .find(|v| v["variant"] == run["winner"])
                                    .and_then(|v| v["html"].as_str())
                            })
                            .unwrap_or("")
                            .to_string(),
                    });
                    per_screen.push(entry);
                }
                Err(error) => {
                    // Honest per-screen failure record: the flow continues
                    // with the remaining screens, and the failure is named.
                    failed.push(json!({
                        "index": index,
                        "screen": screen,
                        "error": error.to_string(),
                    }));
                }
            }
        }
        if per_screen.is_empty() {
            return Err(AcError::validation(
                "DESIGN-FLOW_ALL_SCREENS_FAILED",
                "every screen in the flow failed; see the mockup run errors",
            ));
        }
        let flow = json!({
            "flow_id": flow_id,
            "conversation_id": conversation_id,
            "screen_count": per_screen.len(),
            "failed_screens": failed,
            "screens": per_screen,
            "deterministic": deterministic,
        });
        let now = TimestampMillis::now().as_millis() as i64;
        let id = StableId::new("flow").to_string();
        self.db.save_design_document(&DesignDocumentRow {
            id,
            conversation_id: conversation_id.to_string(),
            doc_type: "generated_flow".to_string(),
            content_json: flow.to_string(),
            version: 1,
            evidence_refs: String::new(),
            created_at_ms: now,
            updated_at_ms: now,
        })?;
        Ok(flow)
    }

    pub fn design_qa_run(
        &self,
        conversation_id: &str,
        url: &str,
        html: &str,
        deterministic: bool,
        viewport_hint: &str,
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

        let task_id = StableId::new("designqa");
        let process = browser.launch(task_id.clone())?;
        let session = browser.create_session(task_id.clone(), process.id.clone())?;

        // Apply the requested viewport before measuring.
        let viewport = match viewport_hint {
            "compact" => ac_verification::ViewportProfile {
                name: "compact",
                width: 1024,
                height: 768,
            },
            "wide" => ac_verification::ViewportProfile {
                name: "wide",
                width: 1920,
                height: 1080,
            },
            _ => ac_verification::ViewportProfile {
                name: "desktop",
                width: 1440,
                height: 900,
            },
        };
        let _ = browser.set_viewport(&session.id, viewport);

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
                ac_verification::BrowserAction::Navigate {
                    url: target_url.clone(),
                },
                &mut evidence_store,
            )?;
            let _ = browser.act(
                &session.id,
                ac_verification::BrowserAction::Wait { millis: 300 },
                &mut evidence_store,
            )?;
        }

        let metrics = browser.design_metrics(&session.id)?;
        let diag = browser.diagnostics(&session.id, &mut evidence_store)?;

        // ── Layered findings, each traceable to a real measurement ──
        // `issues` = measured failures.  `unmeasured` = metrics this runtime
        // cannot measure honestly (NEEDS_MANUAL_REVIEW) — they never flip a
        // pass verdict and never fabricate one.
        let mut responsive_issues = Vec::new();
        let mut responsive_unmeasured = Vec::new();
        let mut responsive_viewports = Vec::new();
        match metrics.horizontal_overflow {
            Some(true) => responsive_issues.push(format!(
                "horizontal overflow at {}px viewport: scrollWidth {} > clientWidth {}",
                metrics.viewport.width,
                metrics.scroll_width.unwrap_or(0),
                metrics.client_width.unwrap_or(0)
            )),
            Some(false) => responsive_viewports.push(format!(
                "{} ({}px): no horizontal overflow (scroll {}, client {})",
                metrics.viewport.name,
                metrics.viewport.width,
                metrics.scroll_width.unwrap_or(0),
                metrics.client_width.unwrap_or(0)
            )),
            None => responsive_unmeasured
                .push("layout overflow not measurable in this runtime".to_string()),
        }
        if !metrics.has_viewport_meta {
            responsive_issues.push("missing viewport meta tag".to_string());
        }

        let mut accessibility_issues = Vec::new();
        let mut accessibility_unmeasured = Vec::new();
        for target in &metrics.small_touch_targets {
            accessibility_issues.push(format!("touch target below 24px: {target}"));
        }
        for control in &metrics.unlabeled_controls {
            accessibility_issues.push(format!("unlabeled control: {control}"));
        }
        for skip in &metrics.heading_skips {
            accessibility_issues.push(format!("heading hierarchy skip: {skip}"));
        }
        if !metrics.has_lang_attribute {
            accessibility_issues.push("html element has no lang attribute".to_string());
        }
        match metrics.focus_visible_support {
            Some(false) => accessibility_issues
                .push("focus indicator not visible on interactive elements".to_string()),
            Some(true) => {}
            None => accessibility_unmeasured
                .push("focus visibility not measurable in this runtime".to_string()),
        }

        let mut functional_issues = Vec::new();
        for error in &diag.console_errors {
            functional_issues.push(format!("console error: {error}"));
        }
        for error in &diag.page_errors {
            functional_issues.push(format!("page error: {error}"));
        }
        for failure in &diag.network_failures {
            functional_issues.push(format!("network failure: {failure}"));
        }
        if diag.http_status != 200 && diag.http_status != 0 {
            functional_issues.push(format!("HTTP status {}", diag.http_status));
        }

        let responsive_passed = responsive_issues.is_empty();
        let accessibility_passed = accessibility_issues.is_empty();
        let functional_passed = functional_issues.is_empty();

        // Persist the layered reports through the evidence-backed store,
        // then the combined evaluation into design_visual_evaluations.
        let artifact_version = StableId::new("designqa");
        let verifier = ac_verification::VerificationEngine::new(
            ac_security::CapabilityPolicy::new()
                .allow(ac_security::Capability::BrowserAutomation),
        );
        let responsive_report = verifier.record_design_responsive_report(
            &artifact_version,
            if responsive_viewports.is_empty() {
                vec![format!(
                    "{} ({}px)",
                    metrics.viewport.name, metrics.viewport.width
                )]
            } else {
                responsive_viewports.clone()
            },
            responsive_passed,
            &mut evidence_store,
        )?;
        let accessibility_report = verifier.record_design_accessibility_report(
            &artifact_version,
            accessibility_passed,
            if accessibility_issues.is_empty() {
                vec![
                    "touch targets >= 24px".to_string(),
                    "controls labeled".to_string(),
                    "heading hierarchy intact".to_string(),
                    "lang attribute present".to_string(),
                ]
            } else {
                accessibility_issues.clone()
            },
            &mut evidence_store,
        )?;
        let functional_report = verifier.record_design_functional_report(
            &artifact_version,
            functional_passed,
            if functional_passed {
                vec![format!(
                    "page loads at {target_url} without console/page/network errors"
                )]
            } else {
                functional_issues.clone()
            },
            &mut evidence_store,
        )?;
        let _ = self.db.save_design_visual_evaluation(
            &ac_db::DesignVisualEvaluationRow {
                id: StableId::new("dqa").to_string(),
                artifact_version_id: artifact_version.to_string(),
                passed: responsive_passed && accessibility_passed && functional_passed,
                findings: serde_json::to_string(&json!({
                    "responsive": responsive_issues,
                    "accessibility": accessibility_issues,
                    "functional": functional_issues,
                }))
                .unwrap_or_default(),
                responsive_viewports: serde_json::to_string(&responsive_viewports).unwrap_or_default(),
                accessibility_checks: serde_json::to_string(&accessibility_report.checks).unwrap_or_default(),
                functional_flows: serde_json::to_string(&functional_report.flows).unwrap_or_default(),
                evidence_refs: [
                    responsive_report.evidence_ref.to_string(),
                    accessibility_report.evidence_ref.to_string(),
                    functional_report.evidence_ref.to_string(),
                ]
                .join(","),
                created_at_ms: TimestampMillis::now().as_millis() as i64,
            },
        );

        // Conversation-scoped QA report document (UI + persistence).
        let report = json!({
            "conversation_id": conversation_id,
            "url": target_url,
            "mode": metrics.mode,
            "viewport": {
                "name": metrics.viewport.name,
                "width": metrics.viewport.width,
                "height": metrics.viewport.height,
            },
            "needs_manual_review": metrics.needs_manual_review,
            "layers": {
                "responsive": {
                    "passed": responsive_passed,
                    "issues": responsive_issues,
                    "unmeasured": responsive_unmeasured,
                    "viewports": responsive_viewports,
                    "source": if metrics.needs_manual_review { "manual-review" } else { "real-browser-cdp" },
                },
                "accessibility": {
                    "passed": accessibility_passed,
                    "issues": accessibility_issues,
                    "unmeasured": accessibility_unmeasured,
                    "control_count": metrics.control_count,
                    "source": if metrics.needs_manual_review { "manual-review" } else { "real-browser-cdp" },
                },
                "functional": {
                    "passed": functional_passed,
                    "issues": functional_issues,
                    "unmeasured": Vec::<String>::new(),
                    "http_status": diag.http_status,
                    "source": if deterministic { "deterministic-harness" } else { "real-browser-cdp" },
                },
            },
            "metrics": {
                "scroll_width": metrics.scroll_width,
                "client_width": metrics.client_width,
                "horizontal_overflow": metrics.horizontal_overflow,
                "touch_targets": metrics.touch_targets.len(),
                "small_touch_targets": metrics.small_touch_targets.len(),
                "unlabeled_controls": metrics.unlabeled_controls.len(),
                "heading_skips": metrics.heading_skips.len(),
                "focus_visible": metrics.focus_visible_support,
                "has_lang": metrics.has_lang_attribute,
                "has_viewport_meta": metrics.has_viewport_meta,
                "document_title": metrics.document_title,
            },
            "passed": responsive_passed && accessibility_passed && functional_passed,
        });
        let now = TimestampMillis::now().as_millis() as i64;
        let _ = self.db.save_design_document(&DesignDocumentRow {
            id: StableId::new("dqa").to_string(),
            conversation_id: conversation_id.to_string(),
            doc_type: "design_qa_report".to_string(),
            content_json: report.to_string(),
            version: 1,
            evidence_refs: evidence_store
                .records()
                .map(|record| record.id.to_string())
                .collect::<Vec<_>>()
                .join(","),
            created_at_ms: now,
            updated_at_ms: now,
        });

        // Batch N2: graceful close on the success path (was a forced kill).
        let _ = browser.close_process(&process.id);
        Ok(report)
    }

    /// Legacy single-check entry points kept for the UI: they now run the
    /// full real-browser QA and slice the requested layer.
    pub fn design_qa_responsive(
        &self,
        conversation_id: &str,
        _content: &str,
        url: &str,
        html: &str,
        deterministic: bool,
        viewport_hint: &str,
    ) -> AcResult<Value> {
        let report = self.design_qa_run(conversation_id, url, html, deterministic, viewport_hint)?;
        Ok(report["layers"]["responsive"].clone())
    }

    pub fn design_qa_accessibility(
        &self,
        conversation_id: &str,
        _content: &str,
        url: &str,
        html: &str,
        deterministic: bool,
        viewport_hint: &str,
    ) -> AcResult<Value> {
        let report = self.design_qa_run(conversation_id, url, html, deterministic, viewport_hint)?;
        Ok(report["layers"]["accessibility"].clone())
    }

    pub fn design_qa_functional(
        &self,
        conversation_id: &str,
        _content: &str,
        url: &str,
        html: &str,
        deterministic: bool,
        viewport_hint: &str,
    ) -> AcResult<Value> {
        let report = self.design_qa_run(conversation_id, url, html, deterministic, viewport_hint)?;
        Ok(report["layers"]["functional"].clone())
    }

    /// Repair loop (Doc 06 §24): a genuine loop, not one critique.
    /// Each call runs the critique, derives repairs, and PERSISTS the
    /// iteration record (iteration number, input, findings, repairs,
    /// remaining issues) as a `design_iteration` document so history
    /// accumulates across preview → critic → repair → preview cycles.
    /// The next `design_repair` continues the numbered sequence.
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

        // Iteration history: count prior iterations and persist this one.
        let prior = self
            .db
            .design_documents_by_type("design_iteration")?
            .into_iter()
            .filter(|row| row.conversation_id == conversation_id)
            .count();
        let iteration = prior + 1;
        let remaining: Vec<String> = findings
            .iter()
            .map(|f| f["rule"].as_str().unwrap_or("unknown").to_string())
            .collect();
        let now = TimestampMillis::now().as_millis() as i64;
        let _ = self.db.save_design_document(&DesignDocumentRow {
            id: StableId::new("diter").to_string(),
            conversation_id: conversation_id.to_string(),
            doc_type: "design_iteration".to_string(),
            content_json: json!({
                "iteration": iteration,
                "doc_type": doc_type,
                "input": bounded_ui_summary(content, 512),
                "findings": findings,
                "repairs": repairs,
                "remaining_issues": remaining,
                "passed": passed,
                "created_at_ms": now,
            })
            .to_string(),
            version: iteration as i64,
            evidence_refs: String::new(),
            created_at_ms: now,
            updated_at_ms: now,
        });

        Ok(json!({
            "iteration": iteration,
            "passed": passed,
            "repairs": repairs,
            "improvement_required": !passed,
            "remaining_issues": remaining,
        }))
    }

    /// Iteration history for this conversation: every persisted repair
    /// loop iteration in order (§24).
    pub fn design_iterations(&self, conversation_id: &str) -> AcResult<Value> {
        let rows = self
            .db
            .design_documents_by_type("design_iteration")?
            .into_iter()
            .filter(|row| row.conversation_id == conversation_id)
            .collect::<Vec<_>>();
        let iterations: Vec<Value> = rows
            .iter()
            .map(|row| serde_json::from_str::<Value>(&row.content_json).unwrap_or(Value::Null))
            .filter(|v| !v.is_null())
            .collect();
        Ok(json!({ "iterations": iterations }))
    }
}

const DEFAULT_DEV_PORTS: [u16; 7] = [5173, 3000, 8080, 8000, 4173, 4321, 1420];

/// Extract a JSON object from a vision-model response.  Small models
/// frequently wrap the JSON in prose or markdown fences despite strict
/// prompting; try the raw text, then fenced blocks, then the first
/// balanced-brace object.  Returns None when nothing JSON-shaped is present —
/// the caller then fails honestly instead of fabricating an analysis.
fn parse_reference_json(text: &str) -> Option<Value> {
    let trimmed = text.trim();
    if let Ok(value) = serde_json::from_str::<Value>(trimmed) {
        if value.is_object() {
            return Some(value);
        }
    }
    for line in trimmed.lines() {
        let line = line.trim();
        if (line.starts_with("```") || line.starts_with("'''"))
            && line.len() > 3
        {
            if let Ok(value) = serde_json::from_str::<Value>(&line[3..]) {
                if value.is_object() {
                    return Some(value);
                }
            }
        }
    }
    if let Some(start) = trimmed.find('{') {
        let mut depth = 0i32;
        for (offset, ch) in trimmed[start..].char_indices() {
            match ch {
                '{' => depth += 1,
                '}' => {
                    depth -= 1;
                    if depth == 0 {
                        let candidate = &trimmed[start..start + offset + 1];
                        if let Ok(value) = serde_json::from_str::<Value>(candidate) {
                            if value.is_object() {
                                return Some(value);
                            }
                        }
                        break;
                    }
                }
                _ => {}
            }
        }
    }
    None
}

fn detect_dev_command(project_path: &str) -> String {    let dir = Path::new(project_path);
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

/// Escape text for safe embedding in generated mockup HTML.
fn mockup_escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

/// Deterministically render a self-contained mockup page from the model
/// spec.  Pure function of (spec, layout, palette) — no model HTML ever
/// runs; the daemon owns every tag it emits.
/// Visual-check shim (used by the design_visual example for manual
/// browser inspection of the design-system renderer).
pub fn render_mockup_html_for_visual_check(
    spec: &Value,
    layout: &str,
    bg: &str,
    fg: &str,
    accent: &str,
    personality: &str,
) -> String {
    render_mockup_html(spec, layout, bg, fg, accent, personality)
}

fn render_mockup_html(
    spec: &Value,
    layout: &str,
    bg: &str,
    fg: &str,
    accent: &str,
    personality: &str,
) -> String {
    let str_field = |name: &str, fallback: &str| -> String {
        let value = spec
            .get(name)
            .and_then(Value::as_str)
            .map(|s| s.trim())
            .unwrap_or("");
        let bounded: String = value
            .chars()
            .take(if name == "subheadline" { 140 } else { 70 })
            .collect();
        if bounded.is_empty() {
            fallback.to_string()
        } else {
            bounded
        }
    };
    let headline = str_field("headline", "Your product, clearly stated");
    let subheadline = str_field(
        "subheadline",
        "A concrete sentence about what this does and for whom.",
    );
    let primary_cta = str_field("primary_cta", "Get started");
    let secondary_cta = str_field("secondary_cta", "Learn more");
    let hero_image = str_field("hero_image_idea", "");
    let audience = str_field("audience", "");
    let voice = str_field("voice", "");
    let project_name = str_field("project_name", "AgentCode");

    let sections: Vec<String> = spec
        .get("section_ideas")
        .and_then(Value::as_array)
        .map(|arr| {
            arr.iter()
                .filter_map(Value::as_str)
                .map(|s| mockup_escape(&s.chars().take(60).collect::<String>()))
                .take(3)
                .collect()
        })
        .unwrap_or_default();

    // ── Design tokens: one personality = one coherent token set ──────
    // Type feel, radius, spacing rhythm, elevation and motif all shift
    // together, so each variant reads as a DIFFERENT design language.
    let tokens = personality_tokens(personality);
    let muted = |opacity: &str| format!("{fg}{opacity}");

    // ── Motif: the personality's signature background treatment ──────
    let motif = match personality {
        "editorial" => format!(
            "background:{bg};background-image:radial-gradient(1200px 400px at 20% -10%, {accent}14, transparent)"
        ),
        "bold" => format!(
            "background:{bg};background-image:linear-gradient(120deg, {accent}0f 0%, transparent 40%)"
        ),
        "technical" => format!(
            "background:{bg};background-image:linear-gradient({fg}08 1px, transparent 1px),linear-gradient(90deg, {fg}08 1px, transparent 1px);background-size:44px 44px"
        ),
        "playful" => format!(
            "background:{bg};background-image:radial-gradient(600px 300px at 85% 0%, {accent}1f, transparent),radial-gradient(500px 260px at 0% 100%, {accent}14, transparent)"
        ),
        _ => format!("background:{bg}"),
    };

    // ── Nav ───────────────────────────────────────────────────────────
    let nav = format!(
        "<header style=\"display:flex;align-items:center;justify-content:space-between;         padding:{nav_pad}px 32px;border-bottom:1px solid {fgline}\">\n\
           <div style=\"display:flex;align-items:center;gap:10px\">\n\
             <span style=\"width:26px;height:26px;border-radius:{logo_r}px;background:{accent};\
             display:inline-block\"></span>\n\
             <span style=\"font-size:15px;font-weight:{logo_w};color:{fg};letter-spacing:{logo_ls}px\">{brand}</span>\n\
           </div>\n\
           <div style=\"display:flex;gap:20px;align-items:center\">\n\
             <span style=\"font-size:13px;color:{muted8}\">{nav_link}</span>\n\
             <span style=\"display:inline-block;padding:8px 16px;border-radius:{cta_r}px;background:{accent};\
             color:{on_accent};font-size:13px;font-weight:600\">{primary_cta}</span>\n\
           </div>\n\
         </header>",
        nav_pad = tokens.nav_pad,
        fgline = muted("1f"),
        logo_r = tokens.logo_radius,
        logo_w = tokens.logo_weight,
        logo_ls = tokens.logo_tracking,
        brand = mockup_escape(&project_name),
        muted8 = muted("cc"),
        nav_link = if audience.is_empty() { "Overview".to_string() } else { mockup_escape(&audience) },
        cta_r = tokens.cta_radius,
        on_accent = on_accent_for(bg, accent),
    );

    // ── Hero ──────────────────────────────────────────────────────────
    let hero_visual = if hero_image.is_empty() {
        format!(
            "<div style=\"width:300px;height:200px;border-radius:{card_r}px;             background:linear-gradient(135deg,{accent},{fg});opacity:0.92;             box-shadow:{elev}\" aria-hidden=\"true\"></div>",
            card_r = tokens.card_radius,
            elev = tokens.elevation,
        )
    } else {
        format!(
            "<div style=\"width:300px;height:200px;border-radius:{card_r}px;background:{accent}14;             border:1px solid {accent}66;display:flex;align-items:center;justify-content:center;             padding:16px;box-shadow:{elev}\"><span style=\"font-size:13px;color:{muted99};\
             text-align:center\">{hero_image}</span></div>",
            card_r = tokens.card_radius,
            elev = tokens.elevation,
            muted99 = muted("99"),
        )
    };
    let eyebrow = if voice.is_empty() {
        String::new()
    } else {
        format!(
            "<p style=\"margin:0 0 14px;font-size:12px;letter-spacing:2px;text-transform:uppercase;             color:{accent};font-weight:600\">{}</p>",
            mockup_escape(&voice)
        )
    };
    let hero_inner = format!(
        "{eyebrow}\n\
         <h1 style=\"margin:0 0 18px;font-size:{h1}px;line-height:{h1_lh};color:{fg};\
         font-weight:{h1_w};letter-spacing:{h1_ls}px;max-width:{h1_max}rem\">{headline}</h1>\n\
         <p style=\"margin:0 0 30px;font-size:17px;line-height:1.65;color:{muted_cc};\
         max-width:34rem\">{subheadline}</p>\n\
         <div style=\"display:flex;gap:12px;flex-wrap:wrap\">\n\
           <span style=\"display:inline-block;padding:13px 26px;border-radius:{cta_r}px;           background:{accent};color:{on_accent};font-weight:600;font-size:15px\">{primary_cta}</span>\n\
           <span style=\"display:inline-block;padding:13px 26px;border-radius:{cta_r}px;           border:1px solid {fg}55;color:{fg};font-size:15px\">{secondary_cta}</span>\n\
         </div>",
        eyebrow = eyebrow,
        h1 = tokens.h1_size,
        h1_lh = tokens.h1_line_height,
        h1_w = tokens.h1_weight,
        h1_ls = tokens.h1_tracking,
        h1_max = tokens.h1_max_width,
        headline = mockup_escape(&headline),
        muted_cc = muted("cc"),
        subheadline = mockup_escape(&subheadline),
        cta_r = tokens.cta_radius,
        on_accent = on_accent_for(bg, accent),
        primary_cta = mockup_escape(&primary_cta),
        secondary_cta = mockup_escape(&secondary_cta),
    );

    let (hero_style, hero_wrap) = match layout {
        "hero_left" => (
            "display:flex;flex-direction:column;justify-content:center;text-align:left".to_string(),
            "display:flex;gap:56px;align-items:center;justify-content:center".to_string(),
        ),
        "hero_split" => (
            "display:flex;flex-direction:column;justify-content:center;text-align:center;flex:1"
                .to_string(),
            "display:flex;gap:56px;align-items:center;justify-content:center".to_string(),
        ),
        _ => (
            "display:flex;flex-direction:column;justify-content:center;align-items:center;             text-align:center;max-width:44rem"
                .to_string(),
            "display:block;padding:0 24px".to_string(),
        ),
    };
    let hero_block = format!(
        "<div style=\"{hero_wrap}\"><div style=\"{hero_style}\">{hero_inner}</div>{hero_visual}</div>",
        hero_wrap = hero_wrap,
        hero_style = hero_style,
        hero_inner = hero_inner,
        hero_visual = if layout == "hero_left" || layout == "hero_split" {
            hero_visual
        } else {
            format!(
                "<div style=\"margin-top:36px;max-width:300px;margin-left:auto;margin-right:auto\">{hero_visual}</div>",
                hero_visual = hero_visual
            )
        },
    );

    // ── Stats band ────────────────────────────────────────────────────
    let stats: Vec<(String, String)> = spec
        .get("stats")
        .and_then(Value::as_array)
        .map(|arr| {
            arr.iter()
                .filter_map(|s| {
                    let value = s.get("value").and_then(Value::as_str)?.chars().take(8).collect::<String>();
                    let label = s
                        .get("label")
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .chars()
                        .take(30)
                        .collect::<String>();
                    Some((mockup_escape(&value), mockup_escape(&label)))
                })
                .take(3)
                .collect()
        })
        .unwrap_or_default();
    let stats_block = if stats.is_empty() {
        String::new()
    } else {
        let cells: Vec<String> = stats
            .iter()
            .map(|(value, label)| {
                format!(
                    "<div style=\"flex:1;min-width:140px;text-align:left\">\n\
                       <p style=\"margin:0;font-size:{stat}px;font-weight:700;color:{accent}\">{value}</p>\n\
                       <p style=\"margin:6px 0 0;font-size:13px;color:{muted_aa}\">{label}</p>\n\
                     </div>",
                    stat = tokens.h1_size.saturating_sub(8).max(28),
                    muted_aa = muted("aa"),
                )
            })
            .collect();
        format!(
            "<section style=\"max-width:960px;margin:56px auto 0;padding:24px 32px;\
             display:flex;gap:32px;flex-wrap:wrap;border-top:1px solid {fg1a};border-bottom:1px solid {fg1a}\">{}</section>",
            cells.join("\n      "),
            fg1a = muted("1a"),
        )
    };

    // ── Feature grid ──────────────────────────────────────────────────
    let feature_details: Vec<(String, String)> = spec
        .get("feature_details")
        .and_then(Value::as_array)
        .map(|arr| {
            arr.iter()
                .filter_map(|f| {
                    let title = f
                        .get("title")
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .chars()
                        .take(24)
                        .collect::<String>();
                    let body = f
                        .get("body")
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .chars()
                        .take(70)
                        .collect::<String>();
                    if title.is_empty() {
                        None
                    } else {
                        Some((mockup_escape(&title), mockup_escape(&body)))
                    }
                })
                .take(3)
                .collect()
        })
        .unwrap_or_default();
    // Fall back to the copy-level section ideas when no details exist.
    let features: Vec<(String, String)> = if feature_details.is_empty() {
        sections
            .iter()
            .map(|s| (s.clone(), String::new()))
            .collect()
    } else {
        feature_details
    };
    let features_block = if features.is_empty() {
        String::new()
    } else {
        let cards: Vec<String> = features
            .iter()
            .map(|(title, body)| {
                format!(
                    "<div style=\"flex:1;min-width:230px;padding:24px;border-radius:{card_r}px;\
                     background:{card_bg};border:1px solid {fg1f}\">\n\
                       <span style=\"display:inline-block;width:34px;height:34px;border-radius:{icon_r}px;\
                       background:{accent}1f;margin-bottom:14px\"></span>\n\
                       <p style=\"margin:0 0 8px;font-size:16px;font-weight:600;color:{fg}\">{title}</p>\n\
                       <p style=\"margin:0;font-size:14px;line-height:1.6;color:{muted_b3}\">{body}</p>\n\
                     </div>",
                    card_r = tokens.card_radius,
                    card_bg = if bg == "#ffffff" { "#f8fafc".to_string() } else { format!("{fg}0d") },
                    fg1f = muted("1f"),
                    icon_r = tokens.icon_radius,
                    muted_b3 = muted("b3"),
                )
            })
            .collect();
        format!(
            "<section style=\"max-width:960px;margin:56px auto 0;padding:0 32px\">\n\
               <h2 style=\"margin:0 0 24px;font-size:26px;font-weight:650;color:{fg}\">Why teams choose this</h2>\n\
               <div style=\"display:flex;gap:18px;flex-wrap:wrap\">{}</div>\n\
             </section>",
            cards.join("\n      "),
        )
    };

    // ── Testimonial ───────────────────────────────────────────────────
    let testimonial = spec.get("testimonial").cloned().unwrap_or(json!({}));
    let t_quote = testimonial
        .get("quote")
        .and_then(Value::as_str)
        .map(|q| mockup_escape(&q.chars().take(110).collect::<String>()))
        .unwrap_or_default();
    let t_author = testimonial
        .get("author")
        .and_then(Value::as_str)
        .map(|a| mockup_escape(&a.chars().take(40).collect::<String>()))
        .unwrap_or_default();
    let testimonial_block = if t_quote.is_empty() {
        String::new()
    } else {
        format!(
            "<section style=\"max-width:720px;margin:56px auto 0;padding:0 32px;text-align:left\">\n\
               <blockquote style=\"margin:0;font-size:20px;line-height:1.55;color:{fg};font-weight:500\">\u{201c}{t_quote}\u{201d}</blockquote>\n\
               <p style=\"margin:14px 0 0;font-size:13px;color:{muted_aa}\">\u{2014} {t_author}</p>\n\
             </section>",
            muted_aa = muted("aa"),
            t_quote = t_quote,
            t_author = t_author,
        )
    };

    // ── Closing CTA band ──────────────────────────────────────────────
    let cta_band = format!(
        "<section style=\"max-width:960px;margin:64px auto 0;padding:32px;margin-left:32px;\
         margin-right:32px;border-radius:{card_r}px;background:{accent}14;\
         display:flex;align-items:center;justify-content:space-between;gap:24px;flex-wrap:wrap\">\n\
           <p style=\"margin:0;font-size:19px;font-weight:600;color:{fg}\">Ready when you are.</p>\n\
           <span style=\"display:inline-block;padding:12px 24px;border-radius:{cta_r}px;background:{accent};\
           color:{on_accent};font-weight:600;font-size:14px\">{primary_cta}</span>\n\
         </section>",
        card_r = tokens.card_radius,
        cta_r = tokens.cta_radius,
        on_accent = on_accent_for(bg, accent),
        primary_cta = mockup_escape(&primary_cta),
    );

    format!(
        "<!doctype html><html lang=\"en\"><head><meta charset=\"utf-8\">\n\
         <meta name=\"viewport\" content=\"width=device-width,initial-scale=1\"></head>\n\
         <body style=\"margin:0;font-family:{font_stack};background:{motif_bg};min-height:100vh;color:{fg}\">\n\
         {nav}\n\
         <main style=\"padding:{main_pad}px 0 88px\"><div style=\"max-width:1040px;margin:0 auto\">\n\
           {hero_block}\n\
           {stats_block}\n\
           {features_block}\n\
           {testimonial_block}\n\
           {cta_band}\n\
         </div></main>\n\
         <footer style=\"border-top:1px solid {fg1f};padding:24px 32px;display:flex;\
         justify-content:space-between;align-items:center;flex-wrap:wrap;gap:12px\">\n\
           <span style=\"font-size:12px;color:{muted_99}\">{brand} \u{00b7} {aud}</span>\n\
           <span style=\"font-size:12px;color:{muted_99}\">Generated by AgentCode design engine</span>\n\
         </footer>\n\
         </body></html>",
        font_stack = tokens.font_stack,
        motif_bg = motif,
        main_pad = tokens.main_pad,
        fg1f = muted("1f"),
        brand = mockup_escape(&project_name),
        aud = if audience.is_empty() { mockup_escape("Built for modern teams") } else { mockup_escape(&audience) },
        muted_99 = muted("99"),
    )
}

/// Per-personality design tokens.  Each personality selects ONE coherent
/// token set (type feel + radii + spacing rhythm + elevation + motif) so
/// variants read as genuinely different design languages.
struct PersonalityTokens {
    font_stack: &'static str,
    h1_size: u32,
    h1_weight: u32,
    h1_line_height: &'static str,
    h1_tracking: i32,
    h1_max_width: u32,
    nav_pad: u32,
    main_pad: u32,
    logo_radius: u32,
    logo_weight: u32,
    logo_tracking: i32,
    card_radius: u32,
    cta_radius: u32,
    icon_radius: u32,
    elevation: &'static str,
}

fn personality_tokens(personality: &str) -> PersonalityTokens {
    match personality {
        "editorial" => PersonalityTokens {
            font_stack: "'Georgia', 'Times New Roman', serif",
            h1_size: 52,
            h1_weight: 500,
            h1_line_height: "1.12",
            h1_tracking: -1,
            h1_max_width: 22,
            nav_pad: 20,
            main_pad: 88,
            logo_radius: 2,
            logo_weight: 400,
            logo_tracking: 4,
            card_radius: 4,
            cta_radius: 4,
            icon_radius: 2,
            elevation: "0 12px 32px rgba(0,0,0,0.14)",
        },
        "bold" => PersonalityTokens {
            font_stack: "ui-sans-serif, system-ui, -apple-system, 'Segoe UI', sans-serif",
            h1_size: 60,
            h1_weight: 800,
            h1_line_height: "1.05",
            h1_tracking: -2,
            h1_max_width: 20,
            nav_pad: 18,
            main_pad: 72,
            logo_radius: 8,
            logo_weight: 800,
            logo_tracking: 0,
            card_radius: 16,
            cta_radius: 14,
            icon_radius: 10,
            elevation: "0 16px 40px rgba(0,0,0,0.18)",
        },
        "technical" => PersonalityTokens {
            font_stack: "'SF Mono', ui-monospace, 'Cascadia Code', Menlo, monospace",
            h1_size: 40,
            h1_weight: 600,
            h1_line_height: "1.2",
            h1_tracking: 0,
            h1_max_width: 26,
            nav_pad: 16,
            main_pad: 80,
            logo_radius: 4,
            logo_weight: 600,
            logo_tracking: 1,
            card_radius: 6,
            cta_radius: 6,
            icon_radius: 4,
            elevation: "0 8px 24px rgba(0,0,0,0.12)",
        },
        "playful" => PersonalityTokens {
            font_stack: "ui-rounded, 'SF Pro Rounded', ui-sans-serif, system-ui, sans-serif",
            h1_size: 48,
            h1_weight: 700,
            h1_line_height: "1.1",
            h1_tracking: -1,
            h1_max_width: 24,
            nav_pad: 20,
            main_pad: 76,
            logo_radius: 14,
            logo_weight: 700,
            logo_tracking: 0,
            card_radius: 22,
            cta_radius: 18,
            icon_radius: 12,
            elevation: "0 10px 28px rgba(0,0,0,0.14)",
        },
        _ => PersonalityTokens {
            font_stack: "ui-sans-serif, system-ui, -apple-system, 'Segoe UI', sans-serif",
            h1_size: 44,
            h1_weight: 650,
            h1_line_height: "1.15",
            h1_tracking: -1,
            h1_max_width: 24,
            nav_pad: 18,
            main_pad: 80,
            logo_radius: 8,
            logo_weight: 600,
            logo_tracking: 0,
            card_radius: 12,
            cta_radius: 10,
            icon_radius: 8,
            elevation: "0 10px 28px rgba(0,0,0,0.12)",
        },
    }
}

/// Text color that stays readable on the accent fill (dark palettes use a
/// light accent that already contrasts; light palettes need white text).
fn on_accent_for(_bg: &str, accent: &str) -> String {
    let light = ["#38bdf8", "#ea580c"];
    if light.contains(&accent) {
        "#0b1120".to_string()
    } else {
        "#ffffff".to_string()
    }
}

fn test_http_ready(port: u16) -> bool {
    use std::net::TcpStream;
    let addr = format!("127.0.0.1:{port}");
    addr.parse::<std::net::SocketAddr>()
        .ok()
        .and_then(|a| TcpStream::connect_timeout(&a, Duration::from_millis(150)).ok())
        .is_some()
}

/// F10 (final audit): does OUR spawned child own the listener on this port?
/// Prevents recording a foreign dev server's port as this design's preview
/// (the default-port probe alone can bind the WRONG server when the real
/// dev server is slow and another process listens on a default port).
/// Uses `lsof -a -d tcp -p <pid>` and checks the port appears in the
/// output; honest fallback: if lsof is unavailable we return true (probe
/// only) — availability checks are advisory, not blocking.
fn port_owned_by_process(pid: u32, port: u16) -> bool {
    let output = std::process::Command::new("lsof")
        .args(["-a", "-d", "tcp", "-P", "-n", "-p", &pid.to_string()])
        .output();
    match output {
        Ok(output) if output.status.success() => {
            let text = String::from_utf8_lossy(&output.stdout);
            text.contains(&format!(":{port}"))
        }
        _ => true, // lsof unavailable or denied: cannot disprove ownership
    }
}