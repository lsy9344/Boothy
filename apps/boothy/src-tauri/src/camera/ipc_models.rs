use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// IPC Protocol Version - Must match between Boothy and Camera Sidecar
pub const IPC_PROTOCOL_VERSION: &str = "1.0.0";

/// IPC Message Type
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum IpcMessageType {
    Request,
    Response,
    Event,
    Error,
}

/// Base IPC Message Envelope
/// All IPC messages follow this JSON-RPC-style structure
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IpcMessage {
    /// Protocol version for compatibility checking
    pub protocol_version: String,

    /// Message type
    pub message_type: IpcMessageType,

    /// Request ID for request/response correlation (optional for events)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,

    /// Correlation ID for end-to-end tracing (capture → transfer → ingest)
    pub correlation_id: String,

    /// ISO 8601 timestamp
    pub timestamp: DateTime<Utc>,

    /// Method name (e.g., "event.camera.photoTransferred", "camera.setSessionDestination")
    pub method: String,

    /// Payload specific to the method
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payload: Option<serde_json::Value>,

    /// Error details (only present if message_type is Error)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<IpcError>,
}

impl IpcMessage {
    /// Create a new event message
    pub fn new_event(method: String, correlation_id: String, payload: serde_json::Value) -> Self {
        Self {
            protocol_version: IPC_PROTOCOL_VERSION.to_string(),
            message_type: IpcMessageType::Event,
            request_id: None,
            correlation_id,
            timestamp: Utc::now(),
            method,
            payload: Some(payload),
            error: None,
        }
    }

    /// Create a new request message
    pub fn new_request(
        method: String,
        correlation_id: String,
        request_id: String,
        payload: serde_json::Value,
    ) -> Self {
        Self {
            protocol_version: IPC_PROTOCOL_VERSION.to_string(),
            message_type: IpcMessageType::Request,
            request_id: Some(request_id),
            correlation_id,
            timestamp: Utc::now(),
            method,
            payload: Some(payload),
            error: None,
        }
    }

    /// Create a new response message
    pub fn new_response(
        method: String,
        correlation_id: String,
        request_id: String,
        payload: serde_json::Value,
    ) -> Self {
        Self {
            protocol_version: IPC_PROTOCOL_VERSION.to_string(),
            message_type: IpcMessageType::Response,
            request_id: Some(request_id),
            correlation_id,
            timestamp: Utc::now(),
            method,
            payload: Some(payload),
            error: None,
        }
    }

    /// Create a new error message
    pub fn new_error(
        method: String,
        correlation_id: String,
        request_id: Option<String>,
        error: IpcError,
    ) -> Self {
        Self {
            protocol_version: IPC_PROTOCOL_VERSION.to_string(),
            message_type: IpcMessageType::Error,
            request_id,
            correlation_id,
            timestamp: Utc::now(),
            method,
            payload: None,
            error: Some(error),
        }
    }

    /// Validate protocol version compatibility
    pub fn validate_version(&self) -> Result<(), IpcError> {
        if self.protocol_version != IPC_PROTOCOL_VERSION {
            return Err(IpcError {
                code: IpcErrorCode::VersionMismatch,
                message: format!(
                    "Protocol version mismatch: expected {}, got {}",
                    IPC_PROTOCOL_VERSION, self.protocol_version
                ),
                context: Some(HashMap::from([
                    ("expected".to_string(), IPC_PROTOCOL_VERSION.to_string()),
                    ("actual".to_string(), self.protocol_version.clone()),
                ])),
            });
        }
        Ok(())
    }
}

/// IPC Error Codes
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum IpcErrorCode {
    /// Protocol version mismatch between Boothy and Sidecar
    VersionMismatch,

    /// IPC connection timeout
    Timeout,

    /// IPC connection disconnected
    Disconnect,

    /// Camera hardware not connected
    CameraNotConnected,

    /// Camera capture failed
    CaptureFailed,

    /// File transfer from camera failed
    FileTransferFailed,

    /// Invalid request payload
    InvalidPayload,

    /// Session destination not set
    SessionDestinationNotSet,

    /// File system error
    FileSystemError,

    /// Unknown/unhandled error
    Unknown,
}

/// IPC Error Details
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct IpcError {
    /// Error code (standardized)
    pub code: IpcErrorCode,

    /// Human-readable error message (customer-safe for display errors)
    pub message: String,

    /// Additional context for diagnostics (optional, may contain technical details)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<HashMap<String, String>>,
}

impl IpcError {
    /// Create a customer-safe error message suitable for UI display
    pub fn customer_safe_message(&self) -> String {
        match self.code {
            IpcErrorCode::CameraNotConnected => {
                "Camera is not connected. Please check the camera connection and try again."
                    .to_string()
            }
            IpcErrorCode::CaptureFailed => "Photo capture failed. Please try again.".to_string(),
            IpcErrorCode::FileTransferFailed => {
                "Failed to transfer photo from camera. Please try again.".to_string()
            }
            IpcErrorCode::Disconnect => {
                "Camera connection lost. Please reconnect the camera.".to_string()
            }
            _ => "An error occurred. Please contact support if this continues.".to_string(),
        }
    }

    /// Create a diagnostic message with full context (for admin/logs)
    pub fn diagnostic_message(&self) -> String {
        let mut msg = format!("[{}] {}", format!("{:?}", self.code), self.message);
        if let Some(context) = &self.context {
            msg.push_str(&format!(" | Context: {:?}", context));
        }
        msg
    }
}

// ============================================================================
// Event Payloads
// ============================================================================

/// Payload for event.camera.photoTransferred
/// Emitted by sidecar when a photo has been fully transferred to the session folder
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PhotoTransferredPayload {
    /// Absolute path to the transferred RAW file
    pub path: PathBuf,

    /// Transfer completion timestamp (ISO 8601)
    pub transferred_at: DateTime<Utc>,

    /// Original filename from camera
    pub original_filename: String,

    /// File size in bytes
    pub file_size: u64,
}

/// Payload for event.camera.captureStarted
/// Emitted by sidecar when capture is initiated (optional, for UI feedback)
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CaptureStartedPayload {
    /// Capture start timestamp
    pub started_at: DateTime<Utc>,
}

/// Payload for event.camera.error
/// Emitted by sidecar when a camera error occurs
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CameraErrorPayload {
    /// Error details
    pub error: IpcError,
}

// ============================================================================
// Request/Response Payloads
// ============================================================================

/// Payload for camera.setSessionDestination request
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetSessionDestinationRequest {
    /// Absolute path to the session's Raw/ folder
    pub destination_path: PathBuf,

    /// Session name for logging/correlation
    pub session_name: String,
}

/// Payload for camera.setSessionDestination response
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetSessionDestinationResponse {
    /// Whether the destination was set successfully
    pub success: bool,

    /// Confirmed destination path
    pub destination_path: PathBuf,
}

/// Payload for camera.getStatus request (no payload needed)

/// Payload for camera.getStatus response
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CameraStatusResponse {
    /// Whether the camera sidecar is connected
    pub connected: bool,

    /// Whether a camera is detected
    pub camera_detected: bool,

    /// Current session destination (if set)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_destination: Option<PathBuf>,

    /// Camera model name (if detected)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub camera_model: Option<String>,
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Generate a unique correlation ID for tracing
pub fn generate_correlation_id() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis();
    format!("corr-{}-{}", timestamp, uuid::Uuid::new_v4())
}

/// Generate a unique request ID
pub fn generate_request_id() -> String {
    format!("req-{}", uuid::Uuid::new_v4())
}
