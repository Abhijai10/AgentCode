// ── Discuss Turn-Into-Plan / Decision capture (G3, Doc 06 §23-25) ────────────
//
// The production conversation model is conversations/conversation_messages.
// The former ac-agent DiscussSession lifecycle (ac-agent/src/discuss.rs) was
// an orphaned parallel model — never called by the daemon — and has been
// deleted (see tests/integration/orphan_guard.rs for the durable guard).
// Structured Discuss plans and accepted decisions live
// in the conversation-scoped `design_documents` store (doc types
// `discuss_plan` / `discuss_decisions`) so they persist with the
// conversation, survive restart, and are project-isolated by the same
// conversation→project binding as every other mode artifact.
//
// Accepted decisions additionally materialize into:
//  1. `memory_decisions` (authoritative structured state) under a
//     deterministic repository identity derived from the project path so
//     decisions are queryable per project and cannot cross projects.
//  2. `DECISIONS.md` in the project worktree with the change recorded in
//     the DB (`decisions_materialization`) — the daemon never writes the
//     file outside the recorded-change path.

impl DaemonService {
    /// Turn a discussion into a structured plan (Doc 06 §23).
    ///
    /// The plan is derived from the actual conversation transcript (never a
    /// bare goal sentence): requirements/constraints/open-questions come
    /// from model extraction when reachable, with a deterministic
    /// structural fallback so the plan is always a real structured
    /// artifact.
    pub fn discuss_turn_into_plan(&mut self, conversation_id: &str) -> AcResult<Value> {
        self.ensure_running()?;
        let conv = self.conversation_or_err(conversation_id)?;
        if conv.mode != "DISCUSS" {
            return Err(AcError::validation(
                "CONVERSATION-WRONG_MODE",
                "discuss_turn_into_plan requires a DISCUSS conversation",
            ));
        }
        let messages = self.db.messages_for_conversation(conversation_id)?;
        if messages.len() < 2 {
            return Err(AcError::validation(
                "DISCUSS-PLAN_NO_DISCUSSION",
                "a plan requires an actual discussion (at least one question and answer)",
            ));
        }

        // Bounded transcript for extraction.
        let transcript = messages
            .iter()
            .rev()
            .take(20)
            .rev()
            .map(|m| format!("{}: {}", m.role, bounded_ui_summary(&m.content, 2048)))
            .collect::<Vec<_>>()
            .join("\n");

        // Model-assisted structured extraction; deterministic fallback when
        // the provider is unavailable.
        let extracted = self.discuss_extract_plan_via_model(&transcript);
        let (goal, requirements, constraints, rejected, open_questions) = match extracted {
            Some(value) => (
                value
                    .get("goal")
                    .and_then(Value::as_str)
                    .unwrap_or(&conv.title)
                    .to_string(),
                plan_string_array(value.get("requirements"), 8),
                plan_string_array(value.get("constraints"), 8),
                plan_string_array(value.get("rejected_approaches"), 8),
                plan_string_array(value.get("open_questions"), 8),
            ),
            None => {
                // Deterministic structural fallback: the plan is built from
                // the real transcript, never fabricated.
                let user_lines: Vec<String> = messages
                    .iter()
                    .filter(|m| m.role == "user")
                    .map(|m| bounded_ui_summary(&m.content, 200))
                    .collect();
                let goal = user_lines
                    .first()
                    .cloned()
                    .unwrap_or_else(|| conv.title.clone());
                let requirements = if user_lines.len() > 1 {
                    user_lines[1..].to_vec()
                } else {
                    vec!["Act on the goal above, as discussed.".to_string()]
                };
                (
                    goal,
                    requirements,
                    Vec::new(),
                    Vec::new(),
                    vec!["Confirm scope and priority with the user before execution.".to_string()],
                )
            }
        };

        // Accepted decisions recorded in this conversation become plan
        // context (they are structured state, not chatter).
        let decisions = self.discuss_decisions_value(conversation_id)?;
        let grounding_sources = self
            .db
            .design_document(conversation_id, "discuss_context")?
            .and_then(|d| serde_json::from_str::<Value>(&d.content_json).ok())
            .and_then(|ctx| {
                ctx["sources"]
                    .as_array()
                    .map(|sources| {
                        sources
                            .iter()
                            .filter_map(|s| s["path"].as_str())
                            .take(12)
                            .map(ToString::to_string)
                            .collect::<Vec<_>>()
                    })
            })
            .unwrap_or_default();

        let plan = json!({
            "goal": goal,
            "requirements": requirements,
            "constraints": constraints,
            "rejected_approaches": rejected,
            "open_questions": open_questions,
            "accepted_decisions": decisions["decisions"],
            "relevant_files": grounding_sources,
            "conversation_id": conversation_id,
        });

        let now = TimestampMillis::now().as_millis() as i64;
        let row = DesignDocumentRow {
            id: StableId::new("dplan").to_string(),
            conversation_id: conversation_id.to_string(),
            doc_type: "discuss_plan".to_string(),
            content_json: plan.to_string(),
            version: 1,
            evidence_refs: String::new(),
            created_at_ms: now,
            updated_at_ms: now,
        };
        self.db.save_design_document(&row)?;
        self.append_message(
            conversation_id,
            "assistant",
            &format!(
                "Structured plan created from this discussion.\n\nGoal: {goal}\nRequirements: {}\nOpen questions: {}\nThe plan is saved; choose Execute Plan to create a mission.",
                plan["requirements"].as_array().map(|a| a.len()).unwrap_or(0),
                plan["open_questions"].as_array().map(|a| a.len()).unwrap_or(0),
            ),
            None,
            r#"{"mode":"discuss","kind":"plan_created"}"#,
        )?;
        Ok(plan)
    }

