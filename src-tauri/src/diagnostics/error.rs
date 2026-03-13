use std::{error::Error, fmt, io};

use serde::Serialize;

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum OperationalLogSeverity {
    Error,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum OperationalLogSurface {
    Silent,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct OperationalLogError {
    pub code: String,
    pub message: String,
    pub severity: OperationalLogSeverity,
    pub retryable: bool,
    pub surface: OperationalLogSurface,
}

impl OperationalLogError {
    pub fn initialization(message: impl Into<String>) -> Self {
        Self::new("diagnostics.initializationFailed", message)
    }

    pub fn invalid_payload(message: impl Into<String>) -> Self {
        Self::new("diagnostics.invalidPayload", message)
    }

    pub fn session_manifest_not_found(message: impl Into<String>) -> Self {
        Self::new("session.manifestNotFound", message)
    }

    pub fn session_manifest_session_mismatch(message: impl Into<String>) -> Self {
        Self::new("session.manifestSessionMismatch", message)
    }

    pub fn migration_invalid_state(message: impl Into<String>) -> Self {
        Self::new("diagnostics.migrationInvalidState", message)
    }

    pub fn storage_failure(message: impl Into<String>) -> Self {
        Self::new("diagnostics.storageFailure", message)
    }

    fn new(code: &str, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            severity: OperationalLogSeverity::Error,
            retryable: false,
            surface: OperationalLogSurface::Silent,
        }
    }
}

impl fmt::Display for OperationalLogError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}: {}", self.code, self.message)
    }
}

impl Error for OperationalLogError {}

impl From<io::Error> for OperationalLogError {
    fn from(error: io::Error) -> Self {
        Self::storage_failure(error.to_string())
    }
}

impl From<rusqlite::Error> for OperationalLogError {
    fn from(error: rusqlite::Error) -> Self {
        Self::storage_failure(error.to_string())
    }
}

impl From<serde_json::Error> for OperationalLogError {
    fn from(error: serde_json::Error) -> Self {
        Self::invalid_payload(error.to_string())
    }
}
