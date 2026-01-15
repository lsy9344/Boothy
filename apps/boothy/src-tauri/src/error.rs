use crate::camera::ipc_models::{IpcError, IpcErrorCode};
use serde::{Deserialize, Serialize};
use std::fmt;

/// Error severity levels for diagnostic purposes
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ErrorSeverity {
    /// Non-critical warning that doesn't block functionality
    Warning,
    /// Error that blocks current operation but allows recovery
    Error,
    /// Critical error requiring immediate attention
    Critical,
}

/// Structured error format: code + message + context
/// Supports dual messaging: customer-safe summary + admin diagnostics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BoothyError {
    /// Machine-readable error code (e.g., "CAMERA_DISCONNECT", "IPC_TIMEOUT")
    pub code: String,

    /// Customer-safe message: actionable, non-technical
    pub message: String,

    /// Admin diagnostic message: technical details for troubleshooting
    pub diagnostic: Option<String>,

    /// Additional context (correlationId, file paths, timestamps, etc.)
    pub context: serde_json::Value,

    /// Error severity for prioritization
    pub severity: ErrorSeverity,
}

impl BoothyError {
    /// Create a new error with customer-safe message only
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            diagnostic: None,
            context: serde_json::json!({}),
            severity: ErrorSeverity::Error,
        }
    }

    /// Create a new error with both customer and admin messages
    pub fn with_diagnostic(
        code: impl Into<String>,
        message: impl Into<String>,
        diagnostic: impl Into<String>,
    ) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            diagnostic: Some(diagnostic.into()),
            context: serde_json::json!({}),
            severity: ErrorSeverity::Error,
        }
    }

    /// Add context information (correlation IDs, file paths, etc.)
    pub fn with_context(mut self, key: &str, value: serde_json::Value) -> Self {
        if let Some(obj) = self.context.as_object_mut() {
            obj.insert(key.to_string(), value);
        }
        self
    }

    /// Set error severity
    pub fn with_severity(mut self, severity: ErrorSeverity) -> Self {
        self.severity = severity;
        self
    }

    /// Get message appropriate for current mode (customer vs admin)
    pub fn message_for_mode(&self, is_admin: bool) -> &str {
        if is_admin {
            self.diagnostic.as_deref().unwrap_or(&self.message)
        } else {
            &self.message
        }
    }
}

impl fmt::Display for BoothyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}] {}", self.code, self.message)
    }
}

impl std::error::Error for BoothyError {}

/// Camera-specific error codes and constructors
pub mod camera {
    use super::*;

    pub const CAMERA_DISCONNECT: &str = "CAMERA_DISCONNECT";
    pub const CAMERA_INIT_FAILED: &str = "CAMERA_INIT_FAILED";
    pub const CAPTURE_FAILED: &str = "CAPTURE_FAILED";
    pub const TRANSFER_FAILED: &str = "TRANSFER_FAILED";
    pub const CAMERA_BUSY: &str = "CAMERA_BUSY";
    pub const CAMERA_NOT_FOUND: &str = "CAMERA_NOT_FOUND";

    pub fn disconnect(diagnostic: impl Into<String>) -> BoothyError {
        BoothyError::with_diagnostic(
            CAMERA_DISCONNECT,
            "Camera disconnected. Please check the camera connection and try again.",
            diagnostic,
        )
        .with_severity(ErrorSeverity::Error)
    }

    pub fn capture_failed(diagnostic: impl Into<String>) -> BoothyError {
        BoothyError::with_diagnostic(
            CAPTURE_FAILED,
            "Failed to capture photo. Please try again.",
            diagnostic,
        )
        .with_severity(ErrorSeverity::Error)
    }

    pub fn transfer_failed(diagnostic: impl Into<String>) -> BoothyError {
        BoothyError::with_diagnostic(
            TRANSFER_FAILED,
            "Failed to transfer photo from camera. Please try again.",
            diagnostic,
        )
        .with_severity(ErrorSeverity::Error)
    }

