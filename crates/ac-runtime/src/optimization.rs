#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResourceTelemetryRecord {
    pub id: StableId,
    pub component: String,
    pub rss_bytes: u64,
    pub cpu_millis: u64,
    pub disk_bytes: u64,
    pub process_count: u32,
    pub worker_count: u32,
    pub browser_sessions: u32,
    pub lsp_sessions: u32,
    pub local_model_loaded: bool,
    pub created_at: TimestampMillis,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TokenUsageRecord {
    pub id: StableId,
    pub task_id: StableId,
    pub provider_call_id: Option<StableId>,
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub context_tokens: u32,
    pub compressed_tokens: u32,
    pub estimated_cost_micros: u64,
    pub verified: bool,
    pub created_at: TimestampMillis,
}

impl TokenUsageRecord {
    pub fn compression_ratio(&self) -> u8 {
        if self.context_tokens == 0 {
            return 100;
        }
        ((u64::from(self.compressed_tokens) * 100) / u64::from(self.context_tokens)).min(100) as u8
    }

    pub fn total_tokens(&self) -> u32 {
        self.input_tokens
            .saturating_add(self.output_tokens)
            .saturating_add(self.context_tokens)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResourcePolicy {
    pub max_workers: u32,
    pub max_rss_bytes: u64,
    pub max_cpu_busy_workers: u32,
    pub budget_limit_micros: Option<u64>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResourceTelemetryInput {
    pub rss_bytes: u64,
    pub cpu_millis: u64,
    pub disk_bytes: u64,
    pub process_count: u32,
    pub worker_count: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResourceGovernorDecision {
    pub admitted_workers: u32,
    pub degraded_mode: bool,
    pub reason: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LocalModelLifecycleDecision {
    pub unload: bool,
    pub reason: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LspLifecycleDecision {
    pub stop_idle: bool,
    pub restart_unhealthy: bool,
    pub reason: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HeavyIndexPolicyDecision {
    pub engine: String,
    pub enabled: bool,
    pub reason: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OptimizationReport {
    pub id: StableId,
    pub total_tokens: u32,
    pub verified_tokens: u32,
    pub total_cost_micros: u64,
    pub cost_per_verified_task_micros: Option<u64>,
    pub average_compression_ratio: u8,
    pub before_after: Vec<String>,
}

#[derive(Default)]
pub struct OptimizationEngine {
    telemetry: Vec<ResourceTelemetryRecord>,
    token_usage: Vec<TokenUsageRecord>,
}

impl OptimizationEngine {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn record_resource_telemetry(
        &mut self,
        component: impl Into<String>,
        snapshot: &ResourceSnapshot,
        input: ResourceTelemetryInput,
    ) -> AcResult<ResourceTelemetryRecord> {
        let component = component.into();
        if component.trim().is_empty() {
            return Err(AcError::validation(
                "OPTIMIZATION-COMPONENT_EMPTY",
                "telemetry component is required",
            ));
        }
        let record = ResourceTelemetryRecord {
            id: StableId::new("resource"),
            component,
            rss_bytes: input.rss_bytes,
            cpu_millis: input.cpu_millis,
            disk_bytes: input.disk_bytes,
            process_count: input.process_count,
            worker_count: input.worker_count,
            browser_sessions: u32::from(snapshot.browser_sessions),
            lsp_sessions: u32::from(snapshot.lsp_sessions),
            local_model_loaded: snapshot.local_model_loaded,
            created_at: TimestampMillis::now(),
        };
        self.telemetry.push(record.clone());
        Ok(record)
    }

    pub fn record_token_usage(
        &mut self,
        task_id: StableId,
        provider_call_id: Option<StableId>,
        usage: TokenUsageInput,
    ) -> AcResult<TokenUsageRecord> {
        if usage.input_tokens == 0 && usage.output_tokens == 0 && usage.context_tokens == 0 {
            return Err(AcError::validation(
                "OPTIMIZATION-TOKENS_EMPTY",
                "token accounting requires at least one non-zero token count",
            ));
        }
        let record = TokenUsageRecord {
            id: StableId::new("usage"),
            task_id,
            provider_call_id,
            input_tokens: usage.input_tokens,
            output_tokens: usage.output_tokens,
            context_tokens: usage.context_tokens,
            compressed_tokens: usage.compressed_tokens,
            estimated_cost_micros: usage.estimated_cost_micros,
            verified: usage.verified,
            created_at: TimestampMillis::now(),
        };
        self.token_usage.push(record.clone());
        Ok(record)
    }

    pub fn govern(
        &self,
        policy: &ResourcePolicy,
        snapshot: &ResourceSnapshot,
        requested_workers: u32,
        spent_micros: u64,
    ) -> ResourceGovernorDecision {
        let mut admitted = requested_workers.min(policy.max_workers);
        let mut reasons = Vec::new();
        if matches!(snapshot.memory_pressure, MemoryPressure::Critical) {
            admitted = admitted.min(1);
            reasons.push("critical memory pressure".to_string());
        } else if matches!(snapshot.memory_pressure, MemoryPressure::Pressure)
            || snapshot.cpu_busy
            || spent_micros > policy.budget_limit_micros.unwrap_or(u64::MAX)
        {
            admitted = admitted.min(policy.max_cpu_busy_workers.max(1));
            reasons.push("resource or budget pressure".to_string());
        }
        ResourceGovernorDecision {
            admitted_workers: admitted,
            degraded_mode: admitted < requested_workers,
            reason: if reasons.is_empty() {
                "within resource policy".to_string()
            } else {
                reasons.join("; ")
            },
        }
    }

    pub fn local_model_lifecycle(
        &self,
        snapshot: &ResourceSnapshot,
        idle_ms: u64,
        pressure_unload_ms: u64,
    ) -> LocalModelLifecycleDecision {
        let unload = snapshot.local_model_loaded
            && (idle_ms >= pressure_unload_ms
                || matches!(snapshot.memory_pressure, MemoryPressure::Pressure | MemoryPressure::Critical));
        LocalModelLifecycleDecision {
            unload,
            reason: if unload {
                "idle or memory pressure unload".to_string()
            } else {
                "model remains available".to_string()
            },
        }
    }

    pub fn lsp_lifecycle(
        &self,
        snapshot: &ResourceSnapshot,
        idle_ms: u64,
        unhealthy: bool,
    ) -> LspLifecycleDecision {
        LspLifecycleDecision {
            stop_idle: snapshot.lsp_sessions > 0 && idle_ms >= 5 * 60 * 1000,
            restart_unhealthy: unhealthy,
            reason: if unhealthy {
                "restart unhealthy server".to_string()
            } else if idle_ms >= 5 * 60 * 1000 {
                "stop idle workspace server".to_string()
            } else {
                "lsp remains active".to_string()
            },
        }
    }

    pub fn heavy_index_policy(
        &self,
        engine: impl Into<String>,
        measured_files: usize,
        expected_reuse: usize,
        snapshot: &ResourceSnapshot,
    ) -> HeavyIndexPolicyDecision {
        let engine = engine.into();
        let enabled = measured_files > 250
            && expected_reuse > 2
            && !matches!(snapshot.memory_pressure, MemoryPressure::Critical);
        HeavyIndexPolicyDecision {
            engine,
            enabled,
            reason: if enabled {
                "benefit exceeds activation threshold".to_string()
            } else {
                "fallback index is cheaper for this workload".to_string()
            },
        }
    }

    pub fn report(&self, before_after: Vec<String>) -> OptimizationReport {
        let total_tokens = self
            .token_usage
            .iter()
            .map(TokenUsageRecord::total_tokens)
            .sum();
        let verified = self
            .token_usage
            .iter()
            .filter(|record| record.verified)
            .collect::<Vec<_>>();
        let verified_tokens = verified.iter().map(|record| record.total_tokens()).sum();
        let total_cost_micros = self
            .token_usage
            .iter()
            .map(|record| record.estimated_cost_micros)
            .sum();
        let average_compression_ratio = if self.token_usage.is_empty() {
            100
        } else {
            (self
                .token_usage
                .iter()
                .map(|record| u32::from(record.compression_ratio()))
                .sum::<u32>()
                / self.token_usage.len() as u32) as u8
        };
        OptimizationReport {
            id: StableId::new("optreport"),
            total_tokens,
            verified_tokens,
            total_cost_micros,
            cost_per_verified_task_micros: (!verified.is_empty())
                .then_some(total_cost_micros / verified.len() as u64),
            average_compression_ratio,
            before_after,
        }
    }

    pub fn telemetry(&self) -> &[ResourceTelemetryRecord] {
        &self.telemetry
    }

    pub fn token_usage(&self) -> &[TokenUsageRecord] {
        &self.token_usage
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TokenUsageInput {
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub context_tokens: u32,
    pub compressed_tokens: u32,
    pub estimated_cost_micros: u64,
    pub verified: bool,
}
