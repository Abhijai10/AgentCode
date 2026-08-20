use std::collections::BTreeMap;

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

    pub fn attempts(&self) -> &[RouteAttempt] {
        &self.attempts
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
