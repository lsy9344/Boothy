use serde::{Deserialize, Serialize};

use crate::{
    contracts::schema_version::{
        CONTRACT_SCHEMA_VERSION, ERROR_ENVELOPE_SCHEMA_VERSION, PROTOCOL_SCHEMA_VERSION,
    },
    session::session_manifest::SessionTiming,
};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SessionStartPayload {
    pub branch_id: String,
    pub session_name: String,
    pub reservation_start_at: Option<String>,
    pub session_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SessionStartResult {
    pub session_id: String,
    pub session_name: String,
    pub session_folder: String,
    pub manifest_path: String,
    pub created_at: String,
    pub preparation_state: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SessionStartSuccessEnvelope {
    pub ok: bool,
    pub value: SessionStartResult,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SessionStartFailureEnvelope {
    pub ok: bool,
    pub error_code: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(untagged)]
pub enum SessionStartEnvelope {
    Success(SessionStartSuccessEnvelope),
    Failure(SessionStartFailureEnvelope),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct InitializeSessionTimingPayload {
    pub session_id: String,
    pub manifest_path: String,
    pub reservation_start_at: String,
    pub session_type: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GetSessionTimingPayload {
    pub session_id: String,
    pub manifest_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GetPostEndOutcomePayload {
    pub session_id: String,
    pub manifest_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ExtendSessionTimingPayload {
    pub session_id: String,
    pub manifest_path: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SessionTimingResult {
    pub session_id: String,
    pub manifest_path: String,
    pub timing: SessionTiming,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SessionTimingSuccessEnvelope {
    pub ok: bool,
    pub value: SessionTimingResult,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SessionTimingFailureEnvelope {
    pub ok: bool,
    pub error_code: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(untagged)]
pub enum SessionTimingEnvelope {
    Success(SessionTimingSuccessEnvelope),
    Failure(SessionTimingFailureEnvelope),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PostEndOutcomeResult {
    pub session_id: String,
    pub actual_shoot_end_at: String,
    pub outcome_kind: String,
    pub guidance_mode: String,
    pub session_name: Option<String>,
    pub show_session_name: bool,
    pub handoff_target_label: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PostEndOutcomeSuccessEnvelope {
    pub ok: bool,
    pub value: PostEndOutcomeResult,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PostEndOutcomeFailureEnvelope {
    pub ok: bool,
    pub error_code: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(untagged)]
pub enum PostEndOutcomeEnvelope {
    Success(PostEndOutcomeSuccessEnvelope),
    Failure(PostEndOutcomeFailureEnvelope),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SelectSessionPresetPayload {
    pub session_id: String,
    pub preset_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SessionActivePresetDto {
    pub preset_id: String,
    pub display_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SelectSessionPresetResult {
    pub manifest_path: String,
    pub updated_at: String,
    pub active_preset: SessionActivePresetDto,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SelectSessionPresetSuccessEnvelope {
    pub ok: bool,
    pub value: SelectSessionPresetResult,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SelectSessionPresetFailureEnvelope {
    pub ok: bool,
    pub error_code: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(untagged)]
pub enum SelectSessionPresetEnvelope {
    Success(SelectSessionPresetSuccessEnvelope),
    Failure(SelectSessionPresetFailureEnvelope),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SessionGalleryRequest {
    pub session_id: String,
    pub manifest_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DeleteSessionPhotoRequest {
    pub session_id: String,
    pub capture_id: String,
    pub manifest_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SessionGalleryItem {
    pub capture_id: String,
    pub session_id: String,
    pub captured_at: String,
    pub display_order: usize,
    pub is_latest: bool,
    pub preview_path: String,
    pub thumbnail_path: String,
    pub label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SessionGallerySnapshot {
    pub schema_version: String,
    pub session_id: String,
    pub session_name: String,
    pub shoot_ends_at: Option<String>,
    pub active_preset_name: Option<String>,
    pub latest_capture_id: Option<String>,
    pub selected_capture_id: Option<String>,
    pub items: Vec<SessionGalleryItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DeleteSessionPhotoResponse {
    pub schema_version: String,
    pub deleted_capture_id: String,
    pub confirmation_message: String,
    pub gallery: SessionGallerySnapshot,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum MockScenario {
    ReadinessSuccess,
    ReadinessDegraded,
    NormalizedError,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CameraReadinessPayload {
    pub desired_camera_id: Option<String>,
    pub mock_scenario: Option<MockScenario>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CameraReadinessRequest {
    pub schema_version: String,
    pub request_id: String,
    pub correlation_id: String,
    pub method: String,
    pub session_id: Option<String>,
    pub payload: CameraReadinessPayload,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CaptureActivePresetDto {
    pub preset_id: String,
    pub label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CaptureCommandPayload {
    pub active_preset: CaptureActivePresetDto,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SidecarCaptureRequestPayload {
    pub active_preset: CaptureActivePresetDto,
    pub capture_id: String,
    pub original_file_name: String,
    pub processed_file_name: String,
    pub original_output_path: String,
    pub processed_output_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CaptureCommandRequest {
    pub schema_version: String,
    pub request_id: String,
    pub correlation_id: String,
    pub method: String,
    pub session_id: String,
    pub payload: CaptureCommandPayload,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SidecarCaptureRequest {
    pub schema_version: String,
    pub request_id: String,
    pub correlation_id: String,
    pub method: String,
    pub session_id: String,
    pub payload: SidecarCaptureRequestPayload,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum CaptureProgressStage {
    CaptureStarted,
    CaptureCompleted,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CaptureProgressPayload {
    pub stage: CaptureProgressStage,
    pub capture_id: String,
    pub percent_complete: u8,
    pub last_updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CaptureProgressEvent {
    pub schema_version: String,
    pub request_id: String,
    pub correlation_id: String,
    pub event: String,
    pub session_id: Option<String>,
    pub payload: CaptureProgressPayload,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SidecarCaptureSuccessResponse {
    pub schema_version: String,
    pub request_id: String,
    pub correlation_id: String,
    pub ok: bool,
    pub session_id: String,
    pub capture_id: String,
    pub original_file_name: String,
    pub processed_file_name: String,
    pub captured_at: String,
    pub manifest_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum CameraConnectionState {
    Connected,
    Reconnecting,
    Disconnected,
    Offline,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum CameraReadiness {
    Pending,
    Ready,
    Degraded,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CameraStatusSnapshot {
    pub connection_state: CameraConnectionState,
    pub readiness: CameraReadiness,
    pub last_updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum ErrorSeverity {
    Info,
    Warning,
    Error,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum CustomerState {
    CameraReconnectNeeded,
    CameraUnavailable,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum CustomerCameraConnectionState {
    Connected,
    NeedsAttention,
    Offline,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum OperatorCameraConnectionState {
    Connected,
    Reconnecting,
    Disconnected,
    Offline,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum OperatorAction {
    CheckCableAndRetry,
    RestartHelper,
    ContactSupport,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct NormalizedErrorEnvelope {
    pub schema_version: String,
    pub code: String,
    pub severity: ErrorSeverity,
    pub retryable: bool,
    pub customer_state: CustomerState,
    pub customer_camera_connection_state: CustomerCameraConnectionState,
    pub operator_camera_connection_state: OperatorCameraConnectionState,
    pub operator_action: OperatorAction,
    pub message: String,
    pub details: Option<String>,
}

impl NormalizedErrorEnvelope {
    pub fn camera_reconnect_needed(
        code: impl Into<String>,
        message: impl Into<String>,
        details: Option<String>,
    ) -> Self {
        Self {
            schema_version: ERROR_ENVELOPE_SCHEMA_VERSION.into(),
            code: code.into(),
            severity: ErrorSeverity::Warning,
            retryable: true,
            customer_state: CustomerState::CameraReconnectNeeded,
            customer_camera_connection_state: CustomerCameraConnectionState::NeedsAttention,
            operator_camera_connection_state: OperatorCameraConnectionState::Reconnecting,
            operator_action: OperatorAction::CheckCableAndRetry,
            message: message.into(),
            details,
        }
    }

    pub fn camera_unavailable(
        code: impl Into<String>,
        message: impl Into<String>,
        details: Option<String>,
    ) -> Self {
        Self {
            schema_version: ERROR_ENVELOPE_SCHEMA_VERSION.into(),
            code: code.into(),
            severity: ErrorSeverity::Error,
            retryable: false,
            customer_state: CustomerState::CameraUnavailable,
            customer_camera_connection_state: CustomerCameraConnectionState::Offline,
            operator_camera_connection_state: OperatorCameraConnectionState::Disconnected,
            operator_action: OperatorAction::ContactSupport,
            message: message.into(),
            details,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum CustomerReadinessConnectionState {
    Preparing,
    Waiting,
    Ready,
    PhoneRequired,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum LastSafeCustomerState {
    Preparing,
    Ready,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CameraReadinessStatus {
    pub session_id: String,
    pub connection_state: CustomerReadinessConnectionState,
    pub capture_enabled: bool,
    pub last_stable_customer_state: Option<LastSafeCustomerState>,
    pub error: Option<NormalizedErrorEnvelope>,
    pub emitted_at: String,
}

impl CameraReadinessStatus {
    pub fn validate(&self) -> Result<(), NormalizedErrorEnvelope> {
        let should_allow_capture = matches!(
            self.connection_state,
            CustomerReadinessConnectionState::Ready
        );

        if self.capture_enabled != should_allow_capture {
            return Err(NormalizedErrorEnvelope::camera_unavailable(
                "camera.contract.invalid",
                "Camera readiness contract is inconsistent.",
                Some(format!(
                    "connectionState={:?} requires captureEnabled={should_allow_capture}, got {}",
                    self.connection_state, self.capture_enabled
                )),
            ));
        }

        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CameraStatusChangedEvent {
    pub schema_version: String,
    pub request_id: String,
    pub correlation_id: String,
    pub event: String,
    pub session_id: Option<String>,
    pub payload: CameraStatusSnapshot,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CameraCommandResult {
    pub schema_version: String,
    pub request_id: String,
    pub correlation_id: String,
    pub ok: bool,
    pub status: CameraStatusSnapshot,
    pub manifest_path: Option<String>,
    pub error: Option<NormalizedErrorEnvelope>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CaptureCommandResult {
    pub schema_version: String,
    pub request_id: String,
    pub correlation_id: String,
    pub ok: bool,
    pub session_id: String,
    pub capture_id: String,
    pub original_file_name: String,
    pub processed_file_name: String,
    pub captured_at: String,
    pub manifest_path: String,
}

impl CameraCommandResult {
    pub fn success(
        request_id: impl Into<String>,
        correlation_id: impl Into<String>,
        status: CameraStatusSnapshot,
    ) -> Self {
        Self {
            schema_version: CONTRACT_SCHEMA_VERSION.into(),
            request_id: request_id.into(),
            correlation_id: correlation_id.into(),
            ok: true,
            status,
            manifest_path: None,
            error: None,
        }
    }

    pub fn failure(
        request_id: impl Into<String>,
        correlation_id: impl Into<String>,
        status: CameraStatusSnapshot,
        error: NormalizedErrorEnvelope,
    ) -> Self {
        Self {
            schema_version: CONTRACT_SCHEMA_VERSION.into(),
            request_id: request_id.into(),
            correlation_id: correlation_id.into(),
            ok: false,
            status,
            manifest_path: None,
            error: Some(error),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ActivePresetDto {
    pub preset_id: String,
    pub label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LatestSessionPhoto {
    pub session_id: String,
    pub capture_id: String,
    pub sequence: u32,
    pub asset_url: String,
    pub captured_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum LatestPhotoState {
    Empty,
    Updating {
        next_capture_id: String,
        preview: Option<LatestSessionPhoto>,
    },
    Ready {
        photo: LatestSessionPhoto,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CaptureConfidenceSnapshot {
    pub session_id: String,
    pub revision: u32,
    pub updated_at: String,
    pub shoot_ends_at: String,
    pub active_preset: ActivePresetDto,
    pub latest_photo: LatestPhotoState,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SidecarSuccessResponse {
    pub schema_version: String,
    pub request_id: String,
    pub correlation_id: String,
    pub ok: bool,
    pub status: CameraStatusSnapshot,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SidecarErrorResponse {
    pub schema_version: String,
    pub request_id: String,
    pub correlation_id: String,
    pub ok: bool,
    pub error: NormalizedErrorEnvelope,
}

impl CameraReadinessRequest {
    pub fn validate(&self) -> Result<(), NormalizedErrorEnvelope> {
        if self.schema_version != PROTOCOL_SCHEMA_VERSION {
            return Err(NormalizedErrorEnvelope::camera_unavailable(
                "camera.protocol.invalid",
                "Camera helper protocol version is not supported.",
                Some(format!(
                    "expected {PROTOCOL_SCHEMA_VERSION}, got {}",
                    self.schema_version
                )),
            ));
        }

        if self.method != "camera.checkReadiness" {
            return Err(NormalizedErrorEnvelope::camera_unavailable(
                "camera.method.invalid",
                "Camera helper method is not supported.",
                Some(format!("unsupported method: {}", self.method)),
            ));
        }

        Ok(())
    }
}

impl CaptureCommandRequest {
    pub fn validate(&self) -> Result<(), NormalizedErrorEnvelope> {
        if self.schema_version != PROTOCOL_SCHEMA_VERSION {
            return Err(NormalizedErrorEnvelope::camera_unavailable(
                "camera.protocol.invalid",
                "Camera helper protocol version is not supported.",
                Some(format!(
                    "expected {PROTOCOL_SCHEMA_VERSION}, got {}",
                    self.schema_version
                )),
            ));
        }

        if self.method != "camera.capture" {
            return Err(NormalizedErrorEnvelope::camera_unavailable(
                "camera.method.invalid",
                "Camera helper method is not supported.",
                Some(format!("unsupported method: {}", self.method)),
            ));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{
        CameraReadinessStatus, CustomerReadinessConnectionState, LastSafeCustomerState,
        SessionStartPayload,
    };

    #[test]
    fn session_start_payload_deserializes_the_session_name_contract() {
        let payload: SessionStartPayload = serde_json::from_value(json!({
            "branchId": "gangnam-main",
            "sessionName": "김보라 오후 세션",
            "reservationStartAt": "2026-03-08T09:00:00.000Z",
            "sessionType": "couponExtended"
        }))
        .expect("session start payload should deserialize");

        assert_eq!(payload.branch_id, "gangnam-main");
        assert_eq!(payload.session_name, "김보라 오후 세션");
        assert_eq!(
            payload.reservation_start_at.as_deref(),
            Some("2026-03-08T09:00:00.000Z")
        );
        assert_eq!(payload.session_type.as_deref(), Some("couponExtended"));
    }

    #[test]
    fn camera_readiness_status_rejects_ready_when_capture_is_disabled() {
        let error = CameraReadinessStatus {
            session_id: "session-001".into(),
            connection_state: CustomerReadinessConnectionState::Ready,
            capture_enabled: false,
            last_stable_customer_state: Some(LastSafeCustomerState::Ready),
            error: None,
            emitted_at: "2026-03-13T09:00:00.000Z".into(),
        }
        .validate()
        .expect_err("ready readiness must enable capture");

        assert_eq!(error.code, "camera.contract.invalid");
    }

    #[test]
    fn camera_readiness_status_accepts_waiting_when_capture_is_disabled() {
        CameraReadinessStatus {
            session_id: "session-001".into(),
            connection_state: CustomerReadinessConnectionState::Waiting,
            capture_enabled: false,
            last_stable_customer_state: Some(LastSafeCustomerState::Ready),
            error: None,
            emitted_at: "2026-03-13T09:00:00.000Z".into(),
        }
        .validate()
        .expect("waiting readiness should keep capture disabled");
    }
}
