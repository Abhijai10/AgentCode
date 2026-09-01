impl ControlPlaneDb {
    // ── Conversations ──────────────────────────────────────────────────────────

    pub fn save_conversation(&self, row: &ConversationRow) -> AcResult<()> {
        self.connection
            .execute(
                "INSERT INTO conversations VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
                 ON CONFLICT(id) DO UPDATE SET
                 title=excluded.title, state=excluded.state,
                 current_mission_id=excluded.current_mission_id,
                 updated_at_ms=excluded.updated_at_ms",
                params![
                    row.id, row.project_path, row.mode, row.title, row.state,
                    row.current_mission_id, row.created_at_ms, row.updated_at_ms
                ],
            )
            .map_err(db_error)?;
        Ok(())
    }

    pub fn conversation(&self, id: &str) -> AcResult<Option<ConversationRow>> {
        self.connection
            .query_row(
                "SELECT id, project_path, mode, title, state, current_mission_id,
                 created_at_ms, updated_at_ms FROM conversations WHERE id=?1",
                [id],
                |row| {
                    Ok(ConversationRow {
                        id: row.get(0)?,
                        project_path: row.get(1)?,
                        mode: row.get(2)?,
                        title: row.get(3)?,
                        state: row.get(4)?,
                        current_mission_id: row.get(5)?,
                        created_at_ms: row.get(6)?,
                        updated_at_ms: row.get(7)?,
                    })
                },
            )
            .optional()
            .map_err(db_error)
    }

    pub fn conversations_for_project(
        &self,
        project_path: &str,
        include_archived: bool,
    ) -> AcResult<Vec<ConversationRow>> {
        let sql = if include_archived {
            "SELECT id, project_path, mode, title, state, current_mission_id,
             created_at_ms, updated_at_ms FROM conversations
             WHERE project_path=?1 AND state!='deleted'
             ORDER BY updated_at_ms DESC"
        } else {
            "SELECT id, project_path, mode, title, state, current_mission_id,
             created_at_ms, updated_at_ms FROM conversations
             WHERE project_path=?1 AND state='active'
             ORDER BY updated_at_ms DESC"
        };
        let mut stmt = self.connection.prepare(sql).map_err(db_error)?;
        let rows = stmt
            .query_map([project_path], |row| {
                Ok(ConversationRow {
                    id: row.get(0)?,
                    project_path: row.get(1)?,
                    mode: row.get(2)?,
                    title: row.get(3)?,
                    state: row.get(4)?,
                    current_mission_id: row.get(5)?,
                    created_at_ms: row.get(6)?,
                    updated_at_ms: row.get(7)?,
                })
            })
            .map_err(db_error)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(db_error)
    }

    pub fn rename_conversation(&self, id: &str, title: &str) -> AcResult<()> {
        let now = TimestampMillis::now().as_millis() as i64;
        self.connection
            .execute(
                "UPDATE conversations SET title=?1, updated_at_ms=?2 WHERE id=?3",
                params![title, now, id],
            )
            .map_err(db_error)?;
        Ok(())
    }

    pub fn touch_conversation(&self, id: &str) -> AcResult<()> {
        let now = TimestampMillis::now().as_millis() as i64;
        self.connection
            .execute(
                "UPDATE conversations SET updated_at_ms=?1 WHERE id=?2",
                params![now, id],
            )
            .map_err(db_error)?;
        Ok(())
    }

    pub fn archive_conversation(&self, id: &str) -> AcResult<()> {
        let now = TimestampMillis::now().as_millis() as i64;
        self.connection
            .execute(
                "UPDATE conversations SET state='archived', updated_at_ms=?1 WHERE id=?2",
                params![now, id],
            )
            .map_err(db_error)?;
        Ok(())
    }

    pub fn delete_conversation(&self, id: &str) -> AcResult<()> {
        let now = TimestampMillis::now().as_millis() as i64;
        self.connection
            .execute(
                "UPDATE conversations SET state='deleted', current_mission_id=NULL, updated_at_ms=?1 WHERE id=?2",
                params![now, id],
            )
            .map_err(db_error)?;
        Ok(())
    }

    pub fn set_conversation_mission(
        &self,
        id: &str,
        mission_id: &str,
    ) -> AcResult<()> {
        let now = TimestampMillis::now().as_millis() as i64;
        self.connection
            .execute(
                "UPDATE conversations SET current_mission_id=?1, updated_at_ms=?2 WHERE id=?3",
                params![mission_id, now, id],
            )
            .map_err(db_error)?;
        Ok(())
    }

    // ── Messages ───────────────────────────────────────────────────────────────

    pub fn save_message(&self, row: &ConversationMessageRow) -> AcResult<()> {
        self.connection
            .execute(
                "INSERT INTO conversation_messages VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![
                    row.id, row.conversation_id, row.role, row.content,
                    row.mission_ref, row.metadata_json, row.created_at_ms
                ],
            )
            .map_err(db_error)?;
        Ok(())
    }

    pub fn messages_for_conversation(
        &self,
        conversation_id: &str,
    ) -> AcResult<Vec<ConversationMessageRow>> {
        let mut stmt = self
            .connection
            .prepare(
                "SELECT id, conversation_id, role, content, mission_ref, metadata_json,
                 created_at_ms FROM conversation_messages
                 WHERE conversation_id=?1 ORDER BY created_at_ms ASC",
            )
            .map_err(db_error)?;
        let rows = stmt
            .query_map([conversation_id], |row| {
                Ok(ConversationMessageRow {
                    id: row.get(0)?,
                    conversation_id: row.get(1)?,
                    role: row.get(2)?,
                    content: row.get(3)?,
                    mission_ref: row.get(4)?,
                    metadata_json: row.get(5)?,
                    created_at_ms: row.get(6)?,
                })
            })
            .map_err(db_error)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(db_error)
    }

    // ── Attachments ─────────────────────────────────────────────────────────────

    pub fn save_attachment(&self, row: &AttachmentRow) -> AcResult<()> {
        self.connection
            .execute(
                "INSERT INTO attachments VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
                params![
                    row.id, row.conversation_id, row.message_id, row.project_path,
                    row.filename, row.mime_type, row.size_bytes, row.sha256,
                    row.storage_key, row.sensitivity, row.created_at_ms
                ],
            )
            .map_err(db_error)?;
        Ok(())
    }

    pub fn attachment(&self, id: &str) -> AcResult<Option<AttachmentRow>> {
        self.connection
            .query_row(
                "SELECT id, conversation_id, message_id, project_path, filename, mime_type,
                 size_bytes, sha256, storage_key, sensitivity, created_at_ms
                 FROM attachments WHERE id=?1",
                [id],
                |row| {
                    Ok(AttachmentRow {
                        id: row.get(0)?,
                        conversation_id: row.get(1)?,
                        message_id: row.get(2)?,
                        project_path: row.get(3)?,
                        filename: row.get(4)?,
                        mime_type: row.get(5)?,
                        size_bytes: row.get(6)?,
                        sha256: row.get(7)?,
                        storage_key: row.get(8)?,
                        sensitivity: row.get(9)?,
                        created_at_ms: row.get(10)?,
                    })
                },
            )
            .optional()
            .map_err(db_error)
    }

    pub fn attachments_for_conversation(
        &self,
        conversation_id: &str,
    ) -> AcResult<Vec<AttachmentRow>> {
        let mut stmt = self
            .connection
            .prepare(
                "SELECT id, conversation_id, message_id, project_path, filename, mime_type,
                 size_bytes, sha256, storage_key, sensitivity, created_at_ms
                 FROM attachments WHERE conversation_id=?1 ORDER BY created_at_ms ASC",
            )
            .map_err(db_error)?;
        let rows = stmt
            .query_map([conversation_id], |row| {
                Ok(AttachmentRow {
                    id: row.get(0)?,
                    conversation_id: row.get(1)?,
                    message_id: row.get(2)?,
                    project_path: row.get(3)?,
                    filename: row.get(4)?,
                    mime_type: row.get(5)?,
                    size_bytes: row.get(6)?,
                    sha256: row.get(7)?,
                    storage_key: row.get(8)?,
                    sensitivity: row.get(9)?,
                    created_at_ms: row.get(10)?,
                })
            })
            .map_err(db_error)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(db_error)
    }

    pub fn delete_attachment(&self, id: &str) -> AcResult<()> {
        self.connection
            .execute("DELETE FROM attachments WHERE id=?1", [id])
            .map_err(db_error)?;
        Ok(())
    }

    pub fn link_message_attachment(
        &self,
        attachment_id: &str,
        message_id: &str,
    ) -> AcResult<()> {
        self.connection
            .execute(
                "UPDATE attachments SET message_id=?1 WHERE id=?2",
                params![message_id, attachment_id],
            )
            .map_err(db_error)?;
        Ok(())
    }
}

