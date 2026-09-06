use std::collections::BTreeMap;
use std::env;

use ac_common::{AcError, AcResult};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SecretRef {
    name: String,
}

impl SecretRef {
    pub fn new(name: impl Into<String>) -> AcResult<Self> {
        let name = name.into();
        if name.trim().is_empty() {
            return Err(AcError::validation(
                "CONFIG-EMPTY_SECRET_REF",
                "secret reference cannot be empty",
            ));
        }
        Ok(Self { name })
    }

    pub fn name(&self) -> &str {
        &self.name
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ConfigValue {
    Plain(String),
    EnvRef(String),
    SecretRef(SecretRef),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Settings {
    values: BTreeMap<String, ConfigValue>,
}

impl Settings {
    pub fn get_plain(&self, key: &str) -> Option<&str> {
        match self.values.get(key) {
            Some(ConfigValue::Plain(value)) => Some(value),
            _ => None,
        }
    }

    pub fn get_secret_ref(&self, key: &str) -> Option<&SecretRef> {
        match self.values.get(key) {
            Some(ConfigValue::SecretRef(value)) => Some(value),
            _ => None,
        }
    }
}

#[derive(Default)]
pub struct SettingsBuilder {
    defaults: BTreeMap<String, ConfigValue>,
    user: BTreeMap<String, ConfigValue>,
    project: BTreeMap<String, ConfigValue>,
    env_refs: BTreeMap<String, ConfigValue>,
    test_overrides: BTreeMap<String, ConfigValue>,
    runtime_flags: BTreeMap<String, ConfigValue>,
}

impl SettingsBuilder {
    pub fn new() -> Self {
        <Self as Default>::default()
    }

    pub fn default(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.defaults
            .insert(key.into(), ConfigValue::Plain(value.into()));
        self
    }

    pub fn user(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.user
            .insert(key.into(), ConfigValue::Plain(value.into()));
        self
    }

    pub fn project(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.project
            .insert(key.into(), ConfigValue::Plain(value.into()));
        self
    }

    pub fn env_ref(mut self, key: impl Into<String>, env_key: impl Into<String>) -> Self {
        self.env_refs
            .insert(key.into(), ConfigValue::EnvRef(env_key.into()));
        self
    }

    pub fn test_override(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.test_overrides
            .insert(key.into(), ConfigValue::Plain(value.into()));
        self
    }

    pub fn runtime_flag(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.runtime_flags
            .insert(key.into(), ConfigValue::Plain(value.into()));
        self
    }

    pub fn secret_ref(mut self, key: impl Into<String>, name: impl Into<String>) -> AcResult<Self> {
        self.project
            .insert(key.into(), ConfigValue::SecretRef(SecretRef::new(name)?));
        Ok(self)
    }

    pub fn build(self) -> AcResult<Settings> {
        let mut values = BTreeMap::new();
        merge(&mut values, self.defaults)?;
        merge(&mut values, self.user)?;
        merge(&mut values, self.project)?;
        merge(&mut values, resolve_env_refs(self.env_refs)?)?;
        merge(&mut values, self.test_overrides)?;
        merge(&mut values, self.runtime_flags)?;
        Ok(Settings { values })
    }
}

fn merge(
    target: &mut BTreeMap<String, ConfigValue>,
    source: BTreeMap<String, ConfigValue>,
) -> AcResult<()> {
    for (key, value) in source {
        if key.trim().is_empty() {
            return Err(AcError::validation(
                "CONFIG-EMPTY_KEY",
                "configuration key cannot be empty",
            ));
        }
        target.insert(key, value);
    }
    Ok(())
}

fn resolve_env_refs(
    source: BTreeMap<String, ConfigValue>,
) -> AcResult<BTreeMap<String, ConfigValue>> {
    let mut resolved = BTreeMap::new();
    for (key, value) in source {
        match value {
            ConfigValue::EnvRef(env_key) => {
                let env_value = env::var(&env_key).map_err(|_| {
                    AcError::validation(
                        "CONFIG-MISSING_ENV_REF",
                        format!("environment reference {env_key} is not set"),
                    )
                })?;
                resolved.insert(key, ConfigValue::Plain(env_value));
            }
            other => {
                resolved.insert(key, other);
            }
        }
    }
    Ok(resolved)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn precedence_highest_layer_wins() {
        let settings = SettingsBuilder::new()
            .default("mode", "default")
            .user("mode", "user")
            .project("mode", "project")
            .test_override("mode", "test")
            .runtime_flag("mode", "runtime")
            .build()
            .unwrap();
        assert_eq!(settings.get_plain("mode"), Some("runtime"));
    }

    #[test]
    fn secrets_are_references_not_plaintext() {
        let settings = SettingsBuilder::new()
            .secret_ref("provider_key", "openai/default")
            .unwrap()
            .build()
            .unwrap();
        assert_eq!(
            settings.get_secret_ref("provider_key").unwrap().name(),
            "openai/default"
        );
        assert_eq!(settings.get_plain("provider_key"), None);
    }
}
