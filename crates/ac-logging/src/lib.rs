use ac_common::TimestampMillis;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Severity {
    Debug,
    Info,
    Warn,
    Error,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LogRecord {
    pub timestamp: TimestampMillis,
    pub component: String,
    pub severity: Severity,
    pub message: String,
    pub correlation_id: Option<String>,
    pub error_code: Option<String>,
}

#[derive(Clone, Debug, Default)]
pub struct Redactor {
    exact_values: Vec<String>,
}

impl Redactor {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_secret(mut self, value: impl Into<String>) -> Self {
        let value = value.into();
        if !value.is_empty() {
            self.exact_values.push(value);
        }
        self
    }

    pub fn redact(&self, input: &str) -> String {
        let mut output = input.to_owned();
        for secret in &self.exact_values {
            output = output.replace(secret, "[REDACTED]");
        }
        output
    }
}

pub struct StructuredLogger {
    component: String,
    redactor: Redactor,
}

impl StructuredLogger {
    pub fn new(component: impl Into<String>, redactor: Redactor) -> Self {
        Self {
            component: component.into(),
            redactor,
        }
    }

    pub fn record(&self, severity: Severity, message: impl AsRef<str>) -> LogRecord {
        LogRecord {
            timestamp: TimestampMillis::now(),
            component: self.component.clone(),
            severity,
            message: self.redactor.redact(message.as_ref()),
            correlation_id: None,
            error_code: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redacts_registered_secret_values() {
        let logger = StructuredLogger::new("kernel", Redactor::new().with_secret("secret-token"));
        let record = logger.record(Severity::Info, "using secret-token");
        assert_eq!(record.message, "using [REDACTED]");
    }
}
