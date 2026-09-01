// ── Conversation Service Methods ─────────────────────────────────────────────

impl DaemonService {
    pub fn create_conversation(
        &self,
        project_path: &str,
        mode: &str,
        title: &str,
    ) -> AcResult<StableId> {
        self.ensure_running()?;
        let id = StableId::new("conv");
        let now = TimestampMillis::now().as_millis() as i64;
        let row = ac_db::ConversationRow {
            id: id.to_string(),
            project_path: project_path.to_string(),
            mode: mode.to_string(),
            title: title.to_string(),
            state: "active".to_string(),
            current_mission_id: None,
            created_at_ms: now,
            updated_at_ms: now,
        };
        self.db.save_conversation(&row)?;
        Ok(id)
    }

    pub fn list_conversations(&self, project_path: &str) -> AcResult<Vec<ac_db::ConversationRow>> {
        self.db.conversations_for_project(project_path, true)
    }

    pub fn get_conversation(&self, id: &str) -> AcResult<Option<ac_db::ConversationRow>> {
        self.db.conversation(id)
    }

    pub fn get_conversation_with_messages(
        &self,
        id: &str,
    ) -> AcResult<Option<ConversationWithMessages>> {
        let conv = self.db.conversation(id)?;
        match conv {
            Some(c) => {
                let messages = self.db.messages_for_conversation(id)?;
                let attachments = self.db.attachments_for_conversation(id)?;
                Ok(Some(ConversationWithMessages {
                    conversation: c,
                    messages,
                    attachments,
                }))
            }
            None => Ok(None),
        }
    }

    pub fn rename_conversation(&self, id: &str, title: &str) -> AcResult<()> {
        self.ensure_running()?;
        if title.trim().is_empty() {
            return Err(AcError::validation(
                "CONVERSATION-EMPTY_TITLE",
                "conversation title must not be empty",
            ));
        }
        self.db.rename_conversation(id, title)
    }

    pub fn archive_conversation(&self, id: &str) -> AcResult<()> {
        self.ensure_running()?;
        self.db.archive_conversation(id)
    }

    pub fn delete_conversation(&self, id: &str) -> AcResult<()> {
        self.ensure_running()?;
        self.db.delete_conversation(id)
    }

    pub fn append_message(
        &self,
        conversation_id: &str,
        role: &str,
        content: &str,
        mission_ref: Option<&str>,
        metadata_json: &str,
    ) -> AcResult<ac_db::ConversationMessageRow> {
        self.ensure_running()?;
        if self.db.conversation(conversation_id)?.is_none() {
            return Err(AcError::validation(
                "CONVERSATION-NOT_FOUND",
                "conversation not found",
            ));
        }
        if role.trim().is_empty() || content.trim().is_empty() {
            return Err(AcError::validation(
                "CONVERSATION-INVALID_MESSAGE",
                "role and content are required",
            ));
        }
        let id = StableId::new("msg");
        let now = TimestampMillis::now().as_millis() as i64;
        let row = ac_db::ConversationMessageRow {
            id: id.to_string(),
            conversation_id: conversation_id.to_string(),
            role: role.to_string(),
            content: content.to_string(),
            mission_ref: mission_ref.map(ToString::to_string),
            metadata_json: metadata_json.to_string(),
            created_at_ms: now,
        };
        self.db.save_message(&row)?;
        self.db.touch_conversation(conversation_id)?;
        Ok(row)
    }

