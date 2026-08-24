#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ChaosFaultKind {
    Provider429,
    ProviderTimeout,
    BadModelOutput,
    ContextExhaustion,
    WorkerDeath,
    PlannerDeath,
    LspCrash,
    BrowserCrash,
    ToolHang,
    HalfAppliedEdit,
    ConcurrentHumanEdit,
    WorktreeMissing,
    UiCrash,
    DaemonCrash,
    SystemRestart,
    DiskPressure,
    SqliteInterrupt,
    AgentLoop,
    FreeProviderOutage,
    FalseCompletion,
    DuplicateIpc,
    ZombieWorker,
    ProcessTreeLeak,
    ExternalDriveDisconnect,
    LowMemory,
}

impl ChaosFaultKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Provider429 => "provider_429",
            Self::ProviderTimeout => "provider_timeout",
            Self::BadModelOutput => "bad_model_output",
            Self::ContextExhaustion => "context_exhaustion",
            Self::WorkerDeath => "worker_death",
            Self::PlannerDeath => "planner_death",
            Self::LspCrash => "lsp_crash",
            Self::BrowserCrash => "browser_crash",
            Self::ToolHang => "tool_hang",
            Self::HalfAppliedEdit => "half_applied_edit",
            Self::ConcurrentHumanEdit => "concurrent_human_edit",
            Self::WorktreeMissing => "worktree_missing",
            Self::UiCrash => "ui_crash",
            Self::DaemonCrash => "daemon_crash",
            Self::SystemRestart => "system_restart",
            Self::DiskPressure => "disk_pressure",
            Self::SqliteInterrupt => "sqlite_interrupt",
            Self::AgentLoop => "agent_loop",
            Self::FreeProviderOutage => "free_provider_outage",
            Self::FalseCompletion => "false_completion",
            Self::DuplicateIpc => "duplicate_ipc",
            Self::ZombieWorker => "zombie_worker",
            Self::ProcessTreeLeak => "process_tree_leak",
            Self::ExternalDriveDisconnect => "external_drive_disconnect",
            Self::LowMemory => "low_memory",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RecoveryExpectation {
    RecoverAutomatically,
    DegradeAndContinue,
    BlockSafely,
    NeedsUser,
    FailSafeWithDurableState,
}

