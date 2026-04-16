use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    thread,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use serde::{Deserialize, Serialize};

use crate::{contracts::dto::HostErrorEnvelope, session::session_paths::SessionPaths};

pub const CANON_HELPER_BUNDLE_DIR: &str = "sidecar/canon-helper";
pub const CAMERA_HELPER_STATUS_FILE_NAME: &str = "camera-helper-status.json";
pub const CAMERA_HELPER_REQUESTS_FILE_NAME: &str = "camera-helper-requests.jsonl";
pub const CAMERA_HELPER_PROCESSED_REQUEST_IDS_FILE_NAME: &str =
    "camera-helper-processed-request-ids.txt";
pub const CAMERA_HELPER_EVENTS_FILE_NAME: &str = "camera-helper-events.jsonl";
pub const CANON_HELPER_STATUS_SCHEMA_VERSION: &str = "canon-helper-status/v1";
pub const CANON_HELPER_READY_SCHEMA_VERSION: &str = "canon-helper-ready/v1";
pub const CANON_HELPER_CAPTURE_REQUEST_SCHEMA_VERSION: &str = "canon-helper-request-capture/v1";
pub const CANON_HELPER_CAPTURE_ACCEPTED_SCHEMA_VERSION: &str = "canon-helper-capture-accepted/v1";
pub const CANON_HELPER_FAST_PREVIEW_READY_SCHEMA_VERSION: &str =
    "canon-helper-fast-preview-ready/v1";
pub const CANON_HELPER_FAST_THUMBNAIL_ATTEMPTED_SCHEMA_VERSION: &str =
    "canon-helper-fast-thumbnail-attempted/v1";
pub const CANON_HELPER_FAST_THUMBNAIL_FAILED_SCHEMA_VERSION: &str =
    "canon-helper-fast-thumbnail-failed/v1";
pub const CANON_HELPER_FILE_ARRIVED_SCHEMA_VERSION: &str = "canon-helper-file-arrived/v1";
pub const CANON_HELPER_RECOVERY_STATUS_SCHEMA_VERSION: &str = "canon-helper-recovery-status/v1";
pub const CANON_HELPER_ERROR_SCHEMA_VERSION: &str = "canon-helper-error/v1";