    /// Register an attachment whose file bytes have already been written to a
    /// path within the project workspace by the UI layer.  The daemon validates
    /// the path, verifies the file content, and persists the metadata row.
    #[allow(clippy::too_many_arguments)]
    pub fn register_attachment(
        &self,
        conversation_id: &str,
        project_path: &str,
        filename: &str,
        mime_type: &str,
        size_bytes: i64,
        content_hash: &str,
        rel_path: &str,
    ) -> AcResult<ac_db::AttachmentRow> {
        self.ensure_running()?;
        let conv = self
            .db
            .conversation(conversation_id)?
            .ok_or_else(|| {
                AcError::validation("CONVERSATION-NOT_FOUND", "conversation not found")
            })?;
        if conv.project_path != project_path {
            return Err(AcError::policy_denied(
                "CONVERSATION-PROJECT_MISMATCH",
                "attachment project does not match conversation project",
            ));
        }
        // Validate MIME type
        let mime_ok = mime_type == "image/png"
            || mime_type == "image/jpeg"
            || mime_type == "image/gif"
            || mime_type == "image/webp"
            || mime_type == "text/plain"
            || mime_type == "text/markdown"
            || mime_type == "application/json"
            || mime_type == "application/pdf"
            || mime_type.starts_with("text/")
            || mime_type == "application/octet-stream";
        if !mime_ok {
            return Err(AcError::validation(
                "CONVERSATION-INVALID_MIME",
                format!("attachment MIME type not allowed: {mime_type}"),
            ));
        }
        // Validate size
        if size_bytes <= 0 || size_bytes > 25 * 1024 * 1024 {
            return Err(AcError::validation(
                "CONVERSATION-INVALID_SIZE",
                "attachment size must be 1-25MB",
            ));
        }
        // Sanitize filename
        let sanitized = sanitize_filename(filename);
        if sanitized.is_empty() {
            return Err(AcError::validation(
                "CONVERSATION-INVALID_FILENAME",
                "filename contains no valid characters",
            ));
        }
        // Validate that the file path is within the project workspace
        let project_canonical = fs::canonicalize(project_path).map_err(|_| {
            AcError::validation("CONVERSATION-PROJECT_PATH", "project path is not accessible")
        })?;
        let file_path = Path::new(project_path).join(rel_path);
        let file_canonical = fs::canonicalize(&file_path).map_err(|_| {
            AcError::validation(
                "CONVERSATION-FILE_NOT_FOUND",
                "attachment file not found on disk",
            )
        })?;
        if !file_canonical.starts_with(&project_canonical) {
            return Err(AcError::policy_denied(
                "CONVERSATION-FILE_ESCAPE",
                "attachment file path escapes the project workspace",
            ));
        }
        // Verify file size and hash
        let mut file = File::open(&file_canonical).map_err(|_| {
            AcError::validation("CONVERSATION-FILE_READ", "could not read attachment file")
        })?;
        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes).map_err(|_| {
            AcError::validation("CONVERSATION-FILE_READ", "could not read attachment bytes")
        })?;
        if bytes.len() as i64 != size_bytes {
            return Err(AcError::validation(
                "CONVERSATION-SIZE_MISMATCH",
                "declared size does not match actual file size",
            ));
        }
        let actual_hash = fnv1a64_hash(&bytes);
        if actual_hash != content_hash {
            return Err(AcError::validation(
                "CONVERSATION-HASH_MISMATCH",
                "declared hash does not match actual file content",
            ));
        }
        let attachment_id = StableId::new("att");
        let storage_key = rel_path.to_string();
        let now = TimestampMillis::now().as_millis() as i64;
        let row = ac_db::AttachmentRow {
            id: attachment_id.to_string(),
            conversation_id: conversation_id.to_string(),
            message_id: None,
            project_path: project_path.to_string(),
            filename: sanitized,
            mime_type: mime_type.to_string(),
            size_bytes,
            sha256: content_hash.to_string(),
            storage_key,
            sensitivity: "normal".to_string(),
            created_at_ms: now,
        };
        self.db.save_attachment(&row)?;
        Ok(row)
    }

    pub fn list_attachments(&self, conversation_id: &str) -> AcResult<Vec<ac_db::AttachmentRow>> {
        self.db.attachments_for_conversation(conversation_id)
    }

    /// Get the absolute file path for an attachment, validated within the
    /// project workspace.  Returns the path after project isolation check.
    pub fn attachment_path(
        &self,
        attachment_id: &str,
        project_path: &str,
    ) -> AcResult<Option<String>> {
        let att = match self.db.attachment(attachment_id)? {
            Some(a) => a,
            None => return Ok(None),
        };
        if att.project_path != project_path {
            return Err(AcError::policy_denied(
                "CONVERSATION-PROJECT_MISMATCH",
                "attachment does not belong to this project",
            ));
        }
        let project_canonical = fs::canonicalize(project_path).map_err(|_| {
            AcError::validation("CONVERSATION-PROJECT_PATH", "project path is not accessible")
        })?;
        let file_path = Path::new(&project_path).join(&att.storage_key);
        let file_canonical = fs::canonicalize(&file_path).map_err(|_| {
            AcError::validation(
                "CONVERSATION-FILE_NOT_FOUND",
                "attachment file not found on disk",
            )
        })?;
        if !file_canonical.starts_with(&project_canonical) {
            return Err(AcError::policy_denied(
                "CONVERSATION-FILE_ESCAPE",
                "attachment file path escapes the project workspace",
            ));
        }
        Ok(Some(file_canonical.to_string_lossy().to_string()))
    }

    pub fn remove_attachment(&self, id: &str) -> AcResult<()> {
        self.ensure_running()?;
        let att = self.db.attachment(id)?.ok_or_else(|| {
            AcError::validation("CONVERSATION-ATTACHMENT_NOT_FOUND", "attachment not found")
        })?;
        let file_path = Path::new(&att.project_path).join(&att.storage_key);
        if file_path.exists() {
            let _ = fs::remove_file(&file_path);
        }
        self.db.delete_attachment(id)
    }

    /// Submit a goal as a mission from a conversation.  Preserves the exact
    /// original request, persists it as a user message, creates a real mission
    /// through the existing backend, and links the mission to the conversation.
    pub fn goal_submit(
        &mut self,
        conversation_id: &str,
        goal: &str,
        attachment_ids: &[String],
    ) -> AcResult<(StableId, StableId)> {
        self.ensure_running()?;
        if goal.trim().is_empty() {
            return Err(AcError::validation(
                "CONVERSATION-EMPTY_GOAL",
                "goal must not be empty",
            ));
        }
        let conv = self
            .db
            .conversation(conversation_id)?
            .ok_or_else(|| {
                AcError::validation("CONVERSATION-NOT_FOUND", "conversation not found")
            })?;
        let workspace = resolve_workspace_root(Some(&conv.project_path), &self.default_workspace_root)?;
        let mut kernel = self.kernel_lock()?;
        let mission_id = kernel.create_mission(goal.to_string())?;
        kernel.transition_mission(&mission_id, ac_kernel::MissionState::Active, Vec::new())?;
        if let Some(mission) = kernel.mission(&mission_id) {
            self.db.put_mission(mission)?;
        }
        for event in kernel.events() {
            let _ = self.db.append_kernel_event(event);
        }
        let session = ac_runtime::AgentSession::new(ac_runtime::Worker::new());
        self.db.save_session(
            session.id(),
            &mission_id,
            session_state(session.state()),
            Some(workspace.to_string_lossy().as_ref()),
        )?;
        self.coordinator.enqueue(QueuedMission {
            mission_id: mission_id.clone(),
            session_id: session.id().clone(),
            goal: goal.to_string(),
            workspace_root: workspace,
            attachments: self.load_goal_attachments(attachment_ids, &conv.project_path)?,
        })?;
        // Persist the user's goal as a message with mission_ref
        let now = TimestampMillis::now().as_millis() as i64;
        let msg_id = StableId::new("msg");
        let msg_row = ac_db::ConversationMessageRow {
            id: msg_id.to_string(),
            conversation_id: conversation_id.to_string(),
            role: "user".to_string(),
            content: goal.to_string(),
            mission_ref: Some(mission_id.to_string()),
            metadata_json: "{}".to_string(),
            created_at_ms: now,
        };
        self.db.save_message(&msg_row)?;
        for att_id in attachment_ids {
            let _ = self.db.link_message_attachment(att_id, &msg_id.to_string());
        }
        self.db.set_conversation_mission(conversation_id, &mission_id.to_string())?;
        Ok((mission_id, session.id().clone()))
    }

    /// DiscussSend: append a user message, build a bounded project-aware
    /// prompt, call the provider for a conversational answer, and persist
    /// the assistant response.  The provider/model routing decision is
    /// persisted through the agent durability path.  This is a read-only
    /// conversational exchange — no repository mutation occurs.
    pub fn discuss_send(
        &mut self,
        conversation_id: &str,
        content: &str,
        attachment_ids: &[String],
    ) -> AcResult<ac_db::ConversationMessageRow> {
        self.ensure_running()?;
        let conv = self
            .db
            .conversation(conversation_id)?
            .ok_or_else(|| {
                AcError::validation("CONVERSATION-NOT_FOUND", "conversation not found")
            })?;
        if conv.mode != "DISCUSS" {
            return Err(AcError::validation(
                "CONVERSATION-WRONG_MODE",
                "discuss_send requires a DISCUSS conversation",
            ));
        }
        if content.trim().is_empty() {
            return Err(AcError::validation(
                "CONVERSATION-EMPTY_CONTENT",
                "message content must not be empty",
            ));
        }
        // Append user message and link attachments
        let user_msg = self.append_message(conversation_id, "user", content, None, "{}")?;
        for att_id in attachment_ids {
            let _ = self.db.link_message_attachment(att_id, &user_msg.id);
        }
        // Load bounded attachment content through the existing security pipeline
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
        // Build bounded context: recent messages + project context + attachments
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
        let prompt = format!(
            "{}\n{}\n{}\n{}\n\n--\nProvide a helpful, project-aware conversational response. \
             Do NOT write code, modify files, or execute commands. \
             Only discuss the project, architecture, design, and implementation ideas.\n",
            project_hint, project_files, recent, attachment_block,
        );
        // Build a lightweight provider registry and request a conversational answer
        let db_path = self.db_path.clone();
        let mut providers = crate::daemon_provider_registry(&self.db, &db_path)
            .map_err(|error| {
                AcError::validation(
                    "DISCUSS-PROVIDER_SETUP",
                    format!("cannot initialize provider registry: {error}"),
                )
            })?;
        let profile = ac_provider::TaskProfile::discuss(
            ac_common::StableId::new("discuss"),
            ac_provider::RoutingProfile::LocalFirst,
        );
        // Bound the required context so already-installed small local models
        // (e.g. qwen2.5-coder:3b with an 8K window) satisfy the routing check.
        let mut profile = profile;
        profile.required_context = 4096;
        let cancel = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let result = providers.request_model(
            &profile,
            prompt,
            4096,
            &|| cancel.load(std::sync::atomic::Ordering::Relaxed),
        );
        let content = match result {
            Ok(ref execution) => {
                let text = ac_agent::provider_events_text(&execution.events);
                // Persist the provider/model routing record through the daemon path
                let mission_id = StableId::new("discuss");
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
                        created_at_ms: ac_common::TimestampMillis::now().as_millis() as i64,
                    };
                    if let Ok(dur) = ac_db::ControlPlaneDb::open(&db_path) {
                        let _ = dur.save_provider_model_record(&ac_db::ProviderModelRecordRow {
                            id: format!("pmr-discuss-{}", StableId::new("record")),
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
            Err(ref failure) => {
                format!("[provider unavailable: {failure:?}]")
            }
        };
        // Append assistant message
        let metadata = json!({
            "mode": "discuss",
            "provider_model": result.as_ref().ok().and_then(|exec| {
                exec.decision.selected.as_ref().map(|s| {
                    json!({"provider_id": s.provider_id.to_string(), "model_name": s.model_name})
                })
            }),
        });
        self.append_message(
            conversation_id,
            "assistant",
            &content,
            None,
            &metadata.to_string(),
        )
    }

    /// Read bounded attachment content from the workspace, validate security
    /// boundaries, and produce typed `ContextAttachment` values for the model
    /// context pipeline.  Each attachment is independently bounded:
    /// - text/code: up to 32 KB extracted content
    /// - images: reference only, never raw bytes
    /// - PDFs/documents: metadata + unsupported extraction state
    /// - oversized or binary: document-unsupported
    fn load_goal_attachments(
        &self,
        attachment_ids: &[String],
        project_path: &str,
    ) -> AcResult<Vec<ContextAttachment>> {
        let mut result = Vec::new();
        for att_id in attachment_ids {
            let row = match self.db.attachment(att_id)? {
                Some(r) => r,
                None => continue,
            };
            if row.project_path != project_path {
                continue;
            }
            // A registered-but-missing file must be skipped, never crash the
            // goal submission.  Only an out-of-project or escaping attachment
            // is a hard error (workspace boundary policy).
            let file_path = match self.attachment_path(att_id, project_path) {
                Ok(Some(p)) => p,
                Ok(None) => continue,
                Err(error)
                    if error.code() == "CONVERSATION-FILE_NOT_FOUND"
                        || error.code() == "CONVERSATION-PROJECT_PATH" =>
                {
                    continue;
                }
                Err(error) => return Err(error),
            };
            let mime = row.mime_type.as_str();
            let content = if mime.starts_with("text/")
                || mime == "application/json"
                || mime == "application/octet-stream"
            {
                let mut bounded = Vec::with_capacity(32768.min(row.size_bytes as usize));
                if let Ok(mut f) = std::fs::File::open(&file_path) {
                    let mut buf = [0u8; 32768];
                    let n = f.read(&mut buf).unwrap_or(0);
                    bounded.extend_from_slice(&buf[..n]);
                }
                let text = String::from_utf8_lossy(&bounded).to_string();
                AttachmentContent::Text(text)
            } else if mime.starts_with("image/") {
                // Reference only — never raw bytes or absolute workspace paths.
                AttachmentContent::Image {
                    reference: row.storage_key.clone(),
                }
            } else {
                AttachmentContent::Document { unsupported: true }
            };
            result.push(ContextAttachment {
                attachment_id: att_id.clone(),
                filename: row.filename,
                mime_type: row.mime_type,
                project_path: row.project_path.clone(),
                conversation_id: row.conversation_id.clone(),
                content,
            });
        }
        Ok(result)
    }
}

#[derive(Clone, Debug)]
pub struct ConversationWithMessages {
    pub conversation: ac_db::ConversationRow,
    pub messages: Vec<ac_db::ConversationMessageRow>,
    pub attachments: Vec<ac_db::AttachmentRow>,
}

// ── Helpers ──────────────────────────────────────────────────────────────────

/// Bounded project file listing for Discuss context.  Reads at most 80 files
/// from the project root (non-recursive, skipping hidden/dependency dirs) so
/// the model has relevant project context without injecting the entire repo.
fn bounded_project_listing(project_path: &str, max_files: usize) -> String {
    let dir = std::path::Path::new(project_path);
    if !dir.is_dir() {
        return String::new();
    }
    let mut entries = Vec::new();
    if let Ok(read) = std::fs::read_dir(dir) {
        for entry in read.flatten().take(max_files) {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with('.') || name == "node_modules" || name == "target" {
                continue;
            }
            let meta = entry.metadata().ok();
            let is_dir = meta.as_ref().map(|m| m.is_dir()).unwrap_or(false);
            let kind = if is_dir { "dir" } else { "file" };
            let size = meta.as_ref().map(|m| m.len()).unwrap_or(0);
            entries.push(format!("  {name} ({kind}, {size} bytes)"));
        }
    }
    if entries.is_empty() {
        String::new()
    } else {
        format!("Project files:\n{}\n", entries.join("\n"))
    }
}

/// FNV-1a 64-bit hash of file bytes, producing a "fnv1a64:" hex string.
/// Consistent with ac-evidence::content_hash for uniformity.
fn fnv1a64_hash(bytes: &[u8]) -> String {
    let hash = bytes
        .iter()
        .fold(0xcbf29ce484222325_u64, |hash, byte| {
            (hash ^ u64::from(*byte)).wrapping_mul(0x100000001b3)
        });
    format!("fnv1a64:{hash:016x}")
}

fn sanitize_filename(name: &str) -> String {
    let name = std::path::Path::new(name)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("attachment");
    name.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.' {
                c
            } else {
                '_'
            }
        })
        .collect::<String>()
        .trim_matches('.')
        .to_string()
}