    pub fn not_found() -> BoothyError {
        BoothyError::new(
            CAMERA_NOT_FOUND,
            "No camera detected. Please connect a camera and restart Boothy.",
        )
        .with_severity(ErrorSeverity::Error)
    }
}

/// IPC-specific error codes and constructors
pub mod ipc {
    use super::*;

    pub const IPC_TIMEOUT: &str = "IPC_TIMEOUT";
    pub const IPC_DISCONNECT: &str = "IPC_DISCONNECT";
    pub const IPC_INVALID_RESPONSE: &str = "IPC_INVALID_RESPONSE";
    pub const SIDECAR_CRASH: &str = "SIDECAR_CRASH";
    pub const SIDECAR_START_FAILED: &str = "SIDECAR_START_FAILED";

    pub fn timeout(operation: &str) -> BoothyError {
        BoothyError::with_diagnostic(
            IPC_TIMEOUT,
            "Camera service is not responding. Please restart Boothy.",
            format!("IPC timeout during operation: {}", operation),
        )
        .with_severity(ErrorSeverity::Error)
    }

    pub fn disconnect() -> BoothyError {
        BoothyError::with_diagnostic(
            IPC_DISCONNECT,
            "Camera service disconnected. Please restart Boothy.",
            "IPC connection to sidecar lost",
        )
        .with_severity(ErrorSeverity::Critical)
    }

    pub fn sidecar_crash(diagnostic: impl Into<String>) -> BoothyError {
        BoothyError::with_diagnostic(
            SIDECAR_CRASH,
            "Camera service stopped unexpectedly. Please restart Boothy.",
            diagnostic,
        )
        .with_severity(ErrorSeverity::Critical)
    }
}

/// Import-specific error codes and constructors
pub mod import {
    use super::*;

    pub const IMPORT_FAILED: &str = "IMPORT_FAILED";
    pub const FILE_NOT_STABLE: &str = "FILE_NOT_STABLE";
    pub const FILE_CORRUPTED: &str = "FILE_CORRUPTED";
    pub const UNSUPPORTED_FORMAT: &str = "UNSUPPORTED_FORMAT";

    pub fn failed(file_path: &str, diagnostic: impl Into<String>) -> BoothyError {
        BoothyError::with_diagnostic(
            IMPORT_FAILED,
            "Failed to import photo. The file may be corrupted.",
            diagnostic,
        )
        .with_context("filePath", serde_json::json!(file_path))
        .with_severity(ErrorSeverity::Error)
    }

    pub fn unsupported_format(file_path: &str) -> BoothyError {
        BoothyError::new(
            UNSUPPORTED_FORMAT,
            "Unsupported file format. Only RAW and JPEG files are supported.",
        )
        .with_context("filePath", serde_json::json!(file_path))
        .with_severity(ErrorSeverity::Warning)
    }
}

/// Export-specific error codes and constructors
pub mod export {
    use super::*;

    pub const EXPORT_FAILED: &str = "EXPORT_FAILED";
    pub const DISK_FULL: &str = "DISK_FULL";
    pub const PERMISSION_DENIED: &str = "PERMISSION_DENIED";

    pub fn failed(destination: &str, diagnostic: impl Into<String>) -> BoothyError {
        BoothyError::with_diagnostic(
            EXPORT_FAILED,
            "Failed to export image. Please try again or choose a different location.",
            diagnostic,
        )
        .with_context("destination", serde_json::json!(destination))
        .with_severity(ErrorSeverity::Error)
    }

    pub fn disk_full(destination: &str) -> BoothyError {
        BoothyError::new(
            DISK_FULL,
            "Not enough disk space. Please free up space and try again.",
        )
        .with_context("destination", serde_json::json!(destination))
        .with_severity(ErrorSeverity::Error)
    }
}

