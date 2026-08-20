use ac_common::{AcError, AcResult, StableId, TimestampMillis};

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
    Proposed,
    Validated,
    Approved,
    Rejected,
    Applied,
    RolledBack,
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
            state: ChangeSetState::Proposed,
            rollback,
            created_at: TimestampMillis::now(),
        })
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

    pub fn mark_rolled_back(&mut self) -> AcResult<()> {
        self.transition(ChangeSetState::RolledBack)
    }

    fn transition(&mut self, next: ChangeSetState) -> AcResult<()> {
        let allowed = matches!(
            (self.state, next),
            (ChangeSetState::Proposed, ChangeSetState::Validated)
                | (ChangeSetState::Proposed, ChangeSetState::Rejected)
                | (ChangeSetState::Validated, ChangeSetState::Approved)
                | (ChangeSetState::Validated, ChangeSetState::Rejected)
                | (ChangeSetState::Approved, ChangeSetState::Applied)
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
}