    /// Execute a saved plan: promote it to a Kernel mission that retains the
    /// full structured context (Doc 06 §24).  Never reduces the plan to one
    /// sentence — the goal text embeds the structured contract.
    pub fn discuss_execute_plan(&mut self, conversation_id: &str) -> AcResult<Value> {
        self.ensure_running()?;
        let _conv = self.conversation_or_err(conversation_id)?;
        let plan_doc = self
            .db
            .design_document(conversation_id, "discuss_plan")?
            .ok_or_else(|| {
                AcError::validation(
                    "DISCUSS-PLAN_MISSING",
                    "turn the discussion into a plan before executing it",
                )
            })?;
        let plan: Value = serde_json::from_str(&plan_doc.content_json).map_err(|error| {
            AcError::validation("DISCUSS-PLAN_CORRUPT", format!("invalid plan JSON: {error}"))
        })?;

        let goal = Self::plan_to_goal_text(&plan);
        let (mission_id, _session_id) = self.goal_submit(conversation_id, &goal, &[])?;

        // Mark the plan promoted and keep the conversation↔plan↔mission link.
        let mut promoted = plan;
        promoted["promoted_mission_id"] = json!(mission_id.to_string());
        let now = TimestampMillis::now().as_millis() as i64;
        self.db.save_design_document(&DesignDocumentRow {
            id: plan_doc.id.clone(),
            conversation_id: conversation_id.to_string(),
            doc_type: "discuss_plan".to_string(),
            content_json: promoted.to_string(),
            version: plan_doc.version + 1,
            evidence_refs: plan_doc.evidence_refs.clone(),
            created_at_ms: plan_doc.created_at_ms,
            updated_at_ms: now,
        })?;

        Ok(json!({
            "mission_id": mission_id.to_string(),
            "plan_goal": goal,
            "conversation_id": conversation_id,
        }))
    }

