use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum HostErrorCode {
    InvalidPayload,
    ProvisioningFailed,
    SessionCaptureOutOfRoot,
    SessionCaptureNotFound,
    SessionCaptureWrongSession,
    SessionManifestInvalid,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct HostErrorEnvelope {
    pub code: HostErrorCode,
    pub message: String,
    pub retryable: bool,
}

impl HostErrorEnvelope {
    pub fn invalid_payload(message: impl Into<String>) -> Self {
        Self {
            code: HostErrorCode::InvalidPayload,
            message: message.into(),
            retryable: false,
        }
    }

    pub fn provisioning_failed(message: impl Into<String>) -> Self {
        Self {
            code: HostErrorCode::ProvisioningFailed,
            message: message.into(),
            retryable: false,
        }
    }

    pub fn session_capture_out_of_root(message: impl Into<String>) -> Self {
        Self {
            code: HostErrorCode::SessionCaptureOutOfRoot,
            message: message.into(),
            retryable: false,
        }
    }

    pub fn session_capture_not_found(message: impl Into<String>) -> Self {
        Self {
            code: HostErrorCode::SessionCaptureNotFound,
            message: message.into(),
            retryable: false,
        }
    }

    pub fn session_capture_wrong_session(message: impl Into<String>) -> Self {
        Self {
            code: HostErrorCode::SessionCaptureWrongSession,
            message: message.into(),
            retryable: false,
        }
    }

    pub fn session_manifest_invalid(message: impl Into<String>) -> Self {
        Self {
            code: HostErrorCode::SessionManifestInvalid,
            message: message.into(),
            retryable: false,
        }
    }
}

impl HostErrorCode {
    pub fn as_str(&self) -> &'static str {
        match self {
            HostErrorCode::InvalidPayload => "host.invalid_payload",
            HostErrorCode::ProvisioningFailed => "host.provisioning_failed",
            HostErrorCode::SessionCaptureOutOfRoot => "session.capture.out_of_root",
            HostErrorCode::SessionCaptureNotFound => "session.capture.not_found",
            HostErrorCode::SessionCaptureWrongSession => "session.capture.wrong_session",
            HostErrorCode::SessionManifestInvalid => "session.manifest.invalid",
        }
    }
}