#[cfg(test)]
mod conversation_tests {
    use super::*;

    fn db() -> ControlPlaneDb {
        let mut db = ControlPlaneDb::open_memory().unwrap();
        db.migrate().unwrap();
        db
    }

    fn row(id: &str, project: &str) -> ConversationRow {
        let now = TimestampMillis::now().as_millis() as i64;
        ConversationRow {
            id: id.to_string(),
            project_path: project.to_string(),
            mode: "GOAL".to_string(),
            title: "Test Chat".to_string(),
            state: "active".to_string(),
            current_mission_id: None,
            created_at_ms: now,
            updated_at_ms: now,
        }
    }

    fn message(id: &str, conversation_id: &str, role: &str, content: &str) -> ConversationMessageRow {
        ConversationMessageRow {
            id: id.to_string(),
            conversation_id: conversation_id.to_string(),
            role: role.to_string(),
            content: content.to_string(),
            mission_ref: None,
            metadata_json: "{}".to_string(),
            created_at_ms: TimestampMillis::now().as_millis() as i64,
        }
    }

    fn attachment(
        id: &str,
        conversation_id: &str,
        project_path: &str,
        filename: &str,
        mime: &str,
    ) -> AttachmentRow {
        AttachmentRow {
            id: id.to_string(),
            conversation_id: conversation_id.to_string(),
            message_id: None,
            project_path: project_path.to_string(),
            filename: filename.to_string(),
            mime_type: mime.to_string(),
            size_bytes: 1024,
            sha256: "abc123".to_string(),
            storage_key: id.to_string(),
            sensitivity: "normal".to_string(),
            created_at_ms: TimestampMillis::now().as_millis() as i64,
        }
    }

