use ac_common::{AcError, AcResult, StableId, TimestampMillis};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FileChangeSummary {
    pub path: String,
    pub additions: u32,
    pub removals: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ChangeOperation {
    WriteFile {
        path: String,
        expected_hash: Option<String>,
        new_hash: String,
    },
    DeleteFile {
        path: String,
        expected_hash: String,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ChangeSetState {
    Created,
    Proposed,
    Validated,
    Approved,
    Rejected,
    Applied,
    Archived,
    RolledBack,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ChangeSetMetadata {
    pub originating_task: StableId,
    pub originating_agent_session: StableId,
    pub files_changed: Vec<FileChangeSummary>,
    pub additions: u32,
    pub removals: u32,
    pub evidence_refs: Vec<StableId>,
    pub verification_passed: Option<bool>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RollbackPlan {
    pub checkpoint_ref: String,
    pub description: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ChangeSet {
    pub id: StableId,
    pub operations: Vec<ChangeOperation>,
    pub state: ChangeSetState,
    pub rollback: Option<RollbackPlan>,
    pub metadata: Option<ChangeSetMetadata>,
    pub created_at: TimestampMillis,
}

impl ChangeSet {
    pub fn propose(
        operations: Vec<ChangeOperation>,
        rollback: Option<RollbackPlan>,
    ) -> AcResult<Self> {
        if operations.is_empty() {
            return Err(AcError::validation(
                "CHANGESET-EMPTY",
                "a changeset must contain at least one operation",
            ));
        }
        Ok(Self {
            id: StableId::new("cs"),
            operations,
            state: ChangeSetState::Created,
            rollback,
            metadata: None,
            created_at: TimestampMillis::now(),
        })
    }

    pub fn attach_metadata(&mut self, metadata: ChangeSetMetadata) -> AcResult<()> {
        if metadata.originating_task.as_str().trim().is_empty()
            || metadata
                .originating_agent_session
                .as_str()
                .trim()
                .is_empty()
        {
            return Err(AcError::validation(
                "CHANGESET-INVALID_METADATA",
                "originating task and agent session are required",
            ));
        }
        self.metadata = Some(metadata);
        Ok(())
    }

    pub fn validate(&mut self) -> AcResult<()> {
        self.transition(ChangeSetState::Validated)
    }

    pub fn approve(&mut self) -> AcResult<()> {
        self.transition(ChangeSetState::Approved)
    }

    pub fn reject(&mut self) -> AcResult<()> {
        self.transition(ChangeSetState::Rejected)
    }

    pub fn mark_applied(&mut self) -> AcResult<()> {
        self.transition(ChangeSetState::Applied)
    }

    pub fn archive(&mut self) -> AcResult<()> {
        self.transition(ChangeSetState::Archived)
    }

    pub fn mark_rolled_back(&mut self) -> AcResult<()> {
        self.transition(ChangeSetState::RolledBack)
    }

    fn transition(&mut self, next: ChangeSetState) -> AcResult<()> {
        let allowed = matches!(
            (self.state, next),
            (ChangeSetState::Created, ChangeSetState::Validated)
                | (ChangeSetState::Created, ChangeSetState::Rejected)
                | (ChangeSetState::Proposed, ChangeSetState::Validated)
                | (ChangeSetState::Proposed, ChangeSetState::Rejected)
                | (ChangeSetState::Validated, ChangeSetState::Approved)
                | (ChangeSetState::Validated, ChangeSetState::Rejected)
                | (ChangeSetState::Approved, ChangeSetState::Applied)
                | (ChangeSetState::Applied, ChangeSetState::Archived)
                | (ChangeSetState::Applied, ChangeSetState::RolledBack)
        );
        if !allowed {
            return Err(AcError::conflict(
                "CHANGESET-INVALID_TRANSITION",
                format!("cannot transition {:?} to {:?}", self.state, next),
            ));
        }
        self.state = next;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn op() -> ChangeOperation {
        ChangeOperation::WriteFile {
            path: "src/lib.rs".to_string(),
            expected_hash: Some("old".to_string()),
            new_hash: "new".to_string(),
        }
    }

    #[test]
    fn changeset_requires_approval_before_applied() {
        let mut changeset = ChangeSet::propose(vec![op()], None).unwrap();
        let err = changeset.mark_applied().unwrap_err();
        assert_eq!(err.code(), "CHANGESET-INVALID_TRANSITION");
        changeset.validate().unwrap();
        changeset.approve().unwrap();
        changeset.mark_applied().unwrap();
        assert_eq!(changeset.state, ChangeSetState::Applied);
    }

    #[test]
    fn changeset_records_metadata_and_archives_after_apply() {
        let mut changeset = ChangeSet::propose(vec![op()], None).unwrap();
        changeset
            .attach_metadata(ChangeSetMetadata {
                originating_task: StableId::new("goal"),
                originating_agent_session: StableId::new("session"),
                files_changed: vec![FileChangeSummary {
                    path: "src/lib.rs".to_string(),
                    additions: 2,
                    removals: 1,
                }],
                additions: 2,
                removals: 1,
                evidence_refs: vec![StableId::new("ev")],
                verification_passed: Some(true),
            })
            .unwrap();
        changeset.validate().unwrap();
        changeset.approve().unwrap();
        changeset.mark_applied().unwrap();
        changeset.archive().unwrap();
        assert_eq!(changeset.state, ChangeSetState::Archived);
        assert_eq!(changeset.metadata.as_ref().unwrap().additions, 2);
    }
}