/// Preset-specific error codes and constructors
pub mod preset {
    use super::*;

    pub const PRESET_NOT_FOUND: &str = "PRESET_NOT_FOUND";
    pub const PRESET_APPLY_FAILED: &str = "PRESET_APPLY_FAILED";
    pub const PRESET_LOAD_FAILED: &str = "PRESET_LOAD_FAILED";

    pub fn not_found(preset_id: &str) -> BoothyError {
        BoothyError::new(
            PRESET_NOT_FOUND,
            "Selected preset not found. Please choose another preset.",
        )
        .with_context("presetId", serde_json::json!(preset_id))
        .with_severity(ErrorSeverity::Error)
    }

    pub fn apply_failed(preset_id: &str, diagnostic: impl Into<String>) -> BoothyError {
        BoothyError::with_diagnostic(
            PRESET_APPLY_FAILED,
            "Failed to apply preset. Please try again.",
            diagnostic,
        )
        .with_context("presetId", serde_json::json!(preset_id))
        .with_severity(ErrorSeverity::Error)
    }
}

/// Session-specific error codes and constructors
pub mod session {
    use super::*;

    pub const SESSION_CREATE_FAILED: &str = "SESSION_CREATE_FAILED";
    pub const SESSION_NOT_FOUND: &str = "SESSION_NOT_FOUND";
    pub const SESSION_LOAD_FAILED: &str = "SESSION_LOAD_FAILED";

    pub fn create_failed(session_name: &str, diagnostic: impl Into<String>) -> BoothyError {
        BoothyError::with_diagnostic(
            SESSION_CREATE_FAILED,
            "Failed to create session. Please try a different name.",
            diagnostic,
        )
        .with_context("sessionName", serde_json::json!(session_name))
        .with_severity(ErrorSeverity::Error)
    }

    pub fn not_found(session_name: &str) -> BoothyError {
        BoothyError::new(SESSION_NOT_FOUND, "Session not found.")
            .with_context("sessionName", serde_json::json!(session_name))
            .with_severity(ErrorSeverity::Error)
    }
}

/// Conversion from IpcError to BoothyError
impl From<IpcError> for BoothyError {
    fn from(ipc_err: IpcError) -> Self {
        let code = format!("{:?}", ipc_err.code).to_uppercase();
        let diagnostic = ipc_err.diagnostic_message();
        let customer_message = ipc_err.customer_safe_message();

        let mut error = BoothyError::with_diagnostic(code, customer_message, diagnostic);

        // Add context from IPC error
        if let Some(context_map) = ipc_err.context {
            for (key, value) in context_map {
                error = error.with_context(&key, serde_json::json!(value));
            }
        }

        // Map IPC error codes to appropriate severity
        error.severity = match ipc_err.code {
            IpcErrorCode::VersionMismatch
            | IpcErrorCode::Disconnect
            | IpcErrorCode::CameraNotConnected => ErrorSeverity::Critical,
            IpcErrorCode::CaptureFailed | IpcErrorCode::FileTransferFailed => ErrorSeverity::Error,
            _ => ErrorSeverity::Error,
        };

        error
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_customer_safe_message() {
        let error = camera::disconnect("EDSDK error code: 0x8001");
        assert_eq!(
            error.message_for_mode(false),
            "Camera disconnected. Please check the camera connection and try again."
        );
    }

    #[test]
    fn test_admin_diagnostic_message() {
        let error = camera::disconnect("EDSDK error code: 0x8001");
        assert_eq!(error.message_for_mode(true), "EDSDK error code: 0x8001");
    }

    #[test]
    fn test_error_with_context() {
        let error = import::failed("test.cr2", "Raw file header invalid")
            .with_context("correlationId", serde_json::json!("abc-123"));

        assert_eq!(error.context["filePath"], "test.cr2");
        assert_eq!(error.context["correlationId"], "abc-123");
    }
}
