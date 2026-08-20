use std::collections::{BTreeMap, VecDeque};

use ac_common::{AcError, AcResult, StableId, TimestampMillis};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ProviderCapability {
    Chat,
    ToolCalls,
    Streaming,
    LocalModel,
    Vision,
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
    Finished,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProviderFailureClass {
    RateLimited,
    Timeout,
    ServerError,
    StreamAborted,
    MalformedResponse,
    Auth,
    Permission,
    ContextOverflow,
    UnsupportedCapability,
    Cancelled,
}

pub trait ProviderAdapter {
    fn stream(
        &self,
        request: &NormalizedInferenceRequest,
        cancel: &dyn Fn() -> bool,
    ) -> Result<Vec<ProviderStreamEvent>, ProviderFailureClass>;
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
    attempts: Vec<RouteAttempt>,
}

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
                }
                Err(failure) => {
                    self.finish_failed(&attempt_id, failure)
                        .map_err(|_| ProviderFailureClass::MalformedResponse)?;
                    if !is_retryable(failure) {
                        return Err(failure);
                    }
                    last_failure = Some(failure);
                }
            }
        }
        Err(last_failure.unwrap_or(ProviderFailureClass::ServerError))
    }

    pub fn attempts(&self) -> &[RouteAttempt] {
        &self.attempts
    }
}

fn is_retryable(failure: ProviderFailureClass) -> bool {
    matches!(
        failure,
        ProviderFailureClass::RateLimited
            | ProviderFailureClass::Timeout
            | ProviderFailureClass::ServerError
            | ProviderFailureClass::StreamAborted
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

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
}
