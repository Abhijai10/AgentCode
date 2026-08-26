use std::path::Path;

use ac_changeset::{ChangeSet, ChangeSetTransaction};
use ac_common::{AcError, AcResult, StableId, TimestampMillis};
use ac_evidence::{EvidenceKind, EvidenceRecord, Provenance};
use ac_git::{CheckpointRecord, WorktreeRecord};
use ac_kernel::{KernelDecisionKind, KernelEvent, Mission, MissionState};
use ac_security::{
    ActiveSecurityInput, ActiveSecurityReport, AiSecurityReport, HookInvocation, HookManifest,
    McpInvocationRecord, McpServerRecord, McpToolRecord, SecurityAdapter, SecurityReportBundle,
    SecurityScanInput, SecurityScanReport, SkillManifest, ThreatModel,
};
use ac_verification::{
    BrowserProcessRecord, BrowserSessionRecord, DevServerRecord, FinalAuditReport,
    RequirementEvidenceLink, ScreenshotEvidence, VerificationEvidenceManifest, VerificationProfile,
    VisualQaReport,
};
use rusqlite::{params, Connection, OptionalExtension};

include!("models.rs");
include!("migrations.rs");
include!("context.rs");
include!("evidence.rs");
include!("verification.rs");
include!("security.rs");
include!("agent.rs");
include!("discuss_design.rs");
include!("desktop_optimization.rs");
include!("chaos_dogfood.rs");
include!("security_release.rs");
include!("git.rs");
include!("tests.rs");