    /// Accept an explicit decision derived from a specific discussion
    /// message (Doc 06 §25).  Never automatic: the user must pass the exact
    /// message id the decision comes from.
    pub fn discuss_accept_decision(
        &mut self,
        conversation_id: &str,
        message_id: &str,
        decision: &str,
        rationale: &str,
    ) -> AcResult<Value> {
        self.ensure_running()?;
        let conv = self.conversation_or_err(conversation_id)?;
        if conv.mode != "DISCUSS" {
            return Err(AcError::validation(
                "CONVERSATION-WRONG_MODE",
                "discuss_accept_decision requires a DISCUSS conversation",
            ));
        }
        if decision.trim().is_empty() {
            return Err(AcError::validation(
                "DISCUSS-DECISION_EMPTY",
                "decision text is required",
            ));
        }
        // The decision must be derived from an actual discussion message.
        let messages = self.db.messages_for_conversation(conversation_id)?;
        let source = messages
            .iter()
            .find(|m| m.id == message_id)
            .ok_or_else(|| {
                AcError::validation(
                    "DISCUSS-DECISION_SOURCE_MISSING",
                    "decision must reference a message in this conversation",
                )
            })?;
        let source_excerpt = bounded_ui_summary(&source.content, 400);

        let decision_id = StableId::new("ddecision").to_string();
        let record = json!({
            "id": decision_id,
            "conversation_id": conversation_id,
            "message_id": message_id,
            "decision": decision,
            "rationale": if rationale.trim().is_empty() {
                format!("derived from discussion message: {source_excerpt}")
            } else {
                rationale.to_string()
            },
            "source_excerpt": source_excerpt,
            "accepted_at_ms": TimestampMillis::now().as_millis() as i64,
        });

        // 1. Authoritative structured state: memory_decisions under the
        //    deterministic repository identity for this project path, so
        //    decisions are project-scoped and cannot leak across projects.
        let repo_id = project_repository_identity(&conv.project_path);
        self.db.save_memory_decision(&ac_db::MemoryDecisionRow {
            id: decision_id.clone(),
            repository_id: repo_id,
            mission_id: None,
            task_id: None,
            branch: None,
            decision: decision.to_string(),
            rationale: record["rationale"].as_str().unwrap_or("").to_string(),
            authority_refs: json!([message_id]).to_string(),
            supersedes: None,
            created_at_ms: TimestampMillis::now().as_millis() as i64,
        })?;

        // 2. Conversation-scoped visible state (appended to the decisions doc).
        let mut decisions_doc = self.discuss_decisions_value(conversation_id)?;
        if let Some(list) = decisions_doc["decisions"].as_array_mut() {
            list.push(record.clone());
        }
        let now = TimestampMillis::now().as_millis() as i64;
        self.db.save_design_document(&DesignDocumentRow {
            id: StableId::new("ddec").to_string(),
            conversation_id: conversation_id.to_string(),
            doc_type: "discuss_decisions".to_string(),
            content_json: decisions_doc.to_string(),
            version: 1,
            evidence_refs: String::new(),
            created_at_ms: now,
            updated_at_ms: now,
        })?;

        // 3. Governed materialization into DECISIONS.md (recorded change).
        let materialized =
            self.materialize_project_decisions(&conv.project_path, conversation_id)?;

        self.append_message(
            conversation_id,
            "assistant",
            &format!(
                "Decision accepted: {decision}\nIt is now part of project memory and \
                 DECISIONS.md (change recorded: {materialized})."
            ),
            None,
            &json!({
                "mode":"discuss",
                "kind":"decision_accepted",
                "decision_id": decision_id
            })
            .to_string(),
        )?;
        Ok(record)
    }