    #[test]
    fn create_conversation() {
        let db = db();
        let r = row("c1", "/proj/a");
        db.save_conversation(&r).unwrap();
        let loaded = db.conversation("c1").unwrap().unwrap();
        assert_eq!(loaded.title, "Test Chat");
        assert_eq!(loaded.project_path, "/proj/a");
        assert_eq!(loaded.mode, "GOAL");
        assert_eq!(loaded.state, "active");
    }

    #[test]
    fn conversation_persistence_across_reopen() {
        let dir = std::env::temp_dir().join(format!("acdb-conv-{}", StableId::new("tmp")));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("conv.sqlite");
        let now = TimestampMillis::now().as_millis() as i64;
        {
            let mut db = ControlPlaneDb::open(&path).unwrap();
            db.migrate().unwrap();
            db.save_conversation(&ConversationRow {
                id: "c2".to_string(),
                project_path: "/proj/b".to_string(),
                mode: "GOAL".to_string(),
                title: "Persisted Chat".to_string(),
                state: "active".to_string(),
                current_mission_id: Some("mission-x".to_string()),
                created_at_ms: now,
                updated_at_ms: now,
            })
            .unwrap();
            let m = message("m2", "c2", "user", "persist me");
            db.save_message(&m).unwrap();
            let a = attachment("att9", "c2", "/proj/b", "note.txt", "text/plain");
            db.save_attachment(&a).unwrap();
        }
        // Reopen the same file — data must survive the restart.
        let mut db = ControlPlaneDb::open(&path).unwrap();
        db.migrate().unwrap();
        let loaded = db.conversation("c2").unwrap().unwrap();
        assert_eq!(loaded.title, "Persisted Chat");
        assert_eq!(loaded.current_mission_id, Some("mission-x".to_string()));
        let msgs = db.messages_for_conversation("c2").unwrap();
        assert_eq!(msgs.len(), 1);
        assert_eq!(msgs[0].content, "persist me");
        let atts = db.attachments_for_conversation("c2").unwrap();
        assert_eq!(atts.len(), 1);
        assert_eq!(atts[0].filename, "note.txt");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn multiple_conversations_per_project() {
        let db = db();
        let now = TimestampMillis::now().as_millis() as i64;
        for i in 0..3 {
            db.save_conversation(&ConversationRow {
                id: format!("m{i}"),
                project_path: "/proj/multi".to_string(),
                mode: "GOAL".to_string(),
                title: format!("Chat {i}"),
                state: "active".to_string(),
                current_mission_id: None,
                created_at_ms: now,
                updated_at_ms: now,
            })
            .unwrap();
        }
        let list = db.conversations_for_project("/proj/multi", false).unwrap();
        assert_eq!(list.len(), 3);
    }

    #[test]
    fn project_isolation() {
        let db = db();
        let now = TimestampMillis::now().as_millis() as i64;
        for (id, project) in [("a1", "/proj/A"), ("a2", "/proj/A"), ("b1", "/proj/B")] {
            db.save_conversation(&ConversationRow {
                id: id.to_string(),
                project_path: project.to_string(),
                mode: "GOAL".to_string(),
                title: id.to_string(),
                state: "active".to_string(),
                current_mission_id: None,
                created_at_ms: now,
                updated_at_ms: now,
            })
            .unwrap();
        }
        let a = db.conversations_for_project("/proj/A", false).unwrap();
        assert_eq!(a.len(), 2);
        assert!(a.iter().any(|c| c.id == "a1"));
        assert!(a.iter().any(|c| c.id == "a2"));
        let b = db.conversations_for_project("/proj/B", false).unwrap();
        assert_eq!(b.len(), 1);
        assert_eq!(b[0].id, "b1");
    }

    #[test]
    fn message_persistence() {
        let db = db();
        let r = row("mc1", "/proj/msg");
        db.save_conversation(&r).unwrap();
        let m = message("m1", "mc1", "user", "hello");
        db.save_message(&m).unwrap();
        let msgs = db.messages_for_conversation("mc1").unwrap();
        assert_eq!(msgs.len(), 1);
        assert_eq!(msgs[0].content, "hello");
        assert_eq!(msgs[0].role, "user");
    }

    #[test]
    fn attachment_persistence() {
        let db = db();
        let r = row("ac1", "/proj/att");
        db.save_conversation(&r).unwrap();
        let a = attachment("att1", "ac1", "/proj/att", "img.png", "image/png");
        db.save_attachment(&a).unwrap();
        let atts = db.attachments_for_conversation("ac1").unwrap();
        assert_eq!(atts.len(), 1);
        assert_eq!(atts[0].filename, "img.png");
        assert_eq!(atts[0].mime_type, "image/png");
        assert_eq!(atts[0].sha256, "abc123");
    }

    #[test]
    fn attachment_metadata() {
        let db = db();
        let r = row("ac2", "/proj/att2");
        db.save_conversation(&r).unwrap();
        let a = attachment("att2", "ac2", "/proj/att2", "doc.pdf", "application/pdf");
        db.save_attachment(&a).unwrap();
        let stored = db.attachment("att2").unwrap().unwrap();
        assert_eq!(stored.sensitivity, "normal");
        assert_eq!(stored.sha256, "abc123");
        assert_eq!(stored.storage_key, "att2");
    }

    #[test]
    fn archive_and_delete() {
        let db = db();
        let r = row("arc1", "/proj/arc");
        db.save_conversation(&r).unwrap();
        assert_eq!(db.conversations_for_project("/proj/arc", false).unwrap().len(), 1);
        db.archive_conversation("arc1").unwrap();
        assert_eq!(db.conversations_for_project("/proj/arc", false).unwrap().len(), 0);
        assert_eq!(db.conversations_for_project("/proj/arc", true).unwrap().len(), 1);
        db.delete_conversation("arc1").unwrap();
        assert_eq!(db.conversations_for_project("/proj/arc", true).unwrap().len(), 0);
    }

    #[test]
    fn set_current_mission() {
        let db = db();
        let r = row("mis1", "/proj/mis");
        db.save_conversation(&r).unwrap();
        db.set_conversation_mission("mis1", "mission-abc").unwrap();
        let loaded = db.conversation("mis1").unwrap().unwrap();
        assert_eq!(loaded.current_mission_id, Some("mission-abc".to_string()));
    }
}