const CAPTURE_EVENT_POLL_INTERVAL_MS: u64 = 10;
const CAPTURE_ROUND_TRIP_FAST_PREVIEW_WAIT_BUDGET_MS: u64 = 500;
// Real camera follow-up captures can take well past 15 seconds before the RAW
// handoff closes. Keep the host budget longer than the helper budget so
// helper-side failures surface first without the host prematurely locking the
// session.
const DEFAULT_CAPTURE_ROUND_TRIP_TIMEOUT_MS: u64 = 50_000;
const CAPTURE_ROUND_TRIP_TIMEOUT_OVERRIDE_FILE_NAME: &str = ".camera-helper-capture-timeout-ms";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CanonHelperReadyMessage {
    #[serde(default)]
    pub schema_version: String,
    #[serde(rename = "type")]
    pub message_type: String,
    pub helper_version: Option<String>,
    pub protocol_version: Option<String>,
    pub runtime_platform: Option<String>,
    pub sdk_family: Option<String>,
    pub sdk_version: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CanonHelperStatusMessage {
    #[serde(default)]
    pub schema_version: String,
    #[serde(rename = "type", default)]
    pub message_type: Option<String>,
    pub session_id: String,
    #[serde(default)]
    pub sequence: Option<u64>,
    pub observed_at: String,
    pub camera_state: String,
    pub helper_state: String,
    #[serde(default)]
    pub camera_model: Option<String>,
    #[serde(default)]
    pub request_id: Option<String>,
    #[serde(default)]
    pub detail_code: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CanonHelperCaptureRequestMessage {
    #[serde(default)]
    pub schema_version: String,
    #[serde(rename = "type")]
    pub message_type: String,
    pub session_id: String,
    pub request_id: String,
    pub requested_at: String,
    pub active_preset_id: String,
    pub active_preset_version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CanonHelperCaptureAcceptedMessage {
    #[serde(default)]
    pub schema_version: String,
    #[serde(rename = "type")]
    pub message_type: String,
    pub session_id: String,
    pub request_id: String,
    pub detail_code: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CanonHelperFastPreviewReadyMessage {
    #[serde(default)]
    pub schema_version: String,
    #[serde(rename = "type")]
    pub message_type: String,
    pub session_id: String,
    pub request_id: String,
    pub capture_id: String,
    pub observed_at: String,
    pub fast_preview_path: String,
    #[serde(default)]
    pub fast_preview_kind: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CanonHelperFastThumbnailAttemptedMessage {
    #[serde(default)]
    pub schema_version: String,
    #[serde(rename = "type")]
    pub message_type: String,
    pub session_id: String,
    pub request_id: String,
    pub capture_id: String,
    pub observed_at: String,
    #[serde(default)]
    pub fast_preview_kind: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CanonHelperFastThumbnailFailedMessage {
    #[serde(default)]
    pub schema_version: String,
    #[serde(rename = "type")]
    pub message_type: String,
    pub session_id: String,
    pub request_id: String,
    pub capture_id: String,
    pub observed_at: String,
    pub detail_code: String,
    #[serde(default)]
    pub fast_preview_kind: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CanonHelperFileArrivedMessage {
    #[serde(default)]
    pub schema_version: String,
    #[serde(rename = "type")]
    pub message_type: String,
    pub session_id: String,
    pub request_id: String,
    pub capture_id: String,
    pub arrived_at: String,
    pub raw_path: String,
    #[serde(default)]
    pub fast_preview_path: Option<String>,
    #[serde(default)]
    pub fast_preview_kind: Option<String>,
}

#[derive(Debug, Clone)]
pub struct CompletedCaptureFastPreview {
    pub asset_path: String,
    pub kind: Option<String>,
}

#[derive(Debug, Clone)]
pub struct FastPreviewReadyUpdate {
    pub request_id: String,
    pub capture_id: String,
    pub asset_path: String,
    pub kind: Option<String>,
    pub visible_at_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CanonHelperRecoveryStatusMessage {
    #[serde(default)]
    pub schema_version: String,
    #[serde(rename = "type")]
    pub message_type: String,
    pub session_id: String,
    pub recovery_state: String,
    pub observed_at: String,
    pub detail_code: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CanonHelperErrorMessage {
    #[serde(default)]
    pub schema_version: String,
    #[serde(rename = "type")]
    pub message_type: String,
    pub session_id: Option<String>,
    pub observed_at: Option<String>,
    pub detail_code: String,
    pub message: Option<String>,
}

#[derive(Debug, Clone)]
pub struct CompletedCaptureRoundTrip {
    pub capture_id: String,
    pub raw_path: String,
    pub fast_preview: Option<CompletedCaptureFastPreview>,
    pub capture_accepted_at_ms: u64,
    pub persisted_at_ms: u64,
}

#[derive(Debug)]
pub enum SidecarClientError {
    StatusUnreadable,
    InvalidStatus,
    RequestWriteFailed,
    EventsUnreadable,
    InvalidEvents,
    CaptureTriggerRetryRequired,
    CaptureTimedOut,
    CaptureRejected,
    RecoveryRequired,
    CaptureSessionMismatch,
    CaptureFileMissing,
    CaptureFileEmpty,
    CaptureFileUnscoped,
    CaptureProtocolViolation,
}

#[derive(Debug, Clone)]
enum CanonHelperEvent {
    CaptureAccepted(CanonHelperCaptureAcceptedMessage),
    FastThumbnailAttempted(CanonHelperFastThumbnailAttemptedMessage),
    FastPreviewReady(CanonHelperFastPreviewReadyMessage),
    FastThumbnailFailed(CanonHelperFastThumbnailFailedMessage),
    FileArrived(CanonHelperFileArrivedMessage),
    RecoveryStatus(CanonHelperRecoveryStatusMessage),
    HelperError(CanonHelperErrorMessage),
}

pub fn bundled_helper_dir() -> PathBuf {
    PathBuf::from(CANON_HELPER_BUNDLE_DIR)
}

pub fn read_latest_status_message(
    base_dir: &Path,
    session_id: &str,
) -> Result<Option<CanonHelperStatusMessage>, SidecarClientError> {
    let status_path = SessionPaths::new(base_dir, session_id)
        .diagnostics_dir
        .join(CAMERA_HELPER_STATUS_FILE_NAME);

    if !status_path.is_file() {
        return Ok(None);
    }

    let contents =
        fs::read_to_string(&status_path).map_err(|_| SidecarClientError::StatusUnreadable)?;
    let normalized_contents = strip_utf8_bom_prefix(&contents);
    let Some(last_non_empty_line) = normalized_contents
        .lines()
        .rev()
        .find(|line| !line.trim().is_empty())
    else {
        return Ok(None);
    };
    let last_non_empty_line = strip_utf8_bom_prefix(last_non_empty_line);

    let message = serde_json::from_str::<CanonHelperStatusMessage>(normalized_contents)
        .or_else(|_| serde_json::from_str::<CanonHelperStatusMessage>(last_non_empty_line))
        .map_err(|_| SidecarClientError::InvalidStatus)?;

    if message.schema_version != CANON_HELPER_STATUS_SCHEMA_VERSION
        || message.message_type.as_deref() != Some("camera-status")
    {
        return Err(SidecarClientError::InvalidStatus);
    }

    Ok(Some(message))
}

pub fn write_capture_request_message(
    base_dir: &Path,
    message: &CanonHelperCaptureRequestMessage,
) -> Result<(), SidecarClientError> {
    let request_path = SessionPaths::try_new(base_dir, &message.session_id)
        .map(|paths| paths.diagnostics_dir.join(CAMERA_HELPER_REQUESTS_FILE_NAME))
        .map_err(|_| SidecarClientError::RequestWriteFailed)?;

    append_json_line(&request_path, message).map_err(|_| SidecarClientError::RequestWriteFailed)
}

pub fn read_capture_request_messages(
    base_dir: &Path,
    session_id: &str,
) -> Result<Vec<CanonHelperCaptureRequestMessage>, SidecarClientError> {
    let request_path = SessionPaths::try_new(base_dir, session_id)
        .map(|paths| paths.diagnostics_dir.join(CAMERA_HELPER_REQUESTS_FILE_NAME))
        .map_err(|_| SidecarClientError::EventsUnreadable)?;

    read_json_lines::<CanonHelperCaptureRequestMessage>(&request_path)
        .map_err(|_| SidecarClientError::InvalidEvents)
}

pub fn read_processed_capture_request_ids(
    base_dir: &Path,
    session_id: &str,
) -> Result<Vec<String>, SidecarClientError> {
    let processed_path = SessionPaths::try_new(base_dir, session_id)
        .map(|paths| {
            paths
                .diagnostics_dir
                .join(CAMERA_HELPER_PROCESSED_REQUEST_IDS_FILE_NAME)
        })
        .map_err(|_| SidecarClientError::EventsUnreadable)?;

    if !processed_path.is_file() {
        return Ok(Vec::new());
    }

    let contents =
        fs::read_to_string(processed_path).map_err(|_| SidecarClientError::EventsUnreadable)?;

    Ok(contents
        .lines()
        .map(strip_utf8_bom_prefix)
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(str::to_string)
        .collect())
}

pub fn read_capture_event_count(
    base_dir: &Path,
    session_id: &str,
) -> Result<usize, SidecarClientError> {
    Ok(read_capture_event_messages(base_dir, session_id)?.len())
}

pub fn read_latest_helper_error_message(
    base_dir: &Path,
    session_id: &str,
) -> Result<Option<CanonHelperErrorMessage>, SidecarClientError> {
    let events = read_capture_event_messages(base_dir, session_id)?;

    Ok(events.into_iter().rev().find_map(|event| match event {
        CanonHelperEvent::HelperError(message)
            if message
                .session_id
                .as_deref()
                .map(|value| value == session_id)
                .unwrap_or(true) =>
        {
            Some(message)
        }
        _ => None,
    }))
}

pub fn wait_for_capture_round_trip<F>(
    base_dir: &Path,
    session_id: &str,
    request_id: &str,
    requested_at: &str,
    starting_event_count: usize,
    mut on_fast_preview_ready: F,
) -> Result<CompletedCaptureRoundTrip, SidecarClientError>
where
    F: FnMut(&CanonHelperFastPreviewReadyMessage),
{
    let timeout_ms = capture_round_trip_timeout_ms(base_dir);
    let timeout_deadline = current_time_ms()
        .map_err(|_| SidecarClientError::CaptureTimedOut)?
        .saturating_add(timeout_ms);
    let mut accepted_at_ms: Option<u64> = None;
    let mut latest_event_count = starting_event_count;
    let mut pending_arrival: Option<CompletedCaptureRoundTrip> = None;
    let mut pending_fast_preview_deadline_ms: Option<u64> = None;

    loop {
        let events = read_capture_event_messages(base_dir, session_id)?;
        let latest_events_len = events.len();

        for event in events.iter().skip(latest_event_count) {
            match event {
                CanonHelperEvent::CaptureAccepted(message) => {
                    if message.request_id != request_id {
                        continue;
                    }

                    if message.session_id != session_id {
                        return Err(SidecarClientError::CaptureSessionMismatch);
                    }

                    accepted_at_ms.get_or_insert(
                        current_time_ms().map_err(|_| SidecarClientError::CaptureTimedOut)?,
                    );
                }
                CanonHelperEvent::FastPreviewReady(message) => {
                    if message.request_id != request_id {
                        continue;
                    }

                    // Fast preview telemetry is advisory. The host only surfaces a
                    // pending preview after RAW persistence succeeds and the preview
                    // has been promoted to the canonical path.
                    if message.session_id != session_id {
                        continue;
                    }

                    on_fast_preview_ready(message);
                    if let Some(pending_arrival) = pending_arrival.as_mut() {
                        pending_arrival.fast_preview = Some(CompletedCaptureFastPreview {
                            asset_path: message.fast_preview_path.clone(),
                            kind: message.fast_preview_kind.clone(),
                        });
                        return Ok(pending_arrival.clone());
                    }
                }
                CanonHelperEvent::FastThumbnailAttempted(message) => {
                    if message.request_id != request_id {
                        continue;
                    }

                    // Diagnostic thumbnail events must never fail the capture round trip.
                    if message.session_id != session_id {
                        continue;
                    }
                }
                CanonHelperEvent::FastThumbnailFailed(message) => {
                    if message.request_id != request_id {
                        continue;
                    }

                    // Diagnostic thumbnail events must never fail the capture round trip.
                    if message.session_id != session_id {
                        continue;
                    }
                }
                CanonHelperEvent::FileArrived(message) => {
                    if message.request_id != request_id {
                        continue;
                    }

                    if message.session_id != session_id {
                        return Err(SidecarClientError::CaptureSessionMismatch);
                    }

                    let accepted_at_ms =
                        accepted_at_ms.ok_or(SidecarClientError::CaptureProtocolViolation)?;
                    let raw_path = validate_arrived_raw_path(base_dir, session_id, message)?;
                    let fast_preview = extract_fast_preview_metadata(message);
                    let persisted_at_ms =
                        current_time_ms().map_err(|_| SidecarClientError::CaptureTimedOut)?;
                    let round_trip = CompletedCaptureRoundTrip {
                        capture_id: message.capture_id.clone(),
                        raw_path,
                        fast_preview,
                        capture_accepted_at_ms: accepted_at_ms,
                        persisted_at_ms,
                    };

                    if round_trip.fast_preview.is_some() {
                        return Ok(round_trip);
                    }

                    pending_fast_preview_deadline_ms = Some(
                        persisted_at_ms
                            .saturating_add(CAPTURE_ROUND_TRIP_FAST_PREVIEW_WAIT_BUDGET_MS),
                    );
                    pending_arrival = Some(round_trip);
                }
                CanonHelperEvent::RecoveryStatus(message) => {
                    if message.session_id != session_id {
                        continue;
                    }

                    if accepted_at_ms.is_some() {
                        return Err(SidecarClientError::RecoveryRequired);
                    }
                }
                CanonHelperEvent::HelperError(message) => {
                    if !message
                        .session_id
                        .as_deref()
                        .map(|value| value == session_id)
                        .unwrap_or(true)
                    {
                        continue;
                    }

                    if helper_error_predates_request(message, requested_at) {
                        continue;
                    }

                    return if is_retryable_capture_helper_error(message) {
                        Err(SidecarClientError::CaptureTriggerRetryRequired)
                    } else {
                        Err(SidecarClientError::CaptureRejected)
                    };
                }
            }
        }

        latest_event_count = latest_events_len;

        let now_ms = current_time_ms().map_err(|_| SidecarClientError::CaptureTimedOut)?;

        if pending_fast_preview_deadline_ms
            .map(|deadline_ms| now_ms >= deadline_ms)
            .unwrap_or(false)
        {
            return pending_arrival.ok_or(SidecarClientError::CaptureProtocolViolation);
        }

        if now_ms >= timeout_deadline {
            return Err(SidecarClientError::CaptureTimedOut);
        }

        thread::sleep(Duration::from_millis(CAPTURE_EVENT_POLL_INTERVAL_MS));
    }
}

pub fn map_capture_round_trip_error(
    session_id: &str,
    error: SidecarClientError,
) -> HostErrorEnvelope {
    let readiness =
        crate::contracts::dto::CaptureReadinessDto::phone_required(session_id.to_string());

    match error {
        SidecarClientError::CaptureTriggerRetryRequired => HostErrorEnvelope::capture_not_ready(
            "초점이 맞지 않았어요. 대상을 다시 맞추는 동안 잠시 기다려 주세요.",
            crate::contracts::dto::CaptureReadinessDto::capture_retry_required(
                session_id.to_string(),
                None,
            ),
        ),
        SidecarClientError::CaptureTimedOut => HostErrorEnvelope::capture_not_ready(
            "사진 저장을 끝내지 못했어요. 가까운 직원에게 알려 주세요.",
            readiness,
        ),
        SidecarClientError::CaptureRejected
        | SidecarClientError::RecoveryRequired
        | SidecarClientError::CaptureSessionMismatch
        | SidecarClientError::CaptureFileMissing
        | SidecarClientError::CaptureFileEmpty
        | SidecarClientError::CaptureFileUnscoped
        | SidecarClientError::CaptureProtocolViolation => HostErrorEnvelope::capture_not_ready(
            "사진 저장을 확인하지 못했어요. 가까운 직원에게 알려 주세요.",
            readiness,
        ),
        SidecarClientError::RequestWriteFailed
        | SidecarClientError::EventsUnreadable
        | SidecarClientError::InvalidEvents => HostErrorEnvelope::persistence(
            "카메라 연결 상태를 확인하지 못했어요. 가까운 직원에게 알려 주세요.",
        ),
        SidecarClientError::StatusUnreadable | SidecarClientError::InvalidStatus => {
            HostErrorEnvelope::persistence(
                "카메라 상태를 읽지 못했어요. 잠시 후 다시 시도해 주세요.",
            )
        }
    }
}

pub fn is_retryable_capture_helper_error(message: &CanonHelperErrorMessage) -> bool {
    match message.detail_code.as_str() {
        "camera-busy" | "capture-focus-not-locked" => true,
        "capture-trigger-failed" => message
            .message
            .as_deref()
            .map(is_legacy_focus_failure_message)
            .unwrap_or(false),
        _ => false,
    }
}

fn helper_error_predates_request(message: &CanonHelperErrorMessage, requested_at: &str) -> bool {
    let Some(observed_at) = message.observed_at.as_deref() else {
        return false;
    };

    let requested_at = crate::session::session_manifest::rfc3339_to_unix_seconds(requested_at);
    let observed_at = crate::session::session_manifest::rfc3339_to_unix_seconds(observed_at);

    match (observed_at, requested_at) {
        (Ok(observed_at_seconds), Ok(requested_at_seconds)) => {
            observed_at_seconds < requested_at_seconds
        }
        _ => false,
    }
}

fn is_legacy_focus_failure_message(message: &str) -> bool {
    message.to_ascii_lowercase().contains("0x00008d01")
}

fn read_capture_event_messages(
    base_dir: &Path,
    session_id: &str,
) -> Result<Vec<CanonHelperEvent>, SidecarClientError> {
    let events_path = SessionPaths::try_new(base_dir, session_id)
        .map(|paths| paths.diagnostics_dir.join(CAMERA_HELPER_EVENTS_FILE_NAME))
        .map_err(|_| SidecarClientError::EventsUnreadable)?;

    if !events_path.is_file() {
        return Ok(Vec::new());
    }

    let contents =
        fs::read_to_string(&events_path).map_err(|_| SidecarClientError::EventsUnreadable)?;
    let mut events = Vec::new();

    for line in contents.lines().filter(|line| !line.trim().is_empty()) {
        let event = parse_capture_event(strip_utf8_bom_prefix(line))?;
        events.push(event);
    }

    Ok(events)
}

fn parse_capture_event(line: &str) -> Result<CanonHelperEvent, SidecarClientError> {
    let value: serde_json::Value =
        serde_json::from_str(line).map_err(|_| SidecarClientError::InvalidEvents)?;
    let message_type = value
        .get("type")
        .and_then(|field| field.as_str())
        .ok_or(SidecarClientError::InvalidEvents)?;
    let schema_version = value
        .get("schemaVersion")
        .and_then(|field| field.as_str())
        .ok_or(SidecarClientError::InvalidEvents)?;

    match (message_type, schema_version) {
        ("capture-accepted", CANON_HELPER_CAPTURE_ACCEPTED_SCHEMA_VERSION) => {
            serde_json::from_value::<CanonHelperCaptureAcceptedMessage>(value)
                .map(CanonHelperEvent::CaptureAccepted)
                .map_err(|_| SidecarClientError::InvalidEvents)
        }
        ("fast-thumbnail-attempted", CANON_HELPER_FAST_THUMBNAIL_ATTEMPTED_SCHEMA_VERSION) => {
            serde_json::from_value::<CanonHelperFastThumbnailAttemptedMessage>(value)
                .map(CanonHelperEvent::FastThumbnailAttempted)
                .map_err(|_| SidecarClientError::InvalidEvents)
        }
        ("fast-preview-ready", CANON_HELPER_FAST_PREVIEW_READY_SCHEMA_VERSION) => {
            serde_json::from_value::<CanonHelperFastPreviewReadyMessage>(value)
                .map(CanonHelperEvent::FastPreviewReady)
                .map_err(|_| SidecarClientError::InvalidEvents)
        }
        ("fast-thumbnail-failed", CANON_HELPER_FAST_THUMBNAIL_FAILED_SCHEMA_VERSION) => {
            serde_json::from_value::<CanonHelperFastThumbnailFailedMessage>(value)
                .map(CanonHelperEvent::FastThumbnailFailed)
                .map_err(|_| SidecarClientError::InvalidEvents)
        }
        ("file-arrived", CANON_HELPER_FILE_ARRIVED_SCHEMA_VERSION) => {
            serde_json::from_value::<CanonHelperFileArrivedMessage>(value)
                .map(CanonHelperEvent::FileArrived)
                .map_err(|_| SidecarClientError::InvalidEvents)
        }
        ("recovery-status", CANON_HELPER_RECOVERY_STATUS_SCHEMA_VERSION) => {
            serde_json::from_value::<CanonHelperRecoveryStatusMessage>(value)
                .map(CanonHelperEvent::RecoveryStatus)
                .map_err(|_| SidecarClientError::InvalidEvents)
        }
        ("helper-error", CANON_HELPER_ERROR_SCHEMA_VERSION) => {
            serde_json::from_value::<CanonHelperErrorMessage>(value)
                .map(CanonHelperEvent::HelperError)
                .map_err(|_| SidecarClientError::InvalidEvents)
        }
        _ => Err(SidecarClientError::InvalidEvents),
    }
}

#[cfg(test)]
mod tests {
    use super::{
        append_json_line, capture_round_trip_timeout_ms, read_latest_status_message,
        wait_for_capture_round_trip,
        CanonHelperCaptureAcceptedMessage, CanonHelperFastPreviewReadyMessage,
        CanonHelperFileArrivedMessage, SidecarClientError, CAMERA_HELPER_EVENTS_FILE_NAME,
        CAMERA_HELPER_STATUS_FILE_NAME, CANON_HELPER_CAPTURE_ACCEPTED_SCHEMA_VERSION,
        CANON_HELPER_FAST_PREVIEW_READY_SCHEMA_VERSION, CANON_HELPER_FILE_ARRIVED_SCHEMA_VERSION,
        DEFAULT_CAPTURE_ROUND_TRIP_TIMEOUT_MS, parse_capture_event,
    };
    use crate::session::session_paths::SessionPaths;
    use std::{
        fs,
        path::PathBuf,
        thread,
        time::{SystemTime, UNIX_EPOCH},
    };

    fn unique_test_root(name: &str) -> PathBuf {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();

        std::env::temp_dir().join(format!("boothy-sidecar-client-{name}-{stamp}"))
    }

    #[test]
    fn parse_capture_event_rejects_helper_events_with_mismatched_schema_versions() {
        let event = r#"{"schemaVersion":"canon-helper-error/v2","type":"helper-error","sessionId":"session_01hs6n1r8b8zc5v4ey2x7b9g1m","detailCode":"capture-download-timeout"}"#;

        let error = parse_capture_event(event)
            .expect_err("mismatched helper event schemaVersion should be rejected");

        assert!(matches!(error, SidecarClientError::InvalidEvents));
    }

    #[test]
    fn read_latest_status_message_rejects_status_with_mismatched_schema_version() {
        let base_dir = unique_test_root("status-schema-version");
        let session_id = "session_01hs6n1r8b8zc5v4ey2x7b9g1m";
        let paths = SessionPaths::new(&base_dir, session_id);
        fs::create_dir_all(&paths.diagnostics_dir).expect("diagnostics directory should exist");
        fs::write(
            paths.diagnostics_dir.join(CAMERA_HELPER_STATUS_FILE_NAME),
            r#"{"schemaVersion":"canon-helper-status/v2","type":"camera-status","sessionId":"session_01hs6n1r8b8zc5v4ey2x7b9g1m","observedAt":"2026-04-10T01:00:15Z","cameraState":"ready","helperState":"healthy"}"#,
        )
        .expect("status fixture should write");

        let error = read_latest_status_message(&base_dir, session_id)
            .expect_err("mismatched status schemaVersion should be rejected");

        assert!(matches!(error, SidecarClientError::InvalidStatus));

        let _ = fs::remove_dir_all(base_dir);
    }

    #[test]
    fn capture_round_trip_waits_briefly_for_late_fast_preview_ready_after_file_arrived() {
        let base_dir = unique_test_root("late-fast-preview-after-file-arrived");
        let session_id = "session_01hs6n1r8b8zc5v4ey2x7b9g1m";
        let request_id = "request_20260414_001";
        let capture_id = "capture_20260414_001";
        let paths = SessionPaths::new(&base_dir, session_id);
        let raw_path = paths
            .captures_originals_dir
            .join(format!("{capture_id}.CR2"));
        let preview_path = paths
            .renders_previews_dir
            .join(format!("{capture_id}.jpg"));
        let events_path = paths.diagnostics_dir.join(CAMERA_HELPER_EVENTS_FILE_NAME);
        fs::create_dir_all(&paths.captures_originals_dir).expect("raw dir should exist");
        fs::create_dir_all(&paths.renders_previews_dir).expect("preview dir should exist");
        fs::write(&raw_path, b"raw-bytes").expect("raw fixture should exist");

        append_json_line(
            &events_path,
            &CanonHelperCaptureAcceptedMessage {
                schema_version: CANON_HELPER_CAPTURE_ACCEPTED_SCHEMA_VERSION.into(),
                message_type: "capture-accepted".into(),
                session_id: session_id.into(),
                request_id: request_id.into(),
                detail_code: None,
            },
        )
        .expect("accepted event should append");
        append_json_line(
            &events_path,
            &CanonHelperFileArrivedMessage {
                schema_version: CANON_HELPER_FILE_ARRIVED_SCHEMA_VERSION.into(),
                message_type: "file-arrived".into(),
                session_id: session_id.into(),
                request_id: request_id.into(),
                capture_id: capture_id.into(),
                arrived_at: "2026-04-14T06:17:26Z".into(),
                raw_path: raw_path.to_string_lossy().into_owned(),
                fast_preview_path: None,
                fast_preview_kind: Some("camera-thumbnail".into()),
            },
        )
        .expect("file arrived event should append");

        let writer_events_path = events_path.clone();
        let writer_preview_path = preview_path.clone();
        let writer = thread::spawn(move || {
            thread::sleep(std::time::Duration::from_millis(75));
            fs::write(&writer_preview_path, [0xFF, 0xD8, 0xFF, 0xE0, 0x00])
                .expect("preview fixture should exist");
            append_json_line(
                &writer_events_path,
                &CanonHelperFastPreviewReadyMessage {
                    schema_version: CANON_HELPER_FAST_PREVIEW_READY_SCHEMA_VERSION.into(),
                    message_type: "fast-preview-ready".into(),
                    session_id: session_id.into(),
                    request_id: request_id.into(),
                    capture_id: capture_id.into(),
                    observed_at: "2026-04-14T06:17:26.300Z".into(),
                    fast_preview_path: writer_preview_path.to_string_lossy().into_owned(),
                    fast_preview_kind: Some("windows-shell-thumbnail".into()),
                },
            )
            .expect("fast preview event should append");
        });

        let mut surfaced_fast_preview = None;
        let result = wait_for_capture_round_trip(
            &base_dir,
            session_id,
            request_id,
            "2026-04-14T06:17:25Z",
            0,
            |message| {
                surfaced_fast_preview = Some(message.fast_preview_path.clone());
            },
        )
        .expect("late fast preview should still attach to the round trip");

        writer.join().expect("writer thread should finish");

        assert_eq!(
            surfaced_fast_preview.as_deref(),
            Some(preview_path.to_string_lossy().as_ref())
        );
        assert_eq!(
            result
                .fast_preview
                .as_ref()
                .map(|preview| preview.asset_path.as_str()),
            Some(preview_path.to_string_lossy().as_ref())
        );
        assert_eq!(
            result
                .fast_preview
                .as_ref()
                .and_then(|preview| preview.kind.as_deref()),
            Some("windows-shell-thumbnail")
        );

        let _ = fs::remove_dir_all(base_dir);
    }

    #[test]
    fn capture_round_trip_timeout_uses_the_latest_default_budget_when_unset() {
        let base_dir = unique_test_root("default-capture-timeout");
        let env_guard = std::env::var_os("BOOTHY_CAPTURE_TIMEOUT_MS");
        std::env::remove_var("BOOTHY_CAPTURE_TIMEOUT_MS");

        let timeout_ms = capture_round_trip_timeout_ms(&base_dir);

        match env_guard {
            Some(value) => std::env::set_var("BOOTHY_CAPTURE_TIMEOUT_MS", value),
            None => std::env::remove_var("BOOTHY_CAPTURE_TIMEOUT_MS"),
        }

        assert_eq!(timeout_ms, DEFAULT_CAPTURE_ROUND_TRIP_TIMEOUT_MS);
    }
}

fn validate_arrived_raw_path(
    base_dir: &Path,
    session_id: &str,
    message: &CanonHelperFileArrivedMessage,
) -> Result<String, SidecarClientError> {
    let paths = SessionPaths::try_new(base_dir, session_id)
        .map_err(|_| SidecarClientError::CaptureFileUnscoped)?;
    let raw_path = PathBuf::from(&message.raw_path);

    if !raw_path.is_absolute() {
        return Err(SidecarClientError::CaptureFileUnscoped);
    }

    let normalized_raw_path = raw_path.to_string_lossy().replace('\\', "/").to_lowercase();
    let normalized_originals_dir = format!(
        "{}/",
        paths
            .captures_originals_dir
            .to_string_lossy()
            .replace('\\', "/")
            .to_lowercase()
    );

    if !normalized_raw_path.starts_with(&normalized_originals_dir) {
        return Err(SidecarClientError::CaptureFileUnscoped);
    }

    let metadata = fs::metadata(&raw_path).map_err(|_| SidecarClientError::CaptureFileMissing)?;

    if !metadata.is_file() {
        return Err(SidecarClientError::CaptureFileMissing);
    }

    if metadata.len() == 0 {
        return Err(SidecarClientError::CaptureFileEmpty);
    }

    Ok(raw_path.to_string_lossy().into_owned())
}

fn extract_fast_preview_metadata(
    message: &CanonHelperFileArrivedMessage,
) -> Option<CompletedCaptureFastPreview> {
    let asset_path = message
        .fast_preview_path
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())?
        .to_string();
    let kind = message
        .fast_preview_kind
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string);

    Some(CompletedCaptureFastPreview { asset_path, kind })
}

fn append_json_line<T: Serialize>(path: &Path, value: &T) -> Result<(), std::io::Error> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let mut file = OpenOptions::new().create(true).append(true).open(path)?;
    let line = serde_json::to_string(value)
        .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidData, error))?;

    file.write_all(line.as_bytes())?;
    file.write_all(b"\n")?;
    file.flush()?;

    Ok(())
}

fn read_json_lines<T>(path: &Path) -> Result<Vec<T>, std::io::Error>
where
    T: for<'de> Deserialize<'de>,
{
    if !path.is_file() {
        return Ok(Vec::new());
    }

    let contents = fs::read_to_string(path)?;
    let mut messages = Vec::new();

    for line in contents.lines().filter(|line| !line.trim().is_empty()) {
        let message = serde_json::from_str::<T>(strip_utf8_bom_prefix(line))
            .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidData, error))?;
        messages.push(message);
    }

    Ok(messages)
}

fn capture_round_trip_timeout_ms(base_dir: &Path) -> u64 {
    let override_path = base_dir.join(CAPTURE_ROUND_TRIP_TIMEOUT_OVERRIDE_FILE_NAME);

    if let Ok(value) = fs::read_to_string(&override_path) {
        if let Ok(timeout_ms) = value.trim().parse::<u64>() {
            if timeout_ms > 0 {
                return timeout_ms;
            }
        }
    }

    std::env::var("BOOTHY_CAPTURE_TIMEOUT_MS")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(DEFAULT_CAPTURE_ROUND_TRIP_TIMEOUT_MS)
}

fn current_time_ms() -> Result<u64, std::time::SystemTimeError> {
    Ok(SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis() as u64)
}

fn strip_utf8_bom_prefix(value: &str) -> &str {
    value.trim_start_matches('\u{feff}')
}