    /// Render the structured plan as the mission goal text — the mission
    /// retains full plan context (never one raw sentence).
    fn plan_to_goal_text(plan: &Value) -> String {
        let goal = plan.get("goal").and_then(Value::as_str).unwrap_or("");
        let requirements = plan_string_array(plan.get("requirements"), 12);
        let constraints = plan_string_array(plan.get("constraints"), 12);
        let rejected = plan_string_array(plan.get("rejected_approaches"), 12);
        let open = plan_string_array(plan.get("open_questions"), 12);
        let decisions = plan
            .get("accepted_decisions")
            .and_then(Value::as_array)
            .map(|items| {
                items
                    .iter()
                    .filter_map(|d| d.get("decision").and_then(Value::as_str))
                    .take(12)
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        let files = plan_string_array(plan.get("relevant_files"), 12);

        let mut out = String::new();
        out.push_str(&format!("GOAL: {goal}\n\n"));
        out.push_str("This goal was promoted from a structured Discuss plan:\n\n");
        if !requirements.is_empty() {
            out.push_str("Requirements:\n");
            for item in &requirements {
                out.push_str(&format!("- {item}\n"));
            }
            out.push('\n');
        }
        if !decisions.is_empty() {
            out.push_str("Accepted decisions from the discussion:\n");
            for item in &decisions {
                out.push_str(&format!("- {item}\n"));
            }
            out.push('\n');
        }
        if !constraints.is_empty() {
            out.push_str("Constraints:\n");
            for item in &constraints {
                out.push_str(&format!("- {item}\n"));
            }
            out.push('\n');
        }
        if !rejected.is_empty() {
            out.push_str("Rejected approaches (do not pursue):\n");
            for item in &rejected {
                out.push_str(&format!("- {item}\n"));
            }
            out.push('\n');
        }
        if !files.is_empty() {
            out.push_str("Relevant files:\n");
            for item in &files {
                out.push_str(&format!("- {item}\n"));
            }
            out.push('\n');
        }
        if !open.is_empty() {
            out.push_str("Open questions to resolve while working:\n");
            for item in &open {
                out.push_str(&format!("- {item}\n"));
            }
        }
        bounded_ui_summary(&out, 12 * 1024)
    }

    /// Model-assisted plan extraction.  Returns None when no provider is
    /// reachable so the deterministic fallback builds the plan.
    fn discuss_extract_plan_via_model(&self, transcript: &str) -> Option<Value> {
        let db_path = self.db_path.clone();
        let mut providers = crate::daemon_provider_registry(&self.db, &db_path).ok()?;
        let mut profile = ac_provider::TaskProfile::discuss(
            StableId::new("discussplan"),
            self.preferred_routing_profile(), // N1: user-persisted routing profile
        );
        profile.required_context = 2048;
        let cancel = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let prompt = format!(
            "Extract a structured plan from this discussion transcript. Respond ONLY with \
             JSON: {{\"goal\": str, \"requirements\": [str], \"constraints\": [str], \
             \"rejected_approaches\": [str], \"open_questions\": [str]}}. No prose.\n\n\
             {transcript}"
        );
        let result = providers.request_model(
            &profile,
            prompt,
            2048,
            &|| cancel.load(std::sync::atomic::Ordering::Relaxed),
        );
        let execution = result.ok()?;
        let text = ac_agent::provider_events_text(&execution.events);
        let start = text.find('{')?;
        let end = text.rfind('}')?;
        let parsed: Value = serde_json::from_str(&text[start..=end]).ok()?;
        if parsed.get("goal").and_then(Value::as_str).is_some() {
            Some(parsed)
        } else {
            None
        }
    }

    fn discuss_decisions_value(&self, conversation_id: &str) -> AcResult<Value> {
        Ok(self
            .db
            .design_document(conversation_id, "discuss_decisions")?
            .and_then(|d| serde_json::from_str::<Value>(&d.content_json).ok())
            .unwrap_or_else(|| json!({"decisions": [], "source_paths": []})))
    }

    /// Governed materialization of accepted project decisions into
    /// DECISIONS.md: the write happens in the project worktree and the
    /// change is recorded in the DB so the evidence path can audit it.
    /// Returns the materialization record id, or "skipped: no decisions".
    fn materialize_project_decisions(
        &self,
        project_path: &str,
        conversation_id: &str,
    ) -> AcResult<String> {
        let decisions_doc = self.discuss_decisions_value(conversation_id)?;
        let decisions = decisions_doc["decisions"].as_array().cloned().unwrap_or_default();
        if decisions.is_empty() {
            return Ok("skipped: no decisions".to_string());
        }
        let path = std::path::Path::new(project_path).join("DECISIONS.md");
        let mut content = String::from("# Project Decisions\n\n");
        content.push_str("Decisions accepted through AgentCode Discuss Mode.\n\n");
        for decision in &decisions {
            let text = decision["decision"].as_str().unwrap_or("");
            let rationale = decision["rationale"].as_str().unwrap_or("");
            let at = decision["accepted_at_ms"].as_i64().unwrap_or(0);
            content.push_str(&format!("- **{text}** — {rationale} (accepted {at})\n"));
        }
        // Governed write: filesystem write + DB-recorded change.
        std::fs::write(&path, content).map_err(|error| {
            AcError::validation(
                "DISCUSS-DECISIONS_MATERIALIZATION",
                format!("could not write DECISIONS.md: {error}"),
            )
        })?;
        let record_id = StableId::new("dmat");
        let now = TimestampMillis::now().as_millis() as i64;
        let row = DesignDocumentRow {
            id: record_id.to_string(),
            conversation_id: conversation_id.to_string(),
            doc_type: "decisions_materialization".to_string(),
            content_json: json!({
                "path": path.to_string_lossy(),
                "decision_count": decisions.len(),
            })
            .to_string(),
            version: 1,
            evidence_refs: String::new(),
            created_at_ms: now,
            updated_at_ms: now,
        };
        self.db.save_design_document(&row)?;
        Ok(record_id.to_string())
    }

    fn conversation_or_err(&self, conversation_id: &str) -> AcResult<ac_db::ConversationRow> {
        self.db
            .conversation(conversation_id)?
            .ok_or_else(|| AcError::validation("CONVERSATION-NOT_FOUND", "conversation not found"))
    }
}

/// Deterministic project identity: decisions stored under this id belong to
/// this exact project path.  Two projects can never share decision rows.
fn project_repository_identity(project_path: &str) -> String {
    let hash = fnv1a64_hash(project_path.as_bytes());
    format!("repo-{hash}")
}

fn plan_string_array(value: Option<&Value>, cap: usize) -> Vec<String> {
    value
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .filter(|s| !s.trim().is_empty())
                .take(cap)
                .map(ToString::to_string)
                .collect()
        })
        .unwrap_or_default()
}