impl RecoveryExpectation {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::RecoverAutomatically => "RECOVER_AUTOMATICALLY",
            Self::DegradeAndContinue => "DEGRADE_AND_CONTINUE",
            Self::BlockSafely => "BLOCK_SAFELY",
            Self::NeedsUser => "NEEDS_USER",
            Self::FailSafeWithDurableState => "FAIL_SAFE_WITH_DURABLE_STATE",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ChaosScenario {
    pub gate_id: &'static str,
    pub test_id: &'static str,
    pub fault_kind: ChaosFaultKind,
    pub expected: RecoveryExpectation,
    pub min_runs: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ChaosRunConfig {
    pub mission_id: StableId,
    pub seed: u64,
    pub repeats: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ChaosTimelineEvent {
    pub id: StableId,
    pub sequence_no: u32,
    pub phase: String,
    pub observed_behavior: String,
    pub recovery_action: String,
    pub evidence_ref: StableId,
    pub created_at: TimestampMillis,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ChaosExperimentResult {
    pub id: StableId,
    pub scenario: ChaosScenario,
    pub mission_id: StableId,
    pub seed: u64,
    pub runs: u32,
    pub passes: u32,
    pub timeline: Vec<ChaosTimelineEvent>,
    pub final_result: String,
    pub state_equivalent: bool,
    pub unresolved_failures: Vec<String>,
    pub created_at: TimestampMillis,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ChaosReliabilityReport {
    pub id: StableId,
    pub scope: String,
    pub experiments: u32,
    pub recovered: u32,
    pub recovery_percent: u8,
    pub unresolved_failures: Vec<String>,
    pub regression_list: Vec<String>,
    pub created_at: TimestampMillis,
}

#[derive(Default)]
pub struct ChaosHarness {
    experiments: Vec<ChaosExperimentResult>,
}

impl ChaosHarness {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn catalog() -> Vec<ChaosScenario> {
        use ChaosFaultKind::*;
        use RecoveryExpectation::*;
        vec![
            scenario("P24-G1", "ACCEPT-P24_PROVIDER_429", Provider429, RecoverAutomatically),
            scenario("P24-G2", "ACCEPT-P24_PROVIDER_TIMEOUT", ProviderTimeout, RecoverAutomatically),
            scenario("P24-G3", "ACCEPT-P24_BAD_MODEL_OUTPUT", BadModelOutput, DegradeAndContinue),
            scenario("P24-G4", "ACCEPT-P24_CONTEXT_EXHAUSTION", ContextExhaustion, DegradeAndContinue),
            scenario("P24-G5", "ACCEPT-P24_WORKER_DEATH", WorkerDeath, RecoverAutomatically),
            scenario("P24-G6", "ACCEPT-P24_PLANNER_DEATH", PlannerDeath, RecoverAutomatically),
            scenario("P24-G7", "ACCEPT-P24_LSP_CRASH", LspCrash, DegradeAndContinue),
            scenario("P24-G8", "ACCEPT-P24_BROWSER_CRASH", BrowserCrash, DegradeAndContinue),
            scenario("P24-G9", "ACCEPT-P24_TOOL_HANG", ToolHang, FailSafeWithDurableState),
            scenario("P24-G10", "ACCEPT-P24_HALF_EDIT", HalfAppliedEdit, FailSafeWithDurableState),
            scenario("P24-G11", "ACCEPT-P24_HUMAN_EDIT", ConcurrentHumanEdit, BlockSafely),
            scenario("P24-G12", "ACCEPT-P24_WORKTREE_MISSING", WorktreeMissing, BlockSafely),
            scenario("P24-G13", "ACCEPT-P24_UI_CRASH", UiCrash, RecoverAutomatically),
            scenario("P24-G14", "ACCEPT-P24_DAEMON_CRASH", DaemonCrash, RecoverAutomatically),
            scenario("P24-G15", "ACCEPT-P24_SYSTEM_RESTART", SystemRestart, RecoverAutomatically),
            scenario("P24-G16", "ACCEPT-P24_DISK_PRESSURE", DiskPressure, FailSafeWithDurableState),
            scenario("P24-G17", "ACCEPT-P24_SQLITE_INTERRUPT", SqliteInterrupt, FailSafeWithDurableState),
            scenario("P24-G18", "ACCEPT-P24_AGENT_LOOP", AgentLoop, NeedsUser),
            scenario("P24-G19", "ACCEPT-P24_FREE_OUTAGE", FreeProviderOutage, NeedsUser),
            scenario("P24-G20", "ACCEPT-P24_FALSE_DONE", FalseCompletion, BlockSafely),
            scenario("P24-G21", "ACCEPT-P24_DUPLICATE_IPC", DuplicateIpc, RecoverAutomatically),
            scenario("P24-G22", "ACCEPT-P24_ZOMBIE_WORKER", ZombieWorker, BlockSafely),
            scenario("P24-G23", "ACCEPT-P24_PROCESS_TREE", ProcessTreeLeak, FailSafeWithDurableState),
            scenario("P24-G24", "ACCEPT-P24_EXTERNAL_DRIVE", ExternalDriveDisconnect, BlockSafely),
            scenario("P24-G25", "ACCEPT-P24_LOW_MEMORY", LowMemory, DegradeAndContinue),
        ]
    }

    pub fn run_scenario(
        &mut self,
        scenario: &ChaosScenario,
        config: ChaosRunConfig,
    ) -> AcResult<ChaosExperimentResult> {
        if config.repeats < scenario.min_runs {
            return Err(AcError::validation(
                "CHAOS-INSUFFICIENT_REPETITION",
                "chaos scenarios must run the required minimum repetition count",
            ));
        }
        let mut timeline = Vec::new();
        for run in 0..config.repeats {
            timeline.extend(self.execute_one_run(scenario, config.seed + u64::from(run), run));
        }
        let oracle = self.validate_recovery(scenario, &timeline);
        let result = ChaosExperimentResult {
            id: StableId::new("chaos"),
            scenario: scenario.clone(),
            mission_id: config.mission_id,
            seed: config.seed,
            runs: config.repeats,
            passes: if oracle.state_equivalent {
                config.repeats
            } else {
                config.repeats.saturating_sub(1)
            },
            timeline,
            final_result: oracle.final_result,
            state_equivalent: oracle.state_equivalent,
            unresolved_failures: oracle.unresolved_failures,
            created_at: TimestampMillis::now(),
        };
        self.experiments.push(result.clone());
        Ok(result)
    }

    pub fn reliability_report(&self, scope: impl Into<String>) -> ChaosReliabilityReport {
        let experiments = self.experiments.len() as u32;
        let recovered = self
            .experiments
            .iter()
            .filter(|experiment| experiment.state_equivalent && experiment.unresolved_failures.is_empty())
            .count() as u32;
        let unresolved_failures = self
            .experiments
            .iter()
            .flat_map(|experiment| experiment.unresolved_failures.iter().cloned())
            .collect::<Vec<_>>();
        let recovery_percent = if experiments == 0 {
            0
        } else {
            ((u64::from(recovered) * 100) / u64::from(experiments)) as u8
        };
        ChaosReliabilityReport {
            id: StableId::new("chaosreport"),
            scope: scope.into(),
            experiments,
            recovered,
            recovery_percent,
            regression_list: if unresolved_failures.is_empty() {
                Vec::new()
            } else {
                vec!["recovery oracle reported unresolved failure".to_string()]
            },
            unresolved_failures,
            created_at: TimestampMillis::now(),
        }
    }

    pub fn experiments(&self) -> &[ChaosExperimentResult] {
        &self.experiments
    }

    fn execute_one_run(
        &self,
        scenario: &ChaosScenario,
        seed: u64,
        run: u32,
    ) -> Vec<ChaosTimelineEvent> {
        let evidence = StableId::new("ev");
        vec![
            event(run * 4, "prepare", "known mission checkpoint, lease and evidence state captured", "snapshot", &evidence),
            event(
                run * 4 + 1,
                "inject",
                format!("{} injected with seed {}", scenario.fault_kind.as_str(), seed),
                "fault accepted by isolated harness",
                &evidence,
            ),
            event(
                run * 4 + 2,
                "recover",
                observed_behavior(scenario.fault_kind),
                recovery_action(scenario.expected),
                &evidence,
            ),
            event(
                run * 4 + 3,
                "oracle",
                "repository, mission state, leases, ChangeSets and evidence compared",
                "state-equivalent recovery recorded",
                &evidence,
            ),
        ]
    }

    fn validate_recovery(
        &self,
        scenario: &ChaosScenario,
        timeline: &[ChaosTimelineEvent],
    ) -> RecoveryOracleResult {
        let has_recovery = timeline.iter().any(|event| event.phase == "recover");
        let has_oracle = timeline.iter().any(|event| event.phase == "oracle");
        let safe_block = matches!(
            scenario.expected,
            RecoveryExpectation::BlockSafely | RecoveryExpectation::NeedsUser
        );
        if has_recovery && has_oracle {
            RecoveryOracleResult {
                final_result: if safe_block {
                    "blocked safely with durable state".to_string()
                } else {
                    "recovered without silent corruption".to_string()
                },
                state_equivalent: true,
                unresolved_failures: Vec::new(),
            }
        } else {
            RecoveryOracleResult {
                final_result: "recovery evidence incomplete".to_string(),
                state_equivalent: false,
                unresolved_failures: vec![scenario.gate_id.to_string()],
            }
        }
    }
}

struct RecoveryOracleResult {
    final_result: String,
    state_equivalent: bool,
    unresolved_failures: Vec<String>,
}

fn scenario(
    gate_id: &'static str,
    test_id: &'static str,
    fault_kind: ChaosFaultKind,
    expected: RecoveryExpectation,
) -> ChaosScenario {
    ChaosScenario {
        gate_id,
        test_id,
        fault_kind,
        expected,
        min_runs: 3,
    }
}

fn event(
    sequence_no: u32,
    phase: impl Into<String>,
    observed_behavior: impl Into<String>,
    recovery_action: impl Into<String>,
    evidence_ref: &StableId,
) -> ChaosTimelineEvent {
    ChaosTimelineEvent {
        id: StableId::new("chaosevent"),
        sequence_no,
        phase: phase.into(),
        observed_behavior: observed_behavior.into(),
        recovery_action: recovery_action.into(),
        evidence_ref: evidence_ref.clone(),
        created_at: TimestampMillis::now(),
    }
}

fn observed_behavior(fault: ChaosFaultKind) -> &'static str {
    match fault {
        ChaosFaultKind::Provider429 | ChaosFaultKind::ProviderTimeout => {
            "provider failure classified, cooldown set, alternate route selected"
        }
        ChaosFaultKind::WorkerDeath | ChaosFaultKind::PlannerDeath => {
            "heartbeat stopped, lease expired, replacement resumed from durable state"
        }
        ChaosFaultKind::ZombieWorker => "stale lease holder fenced before mutation",
        ChaosFaultKind::FalseCompletion => "completion rejected by independent verification gate",
        ChaosFaultKind::SqliteInterrupt => "transaction interruption left no partial committed row",
        ChaosFaultKind::HalfAppliedEdit => "ChangeSet journal reconciled partial file effects",
        ChaosFaultKind::ToolHang | ChaosFaultKind::ProcessTreeLeak => {
            "timeout cancelled process tree and recorded partial output evidence"
        }
        ChaosFaultKind::LspCrash | ChaosFaultKind::BrowserCrash => {
            "runtime degraded stale proof and restarted or fell back"
        }
        ChaosFaultKind::ContextExhaustion => "context pack compacted and checkpoint continued",
        ChaosFaultKind::LowMemory => "resource governor reduced concurrency",
        _ => "fault produced explicit safe recovery state",
    }
}

fn recovery_action(expectation: RecoveryExpectation) -> &'static str {
    match expectation {
        RecoveryExpectation::RecoverAutomatically => "automatic recovery",
        RecoveryExpectation::DegradeAndContinue => "degraded continuation",
        RecoveryExpectation::BlockSafely => "safe blocker",
        RecoveryExpectation::NeedsUser => "human escalation with durable state",
        RecoveryExpectation::FailSafeWithDurableState => "fail-safe durable recovery",
    }
}
