use std::fmt;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

static NEXT_ID: AtomicU64 = AtomicU64::new(1);

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct StableId(String);

impl StableId {
    pub fn new(prefix: &str) -> Self {
        let sequence = NEXT_ID.fetch_add(1, Ordering::Relaxed);
        Self(format!("{}-{:016x}", prefix, sequence))
    }

    pub fn from_existing(value: impl Into<String>) -> Result<Self, AcError> {
        let value = value.into();
        if value.trim().is_empty() {
            return Err(AcError::validation(
                "COMMON-EMPTY_ID",
                "stable id cannot be empty",
            ));
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for StableId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Retryability {
    Retryable,
    NotRetryable,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ErrorKind {
    Validation,
    Conflict,
    PolicyDenied,
    Unavailable,
    Internal,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AcError {
    code: &'static str,
    message: String,
    kind: ErrorKind,
    retryability: Retryability,
}

impl AcError {
    pub fn new(
        code: &'static str,
        message: impl Into<String>,
        kind: ErrorKind,
        retryability: Retryability,
    ) -> Self {
        Self {
            code,
            message: message.into(),
            kind,
            retryability,
        }
    }

    pub fn validation(code: &'static str, message: impl Into<String>) -> Self {
        Self::new(
            code,
            message,
            ErrorKind::Validation,
            Retryability::NotRetryable,
        )
    }

    pub fn conflict(code: &'static str, message: impl Into<String>) -> Self {
        Self::new(
            code,
            message,
            ErrorKind::Conflict,
            Retryability::NotRetryable,
        )
    }

    pub fn policy_denied(code: &'static str, message: impl Into<String>) -> Self {
        Self::new(
            code,
            message,
            ErrorKind::PolicyDenied,
            Retryability::NotRetryable,
        )
    }

    pub fn code(&self) -> &'static str {
        self.code
    }

    pub fn kind(&self) -> ErrorKind {
        self.kind
    }

    pub fn retryability(&self) -> Retryability {
        self.retryability
    }
}

impl fmt::Display for AcError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for AcError {}

pub type AcResult<T> = Result<T, AcError>;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TimestampMillis(u128);

impl TimestampMillis {
    pub fn now() -> Self {
        let millis = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis();
        Self(millis)
    }

    pub fn from_millis(value: u128) -> Self {
        Self(value)
    }

    pub fn as_millis(self) -> u128 {
        self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stable_ids_are_prefixed_and_unique() {
        let first = StableId::new("test");
        let second = StableId::new("test");
        assert!(first.as_str().starts_with("test-"));
        assert_ne!(first, second);
    }

    #[test]
    fn empty_existing_id_is_rejected() {
        let err = StableId::from_existing(" ").unwrap_err();
        assert_eq!(err.code(), "COMMON-EMPTY_ID");
    }
}
