use std::collections::{BTreeMap, VecDeque};
use std::io::Read;
use std::time::Duration;

use ac_common::{AcError, AcResult, StableId, TimestampMillis};
use reqwest::blocking::{Client, Response};
use reqwest::header::{HeaderMap, HeaderName, HeaderValue, AUTHORIZATION, CONTENT_TYPE};
use reqwest::Url;
use serde_json::{json, Value};

pub mod catalog;
pub mod discovery;

pub use catalog::{
    resolve_credential_ref, test_provider_account, DiscoveredModel, ProviderAccount,
    ProviderAccountStatus, ProviderCatalogEntry, ProviderConnectionKind, ProviderConnectionTest,
};
pub use discovery::{discover_models, ModelDiscoveryKind};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ProviderCapability {
    Chat,
    ToolCalls,
    Streaming,
    LocalModel,
    Vision,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PrivacyClass {
    LocalOnly,
    Standard,
    ExternalAllowed,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RoutingProfile {
    FreeOnly,
    FreeFirst,
    LocalFirst,
    QualityFirst,
    PaidAllowed,
    Offline,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum NormalizedProviderFailure {
    RateLimit,
    QuotaExhausted,
    AuthFailed,
    Timeout,
    ModelUnavailable,
    ProviderUnavailable,
    ContextLimit,
    BadResponse,
    ToolCallFailure,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProviderConnection {
    pub id: StableId,
    pub provider_id: StableId,
    pub account_ref: String,
    pub credential_ref: Option<String>,
    pub quota_domain: String,
    pub endpoint_ref: String,
    pub enabled: bool,
    pub paid: bool,
    pub local: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModelIdentity {
    pub id: StableId,
    pub family: String,
    pub context_window: u32,
    pub coding_score: u8,
    pub reasoning_score: u8,
    pub vision: bool,
    pub tool_use: bool,
    pub structured_output: bool,
    pub input_cost_micros: u32,
    pub output_cost_micros: u32,
    pub privacy: PrivacyClass,
    pub metadata_source: ModelMetadataSource,
    pub pricing_unit: PricingUnit,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ModelMetadataSource {
    Configured,
    Discovered,
    DefaultUnknown,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PricingUnit {
    PerTokenMicrosUsd,
    Unknown,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModelRoute {
    pub id: StableId,
    pub model_identity_id: StableId,
    pub model_id: StableId,
    pub provider_id: StableId,
    pub connection_id: StableId,
    pub model_name: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TaskProfile {
    pub task_id: StableId,
    pub role: String,
    pub task_type: String,
    pub complexity: u8,
    pub risk: u8,
    pub required_context: u32,
    pub requires_vision: bool,
    pub requires_tool_use: bool,
    pub requires_structured_output: bool,
    pub privacy: PrivacyClass,
    pub routing_profile: RoutingProfile,
    pub max_input_cost_micros: Option<u32>,
}

impl TaskProfile {
    pub fn coding(task_id: StableId, routing_profile: RoutingProfile) -> Self {
        Self {
            task_id,
            role: "implementation".to_string(),
            task_type: "coding".to_string(),
            complexity: 5,
            risk: 5,
            required_context: 8192,
            requires_vision: false,
            requires_tool_use: false,
            requires_structured_output: true,
            privacy: PrivacyClass::Standard,
            routing_profile,
            max_input_cost_micros: None,
        }
    }

    pub fn discuss(task_id: StableId, routing_profile: RoutingProfile) -> Self {
        Self {
            task_id,
            role: "discussion".to_string(),
            task_type: "repo_discussion".to_string(),
            complexity: 4,
            risk: 2,
            required_context: 12_000,
            requires_vision: false,
            requires_tool_use: false,
            requires_structured_output: true,
            privacy: PrivacyClass::Standard,
            routing_profile,
            max_input_cost_micros: Some(2_000),
        }
    }

    pub fn design(
        task_id: StableId,
        routing_profile: RoutingProfile,
        requires_vision: bool,
    ) -> Self {
        Self {
            task_id,
            role: "design_critic".to_string(),
            task_type: "design_studio".to_string(),
            complexity: 6,
            risk: 4,
            required_context: 16_000,
            requires_vision,
            requires_tool_use: false,
            requires_structured_output: true,
            privacy: PrivacyClass::Standard,
            routing_profile,
            max_input_cost_micros: Some(4_000),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RouteCandidate {
    pub route_id: StableId,
    pub model_identity_id: StableId,
    pub provider_id: StableId,
    pub connection_id: StableId,
    pub model_name: String,
    pub score: i32,
    pub reasons: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RoutingDecision {
    pub id: StableId,
    pub task_id: StableId,
    pub candidates: Vec<RouteCandidate>,
    pub selected: Option<RouteCandidate>,
    pub rejected: Vec<String>,
    pub fallback_reason: Option<String>,
    pub latency_ms: u64,
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub total_tokens: u32,
    pub usage_source: UsageSource,
    pub estimated_cost_micros: u64,
    pub cost_known: bool,
    pub created_at: TimestampMillis,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RouteExecution {
    pub events: Vec<ProviderStreamEvent>,
    pub decision: RoutingDecision,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CostTier {
    Free,
    Cheap,
    Standard,
    Premium,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModelSelectionRequirement {
    pub task_id: StableId,
    pub min_quality: u8,
    pub max_cost_tier: CostTier,
    pub prefer_low_latency: bool,
    pub requires_vision: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CostOptimizedRoute {
    pub candidate: RouteCandidate,
    pub cost_tier: CostTier,
    pub estimated_latency_ms: u64,
    pub quality_score: u8,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HealthObservation {
    pub connection_id: StableId,
    pub failure: Option<NormalizedProviderFailure>,
    pub latency_ms: u64,
    pub created_at: TimestampMillis,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CircuitBreaker {
    pub connection_id: StableId,
    pub transient_failures: u32,
    pub cooldown_until_ms: Option<u128>,
}

impl CircuitBreaker {
    fn healthy(connection_id: StableId) -> Self {
        Self {
            connection_id,
            transient_failures: 0,
            cooldown_until_ms: None,
        }
    }

    fn is_open(&self) -> bool {
        self.cooldown_until_ms
            .map(|until| TimestampMillis::now().as_millis() < until)
            .unwrap_or(false)
    }

    fn record_success(&mut self) {
        self.transient_failures = 0;
        self.cooldown_until_ms = None;
    }

    fn record_failure(&mut self, failure: NormalizedProviderFailure) {
        if matches!(
            failure,
            NormalizedProviderFailure::RateLimit
                | NormalizedProviderFailure::Timeout
                | NormalizedProviderFailure::ProviderUnavailable
        ) {
            self.transient_failures += 1;
            if self.transient_failures >= 2 {
                self.cooldown_until_ms = Some(TimestampMillis::now().as_millis() + 10 * 60 * 1000);
            }
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProviderDefinition {
    pub id: StableId,
    pub name: String,
    pub credential_ref: Option<String>,
    pub capabilities: Vec<ProviderCapability>,
    pub quota_domain: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModelCapability {
    pub id: StableId,
    pub provider_id: StableId,
    pub model_name: String,
    pub capabilities: Vec<ProviderCapability>,
    pub context_window: u32,
    pub discovered_at: TimestampMillis,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NormalizedInferenceRequest {
    pub model_id: StableId,
    pub prompt: String,
    pub required: Vec<ProviderCapability>,
    pub max_output_tokens: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ProviderStreamEvent {
    Delta(String),
    ToolCall {
        name: String,
        payload: String,
    },
    Usage {
        input_tokens: u32,
        output_tokens: u32,
    },
    EstimatedUsage {
        input_tokens: u32,
        output_tokens: u32,
    },
    Finished,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StructuredAgentResponse {
    pub plan: Vec<String>,
    pub assumptions: Vec<String>,
    pub actions: Vec<String>,
    pub verification_requirements: Vec<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProviderResponseFormat {
    Structured,
    LegacyFallback,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ValidatedProviderResponse {
    pub response: StructuredAgentResponse,
    pub format: ProviderResponseFormat,
}

impl StructuredAgentResponse {
    pub fn validate(&self) -> AcResult<()> {
        if self.plan.is_empty()
            || self.actions.is_empty()
            || self.verification_requirements.is_empty()
            || self.plan.iter().any(|item| item.trim().is_empty())
            || self.actions.iter().any(|item| item.trim().is_empty())
            || self
                .verification_requirements
                .iter()
                .any(|item| item.trim().is_empty())
        {
            return Err(AcError::validation(
                "PROVIDER-INVALID_STRUCTURED_RESPONSE",
                "structured provider output requires plan, actions, and verification requirements",
            ));
        }
        Ok(())
    }

    pub fn from_legacy_text(text: &str) -> AcResult<Self> {
        let mut plan = Vec::new();
        let mut assumptions = Vec::new();
        let mut actions = Vec::new();
        let mut verification_requirements = Vec::new();
        for line in text.lines().map(str::trim).filter(|line| !line.is_empty()) {
            if let Some(value) = line
                .strip_prefix("plan=")
                .or_else(|| line.strip_prefix("plan:"))
            {
                plan.push(value.trim().to_string());
            } else if let Some(value) = line
                .strip_prefix("assumption=")
                .or_else(|| line.strip_prefix("assumption:"))
            {
                assumptions.push(value.trim().to_string());
            } else if let Some(value) = line
                .strip_prefix("action=")
                .or_else(|| line.strip_prefix("action:"))
            {
                actions.push(value.trim().to_string());
            } else if let Some(value) = line
                .strip_prefix("verify=")
                .or_else(|| line.strip_prefix("verification="))
                .or_else(|| line.strip_prefix("verification_requirement="))
            {
                verification_requirements.push(value.trim().to_string());
            }
        }
        if plan.is_empty() && !text.trim().is_empty() {
            plan.push(text.trim().to_string());
        }
        if actions.is_empty() && !plan.is_empty() {
            actions.push("review provider plan".to_string());
        }
        if verification_requirements.is_empty() && !plan.is_empty() {
            verification_requirements.push("run configured verification profile".to_string());
        }
        let response = Self {
            plan,
            assumptions,
            actions,
            verification_requirements,
        };
        response.validate()?;
        Ok(response)
    }
}

pub fn parse_structured_agent_response(raw: &str) -> AcResult<StructuredAgentResponse> {
    let raw = raw.trim();
    if !(raw.starts_with('{') && raw.ends_with('}')) {
        return Err(AcError::validation(
            "PROVIDER-STRUCTURED_RESPONSE_REQUIRED",
            "structured provider output must be a JSON object",
        ));
    }
    let response = StructuredAgentResponse {
        plan: extract_string_array(raw, "plan")?,
        assumptions: extract_string_array(raw, "assumptions").unwrap_or_default(),
        actions: extract_string_array(raw, "actions")?,
        verification_requirements: extract_string_array(raw, "verification_requirements")?,
    };
    response.validate()?;
    Ok(response)
}

pub fn validate_provider_events(
    events: &[ProviderStreamEvent],
    allow_legacy_fallback: bool,
) -> AcResult<ValidatedProviderResponse> {
    let raw = events
        .iter()
        .filter_map(|event| match event {
            ProviderStreamEvent::Delta(text) => Some(text.as_str()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("");
    match parse_structured_agent_response(&raw) {
        Ok(response) => Ok(ValidatedProviderResponse {
            response,
            format: ProviderResponseFormat::Structured,
        }),
        Err(_) if allow_legacy_fallback => Ok(ValidatedProviderResponse {
            response: StructuredAgentResponse::from_legacy_text(&raw)?,
            format: ProviderResponseFormat::LegacyFallback,
        }),
        Err(error) => Err(error),
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProviderFailureClass {
    RateLimited,
    RateLimitedAfter(u64),
    Timeout,
    ServerError,
    Network,
    StreamAborted,
    MalformedResponse,
    InvalidRequest,
    Auth,
    Permission,
    ContextOverflow,
    UnsupportedCapability,
    ModelUnavailable,
    Cancelled,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UsageSource {
    ProviderReported,
    Estimated,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TokenUsage {
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub total_tokens: u32,
    pub source: UsageSource,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CostEstimate {
    pub model_identity_id: StableId,
    pub input_cost_micros: u32,
    pub output_cost_micros: u32,
    pub unit: PricingUnit,
    pub calculated_cost_micros: Option<u64>,
}

pub trait ProviderAdapter {
    fn stream(
        &self,
        request: &NormalizedInferenceRequest,
        cancel: &dyn Fn() -> bool,
    ) -> Result<Vec<ProviderStreamEvent>, ProviderFailureClass>;
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConfiguredProviderAdapter {
    pub endpoint_ref: String,
    pub credential_ref: String,
    pub model_name: String,
}

impl ConfiguredProviderAdapter {
    pub fn new(
        endpoint_ref: impl Into<String>,
        credential_ref: impl Into<String>,
        model_name: impl Into<String>,
    ) -> AcResult<Self> {
        let endpoint_ref = endpoint_ref.into();
        let credential_ref = credential_ref.into();
        let model_name = model_name.into();
        if endpoint_ref.trim().is_empty()
            || credential_ref.trim().is_empty()
            || model_name.trim().is_empty()
        {
            return Err(AcError::validation(
                "PROVIDER-INVALID_CONFIG",
                "endpoint, credential reference, and model name are required",
            ));
        }
        Ok(Self {
            endpoint_ref,
            credential_ref,
            model_name,
        })
    }
}

impl ProviderAdapter for ConfiguredProviderAdapter {
    fn stream(
        &self,
        _request: &NormalizedInferenceRequest,
        cancel: &dyn Fn() -> bool,
    ) -> Result<Vec<ProviderStreamEvent>, ProviderFailureClass> {
        if cancel() {
            return Err(ProviderFailureClass::Cancelled);
        }
        Err(ProviderFailureClass::UnsupportedCapability)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HttpProviderAdapter {
    pub endpoint: String,
    pub credential_env: Option<String>,
    pub model_name: String,
    pub timeout_ms: u64,
    pub connect_timeout_ms: u64,
    pub max_response_bytes: usize,
    pub custom_headers: Vec<(String, String)>,
    pub allow_plain_http_remote: bool,
    provider_kind: HttpProviderKind,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HttpProviderOptions {
    pub endpoint: String,
    pub credential_env: Option<String>,
    pub model_name: String,
    pub connect_timeout_ms: u64,
    pub read_timeout_ms: u64,
    pub max_response_bytes: usize,
    pub custom_headers: Vec<(String, String)>,
    pub allow_plain_http_remote: bool,
}

impl HttpProviderOptions {
    pub fn new(endpoint: impl Into<String>, model_name: impl Into<String>) -> Self {
        Self {
            endpoint: endpoint.into(),
            credential_env: None,
            model_name: model_name.into(),
            connect_timeout_ms: 10_000,
            read_timeout_ms: 60_000,
            max_response_bytes: 2 * 1024 * 1024,
            custom_headers: Vec::new(),
            allow_plain_http_remote: false,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum HttpProviderKind {
    OpenAiCompatible,
    OpenAiChatCompletions,
    AnthropicMessages,
    GeminiGenerateContent,
    OllamaChat,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OpenAIProviderAdapter {
    inner: HttpProviderAdapter,
}

impl OpenAIProviderAdapter {
    pub fn new(endpoint: impl Into<String>, model_name: impl Into<String>) -> AcResult<Self> {
        Ok(Self {
            inner: HttpProviderAdapter::new_kind(
                HttpProviderOptions {
                    credential_env: Some("OPENAI_API_KEY".to_string()),
                    ..HttpProviderOptions::new(endpoint, model_name)
                },
                HttpProviderKind::OpenAiChatCompletions,
            )?,
        })
    }

    pub fn with_options(options: HttpProviderOptions) -> AcResult<Self> {
        Ok(Self {
            inner: HttpProviderAdapter::new_kind(options, HttpProviderKind::OpenAiChatCompletions)?,
        })
    }
}

impl ProviderAdapter for OpenAIProviderAdapter {
    fn stream(
        &self,
        request: &NormalizedInferenceRequest,
        cancel: &dyn Fn() -> bool,
    ) -> Result<Vec<ProviderStreamEvent>, ProviderFailureClass> {
        self.inner.stream(request, cancel)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AnthropicProviderAdapter {
    inner: HttpProviderAdapter,
}

impl AnthropicProviderAdapter {
    pub fn new(endpoint: impl Into<String>, model_name: impl Into<String>) -> AcResult<Self> {
        Ok(Self {
            inner: HttpProviderAdapter::new_kind(
                HttpProviderOptions {
                    credential_env: Some("ANTHROPIC_API_KEY".to_string()),
                    ..HttpProviderOptions::new(endpoint, model_name)
                },
                HttpProviderKind::AnthropicMessages,
            )?,
        })
    }

    pub fn with_options(options: HttpProviderOptions) -> AcResult<Self> {
        Ok(Self {
            inner: HttpProviderAdapter::new_kind(options, HttpProviderKind::AnthropicMessages)?,
        })
    }
}

impl ProviderAdapter for AnthropicProviderAdapter {
    fn stream(
        &self,
        request: &NormalizedInferenceRequest,
        cancel: &dyn Fn() -> bool,
    ) -> Result<Vec<ProviderStreamEvent>, ProviderFailureClass> {
        self.inner.stream(request, cancel)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GeminiProviderAdapter {
    inner: HttpProviderAdapter,
}

impl GeminiProviderAdapter {
    pub fn new(endpoint: impl Into<String>, model_name: impl Into<String>) -> AcResult<Self> {
        Ok(Self {
            inner: HttpProviderAdapter::new_kind(
                HttpProviderOptions {
                    credential_env: Some("GEMINI_API_KEY".to_string()),
                    ..HttpProviderOptions::new(endpoint, model_name)
                },
                HttpProviderKind::GeminiGenerateContent,
            )?,
        })
    }

    pub fn with_options(options: HttpProviderOptions) -> AcResult<Self> {
        Ok(Self {
            inner: HttpProviderAdapter::new_kind(options, HttpProviderKind::GeminiGenerateContent)?,
        })
    }
}

impl ProviderAdapter for GeminiProviderAdapter {
    fn stream(
        &self,
        request: &NormalizedInferenceRequest,
        cancel: &dyn Fn() -> bool,
    ) -> Result<Vec<ProviderStreamEvent>, ProviderFailureClass> {
        self.inner.stream(request, cancel)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OllamaProviderAdapter {
    inner: HttpProviderAdapter,
}

impl OllamaProviderAdapter {
    pub fn new(endpoint: impl Into<String>, model_name: impl Into<String>) -> AcResult<Self> {
        Ok(Self {
            inner: HttpProviderAdapter::new_kind(
                HttpProviderOptions::new(endpoint, model_name),
                HttpProviderKind::OllamaChat,
            )?,
        })
    }

    pub fn with_options(options: HttpProviderOptions) -> AcResult<Self> {
        Ok(Self {
            inner: HttpProviderAdapter::new_kind(options, HttpProviderKind::OllamaChat)?,
        })
    }
}

impl ProviderAdapter for OllamaProviderAdapter {
    fn stream(
        &self,
        request: &NormalizedInferenceRequest,
        cancel: &dyn Fn() -> bool,
    ) -> Result<Vec<ProviderStreamEvent>, ProviderFailureClass> {
        self.inner.stream(request, cancel)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LMStudioProviderAdapter {
    inner: HttpProviderAdapter,
}

impl LMStudioProviderAdapter {
    pub fn new(endpoint: impl Into<String>, model_name: impl Into<String>) -> AcResult<Self> {
        Ok(Self {
            inner: HttpProviderAdapter::new_kind(
                HttpProviderOptions::new(endpoint, model_name),
                HttpProviderKind::OpenAiCompatible,
            )?,
        })
    }

    pub fn with_options(options: HttpProviderOptions) -> AcResult<Self> {
        Ok(Self {
            inner: HttpProviderAdapter::new_kind(options, HttpProviderKind::OpenAiCompatible)?,
        })
    }
}

impl ProviderAdapter for LMStudioProviderAdapter {
    fn stream(
        &self,
        request: &NormalizedInferenceRequest,
        cancel: &dyn Fn() -> bool,
    ) -> Result<Vec<ProviderStreamEvent>, ProviderFailureClass> {
        self.inner.stream(request, cancel)
    }
}

impl HttpProviderAdapter {
    pub fn new(
        endpoint: impl Into<String>,
        credential_env: Option<String>,
        model_name: impl Into<String>,
        timeout_ms: u64,
    ) -> AcResult<Self> {
        Self::new_kind(
            HttpProviderOptions {
                credential_env,
                read_timeout_ms: timeout_ms,
                ..HttpProviderOptions::new(endpoint, model_name)
            },
            HttpProviderKind::OpenAiCompatible,
        )
    }

    fn new_kind(options: HttpProviderOptions, provider_kind: HttpProviderKind) -> AcResult<Self> {
        validate_endpoint(&options.endpoint, options.allow_plain_http_remote)?;
        if options.model_name.trim().is_empty()
            || options.connect_timeout_ms == 0
            || options.read_timeout_ms == 0
            || options.max_response_bytes == 0
        {
            return Err(AcError::validation(
                "PROVIDER-INVALID_HTTP_CONFIG",
                "endpoint, model name, non-zero timeouts, and response bound are required",
            ));
        }
        validate_custom_headers(&options.custom_headers)?;
        Ok(Self {
            endpoint: options.endpoint,
            credential_env: options.credential_env,
            model_name: options.model_name,
            timeout_ms: options.read_timeout_ms,
            connect_timeout_ms: options.connect_timeout_ms,
            max_response_bytes: options.max_response_bytes,
            custom_headers: options.custom_headers,
            allow_plain_http_remote: options.allow_plain_http_remote,
            provider_kind,
        })
    }

    pub fn request_json(&self, request: &NormalizedInferenceRequest) -> Value {
        match self.provider_kind {
            HttpProviderKind::OpenAiCompatible | HttpProviderKind::OpenAiChatCompletions => json!({
                "model": self.model_name,
                "messages": [{"role": "user", "content": request.prompt}],
                "max_tokens": request.max_output_tokens,
                "stream": true
            }),
            HttpProviderKind::AnthropicMessages => json!({
                "model": self.model_name,
                "messages": [{"role": "user", "content": request.prompt}],
                "max_tokens": request.max_output_tokens,
                "stream": true
            }),
            HttpProviderKind::GeminiGenerateContent => json!({
                "contents": [{"role": "user", "parts": [{"text": request.prompt}]}],
                "generationConfig": {"maxOutputTokens": request.max_output_tokens}
            }),
            HttpProviderKind::OllamaChat => json!({
                "model": self.model_name,
                "messages": [{"role": "user", "content": request.prompt}],
                "stream": true,
                "format": "json"
            }),
        }
    }

    pub fn auth_headers(&self) -> Result<HeaderMap, ProviderFailureClass> {
        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        for (name, value) in &self.custom_headers {
            headers.insert(
                HeaderName::from_bytes(name.as_bytes())
                    .map_err(|_| ProviderFailureClass::InvalidRequest)?,
                HeaderValue::from_str(value).map_err(|_| ProviderFailureClass::InvalidRequest)?,
            );
        }
        if let Some(credential) = &self.credential_env {
            let key =
                resolve_adapter_credential(credential).map_err(|_| ProviderFailureClass::Auth)?;
            match self.provider_kind {
                HttpProviderKind::AnthropicMessages => {
                    headers.insert(
                        "x-api-key",
                        HeaderValue::from_str(&key).map_err(|_| ProviderFailureClass::Auth)?,
                    );
                    headers.insert("anthropic-version", HeaderValue::from_static("2023-06-01"));
                }
                HttpProviderKind::GeminiGenerateContent => {
                    headers.insert(
                        "x-goog-api-key",
                        HeaderValue::from_str(&key).map_err(|_| ProviderFailureClass::Auth)?,
                    );
                }
                _ => {
                    let value = format!("Bearer {key}");
                    headers.insert(
                        AUTHORIZATION,
                        HeaderValue::from_str(&value).map_err(|_| ProviderFailureClass::Auth)?,
                    );
                }
            }
        }
        Ok(headers)
    }
}

impl ProviderAdapter for HttpProviderAdapter {
    fn stream(
        &self,
        request: &NormalizedInferenceRequest,
        cancel: &dyn Fn() -> bool,
    ) -> Result<Vec<ProviderStreamEvent>, ProviderFailureClass> {
        if cancel() {
            return Err(ProviderFailureClass::Cancelled);
        }
        let client = Client::builder()
            .connect_timeout(Duration::from_millis(self.connect_timeout_ms))
            .timeout(Duration::from_millis(self.timeout_ms))
            .build()
            .map_err(|_| ProviderFailureClass::ServerError)?;
        let response = client
            .post(&self.endpoint)
            .headers(self.auth_headers()?)
            .json(&self.request_json(request))
            .send()
            .map_err(map_reqwest_error)?;
        if cancel() {
            return Err(ProviderFailureClass::Cancelled);
        }
        let events = events_from_response(
            response,
            self.provider_kind,
            self.max_response_bytes,
            cancel,
        )?;
        Ok(ensure_usage_event(events, &request.prompt))
    }
}

#[derive(Clone, Debug, Default)]
pub struct ScriptedProvider {
    responses: Vec<Result<Vec<ProviderStreamEvent>, ProviderFailureClass>>,
}

impl ScriptedProvider {
    pub fn new(responses: Vec<Result<Vec<ProviderStreamEvent>, ProviderFailureClass>>) -> Self {
        Self { responses }
    }
}

impl ProviderAdapter for ScriptedProvider {
    fn stream(
        &self,
        _request: &NormalizedInferenceRequest,
        cancel: &dyn Fn() -> bool,
    ) -> Result<Vec<ProviderStreamEvent>, ProviderFailureClass> {
        if cancel() {
            return Err(ProviderFailureClass::Cancelled);
        }
        self.responses
            .first()
            .cloned()
            .unwrap_or_else(|| Ok(vec![ProviderStreamEvent::Finished]))
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RouteAttempt {
    pub id: StableId,
    pub model_id: StableId,
    pub provider_id: StableId,
    pub started_at: TimestampMillis,
    pub events: Vec<ProviderStreamEvent>,
    pub failure: Option<ProviderFailureClass>,
}

#[derive(Default)]
pub struct ProviderRegistry {
    providers: BTreeMap<StableId, ProviderDefinition>,
    models: BTreeMap<StableId, ModelCapability>,
    adapters: BTreeMap<StableId, Box<dyn ProviderAdapter>>,
    connections: BTreeMap<StableId, ProviderConnection>,
    model_identities: BTreeMap<StableId, ModelIdentity>,
    routes: BTreeMap<StableId, ModelRoute>,
    circuit_breakers: BTreeMap<StableId, CircuitBreaker>,
    health_observations: Vec<HealthObservation>,
    routing_decisions: Vec<RoutingDecision>,
    attempts: Vec<RouteAttempt>,
}

const MAX_PROVIDER_HISTORY: usize = 256;

impl ProviderRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register_provider(
        &mut self,
        name: impl Into<String>,
        credential_ref: Option<String>,
        capabilities: Vec<ProviderCapability>,
        quota_domain: impl Into<String>,
    ) -> AcResult<StableId> {
        let name = name.into();
        let quota_domain = quota_domain.into();
        if name.trim().is_empty() || quota_domain.trim().is_empty() {
            return Err(AcError::validation(
                "PROVIDER-INVALID_DEFINITION",
                "provider name and quota domain are required",
            ));
        }
        let id = StableId::new("provider");
        self.providers.insert(
            id.clone(),
            ProviderDefinition {
                id: id.clone(),
                name,
                credential_ref,
                capabilities,
                quota_domain,
            },
        );
        Ok(id)
    }

    pub fn register_adapter(
        &mut self,
        provider_id: &StableId,
        adapter: Box<dyn ProviderAdapter>,
    ) -> AcResult<()> {
        if !self.providers.contains_key(provider_id) {
            return Err(AcError::validation(
                "PROVIDER-UNKNOWN_PROVIDER",
                "provider must be registered before adapter",
            ));
        }
        self.adapters.insert(provider_id.clone(), adapter);
        Ok(())
    }

    pub fn register_model(
        &mut self,
        provider_id: &StableId,
        model_name: impl Into<String>,
        capabilities: Vec<ProviderCapability>,
        context_window: u32,
    ) -> AcResult<StableId> {
        let model_name = model_name.into();
        if !self.providers.contains_key(provider_id) {
            return Err(AcError::validation(
                "PROVIDER-UNKNOWN_PROVIDER",
                "provider must be registered before models",
            ));
        }
        if model_name.trim().is_empty() || context_window == 0 {
            return Err(AcError::validation(
                "PROVIDER-INVALID_MODEL",
                "model name and context window are required",
            ));
        }
        let id = StableId::new("model");
        self.models.insert(
            id.clone(),
            ModelCapability {
                id: id.clone(),
                provider_id: provider_id.clone(),
                model_name,
                capabilities,
                context_window,
                discovered_at: TimestampMillis::now(),
            },
        );
        Ok(id)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn register_connection(
        &mut self,
        provider_id: &StableId,
        account_ref: impl Into<String>,
        credential_ref: Option<String>,
        quota_domain: impl Into<String>,
        endpoint_ref: impl Into<String>,
        enabled: bool,
        paid: bool,
        local: bool,
    ) -> AcResult<StableId> {
        if !self.providers.contains_key(provider_id) {
            return Err(AcError::validation(
                "PROVIDER-UNKNOWN_PROVIDER",
                "provider must exist before registering a connection",
            ));
        }
        let account_ref = account_ref.into();
        let quota_domain = quota_domain.into();
        let endpoint_ref = endpoint_ref.into();
        if account_ref.trim().is_empty()
            || quota_domain.trim().is_empty()
            || endpoint_ref.trim().is_empty()
        {
            return Err(AcError::validation(
                "PROVIDER-INVALID_CONNECTION",
                "account, quota domain, and endpoint reference are required",
            ));
        }
        let id = StableId::new("connection");
        self.connections.insert(
            id.clone(),
            ProviderConnection {
                id: id.clone(),
                provider_id: provider_id.clone(),
                account_ref,
                credential_ref,
                quota_domain,
                endpoint_ref,
                enabled,
                paid,
                local,
            },
        );
        self.circuit_breakers
            .insert(id.clone(), CircuitBreaker::healthy(id.clone()));
        Ok(id)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn register_model_identity(
        &mut self,
        family: impl Into<String>,
        context_window: u32,
        coding_score: u8,
        reasoning_score: u8,
        vision: bool,
        tool_use: bool,
        structured_output: bool,
        input_cost_micros: u32,
        output_cost_micros: u32,
        privacy: PrivacyClass,
    ) -> AcResult<StableId> {
        let family = family.into();
        if family.trim().is_empty() || context_window == 0 {
            return Err(AcError::validation(
                "PROVIDER-INVALID_MODEL_IDENTITY",
                "model family and context window are required",
            ));
        }
        let id = StableId::new("model-family");
        self.model_identities.insert(
            id.clone(),
            ModelIdentity {
                id: id.clone(),
                family,
                context_window,
                coding_score,
                reasoning_score,
                vision,
                tool_use,
                structured_output,
                input_cost_micros,
                output_cost_micros,
                privacy,
                metadata_source: ModelMetadataSource::Configured,
                pricing_unit: if input_cost_micros == 0 && output_cost_micros == 0 {
                    PricingUnit::Unknown
                } else {
                    PricingUnit::PerTokenMicrosUsd
                },
            },
        );
        Ok(id)
    }

    pub fn register_model_route(
        &mut self,
        model_identity_id: &StableId,
        model_id: &StableId,
        connection_id: &StableId,
    ) -> AcResult<StableId> {
        let identity = self
            .model_identities
            .get(model_identity_id)
            .ok_or_else(|| {
                AcError::validation(
                    "PROVIDER-UNKNOWN_MODEL_IDENTITY",
                    "model identity not found",
                )
            })?;
        let model = self.models.get(model_id).ok_or_else(|| {
            AcError::validation("PROVIDER-UNKNOWN_MODEL", "model capability not found")
        })?;
        let connection = self.connections.get(connection_id).ok_or_else(|| {
            AcError::validation(
                "PROVIDER-UNKNOWN_CONNECTION",
                "provider connection not found",
            )
        })?;
        if model.provider_id != connection.provider_id {
            return Err(AcError::validation(
                "PROVIDER-ROUTE_PROVIDER_MISMATCH",
                "model and connection must belong to the same provider",
            ));
        }
        let id = StableId::new("route");
        self.routes.insert(
            id.clone(),
            ModelRoute {
                id: id.clone(),
                model_identity_id: identity.id.clone(),
                model_id: model.id.clone(),
                provider_id: model.provider_id.clone(),
                connection_id: connection.id.clone(),
                model_name: model.model_name.clone(),
            },
        );
        Ok(id)
    }

    pub fn select_model(&self, required: &[ProviderCapability]) -> AcResult<&ModelCapability> {
        self.models
            .values()
            .find(|model| required.iter().all(|cap| model.capabilities.contains(cap)))
            .ok_or_else(|| {
                AcError::validation(
                    "PROVIDER-NO_CAPABLE_MODEL",
                    "no registered model satisfies requested capabilities",
                )
            })
    }

    pub fn normalize_request(
        &self,
        prompt: impl Into<String>,
        required: Vec<ProviderCapability>,
        max_output_tokens: u32,
    ) -> AcResult<NormalizedInferenceRequest> {
        let prompt = prompt.into();
        if prompt.trim().is_empty() || max_output_tokens == 0 {
            return Err(AcError::validation(
                "PROVIDER-INVALID_REQUEST",
                "prompt and output token budget are required",
            ));
        }
        let model = self.select_model(&required)?;
        Ok(NormalizedInferenceRequest {
            model_id: model.id.clone(),
            prompt,
            required,
            max_output_tokens,
        })
    }

    pub fn start_attempt(&mut self, request: &NormalizedInferenceRequest) -> AcResult<StableId> {
        let model = self.models.get(&request.model_id).ok_or_else(|| {
            AcError::validation("PROVIDER-UNKNOWN_MODEL", "request model is not registered")
        })?;
        let id = StableId::new("route");
        self.attempts.push(RouteAttempt {
            id: id.clone(),
            model_id: model.id.clone(),
            provider_id: model.provider_id.clone(),
            started_at: TimestampMillis::now(),
            events: Vec::new(),
            failure: None,
        });
        truncate_front(&mut self.attempts, MAX_PROVIDER_HISTORY);
        Ok(id)
    }

    pub fn record_event(
        &mut self,
        attempt_id: &StableId,
        event: ProviderStreamEvent,
    ) -> AcResult<()> {
        let attempt = self
            .attempts
            .iter_mut()
            .find(|attempt| &attempt.id == attempt_id)
            .ok_or_else(|| {
                AcError::validation("PROVIDER-UNKNOWN_ATTEMPT", "route attempt not found")
            })?;
        attempt.events.push(event);
        Ok(())
    }

    pub fn finish_failed(
        &mut self,
        attempt_id: &StableId,
        failure: ProviderFailureClass,
    ) -> AcResult<()> {
        let attempt = self
            .attempts
            .iter_mut()
            .find(|attempt| &attempt.id == attempt_id)
            .ok_or_else(|| {
                AcError::validation("PROVIDER-UNKNOWN_ATTEMPT", "route attempt not found")
            })?;
        attempt.failure = Some(failure);
        Ok(())
    }

    pub fn stream_with_retry(
        &mut self,
        request: &NormalizedInferenceRequest,
        max_attempts: usize,
        cancel: &dyn Fn() -> bool,
    ) -> Result<Vec<ProviderStreamEvent>, ProviderFailureClass> {
        let model = self
            .models
            .get(&request.model_id)
            .ok_or(ProviderFailureClass::UnsupportedCapability)?
            .clone();
        let mut remaining = VecDeque::from_iter(0..max_attempts.max(1));
        let mut last_failure = None;
        while remaining.pop_front().is_some() {
            if cancel() {
                return Err(ProviderFailureClass::Cancelled);
            }
            let attempt_id = self
                .start_attempt(request)
                .map_err(|_| ProviderFailureClass::MalformedResponse)?;
            let result = {
                let adapter = self
                    .adapters
                    .get(&model.provider_id)
                    .ok_or(ProviderFailureClass::UnsupportedCapability)?;
                adapter.stream(request, cancel)
            };
            match result {
                Ok(events) => {
                    for event in &events {
                        self.record_event(&attempt_id, event.clone())
                            .map_err(|_| ProviderFailureClass::MalformedResponse)?;
                    }
                    if matches!(events.last(), Some(ProviderStreamEvent::Finished)) {
                        return Ok(events);
                    }
                    self.finish_failed(&attempt_id, ProviderFailureClass::StreamAborted)
                        .map_err(|_| ProviderFailureClass::MalformedResponse)?;
                    last_failure = Some(ProviderFailureClass::StreamAborted);
                    sleep_retry_delay(
                        ProviderFailureClass::StreamAborted,
                        max_attempts - remaining.len(),
                    );
                }
                Err(failure) => {
                    self.finish_failed(&attempt_id, failure)
                        .map_err(|_| ProviderFailureClass::MalformedResponse)?;
                    if !is_retryable(failure) {
                        return Err(failure);
                    }
                    last_failure = Some(failure);
                    sleep_retry_delay(failure, max_attempts - remaining.len());
                }
            }
        }
        Err(last_failure.unwrap_or(ProviderFailureClass::ServerError))
    }

    pub fn attempts(&self) -> &[RouteAttempt] {
        &self.attempts
    }

    pub fn provider_count(&self) -> usize {
        self.providers.len()
    }

    pub fn provider_names(&self) -> Vec<&str> {
        self.providers
            .values()
            .map(|provider| provider.name.as_str())
            .collect()
    }

    pub fn ranked_candidates(&self, profile: &TaskProfile) -> Vec<RouteCandidate> {
        let mut rejected = Vec::new();
        let mut candidates = self
            .routes
            .values()
            .filter_map(|route| self.candidate_for_route(route, profile, &mut rejected).ok())
            .collect::<Vec<_>>();
        candidates.sort_by_key(|candidate| std::cmp::Reverse(candidate.score));
        candidates
    }

    pub fn select_cost_optimized_route(
        &self,
        requirement: &ModelSelectionRequirement,
    ) -> AcResult<CostOptimizedRoute> {
        let routing_profile = match requirement.max_cost_tier {
            CostTier::Free => RoutingProfile::FreeOnly,
            CostTier::Cheap => RoutingProfile::FreeFirst,
            CostTier::Standard => RoutingProfile::LocalFirst,
            CostTier::Premium => RoutingProfile::QualityFirst,
        };
        let mut profile = TaskProfile::coding(requirement.task_id.clone(), routing_profile);
        profile.requires_vision = requirement.requires_vision;
        profile.complexity = requirement.min_quality.min(10);
        let mut candidates = self
            .ranked_candidates(&profile)
            .into_iter()
            .filter_map(|candidate| {
                let identity = self.model_identities.get(&candidate.model_identity_id)?;
                let connection = self.connections.get(&candidate.connection_id)?;
                let quality_score = identity.coding_score.max(identity.reasoning_score);
                let tier = cost_tier(identity.input_cost_micros, connection.paid);
                (quality_score >= requirement.min_quality
                    && cost_tier_rank(tier) <= cost_tier_rank(requirement.max_cost_tier))
                .then_some(CostOptimizedRoute {
                    estimated_latency_ms: if connection.local { 250 } else { 900 },
                    candidate,
                    cost_tier: tier,
                    quality_score,
                })
            })
            .collect::<Vec<_>>();
        candidates.sort_by_key(|route| {
            (
                cost_tier_rank(route.cost_tier),
                if requirement.prefer_low_latency {
                    route.estimated_latency_ms
                } else {
                    0
                },
                std::cmp::Reverse(route.quality_score),
            )
        });
        candidates.into_iter().next().ok_or_else(|| {
            AcError::validation(
                "PROVIDER-NO_COST_OPTIMIZED_ROUTE",
                "no route satisfies quality, capability, and cost requirements",
            )
        })
    }

    pub fn request_model(
        &mut self,
        profile: &TaskProfile,
        prompt: impl Into<String>,
        max_output_tokens: u32,
        cancel: &dyn Fn() -> bool,
    ) -> Result<RouteExecution, ProviderFailureClass> {
        let prompt = prompt.into();
        let mut rejected = Vec::new();
        let mut candidates = self
            .routes
            .values()
            .filter_map(|route| self.candidate_for_route(route, profile, &mut rejected).ok())
            .collect::<Vec<_>>();
        candidates.sort_by_key(|candidate| std::cmp::Reverse(candidate.score));
        let mut fallback_reason = None;
        let mut last_failure = None;
        for candidate in candidates.clone() {
            if cancel() {
                return Err(ProviderFailureClass::Cancelled);
            }
            let route = self
                .routes
                .get(&candidate.route_id)
                .ok_or(ProviderFailureClass::UnsupportedCapability)?
                .clone();
            let request = NormalizedInferenceRequest {
                model_id: route.model_id,
                prompt: prompt.clone(),
                required: required_from_profile(profile),
                max_output_tokens,
            };
            for attempt_index in 0..2 {
                let attempt_id = self
                    .start_attempt(&request)
                    .map_err(|_| ProviderFailureClass::MalformedResponse)?;
                let started = TimestampMillis::now();
                let result = {
                    let adapter = self
                        .adapters
                        .get(&candidate.provider_id)
                        .ok_or(ProviderFailureClass::UnsupportedCapability)?;
                    adapter.stream(&request, cancel)
                };
                let latency_ms = TimestampMillis::now()
                    .as_millis()
                    .saturating_sub(started.as_millis()) as u64;
                match result {
                    Ok(events) if matches!(events.last(), Some(ProviderStreamEvent::Finished)) => {
                        for event in &events {
                            self.record_event(&attempt_id, event.clone())
                                .map_err(|_| ProviderFailureClass::MalformedResponse)?;
                        }
                        self.record_route_success(&candidate.connection_id, latency_ms);
                        let usage = token_usage(&events);
                        let cost = self.estimate_cost(
                            &candidate.model_identity_id,
                            usage.input_tokens,
                            usage.output_tokens,
                        );
                        let decision = self.routing_decision(
                            profile,
                            candidates,
                            rejected,
                            Some(candidate),
                            fallback_reason,
                            latency_ms,
                            usage,
                            cost,
                        );
                        self.routing_decisions.push(decision.clone());
                        truncate_front(&mut self.routing_decisions, MAX_PROVIDER_HISTORY);
                        return Ok(RouteExecution { events, decision });
                    }
                    Ok(events) => {
                        for event in &events {
                            self.record_event(&attempt_id, event.clone())
                                .map_err(|_| ProviderFailureClass::MalformedResponse)?;
                        }
                        self.finish_failed(&attempt_id, ProviderFailureClass::StreamAborted)
                            .map_err(|_| ProviderFailureClass::MalformedResponse)?;
                        let normalized = normalize_failure(ProviderFailureClass::StreamAborted);
                        self.record_route_failure(
                            &candidate.connection_id,
                            normalized.clone(),
                            latency_ms,
                        );
                        fallback_reason = Some(format!(
                            "{}:{:?}:attempt{}",
                            candidate.connection_id,
                            normalized,
                            attempt_index + 1
                        ));
                        last_failure = Some(ProviderFailureClass::StreamAborted);
                        if !is_retryable(ProviderFailureClass::StreamAborted) || attempt_index == 1
                        {
                            break;
                        }
                        sleep_retry_delay(ProviderFailureClass::StreamAborted, attempt_index);
                    }
                    Err(failure) => {
                        self.finish_failed(&attempt_id, failure)
                            .map_err(|_| ProviderFailureClass::MalformedResponse)?;
                        let normalized = normalize_failure(failure);
                        self.record_route_failure(
                            &candidate.connection_id,
                            normalized.clone(),
                            latency_ms,
                        );
                        fallback_reason = Some(format!(
                            "{}:{:?}:attempt{}",
                            candidate.connection_id,
                            normalized,
                            attempt_index + 1
                        ));
                        last_failure = Some(failure);
                        if !is_retryable(failure) || attempt_index == 1 {
                            break;
                        }
                        sleep_retry_delay(failure, attempt_index);
                    }
                }
            }
        }
        let decision = self.routing_decision(
            profile,
            candidates,
            rejected,
            None,
            fallback_reason,
            0,
            TokenUsage {
                input_tokens: 0,
                output_tokens: 0,
                total_tokens: 0,
                source: UsageSource::Estimated,
            },
            CostEstimate {
                model_identity_id: StableId::new("model-family"),
                input_cost_micros: 0,
                output_cost_micros: 0,
                unit: PricingUnit::Unknown,
                calculated_cost_micros: None,
            },
        );
        self.routing_decisions.push(decision);
        truncate_front(&mut self.routing_decisions, MAX_PROVIDER_HISTORY);
        Err(last_failure.unwrap_or(ProviderFailureClass::UnsupportedCapability))
    }

    pub fn routing_decisions(&self) -> &[RoutingDecision] {
        &self.routing_decisions
    }

    pub fn health_observations(&self) -> &[HealthObservation] {
        &self.health_observations
    }

    pub fn circuit_breaker(&self, connection_id: &StableId) -> Option<&CircuitBreaker> {
        self.circuit_breakers.get(connection_id)
    }

    #[allow(clippy::too_many_arguments)]
    fn routing_decision(
        &self,
        profile: &TaskProfile,
        candidates: Vec<RouteCandidate>,
        rejected: Vec<String>,
        selected: Option<RouteCandidate>,
        fallback_reason: Option<String>,
        latency_ms: u64,
        usage: TokenUsage,
        cost: CostEstimate,
    ) -> RoutingDecision {
        RoutingDecision {
            id: StableId::new("routing"),
            task_id: profile.task_id.clone(),
            candidates,
            selected,
            rejected,
            fallback_reason,
            latency_ms,
            input_tokens: usage.input_tokens,
            output_tokens: usage.output_tokens,
            total_tokens: usage.total_tokens,
            usage_source: usage.source,
            estimated_cost_micros: cost.calculated_cost_micros.unwrap_or(0),
            cost_known: cost.calculated_cost_micros.is_some(),
            created_at: TimestampMillis::now(),
        }
    }

    fn candidate_for_route(
        &self,
        route: &ModelRoute,
        profile: &TaskProfile,
        rejected: &mut Vec<String>,
    ) -> Result<RouteCandidate, ()> {
        let identity = self
            .model_identities
            .get(&route.model_identity_id)
            .ok_or(())?;
        let connection = self.connections.get(&route.connection_id).ok_or(())?;
        let model = self.models.get(&route.model_id).ok_or(())?;
        let breaker_open = self
            .circuit_breakers
            .get(&connection.id)
            .map(CircuitBreaker::is_open)
            .unwrap_or(false);
        let reject = |rejected: &mut Vec<String>, reason: String| {
            rejected.push(format!("{}:{}", route.id, reason));
            Err(())
        };
        if !connection.enabled {
            return reject(rejected, "connection_disabled".to_string());
        }
        if breaker_open {
            return reject(rejected, "circuit_open".to_string());
        }
        if profile.routing_profile == RoutingProfile::Offline && !connection.local {
            return reject(rejected, "offline_requires_local".to_string());
        }
        if profile.routing_profile == RoutingProfile::FreeOnly && connection.paid {
            return reject(rejected, "paid_disallowed".to_string());
        }
        if profile.privacy == PrivacyClass::LocalOnly && !connection.local {
            return reject(rejected, "privacy_requires_local".to_string());
        }
        if profile.required_context > identity.context_window {
            return reject(rejected, "context_too_small".to_string());
        }
        if profile.requires_vision && !identity.vision {
            return reject(rejected, "vision_missing".to_string());
        }
        if profile.requires_tool_use && !identity.tool_use {
            return reject(rejected, "tool_use_missing".to_string());
        }
        if profile.requires_structured_output && !identity.structured_output {
            return reject(rejected, "structured_output_missing".to_string());
        }
        if !required_from_profile(profile)
            .iter()
            .all(|capability| model.capabilities.contains(capability))
        {
            return reject(rejected, "capability_missing".to_string());
        }
        if let Some(max_cost) = profile.max_input_cost_micros {
            if identity.input_cost_micros > max_cost
                && profile.routing_profile != RoutingProfile::PaidAllowed
            {
                return reject(rejected, "budget_exceeded".to_string());
            }
        }
        let mut score = if profile.role.contains("verification") || profile.role.contains("critic")
        {
            identity.reasoning_score as i32 * 2 + identity.coding_score as i32
        } else {
            identity.coding_score as i32 * 2 + identity.reasoning_score as i32
        };
        score += (identity.context_window / 4096).min(20) as i32;
        score -= (profile
            .complexity
            .saturating_sub(identity.reasoning_score / 20)) as i32;
        if connection.local {
            score += match profile.routing_profile {
                RoutingProfile::LocalFirst | RoutingProfile::Offline => 50,
                _ => 5,
            };
        }
        if !connection.paid {
            score += match profile.routing_profile {
                RoutingProfile::FreeOnly | RoutingProfile::FreeFirst => 60,
                _ => 5,
            };
        } else if profile.routing_profile == RoutingProfile::QualityFirst {
            score += 10;
        }
        Ok(RouteCandidate {
            route_id: route.id.clone(),
            model_identity_id: identity.id.clone(),
            provider_id: route.provider_id.clone(),
            connection_id: connection.id.clone(),
            model_name: route.model_name.clone(),
            score,
            reasons: vec![
                format!("role:{}", profile.role),
                format!("task:{}", profile.task_type),
                format!("profile:{:?}", profile.routing_profile),
                format!("local:{}", connection.local),
                format!("paid:{}", connection.paid),
            ],
        })
    }

    fn record_route_success(&mut self, connection_id: &StableId, latency_ms: u64) {
        if let Some(breaker) = self.circuit_breakers.get_mut(connection_id) {
            breaker.record_success();
        }
        self.health_observations.push(HealthObservation {
            connection_id: connection_id.clone(),
            failure: None,
            latency_ms,
            created_at: TimestampMillis::now(),
        });
        truncate_front(&mut self.health_observations, MAX_PROVIDER_HISTORY);
    }

    fn record_route_failure(
        &mut self,
        connection_id: &StableId,
        failure: NormalizedProviderFailure,
        latency_ms: u64,
    ) {
        if let Some(breaker) = self.circuit_breakers.get_mut(connection_id) {
            breaker.record_failure(failure.clone());
        }
        self.health_observations.push(HealthObservation {
            connection_id: connection_id.clone(),
            failure: Some(failure),
            latency_ms,
            created_at: TimestampMillis::now(),
        });
        truncate_front(&mut self.health_observations, MAX_PROVIDER_HISTORY);
    }

    fn estimate_cost(
        &self,
        model_identity_id: &StableId,
        input_tokens: u32,
        output_tokens: u32,
    ) -> CostEstimate {
        self.model_identities
            .get(model_identity_id)
            .map(|identity| {
                let calculated_cost_micros =
                    (identity.pricing_unit == PricingUnit::PerTokenMicrosUsd).then(|| {
                        input_tokens as u64 * identity.input_cost_micros as u64
                            + output_tokens as u64 * identity.output_cost_micros as u64
                    });
                CostEstimate {
                    model_identity_id: identity.id.clone(),
                    input_cost_micros: identity.input_cost_micros,
                    output_cost_micros: identity.output_cost_micros,
                    unit: identity.pricing_unit,
                    calculated_cost_micros,
                }
            })
            .unwrap_or(CostEstimate {
                model_identity_id: model_identity_id.clone(),
                input_cost_micros: 0,
                output_cost_micros: 0,
                unit: PricingUnit::Unknown,
                calculated_cost_micros: None,
            })
    }
}

fn is_retryable(failure: ProviderFailureClass) -> bool {
    matches!(
        failure,
        ProviderFailureClass::RateLimited
            | ProviderFailureClass::RateLimitedAfter(_)
            | ProviderFailureClass::Timeout
            | ProviderFailureClass::ServerError
            | ProviderFailureClass::Network
            | ProviderFailureClass::StreamAborted
    )
}

fn sleep_retry_delay(failure: ProviderFailureClass, attempt_index: usize) {
    std::thread::sleep(Duration::from_millis(retry_delay_millis(
        failure,
        attempt_index,
    )));
}

fn retry_delay_millis(failure: ProviderFailureClass, attempt_index: usize) -> u64 {
    if let ProviderFailureClass::RateLimitedAfter(seconds) = failure {
        return seconds.saturating_mul(1_000).min(30_000);
    }
    let base = 100_u64.saturating_mul(1_u64 << attempt_index.min(5));
    let jitter = (attempt_index as u64).wrapping_mul(37) % 41;
    base.saturating_add(jitter).min(5_000)
}

pub fn normalize_failure(failure: ProviderFailureClass) -> NormalizedProviderFailure {
    match failure {
        ProviderFailureClass::RateLimited | ProviderFailureClass::RateLimitedAfter(_) => {
            NormalizedProviderFailure::RateLimit
        }
        ProviderFailureClass::Timeout => NormalizedProviderFailure::Timeout,
        ProviderFailureClass::ServerError | ProviderFailureClass::Network => {
            NormalizedProviderFailure::ProviderUnavailable
        }
        ProviderFailureClass::StreamAborted => NormalizedProviderFailure::ProviderUnavailable,
        ProviderFailureClass::MalformedResponse | ProviderFailureClass::InvalidRequest => {
            NormalizedProviderFailure::BadResponse
        }
        ProviderFailureClass::Auth | ProviderFailureClass::Permission => {
            NormalizedProviderFailure::AuthFailed
        }
        ProviderFailureClass::ContextOverflow => NormalizedProviderFailure::ContextLimit,
        ProviderFailureClass::UnsupportedCapability | ProviderFailureClass::ModelUnavailable => {
            NormalizedProviderFailure::ModelUnavailable
        }
        ProviderFailureClass::Cancelled => NormalizedProviderFailure::ProviderUnavailable,
    }
}

fn required_from_profile(profile: &TaskProfile) -> Vec<ProviderCapability> {
    let mut required = vec![ProviderCapability::Chat];
    if profile.requires_tool_use {
        required.push(ProviderCapability::ToolCalls);
    }
    if profile.requires_vision {
        required.push(ProviderCapability::Vision);
    }
    required
}

fn cost_tier(input_cost_micros: u32, paid: bool) -> CostTier {
    if !paid {
        CostTier::Free
    } else if input_cost_micros <= 1_000 {
        CostTier::Cheap
    } else if input_cost_micros <= 5_000 {
        CostTier::Standard
    } else {
        CostTier::Premium
    }
}

fn cost_tier_rank(tier: CostTier) -> u8 {
    match tier {
        CostTier::Free => 0,
        CostTier::Cheap => 1,
        CostTier::Standard => 2,
        CostTier::Premium => 3,
    }
}

fn token_usage(events: &[ProviderStreamEvent]) -> TokenUsage {
    let (input_tokens, output_tokens, source) = events
        .iter()
        .find_map(|event| {
            if let ProviderStreamEvent::Usage {
                input_tokens,
                output_tokens,
            } = event
            {
                Some((*input_tokens, *output_tokens, UsageSource::ProviderReported))
            } else if let ProviderStreamEvent::EstimatedUsage {
                input_tokens,
                output_tokens,
            } = event
            {
                Some((*input_tokens, *output_tokens, UsageSource::Estimated))
            } else {
                None
            }
        })
        .unwrap_or((0, 0, UsageSource::Estimated));
    TokenUsage {
        input_tokens,
        output_tokens,
        total_tokens: input_tokens.saturating_add(output_tokens),
        source,
    }
}

fn ensure_usage_event(
    mut events: Vec<ProviderStreamEvent>,
    prompt: &str,
) -> Vec<ProviderStreamEvent> {
    if events.iter().any(|event| {
        matches!(
            event,
            ProviderStreamEvent::Usage { .. } | ProviderStreamEvent::EstimatedUsage { .. }
        )
    }) {
        return events;
    }
    let output = events
        .iter()
        .filter_map(|event| {
            if let ProviderStreamEvent::Delta(text) = event {
                Some(text.as_str())
            } else {
                None
            }
        })
        .collect::<Vec<_>>()
        .join("");
    let insert_at = events
        .iter()
        .position(|event| matches!(event, ProviderStreamEvent::Finished))
        .unwrap_or(events.len());
    events.insert(
        insert_at,
        ProviderStreamEvent::EstimatedUsage {
            input_tokens: estimate_token_count(prompt),
            output_tokens: estimate_token_count(&output),
        },
    );
    events
}

fn estimate_token_count(text: &str) -> u32 {
    let chars = text.chars().count();
    u32::try_from(chars.div_ceil(4)).unwrap_or(u32::MAX).max(1)
}

fn validate_endpoint(endpoint: &str, allow_plain_http_remote: bool) -> AcResult<()> {
    let url = Url::parse(endpoint).map_err(|_| {
        AcError::validation(
            "PROVIDER-INVALID_HTTP_CONFIG",
            "provider endpoint must be a valid URL",
        )
    })?;
    if is_metadata_or_link_local_endpoint(&url) {
        return Err(AcError::validation(
            "PROVIDER-METADATA_ENDPOINT_DENIED",
            "provider endpoint may not target metadata or link-local hosts",
        ));
    }
    match url.scheme() {
        "https" => Ok(()),
        "http" if is_loopback_endpoint(&url) => Ok(()),
        "http" if allow_plain_http_remote && is_private_lan_endpoint(&url) => Ok(()),
        "http" => Err(AcError::validation(
            "PROVIDER-INSECURE_REMOTE_ENDPOINT",
            "remote provider endpoints must use HTTPS; plain HTTP is limited to loopback or explicitly opted-in private LAN model endpoints",
        )),
        _ => Err(AcError::validation(
            "PROVIDER-INVALID_HTTP_CONFIG",
            "provider endpoint must use http or https",
        )),
    }
}

fn is_loopback_endpoint(url: &Url) -> bool {
    let Some(host) = url.host_str() else {
        return false;
    };
    host == "localhost" || host == "127.0.0.1" || host == "::1" || host == "[::1]"
}

fn is_private_lan_endpoint(url: &Url) -> bool {
    let Some(host) = url.host_str().map(|host| host.trim_matches(['[', ']'])) else {
        return false;
    };
    if host.starts_with("10.") || host.starts_with("192.168.") {
        return true;
    }
    let mut parts = host.split('.');
    matches!(
        (
            parts.next().and_then(|part| part.parse::<u8>().ok()),
            parts.next().and_then(|part| part.parse::<u8>().ok()),
            parts.next(),
            parts.next(),
        ),
        (Some(172), Some(16..=31), Some(_), Some(_))
    )
}

fn is_metadata_or_link_local_endpoint(url: &Url) -> bool {
    let Some(host) = url.host_str().map(|host| host.trim_matches(['[', ']'])) else {
        return false;
    };
    host == "169.254.169.254"
        || host.starts_with("169.254.")
        || host.eq_ignore_ascii_case("metadata.google.internal")
        || host.to_ascii_lowercase().starts_with("fe80:")
}

fn validate_custom_headers(headers: &[(String, String)]) -> AcResult<()> {
    for (name, value) in headers {
        let normalized = name.to_ascii_lowercase();
        if normalized == "authorization"
            || normalized == "proxy-authorization"
            || normalized == "api-key"
            || normalized == "x-api-key"
            || normalized == "x-goog-api-key"
            || normalized == "x-auth-token"
            || normalized == "x-api-token"
            || normalized == "cookie"
        {
            return Err(AcError::validation(
                "PROVIDER-UNSAFE_CUSTOM_HEADER",
                "custom provider headers may not contain credentials",
            ));
        }
        HeaderName::from_bytes(name.as_bytes()).map_err(|_| {
            AcError::validation(
                "PROVIDER-INVALID_HTTP_CONFIG",
                "custom provider header names must be valid HTTP header names",
            )
        })?;
        HeaderValue::from_str(value).map_err(|_| {
            AcError::validation(
                "PROVIDER-INVALID_HTTP_CONFIG",
                "custom provider header values must be valid HTTP header values",
            )
        })?;
    }
    Ok(())
}

fn extract_string_array(raw: &str, key: &str) -> AcResult<Vec<String>> {
    let needle = format!("\"{}\"", key);
    let after_key = raw
        .split_once(&needle)
        .map(|(_, after)| after)
        .ok_or_else(|| {
            AcError::validation(
                "PROVIDER-MISSING_STRUCTURED_FIELD",
                format!("structured provider output missing {key}"),
            )
        })?;
    let after_colon = after_key
        .split_once(':')
        .map(|(_, after)| after)
        .ok_or_else(|| {
            AcError::validation(
                "PROVIDER-MALFORMED_STRUCTURED_FIELD",
                format!("structured provider output field {key} is malformed"),
            )
        })?;
    let start = after_colon.find('[').ok_or_else(|| {
        AcError::validation(
            "PROVIDER-MALFORMED_STRUCTURED_FIELD",
            format!("structured provider output field {key} must be an array"),
        )
    })?;
    let array = &after_colon[start + 1..];
    let end = array.find(']').ok_or_else(|| {
        AcError::validation(
            "PROVIDER-MALFORMED_STRUCTURED_FIELD",
            format!("structured provider output field {key} must close its array"),
        )
    })?;
    Ok(array[..end]
        .split(',')
        .filter_map(|value| {
            let trimmed = value.trim();
            if trimmed.starts_with('"') && trimmed.ends_with('"') && trimmed.len() >= 2 {
                Some(
                    trimmed[1..trimmed.len() - 1]
                        .replace("\\\"", "\"")
                        .replace("\\\\", "\\")
                        .trim()
                        .to_string(),
                )
            } else {
                None
            }
        })
        .filter(|value| !value.is_empty())
        .collect())
}

pub(crate) fn map_reqwest_error(error: reqwest::Error) -> ProviderFailureClass {
    if error.is_timeout() {
        ProviderFailureClass::Timeout
    } else if error.is_builder() {
        ProviderFailureClass::MalformedResponse
    } else if error.is_connect() || error.is_request() {
        ProviderFailureClass::Network
    } else {
        ProviderFailureClass::ServerError
    }
}

/// Resolve a credential configuration value to its secret, using the single
/// canonical backend resolver.  Full references (`env:NAME`, `secret:NAME`)
/// are resolved through `resolve_credential_ref` so a persisted `secret:NAME`
/// account reference is honored on the real request path.  Bare environment
/// variable names (the legacy adapter configuration shape) are read directly
/// from the environment so those configurations keep working.
fn resolve_adapter_credential(credential: &str) -> AcResult<String> {
    if credential.starts_with("env:") || credential.starts_with("secret:") {
        catalog::resolve_credential_ref(credential)
    } else {
        std::env::var(credential).map_err(|_| {
            AcError::validation(
                "PROVIDER-CREDENTIAL_UNAVAILABLE",
                format!("environment credential {credential} is not set"),
            )
        })
    }
}

fn events_from_response(
    mut response: Response,
    provider_kind: HttpProviderKind,
    max_response_bytes: usize,
    cancel: &dyn Fn() -> bool,
) -> Result<Vec<ProviderStreamEvent>, ProviderFailureClass> {
    let status = response.status().as_u16();
    let retry_after_seconds = response
        .headers()
        .get("retry-after")
        .and_then(|value| value.to_str().ok())
        .and_then(parse_retry_after_seconds);
    let mut body = Vec::new();
    let mut chunk = [0_u8; 256];
    loop {
        if cancel() {
            return Err(ProviderFailureClass::Cancelled);
        }
        let read = response.read(&mut chunk).map_err(|error| {
            if error.kind() == std::io::ErrorKind::TimedOut {
                ProviderFailureClass::Timeout
            } else {
                ProviderFailureClass::StreamAborted
            }
        })?;
        if read == 0 {
            break;
        }
        body.extend_from_slice(&chunk[..read]);
        if body.len() > max_response_bytes {
            return Err(ProviderFailureClass::MalformedResponse);
        }
    }
    let body = String::from_utf8(body).map_err(|_| ProviderFailureClass::MalformedResponse)?;
    events_from_status_and_body_with_retry_after(status, &body, provider_kind, retry_after_seconds)
}

#[cfg(test)]
fn events_from_status_and_body(
    status: u16,
    body: &str,
    provider_kind: HttpProviderKind,
) -> Result<Vec<ProviderStreamEvent>, ProviderFailureClass> {
    events_from_status_and_body_with_retry_after(status, body, provider_kind, None)
}

fn events_from_status_and_body_with_retry_after(
    status: u16,
    body: &str,
    provider_kind: HttpProviderKind,
    retry_after_seconds: Option<u64>,
) -> Result<Vec<ProviderStreamEvent>, ProviderFailureClass> {
    match status {
        200..=299 => parse_provider_body(body, provider_kind),
        401 | 403 => Err(ProviderFailureClass::Auth),
        408 | 504 => Err(ProviderFailureClass::Timeout),
        429 => Err(retry_after_seconds
            .map(ProviderFailureClass::RateLimitedAfter)
            .unwrap_or(ProviderFailureClass::RateLimited)),
        400 => provider_error_class(body).map_err(|failure| {
            if failure == ProviderFailureClass::MalformedResponse {
                ProviderFailureClass::InvalidRequest
            } else {
                failure
            }
        }),
        404 => provider_error_class(body).map_err(|failure| {
            if failure == ProviderFailureClass::MalformedResponse {
                ProviderFailureClass::ModelUnavailable
            } else {
                failure
            }
        }),
        413 => Err(ProviderFailureClass::ContextOverflow),
        status if (400..=499).contains(&status) => provider_error_class(body),
        _ => Err(ProviderFailureClass::ServerError),
    }
}

fn parse_retry_after_seconds(value: &str) -> Option<u64> {
    value
        .trim()
        .parse::<u64>()
        .ok()
        .map(|seconds| seconds.min(30))
}

fn truncate_front<T>(items: &mut Vec<T>, limit: usize) {
    let overflow = items.len().saturating_sub(limit);
    if overflow > 0 {
        items.drain(0..overflow);
    }
}

fn parse_provider_body(
    body: &str,
    provider_kind: HttpProviderKind,
) -> Result<Vec<ProviderStreamEvent>, ProviderFailureClass> {
    let trimmed = body.trim();
    if trimmed.is_empty() {
        return Err(ProviderFailureClass::MalformedResponse);
    }
    if trimmed.contains("\ndata:") || trimmed.starts_with("data:") {
        return parse_sse_body(trimmed, provider_kind);
    }
    if trimmed.lines().count() > 1 && trimmed.lines().all(|line| line.trim().starts_with('{')) {
        return parse_json_lines(trimmed, provider_kind);
    }
    let value: Value =
        serde_json::from_str(trimmed).map_err(|_| ProviderFailureClass::MalformedResponse)?;
    events_from_json_value(&value, provider_kind)
}

fn parse_sse_body(
    body: &str,
    provider_kind: HttpProviderKind,
) -> Result<Vec<ProviderStreamEvent>, ProviderFailureClass> {
    let mut events = Vec::new();
    let mut data_fields = Vec::new();
    for line in body.lines() {
        let line = line.trim_end_matches('\r');
        if line.is_empty() {
            flush_sse_data(&mut events, &mut data_fields, provider_kind)?;
        } else if let Some(payload) = line.strip_prefix("data:") {
            if sse_payload_complete(&data_fields) {
                flush_sse_data(&mut events, &mut data_fields, provider_kind)?;
            }
            data_fields.push(payload.trim_start().to_string());
        }
    }
    flush_sse_data(&mut events, &mut data_fields, provider_kind)?;
    finish_events(events)
}

fn flush_sse_data(
    events: &mut Vec<ProviderStreamEvent>,
    data_fields: &mut Vec<String>,
    provider_kind: HttpProviderKind,
) -> Result<(), ProviderFailureClass> {
    if data_fields.is_empty() {
        return Ok(());
    }
    let payload = data_fields.join("\n");
    data_fields.clear();
    let payload = payload.trim();
    if payload == "[DONE]" {
        events.push(ProviderStreamEvent::Finished);
        return Ok(());
    }
    let value: Value =
        serde_json::from_str(payload).map_err(|_| ProviderFailureClass::MalformedResponse)?;
    if let Some(failure) = provider_error_class_from_value(&value) {
        return Err(failure);
    }
    append_json_events(events, &value, provider_kind)
}

fn sse_payload_complete(data_fields: &[String]) -> bool {
    if data_fields.is_empty() {
        return false;
    }
    let payload = data_fields.join("\n");
    payload.trim() == "[DONE]" || serde_json::from_str::<Value>(&payload).is_ok()
}

fn parse_json_lines(
    body: &str,
    provider_kind: HttpProviderKind,
) -> Result<Vec<ProviderStreamEvent>, ProviderFailureClass> {
    let mut events = Vec::new();
    for line in body.lines().map(str::trim).filter(|line| !line.is_empty()) {
        let value: Value =
            serde_json::from_str(line).map_err(|_| ProviderFailureClass::MalformedResponse)?;
        if let Some(failure) = provider_error_class_from_value(&value) {
            return Err(failure);
        }
        append_json_events(&mut events, &value, provider_kind)?;
    }
    finish_events(events)
}

fn events_from_json_value(
    value: &Value,
    provider_kind: HttpProviderKind,
) -> Result<Vec<ProviderStreamEvent>, ProviderFailureClass> {
    let mut events = Vec::new();
    if let Some(failure) = provider_error_class_from_value(value) {
        return Err(failure);
    }
    append_json_events(&mut events, value, provider_kind)?;
    finish_events(events)
}

fn append_json_events(
    events: &mut Vec<ProviderStreamEvent>,
    value: &Value,
    provider_kind: HttpProviderKind,
) -> Result<(), ProviderFailureClass> {
    if let Some(text) = extract_provider_text(value, provider_kind) {
        if !text.is_empty() {
            events.push(ProviderStreamEvent::Delta(text));
        }
    }
    if let Some((input_tokens, output_tokens)) = extract_usage(value, provider_kind) {
        events.push(ProviderStreamEvent::Usage {
            input_tokens,
            output_tokens,
        });
    }
    if provider_finished(value, provider_kind) {
        events.push(ProviderStreamEvent::Finished);
    }
    Ok(())
}

fn finish_events(
    mut events: Vec<ProviderStreamEvent>,
) -> Result<Vec<ProviderStreamEvent>, ProviderFailureClass> {
    if !events
        .iter()
        .any(|event| matches!(event, ProviderStreamEvent::Delta(_)))
    {
        return Err(ProviderFailureClass::MalformedResponse);
    }
    if !matches!(events.last(), Some(ProviderStreamEvent::Finished)) {
        events.push(ProviderStreamEvent::Finished);
    }
    Ok(events)
}

fn provider_error_class(body: &str) -> Result<Vec<ProviderStreamEvent>, ProviderFailureClass> {
    let Ok(value) = serde_json::from_str::<Value>(body) else {
        return Err(ProviderFailureClass::MalformedResponse);
    };
    Err(provider_error_class_from_value(&value).unwrap_or(ProviderFailureClass::MalformedResponse))
}

pub(crate) fn provider_error_class_from_value(value: &Value) -> Option<ProviderFailureClass> {
    let error = value.get("error")?;
    let code = error
        .get("code")
        .or_else(|| error.get("type"))
        .or_else(|| error.get("status"))
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_ascii_lowercase();
    let message = error
        .get("message")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_ascii_lowercase();
    let combined = format!("{code} {message}");
    if combined.contains("auth")
        || combined.contains("permission")
        || combined.contains("api key")
        || combined.contains("unauthorized")
    {
        Some(ProviderFailureClass::Auth)
    } else if combined.contains("rate") || combined.contains("quota") {
        Some(ProviderFailureClass::RateLimited)
    } else if combined.contains("context")
        || combined.contains("token") && combined.contains("limit")
    {
        Some(ProviderFailureClass::ContextOverflow)
    } else if combined.contains("model")
        && (combined.contains("not found") || combined.contains("unavailable"))
    {
        Some(ProviderFailureClass::ModelUnavailable)
    } else if combined.contains("invalid") || combined.contains("bad request") {
        Some(ProviderFailureClass::InvalidRequest)
    } else {
        Some(ProviderFailureClass::MalformedResponse)
    }
}

fn extract_provider_text(value: &Value, provider_kind: HttpProviderKind) -> Option<String> {
    match provider_kind {
        HttpProviderKind::OpenAiCompatible | HttpProviderKind::OpenAiChatCompletions => value
            .pointer("/choices/0/delta/content")
            .or_else(|| value.pointer("/choices/0/message/content"))
            .or_else(|| value.pointer("/delta"))
            .and_then(Value::as_str)
            .map(ToString::to_string),
        HttpProviderKind::AnthropicMessages => value
            .pointer("/delta/text")
            .or_else(|| value.pointer("/content/0/text"))
            .and_then(Value::as_str)
            .map(ToString::to_string),
        HttpProviderKind::GeminiGenerateContent => value
            .pointer("/candidates/0/content/parts/0/text")
            .and_then(Value::as_str)
            .map(ToString::to_string),
        HttpProviderKind::OllamaChat => value
            .pointer("/message/content")
            .or_else(|| value.pointer("/response"))
            .and_then(Value::as_str)
            .map(ToString::to_string),
    }
}

fn extract_usage(value: &Value, provider_kind: HttpProviderKind) -> Option<(u32, u32)> {
    let as_u32 = |pointer: &str| {
        value
            .pointer(pointer)
            .and_then(Value::as_u64)
            .and_then(|value| u32::try_from(value).ok())
    };
    match provider_kind {
        HttpProviderKind::OpenAiCompatible | HttpProviderKind::OpenAiChatCompletions => Some((
            as_u32("/usage/prompt_tokens")?,
            as_u32("/usage/completion_tokens")?,
        )),
        HttpProviderKind::AnthropicMessages => Some((
            as_u32("/usage/input_tokens")?,
            as_u32("/usage/output_tokens")?,
        )),
        HttpProviderKind::GeminiGenerateContent => Some((
            as_u32("/usageMetadata/promptTokenCount")?,
            as_u32("/usageMetadata/candidatesTokenCount")?,
        )),
        HttpProviderKind::OllamaChat => {
            Some((as_u32("/prompt_eval_count")?, as_u32("/eval_count")?))
        }
    }
}

fn provider_finished(value: &Value, provider_kind: HttpProviderKind) -> bool {
    match provider_kind {
        HttpProviderKind::OpenAiCompatible | HttpProviderKind::OpenAiChatCompletions => value
            .pointer("/choices/0/finish_reason")
            .is_some_and(|reason| !reason.is_null()),
        HttpProviderKind::AnthropicMessages => value
            .get("type")
            .and_then(Value::as_str)
            .is_some_and(|event_type| event_type == "message_stop"),
        HttpProviderKind::GeminiGenerateContent => {
            value.pointer("/candidates/0/finishReason").is_some()
        }
        HttpProviderKind::OllamaChat => value.get("done").and_then(Value::as_bool).unwrap_or(false),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::sync::mpsc;
    use std::sync::Mutex;
    use std::thread;
    use std::time::Duration as StdDuration;

    struct RetryProvider {
        responses: Mutex<VecDeque<Result<Vec<ProviderStreamEvent>, ProviderFailureClass>>>,
    }

    impl ProviderAdapter for RetryProvider {
        fn stream(
            &self,
            _request: &NormalizedInferenceRequest,
            cancel: &dyn Fn() -> bool,
        ) -> Result<Vec<ProviderStreamEvent>, ProviderFailureClass> {
            if cancel() {
                return Err(ProviderFailureClass::Cancelled);
            }
            self.responses
                .lock()
                .unwrap()
                .pop_front()
                .unwrap_or(Err(ProviderFailureClass::ServerError))
        }
    }

    struct Phase4Fabric {
        registry: ProviderRegistry,
        free_connection: StableId,
        paid_connection: StableId,
        local_connection: StableId,
    }

    fn successful_events(text: &str) -> Vec<ProviderStreamEvent> {
        vec![
            ProviderStreamEvent::Delta(text.to_string()),
            ProviderStreamEvent::Usage {
                input_tokens: 7,
                output_tokens: 11,
            },
            ProviderStreamEvent::Finished,
        ]
    }

    fn request() -> NormalizedInferenceRequest {
        NormalizedInferenceRequest {
            model_id: StableId::new("model"),
            prompt: "plan".to_string(),
            required: vec![ProviderCapability::Chat],
            max_output_tokens: 32,
        }
    }

    fn mock_server(
        status: u16,
        body: &'static str,
        delay: Option<StdDuration>,
    ) -> Option<(String, mpsc::Receiver<String>)> {
        let listener = match TcpListener::bind("127.0.0.1:0") {
            Ok(listener) => listener,
            Err(error) if error.kind() == std::io::ErrorKind::PermissionDenied => return None,
            Err(error) => panic!("mock provider server bind failed: {error}"),
        };
        let endpoint = format!("http://{}/v1/chat", listener.local_addr().unwrap());
        let (tx, rx) = mpsc::channel();
        thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut request = [0_u8; 4096];
            let read = stream.read(&mut request).unwrap_or(0);
            tx.send(String::from_utf8_lossy(&request[..read]).to_string())
                .unwrap();
            if let Some(delay) = delay {
                thread::sleep(delay);
            }
            let reason = if status == 200 { "OK" } else { "ERR" };
            let response = format!(
                "HTTP/1.1 {status} {reason}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
            let _ = stream.write_all(response.as_bytes());
        });
        Some((endpoint, rx))
    }

    #[allow(clippy::too_many_arguments)]
    fn register_route(
        registry: &mut ProviderRegistry,
        provider_name: &str,
        adapter: Box<dyn ProviderAdapter>,
        family: &str,
        model_name: &str,
        coding_score: u8,
        reasoning_score: u8,
        paid: bool,
        local: bool,
    ) -> (StableId, StableId, StableId) {
        let provider = registry
            .register_provider(
                provider_name,
                Some(format!("secret:{}", provider_name)),
                vec![ProviderCapability::Chat, ProviderCapability::Streaming],
                provider_name,
            )
            .unwrap();
        registry.register_adapter(&provider, adapter).unwrap();
        let model = registry
            .register_model(
                &provider,
                model_name,
                vec![ProviderCapability::Chat, ProviderCapability::Streaming],
                16_384,
            )
            .unwrap();
        let connection = registry
            .register_connection(
                &provider,
                format!("account:{}", provider_name),
                Some(format!("env:{}_KEY", provider_name.to_ascii_uppercase())),
                provider_name,
                format!("config:{}.endpoint", provider_name),
                true,
                paid,
                local,
            )
            .unwrap();
        let identity = registry
            .register_model_identity(
                family,
                16_384,
                coding_score,
                reasoning_score,
                false,
                false,
                true,
                if paid { 2 } else { 0 },
                if paid { 4 } else { 0 },
                if local {
                    PrivacyClass::LocalOnly
                } else {
                    PrivacyClass::ExternalAllowed
                },
            )
            .unwrap();
        let route = registry
            .register_model_route(&identity, &model, &connection)
            .unwrap();
        (connection, identity, route)
    }

    fn phase4_fabric(
        free_adapter: Box<dyn ProviderAdapter>,
        paid_adapter: Box<dyn ProviderAdapter>,
    ) -> Phase4Fabric {
        let mut registry = ProviderRegistry::new();
        let (free_connection, _, _) = register_route(
            &mut registry,
            "free-remote",
            free_adapter,
            "qwen-coder-free",
            "free-coder",
            90,
            65,
            false,
            false,
        );
        let (paid_connection, _, _) = register_route(
            &mut registry,
            "paid-remote",
            paid_adapter,
            "deepseek-paid",
            "paid-coder",
            95,
            90,
            true,
            false,
        );
        let (local_connection, _, _) = register_route(
            &mut registry,
            "ollama-local",
            Box::new(ScriptedProvider::new(vec![Ok(successful_events("local"))])),
            "qwen2.5-coder:3b",
            "qwen2.5-coder:3b",
            70,
            60,
            false,
            true,
        );
        Phase4Fabric {
            registry,
            free_connection,
            paid_connection,
            local_connection,
        }
    }

    #[test]
    fn model_selection_uses_capabilities_not_provider_state() {
        let mut registry = ProviderRegistry::new();
        let provider = registry
            .register_provider("local", None, vec![ProviderCapability::LocalModel], "local")
            .unwrap();
        registry
            .register_model(
                &provider,
                "small",
                vec![ProviderCapability::Chat, ProviderCapability::Streaming],
                8192,
            )
            .unwrap();

        let request = registry
            .normalize_request("hello", vec![ProviderCapability::Streaming], 128)
            .unwrap();
        let attempt = registry.start_attempt(&request).unwrap();
        registry
            .record_event(&attempt, ProviderStreamEvent::Delta("hi".to_string()))
            .unwrap();

        assert_eq!(registry.attempts()[0].events.len(), 1);
    }

    #[test]
    fn unsupported_capability_is_explicit() {
        let registry = ProviderRegistry::new();
        let err = registry
            .select_model(&[ProviderCapability::Vision])
            .unwrap_err();
        assert_eq!(err.code(), "PROVIDER-NO_CAPABLE_MODEL");
    }

    #[test]
    fn phase20_21_profiles_express_discuss_and_design_routing_needs() {
        let discuss = TaskProfile::discuss(StableId::new("task"), RoutingProfile::LocalFirst);
        assert_eq!(discuss.task_type, "repo_discussion");
        assert!(discuss.requires_structured_output);
        assert!(!discuss.requires_tool_use);

        let design = TaskProfile::design(StableId::new("task"), RoutingProfile::QualityFirst, true);
        assert_eq!(design.task_type, "design_studio");
        assert!(design.requires_vision);
        assert!(design.role.contains("critic"));
    }

    #[test]
    fn phase23_cost_optimized_selection_prefers_affordable_capable_routes() {
        let fabric = phase4_fabric(
            Box::new(ScriptedProvider::new(vec![Ok(successful_events("free"))])),
            Box::new(ScriptedProvider::new(vec![Ok(successful_events("paid"))])),
        );
        let route = fabric
            .registry
            .select_cost_optimized_route(&ModelSelectionRequirement {
                task_id: StableId::new("task"),
                min_quality: 70,
                max_cost_tier: CostTier::Free,
                prefer_low_latency: true,
                requires_vision: false,
            })
            .unwrap();
        assert_eq!(route.cost_tier, CostTier::Free);
        assert!(route.quality_score >= 70);
    }

    #[test]
    fn provider_streaming_retries_and_records_lifecycle() {
        let mut registry = ProviderRegistry::new();
        let provider = registry
            .register_provider(
                "mock",
                Some("env:AGENTCODE_PROVIDER_KEY".to_string()),
                vec![ProviderCapability::Chat, ProviderCapability::Streaming],
                "mock",
            )
            .unwrap();
        registry
            .register_adapter(
                &provider,
                Box::new(RetryProvider {
                    responses: Mutex::new(VecDeque::from([
                        Err(ProviderFailureClass::Timeout),
                        Ok(vec![
                            ProviderStreamEvent::Delta("plan".to_string()),
                            ProviderStreamEvent::Usage {
                                input_tokens: 3,
                                output_tokens: 5,
                            },
                            ProviderStreamEvent::Finished,
                        ]),
                    ])),
                }),
            )
            .unwrap();
        registry
            .register_model(
                &provider,
                "mock-model",
                vec![ProviderCapability::Chat, ProviderCapability::Streaming],
                4096,
            )
            .unwrap();
        let request = registry
            .normalize_request("plan task", vec![ProviderCapability::Chat], 128)
            .unwrap();
        let events = registry.stream_with_retry(&request, 2, &|| false).unwrap();
        assert!(matches!(events.last(), Some(ProviderStreamEvent::Finished)));
        assert_eq!(registry.attempts().len(), 2);
        assert_eq!(
            registry.attempts()[0].failure,
            Some(ProviderFailureClass::Timeout)
        );
    }

    #[test]
    fn provider_streaming_obeys_cancellation() {
        let mut registry = ProviderRegistry::new();
        let provider = registry
            .register_provider("mock", None, vec![ProviderCapability::Chat], "mock")
            .unwrap();
        registry
            .register_adapter(&provider, Box::new(ScriptedProvider::default()))
            .unwrap();
        registry
            .register_model(
                &provider,
                "mock-model",
                vec![ProviderCapability::Chat],
                4096,
            )
            .unwrap();
        let request = registry
            .normalize_request("plan task", vec![ProviderCapability::Chat], 128)
            .unwrap();
        let failure = registry
            .stream_with_retry(&request, 1, &|| true)
            .unwrap_err();
        assert_eq!(failure, ProviderFailureClass::Cancelled);
    }

    #[test]
    fn configured_provider_adapter_is_not_a_production_fake_completion() {
        let adapter = ConfiguredProviderAdapter::new(
            "config:provider.endpoint",
            "credential:provider_key_ref",
            "m",
        )
        .unwrap();
        let request = NormalizedInferenceRequest {
            model_id: StableId::new("model"),
            prompt: "hello world".to_string(),
            required: vec![ProviderCapability::Chat],
            max_output_tokens: 16,
        };
        assert_eq!(
            adapter.stream(&request, &|| false).unwrap_err(),
            ProviderFailureClass::UnsupportedCapability
        );
        assert!(!format!("{adapter:?}").contains("provider.key="));
    }

    #[test]
    fn http_provider_adapter_uses_reqwest_auth_headers_and_parses_openai_stream() {
        std::env::set_var("AGENTCODE_TEST_PROVIDER_KEY", "test-key");
        let body = "data: {\"choices\":[{\"delta\":{\"content\":\"goal=fix\\n\"}}]}\n\
data: {\"choices\":[{\"delta\":{\"content\":\"verify=status:0\"},\"finish_reason\":\"stop\"}],\"usage\":{\"prompt_tokens\":4,\"completion_tokens\":6}}\n\
data: [DONE]\n";
        let Some((endpoint, received)) = mock_server(200, body, None) else {
            eprintln!("loopback provider mock server bind is environment-blocked");
            return;
        };
        let adapter = HttpProviderAdapter::new(
            endpoint,
            Some("AGENTCODE_TEST_PROVIDER_KEY".to_string()),
            "fixture-model",
            1000,
        )
        .unwrap();
        let request = request();
        let events = adapter.stream(&request, &|| false).unwrap();
        let wire_request = received.recv_timeout(StdDuration::from_secs(1)).unwrap();
        assert!(wire_request.contains("authorization: Bearer test-key"));
        assert!(wire_request.contains("\"messages\""));
        assert!(matches!(events.last(), Some(ProviderStreamEvent::Finished)));
        assert!(
            matches!(&events[0], ProviderStreamEvent::Delta(text) if text.contains("goal=fix"))
        );
        let usage = token_usage(&events);
        assert_eq!((usage.input_tokens, usage.output_tokens), (4, 6));
        assert_eq!(usage.source, UsageSource::ProviderReported);
    }

    #[test]
    fn provider_request_formatting_and_auth_headers_are_provider_specific() {
        std::env::set_var("ANTHROPIC_API_KEY", "anthropic-key");
        std::env::set_var("GEMINI_API_KEY", "gemini-key");
        let request = request();
        let anthropic = AnthropicProviderAdapter::new("http://127.0.0.1:1/v1/messages", "claude")
            .unwrap()
            .inner;
        let anthropic_headers = anthropic.auth_headers().unwrap();
        assert!(anthropic.request_json(&request).get("messages").is_some());
        assert_eq!(
            anthropic_headers
                .get("x-api-key")
                .unwrap()
                .to_str()
                .unwrap(),
            "anthropic-key"
        );
        assert!(anthropic_headers.get("anthropic-version").is_some());

        let gemini = GeminiProviderAdapter::new(
            "http://127.0.0.1:1/v1beta/models/gemini:generateContent",
            "gemini",
        )
        .unwrap()
        .inner;
        assert!(gemini.request_json(&request).get("contents").is_some());
        assert_eq!(
            gemini
                .auth_headers()
                .unwrap()
                .get("x-goog-api-key")
                .unwrap()
                .to_str()
                .unwrap(),
            "gemini-key"
        );
    }

    #[test]
    fn provider_response_parsing_handles_provider_shapes_and_failures() {
        let openai = events_from_status_and_body(
            200,
            "{\"choices\":[{\"message\":{\"content\":\"plan=ok\"},\"finish_reason\":\"stop\"}],\"usage\":{\"prompt_tokens\":2,\"completion_tokens\":3}}",
            HttpProviderKind::OpenAiChatCompletions,
        )
        .unwrap();
        let usage = token_usage(&openai);
        assert_eq!((usage.input_tokens, usage.output_tokens), (2, 3));
        let anthropic = events_from_status_and_body(
            200,
            "{\"content\":[{\"text\":\"plan=anthropic\"}],\"usage\":{\"input_tokens\":5,\"output_tokens\":7},\"type\":\"message_stop\"}",
            HttpProviderKind::AnthropicMessages,
        )
        .unwrap();
        let usage = token_usage(&anthropic);
        assert_eq!((usage.input_tokens, usage.output_tokens), (5, 7));
        let gemini = events_from_status_and_body(
            200,
            "{\"candidates\":[{\"content\":{\"parts\":[{\"text\":\"plan=gemini\"}]},\"finishReason\":\"STOP\"}],\"usageMetadata\":{\"promptTokenCount\":11,\"candidatesTokenCount\":13}}",
            HttpProviderKind::GeminiGenerateContent,
        )
        .unwrap();
        let usage = token_usage(&gemini);
        assert_eq!((usage.input_tokens, usage.output_tokens), (11, 13));
        let ollama = events_from_status_and_body(
            200,
            "{\"message\":{\"content\":\"plan=ollama\"},\"done\":true,\"prompt_eval_count\":17,\"eval_count\":19}",
            HttpProviderKind::OllamaChat,
        )
        .unwrap();
        let usage = token_usage(&ollama);
        assert_eq!((usage.input_tokens, usage.output_tokens), (17, 19));
        assert_eq!(
            events_from_status_and_body(429, "rate limited", HttpProviderKind::OpenAiCompatible)
                .unwrap_err(),
            ProviderFailureClass::RateLimited
        );
        assert_eq!(
            events_from_status_and_body(200, "not json", HttpProviderKind::OpenAiCompatible)
                .unwrap_err(),
            ProviderFailureClass::MalformedResponse
        );
    }

    #[test]
    fn streaming_parsers_handle_multiline_sse_errors_and_ndjson() {
        let multiline = "data: {\"choices\":[{\"delta\":{\"content\":\"hello\"},\n\
data: \"finish_reason\":\"stop\"}]}\n\n\
data: [DONE]\n\n";
        let events = parse_sse_body(multiline, HttpProviderKind::OpenAiChatCompletions).unwrap();
        assert!(matches!(&events[0], ProviderStreamEvent::Delta(text) if text == "hello"));

        let provider_error =
            "data: {\"error\":{\"type\":\"rate_limit_error\",\"message\":\"try later\"}}\n\n";
        assert_eq!(
            parse_sse_body(provider_error, HttpProviderKind::OpenAiChatCompletions).unwrap_err(),
            ProviderFailureClass::RateLimited
        );

        let ndjson = "{\"message\":{\"content\":\"one\"},\"done\":false}\n\
{\"message\":{\"content\":\"two\"},\"done\":true,\"prompt_eval_count\":3,\"eval_count\":4}\n";
        let events = parse_json_lines(ndjson, HttpProviderKind::OllamaChat).unwrap();
        assert_eq!(token_usage(&events).total_tokens, 7);
        assert!(matches!(events.last(), Some(ProviderStreamEvent::Finished)));

        assert_eq!(
            parse_sse_body("data: {not-json}\n\n", HttpProviderKind::OpenAiCompatible).unwrap_err(),
            ProviderFailureClass::MalformedResponse
        );
    }

    #[test]
    fn http_provider_config_rejects_unsafe_remote_http_and_secret_headers() {
        assert_eq!(
            HttpProviderAdapter::new("http://api.example.test/v1/chat", None, "model", 1_000)
                .unwrap_err()
                .code(),
            "PROVIDER-INSECURE_REMOTE_ENDPOINT"
        );
        let mut options = HttpProviderOptions::new("https://api.example.test/v1/chat", "model");
        options
            .custom_headers
            .push(("Authorization".to_string(), "secret".to_string()));
        assert_eq!(
            HttpProviderAdapter::new_kind(options, HttpProviderKind::OpenAiCompatible)
                .unwrap_err()
                .code(),
            "PROVIDER-UNSAFE_CUSTOM_HEADER"
        );
        let mut options = HttpProviderOptions::new("https://api.example.test/v1/chat", "model");
        options
            .custom_headers
            .push(("Proxy-Authorization".to_string(), "secret".to_string()));
        assert_eq!(
            HttpProviderAdapter::new_kind(options, HttpProviderKind::OpenAiCompatible)
                .unwrap_err()
                .code(),
            "PROVIDER-UNSAFE_CUSTOM_HEADER"
        );
        let mut options = HttpProviderOptions::new("https://api.example.test/v1/chat", "model");
        options
            .custom_headers
            .push(("API-Key".to_string(), "secret".to_string()));
        assert_eq!(
            HttpProviderAdapter::new_kind(options, HttpProviderKind::OpenAiCompatible)
                .unwrap_err()
                .code(),
            "PROVIDER-UNSAFE_CUSTOM_HEADER"
        );
    }

    #[test]
    fn http_provider_adapter_reports_timeout() {
        let Some((endpoint, _received)) = mock_server(
            200,
            "{\"choices\":[{\"message\":{\"content\":\"late\"}}]}",
            Some(StdDuration::from_millis(150)),
        ) else {
            eprintln!("loopback provider mock server bind is environment-blocked");
            return;
        };
        let adapter = HttpProviderAdapter::new(endpoint, None, "fixture-model", 25).unwrap();
        assert_eq!(
            adapter.stream(&request(), &|| false).unwrap_err(),
            ProviderFailureClass::Timeout
        );
    }

    #[test]
    fn structured_agent_response_validates_and_rejects_invalid_output() {
        let raw = r#"{
            "plan": ["inspect task"],
            "assumptions": ["workspace is available"],
            "actions": ["edit file"],
            "verification_requirements": ["run tests"]
        }"#;
        let parsed = parse_structured_agent_response(raw).unwrap();
        assert_eq!(parsed.plan, vec!["inspect task"]);
        assert_eq!(parsed.actions, vec!["edit file"]);
        assert!(parse_structured_agent_response(
            r#"{"plan":["inspect"],"assumptions":[],"actions":[],"verification_requirements":[]}"#
        )
        .is_err());
    }

    #[test]
    fn provider_events_accept_legacy_fallback_without_breaking_scripted_provider() {
        let events = successful_events("plan=fix bug\naction=patch file\nverify=cargo test");
        let validated = validate_provider_events(&events, true).unwrap();
        assert_eq!(validated.format, ProviderResponseFormat::LegacyFallback);
        assert_eq!(validated.response.plan, vec!["fix bug"]);
        assert!(validate_provider_events(&events, false).is_err());
    }

    #[test]
    fn model_broker_ranks_candidates_independent_of_connections() {
        let fabric = phase4_fabric(
            Box::new(ScriptedProvider::new(vec![Ok(successful_events("free"))])),
            Box::new(ScriptedProvider::new(vec![Ok(successful_events("paid"))])),
        );
        let candidates = fabric.registry.ranked_candidates(&TaskProfile::coding(
            StableId::new("task"),
            RoutingProfile::FreeFirst,
        ));
        assert!(candidates.len() >= 2);
        assert_eq!(candidates[0].connection_id, fabric.free_connection);
        assert!(candidates[0]
            .reasons
            .iter()
            .any(|reason| reason == "paid:false"));
    }

    #[test]
    fn rate_limit_triggers_fallback_cooldown_and_persists_decision_data() {
        let mut fabric = phase4_fabric(
            Box::new(ScriptedProvider::new(vec![Err(
                ProviderFailureClass::RateLimited,
            )])),
            Box::new(ScriptedProvider::new(vec![Ok(successful_events("paid"))])),
        );
        let execution = fabric
            .registry
            .request_model(
                &TaskProfile::coding(StableId::new("task"), RoutingProfile::FreeFirst),
                "fix bug",
                128,
                &|| false,
            )
            .unwrap();
        assert_eq!(
            execution.decision.selected.as_ref().unwrap().connection_id,
            fabric.paid_connection
        );
        assert!(execution
            .decision
            .fallback_reason
            .as_ref()
            .unwrap()
            .contains("RateLimit"));
        assert!(fabric
            .registry
            .circuit_breaker(&fabric.free_connection)
            .unwrap()
            .is_open());
        assert_eq!(fabric.registry.routing_decisions().len(), 1);
        assert!(format!("{:?}", execution.decision).contains("connection-"));
        assert!(!format!("{:?}", execution.decision).contains("_KEY"));
    }

    #[test]
    fn timeout_triggers_recovery_without_manual_task_restart() {
        let mut fabric = phase4_fabric(
            Box::new(ScriptedProvider::new(vec![Err(
                ProviderFailureClass::Timeout,
            )])),
            Box::new(ScriptedProvider::new(vec![Ok(successful_events(
                "recovered",
            ))])),
        );
        let execution = fabric
            .registry
            .request_model(
                &TaskProfile::coding(StableId::new("task"), RoutingProfile::FreeFirst),
                "recover",
                128,
                &|| false,
            )
            .unwrap();
        assert!(matches!(
            &execution.events[0],
            ProviderStreamEvent::Delta(text) if text == "recovered"
        ));
        assert_eq!(fabric.registry.attempts().len(), 3);
        assert!(fabric
            .registry
            .circuit_breaker(&fabric.free_connection)
            .unwrap()
            .is_open());
    }

    #[test]
    fn successful_recovery_closes_open_circuit_and_restores_health() {
        let mut fabric = phase4_fabric(
            Box::new(ScriptedProvider::new(vec![Ok(successful_events("free"))])),
            Box::new(ScriptedProvider::new(vec![Ok(successful_events("paid"))])),
        );
        let breaker = fabric
            .registry
            .circuit_breakers
            .get_mut(&fabric.free_connection)
            .unwrap();
        breaker.transient_failures = 2;
        breaker.cooldown_until_ms = None;
        let execution = fabric
            .registry
            .request_model(
                &TaskProfile::coding(StableId::new("task"), RoutingProfile::FreeFirst),
                "recover",
                128,
                &|| false,
            )
            .unwrap();
        assert_eq!(
            execution.decision.selected.as_ref().unwrap().connection_id,
            fabric.free_connection
        );
        let breaker = fabric
            .registry
            .circuit_breaker(&fabric.free_connection)
            .unwrap();
        assert_eq!(breaker.transient_failures, 0);
        assert!(!breaker.is_open());
    }

    #[test]
    fn estimated_usage_and_unknown_remote_cost_are_recorded_honestly() {
        let mut registry = ProviderRegistry::new();
        let (_, _, _) = register_route(
            &mut registry,
            "unknown-price",
            Box::new(ScriptedProvider::new(vec![Ok(vec![
                ProviderStreamEvent::Delta("response without usage".to_string()),
                ProviderStreamEvent::EstimatedUsage {
                    input_tokens: 5,
                    output_tokens: 6,
                },
                ProviderStreamEvent::Finished,
            ])])),
            "unknown-price-family",
            "unknown-price-model",
            80,
            80,
            true,
            false,
        );
        for identity in registry.model_identities.values_mut() {
            identity.pricing_unit = PricingUnit::Unknown;
            identity.input_cost_micros = 0;
            identity.output_cost_micros = 0;
        }
        let execution = registry
            .request_model(
                &TaskProfile::coding(StableId::new("task"), RoutingProfile::PaidAllowed),
                "usage",
                128,
                &|| false,
            )
            .unwrap();
        assert_eq!(execution.decision.usage_source, UsageSource::Estimated);
        assert_eq!(execution.decision.total_tokens, 11);
        assert!(!execution.decision.cost_known);
    }

    #[test]
    fn paid_policy_blocks_and_paid_fallback_requires_explicit_allowance() {
        let mut free_unavailable = phase4_fabric(
            Box::new(ScriptedProvider::new(vec![Err(
                ProviderFailureClass::ServerError,
            )])),
            Box::new(ScriptedProvider::new(vec![Ok(successful_events("paid"))])),
        );
        free_unavailable
            .registry
            .connections
            .get_mut(&free_unavailable.local_connection)
            .unwrap()
            .enabled = false;
        let blocked = free_unavailable
            .registry
            .request_model(
                &TaskProfile::coding(StableId::new("task"), RoutingProfile::FreeOnly),
                "free only",
                128,
                &|| false,
            )
            .unwrap_err();
        assert_eq!(blocked, ProviderFailureClass::ServerError);
        let paid_rejected = free_unavailable
            .registry
            .routing_decisions()
            .last()
            .unwrap()
            .rejected
            .iter()
            .any(|reason| reason.contains("paid_disallowed"));
        assert!(paid_rejected);

        let mut paid_allowed = phase4_fabric(
            Box::new(ScriptedProvider::new(vec![Err(
                ProviderFailureClass::ServerError,
            )])),
            Box::new(ScriptedProvider::new(vec![Ok(successful_events("paid"))])),
        );
        paid_allowed
            .registry
            .connections
            .get_mut(&paid_allowed.local_connection)
            .unwrap()
            .enabled = false;
        let execution = paid_allowed
            .registry
            .request_model(
                &TaskProfile::coding(StableId::new("task"), RoutingProfile::FreeFirst),
                "paid fallback",
                128,
                &|| false,
            )
            .unwrap();
        assert_eq!(
            execution.decision.selected.as_ref().unwrap().connection_id,
            paid_allowed.paid_connection
        );
    }

    #[test]
    fn local_ollama_route_is_selected_for_offline_profile() {
        let mut fabric = phase4_fabric(
            Box::new(ScriptedProvider::new(vec![Ok(successful_events("free"))])),
            Box::new(ScriptedProvider::new(vec![Ok(successful_events("paid"))])),
        );
        let mut profile = TaskProfile::coding(StableId::new("task"), RoutingProfile::Offline);
        profile.privacy = PrivacyClass::LocalOnly;
        let execution = fabric
            .registry
            .request_model(&profile, "local", 128, &|| false)
            .unwrap();
        assert_eq!(
            execution.decision.selected.as_ref().unwrap().connection_id,
            fabric.local_connection
        );
    }

    #[test]
    fn provider_failure_categories_normalize_to_phase4_contract() {
        assert_eq!(
            normalize_failure(ProviderFailureClass::RateLimited),
            NormalizedProviderFailure::RateLimit
        );
        assert_eq!(
            normalize_failure(ProviderFailureClass::Auth),
            NormalizedProviderFailure::AuthFailed
        );
        assert_eq!(
            normalize_failure(ProviderFailureClass::ContextOverflow),
            NormalizedProviderFailure::ContextLimit
        );
        assert_eq!(
            normalize_failure(ProviderFailureClass::MalformedResponse),
            NormalizedProviderFailure::BadResponse
        );
    }

    #[test]
    fn diversity_gate_can_select_different_model_families_by_role() {
        let mut registry = ProviderRegistry::new();
        let (_, impl_identity, _) = register_route(
            &mut registry,
            "impl-remote",
            Box::new(ScriptedProvider::new(vec![Ok(successful_events("impl"))])),
            "implementation-family-a",
            "impl-model",
            95,
            40,
            false,
            false,
        );
        let (_, verify_identity, _) = register_route(
            &mut registry,
            "verify-remote",
            Box::new(ScriptedProvider::new(vec![Ok(successful_events("verify"))])),
            "verification-family-b",
            "verify-model",
            50,
            95,
            false,
            false,
        );
        let implementation = registry.ranked_candidates(&TaskProfile::coding(
            StableId::new("task"),
            RoutingProfile::QualityFirst,
        ));
        let mut verification =
            TaskProfile::coding(StableId::new("task"), RoutingProfile::QualityFirst);
        verification.role = "verification".to_string();
        let verification = registry.ranked_candidates(&verification);
        assert_eq!(implementation[0].model_identity_id, impl_identity);
        assert_eq!(verification[0].model_identity_id, verify_identity);
    }

    #[test]
    fn endpoint_policy_blocks_metadata_and_requires_private_lan_http_opt_in() {
        assert!(
            HttpProviderAdapter::new("http://127.0.0.1:11434/api/chat", None, "m", 100).is_ok()
        );
        assert_eq!(
            HttpProviderAdapter::new("http://169.254.169.254/latest", None, "m", 100)
                .unwrap_err()
                .code(),
            "PROVIDER-METADATA_ENDPOINT_DENIED"
        );
        assert_eq!(
            HttpProviderAdapter::new("http://192.168.1.5:8080/v1/chat", None, "m", 100)
                .unwrap_err()
                .code(),
            "PROVIDER-INSECURE_REMOTE_ENDPOINT"
        );
        let mut options = HttpProviderOptions::new("http://192.168.1.5:8080/v1/chat", "m");
        options.allow_plain_http_remote = true;
        assert!(HttpProviderAdapter::new_kind(options, HttpProviderKind::OpenAiCompatible).is_ok());
        let mut options = HttpProviderOptions::new("http://203.0.113.10:8080/v1/chat", "m");
        options.allow_plain_http_remote = true;
        assert_eq!(
            HttpProviderAdapter::new_kind(options, HttpProviderKind::OpenAiCompatible)
                .unwrap_err()
                .code(),
            "PROVIDER-INSECURE_REMOTE_ENDPOINT"
        );
    }

    #[test]
    fn retry_after_and_backoff_are_bounded_without_sleeping() {
        assert_eq!(parse_retry_after_seconds("2"), Some(2));
        assert_eq!(parse_retry_after_seconds("999"), Some(30));
        assert_eq!(
            events_from_status_and_body_with_retry_after(
                429,
                "rate limited",
                HttpProviderKind::OpenAiCompatible,
                Some(2),
            )
            .unwrap_err(),
            ProviderFailureClass::RateLimitedAfter(2)
        );
        assert_eq!(
            retry_delay_millis(ProviderFailureClass::RateLimitedAfter(99), 0),
            30_000
        );
        assert!(retry_delay_millis(ProviderFailureClass::Timeout, 7) <= 5_000);
    }

    #[test]
    fn provider_transient_histories_are_bounded() {
        let mut registry = ProviderRegistry::new();
        let _ = register_route(
            &mut registry,
            "bounded",
            Box::new(ScriptedProvider::new(vec![Ok(successful_events("ok"))])),
            "family",
            "model",
            80,
            80,
            false,
            false,
        );
        let request = registry
            .normalize_request("hello", vec![ProviderCapability::Chat], 64)
            .unwrap();
        for _ in 0..(MAX_PROVIDER_HISTORY + 10) {
            let _ = registry.start_attempt(&request).unwrap();
        }
        assert_eq!(registry.attempts().len(), MAX_PROVIDER_HISTORY);
    }

    #[test]
    fn http_provider_adapter_resolves_secret_reference_for_auth() {
        let tmp = std::env::temp_dir().join(format!("ac-secret-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();
        let secret_path = tmp.join("test-secret");
        std::fs::write(&secret_path, b"sk-live-secret-value").unwrap();
        std::env::set_var("AGENTCODE_SECRET_DIR", tmp.as_os_str());

        let adapter = HttpProviderAdapter::new(
            "http://127.0.0.1:1/v1/chat",
            Some("secret:test-secret".to_string()),
            "fixture-model",
            1000,
        )
        .unwrap();
        let headers = adapter.auth_headers().unwrap();
        let auth = headers.get(AUTHORIZATION).unwrap().to_str().unwrap();
        assert_eq!(auth, "Bearer sk-live-secret-value");

        // Also prove the full stream() path sends the correct header
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let endpoint = format!("http://{}/v1/chat", listener.local_addr().unwrap());
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut buf = [0_u8; 4096];
            let n = stream.read(&mut buf).unwrap_or(0);
            let request = String::from_utf8_lossy(&buf[..n]).to_string();
            tx.send(request).unwrap();
            let body = "{\"choices\":[{\"message\":{\"content\":\"ok\"},\"finish_reason\":\"stop\"}],\"usage\":{\"prompt_tokens\":1,\"completion_tokens\":1}}";
            let response = format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}", body.len(), body);
            let _ = stream.write_all(response.as_bytes());
        });

        let adapter = HttpProviderAdapter::new(
            endpoint,
            Some("secret:test-secret".to_string()),
            "fixture-model",
            5000,
        )
        .unwrap();
        let request = request();
        let events = adapter.stream(&request, &|| false).unwrap();
        let wire = rx.recv_timeout(StdDuration::from_secs(2)).unwrap();
        assert!(
            wire.to_ascii_lowercase()
                .contains("authorization: bearer sk-live-secret-value"),
            "authenticated request must carry the resolved secret"
        );
        assert!(matches!(events.last(), Some(ProviderStreamEvent::Finished)));

        std::env::remove_var("AGENTCODE_SECRET_DIR");
        let _ = std::fs::remove_dir_all(&tmp);
    }
}
