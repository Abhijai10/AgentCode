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
}

#[derive(Clone, Debug)]
pub struct ConversationWithMessages {
    pub conversation: ac_db::ConversationRow,
    pub messages: Vec<ac_db::ConversationMessageRow>,
    pub attachments: Vec<ac_db::AttachmentRow>,
}

// ── Helpers ──────────────────────────────────────────────────────────────────

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