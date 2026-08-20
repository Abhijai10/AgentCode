use std::collections::BTreeMap;

use ac_changeset::{ChangeSet, ChangeSetState};
use ac_common::{AcError, AcResult, StableId, TimestampMillis};
use ac_evidence::EvidenceRecord;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum KernelLifecycle {
    Created,
    Running,
    Stopping,
    Stopped,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MissionState {
    Created,
    Active,
    Completed,
    Cancelled,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Mission {
    pub id: StableId,
    pub original_goal: String,
    pub state: MissionState,
    pub created_at: TimestampMillis,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum KernelDecisionKind {
    CreateMission,
    ActivateMission,
    CompleteMission,
    CancelMission,
    ApproveChangeSet,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct KernelEvent {
    pub id: StableId,
    pub decision: KernelDecisionKind,
    pub subject: StableId,
    pub evidence_refs: Vec<StableId>,
    pub created_at: TimestampMillis,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PermissionDecision {
    Allow,
    Deny,
    RequireApproval,
}

pub trait PolicyBoundary {
    fn evaluate(&self, decision: KernelDecisionKind) -> PermissionDecision;
}

#[derive(Default)]
pub struct AllowAllPolicy;

impl PolicyBoundary for AllowAllPolicy {
    fn evaluate(&self, _decision: KernelDecisionKind) -> PermissionDecision {
        PermissionDecision::Allow
    }
}

pub struct Kernel<P: PolicyBoundary> {
    lifecycle: KernelLifecycle,
    policy: P,
    missions: BTreeMap<StableId, Mission>,
    events: Vec<KernelEvent>,
}

impl<P: PolicyBoundary> Kernel<P> {
    pub fn new(policy: P) -> Self {
        Self {
            lifecycle: KernelLifecycle::Created,
            policy,
            missions: BTreeMap::new(),
            events: Vec::new(),
        }
    }

    pub fn start(&mut self) -> AcResult<()> {
        match self.lifecycle {
            KernelLifecycle::Created | KernelLifecycle::Stopped => {
                self.lifecycle = KernelLifecycle::Running;
                Ok(())
            }
            _ => Err(AcError::conflict(
                "KERNEL-INVALID_LIFECYCLE",
                "kernel can only start from created or stopped",
            )),
        }
    }

    pub fn stop(&mut self) -> AcResult<()> {
        match self.lifecycle {
            KernelLifecycle::Running => {
                self.lifecycle = KernelLifecycle::Stopping;
                self.lifecycle = KernelLifecycle::Stopped;
                Ok(())
            }
            _ => Err(AcError::conflict(
                "KERNEL-INVALID_LIFECYCLE",
                "kernel can only stop from running",
            )),
        }
    }

    pub fn lifecycle(&self) -> KernelLifecycle {
        self.lifecycle
    }

    pub fn create_mission(&mut self, original_goal: impl Into<String>) -> AcResult<StableId> {
        self.ensure_running()?;
        self.ensure_policy(KernelDecisionKind::CreateMission)?;
        let original_goal = original_goal.into();
        if original_goal.trim().is_empty() {
            return Err(AcError::validation(
                "KERNEL-EMPTY_GOAL",
                "mission goal cannot be empty",
            ));
        }
        let id = StableId::new("mission");
        let mission = Mission {
            id: id.clone(),
            original_goal,
            state: MissionState::Created,
            created_at: TimestampMillis::now(),
        };
        self.missions.insert(id.clone(), mission);
        self.record(KernelDecisionKind::CreateMission, id.clone(), Vec::new());
        Ok(id)
    }

    pub fn transition_mission(
        &mut self,
        id: &StableId,
        next: MissionState,
        evidence_refs: Vec<StableId>,
    ) -> AcResult<()> {
        self.ensure_running()?;
        let decision = match next {
            MissionState::Active => KernelDecisionKind::ActivateMission,
            MissionState::Completed => KernelDecisionKind::CompleteMission,
            MissionState::Cancelled => KernelDecisionKind::CancelMission,
            MissionState::Created => {
                return Err(AcError::conflict(
                    "KERNEL-INVALID_MISSION_TRANSITION",
                    "cannot transition back to created",
                ));
            }
        };
        self.ensure_policy(decision)?;
        let mission = self.missions.get_mut(id).ok_or_else(|| {
            AcError::validation("KERNEL-UNKNOWN_MISSION", "mission does not exist")
        })?;
        let allowed = matches!(
            (mission.state, next),
            (MissionState::Created, MissionState::Active)
                | (MissionState::Active, MissionState::Completed)
                | (MissionState::Active, MissionState::Cancelled)
        );
        if !allowed {
            return Err(AcError::conflict(
                "KERNEL-INVALID_MISSION_TRANSITION",
                format!("cannot transition {:?} to {:?}", mission.state, next),
            ));
        }
        mission.state = next;
        self.record(decision, id.clone(), evidence_refs);
        Ok(())
    }

    pub fn approve_changeset(&mut self, changeset: &mut ChangeSet) -> AcResult<()> {
        self.ensure_running()?;
        self.ensure_policy(KernelDecisionKind::ApproveChangeSet)?;
        if changeset.state != ChangeSetState::Validated {
            return Err(AcError::conflict(
                "KERNEL-CHANGESET_NOT_VALIDATED",
                "kernel only approves validated changesets",
            ));
        }
        changeset.approve()?;
        self.record(
            KernelDecisionKind::ApproveChangeSet,
            changeset.id.clone(),
            Vec::new(),
        );
        Ok(())
    }

    pub fn events(&self) -> &[KernelEvent] {
        &self.events
    }

    pub fn mission(&self, id: &StableId) -> Option<&Mission> {
        self.missions.get(id)
    }

    pub fn accept_evidence(&self, evidence: &EvidenceRecord) -> StableId {
        evidence.id.clone()
    }

    fn ensure_running(&self) -> AcResult<()> {
        if self.lifecycle != KernelLifecycle::Running {
            return Err(AcError::conflict(
                "KERNEL-NOT_RUNNING",
                "kernel decisions require running lifecycle",
            ));
        }
        Ok(())
    }

    fn ensure_policy(&self, decision: KernelDecisionKind) -> AcResult<()> {
        match self.policy.evaluate(decision) {
            PermissionDecision::Allow => Ok(()),
            PermissionDecision::Deny => Err(AcError::policy_denied(
                "KERNEL-POLICY_DENIED",
                "policy denied kernel decision",
            )),
            PermissionDecision::RequireApproval => Err(AcError::policy_denied(
                "KERNEL-APPROVAL_REQUIRED",
                "policy requires approval before kernel decision",
            )),
        }
    }

    fn record(
        &mut self,
        decision: KernelDecisionKind,
        subject: StableId,
        evidence_refs: Vec<StableId>,
    ) {
        self.events.push(KernelEvent {
            id: StableId::new("ke"),
            decision,
            subject,
            evidence_refs,
            created_at: TimestampMillis::now(),
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct DenyPolicy;

    impl PolicyBoundary for DenyPolicy {
        fn evaluate(&self, _decision: KernelDecisionKind) -> PermissionDecision {
            PermissionDecision::Deny
        }
    }

    #[test]
    fn kernel_owns_mission_state_transitions() {
        let mut kernel = Kernel::new(AllowAllPolicy);
        kernel.start().unwrap();
        let mission_id = kernel.create_mission("build foundation").unwrap();
        kernel
            .transition_mission(&mission_id, MissionState::Active, Vec::new())
            .unwrap();
        kernel
            .transition_mission(&mission_id, MissionState::Completed, Vec::new())
            .unwrap();
        assert_eq!(
            kernel.mission(&mission_id).unwrap().state,
            MissionState::Completed
        );
        assert_eq!(kernel.events().len(), 3);
    }

    #[test]
    fn policy_boundary_can_block_kernel_decision() {
        let mut kernel = Kernel::new(DenyPolicy);
        kernel.start().unwrap();
        let err = kernel.create_mission("blocked").unwrap_err();
        assert_eq!(err.code(), "KERNEL-POLICY_DENIED");
    }
}
