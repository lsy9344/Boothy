use chrono::{Duration, SecondsFormat, Utc};
use std::path::{Component, Path, PathBuf};

use crate::contracts::dto::{
    ActivePresetDto, CameraCommandResult, CameraConnectionState, CameraReadiness,
    CameraReadinessRequest, CameraReadinessStatus, CameraStatusChangedEvent, CameraStatusSnapshot,
    CaptureConfidenceSnapshot, CustomerReadinessConnectionState, LatestPhotoState,
    NormalizedErrorEnvelope,
};
use crate::session::session_manifest::SessionManifest;

use super::sidecar_client::{run_mock_readiness_sidecar, SidecarClientConfig};

pub trait RecordingProgressSink {
    fn record_status(&mut self, event: CameraStatusChangedEvent);
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CameraHostConfig {
    pub sidecar: SidecarClientConfig,
}

impl Default for CameraHostConfig {
    fn default() -> Self {
        Self {
            sidecar: SidecarClientConfig::default(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct CameraHost {
    config: CameraHostConfig,
}

impl CameraHost {
    pub fn new(config: CameraHostConfig) -> Self {
        Self { config }
    }

    pub fn get_readiness_snapshot(&self, session_id: &str) -> CameraReadinessStatus {
        let request = build_snapshot_request(session_id);

        match run_mock_readiness_sidecar(&self.config.sidecar, &request) {
            Ok(outcome) => {
                if let Some(success) = outcome.success {
                    return map_readiness_status(session_id, &success.status, outcome.error.as_ref());
                }

                if let Some(error) = outcome.error {
                    return validated_readiness_status(CameraReadinessStatus {
                        session_id: session_id.into(),
                        connection_state: map_error_connection_state(&error),
                        capture_enabled: false,
                        last_stable_customer_state: None,
                        error: Some(error),
                        emitted_at: now_iso(),
                    });
                }

                unavailable_readiness_status(
                    session_id,
                    "camera.sidecar.empty_response",
                    "Camera helper returned no readiness result.",
                    None,
                )
            }
            Err(error) => unavailable_readiness_status(
                session_id,
                "camera.sidecar.unavailable",
                "Camera helper could not be reached.",
                Some(error),
            ),
        }
    }

    pub fn watch_readiness<S: RecordingProgressSink>(&self, session_id: &str, sink: &mut S) {
        sink.record_status(CameraStatusChangedEvent {
            schema_version: crate::contracts::schema_version::PROTOCOL_SCHEMA_VERSION.into(),
            request_id: format!("watch-{session_id}"),
            correlation_id: session_id.into(),
            event: "camera.statusChanged".into(),
            session_id: Some(session_id.into()),
            payload: CameraStatusSnapshot {
                connection_state: CameraConnectionState::Disconnected,
                readiness: CameraReadiness::Pending,
                last_updated_at: now_iso(),
            },
        });
    }

    pub fn run_readiness_flow<S: RecordingProgressSink>(
        &self,
        request: CameraReadinessRequest,
        sink: &mut S,
    ) -> CameraCommandResult {
        if let Err(error) = request.validate() {
            return CameraCommandResult::failure(
                request.request_id,
                request.correlation_id,
                degraded_status(),
                error,
            );
        }

        match run_mock_readiness_sidecar(&self.config.sidecar, &request) {
            Err(error) => {
                CameraCommandResult::failure(
                    request.request_id,
                    request.correlation_id,
                    degraded_status(),
                    NormalizedErrorEnvelope::camera_unavailable(
                        "camera.sidecar.unavailable",
                        "Camera helper could not be reached.",
                        Some(error),
                    ),
                )
            }
            Ok(outcome) => {
                for event in outcome.progress_events {
                    sink.record_status(event);
                }

                if let Some(success) = outcome.success {
                    return CameraCommandResult::success(
                        success.request_id,
                        success.correlation_id,
                        success.status,
                    );
                }

                if let Some(error) = outcome.error {
                    return CameraCommandResult::failure(
                        request.request_id,
                        request.correlation_id,
                        degraded_status(),
                        error,
                    );
                }

                CameraCommandResult::failure(
                    request.request_id,
                    request.correlation_id,
                    degraded_status(),
                    NormalizedErrorEnvelope::camera_unavailable(
                        "camera.sidecar.empty_response",
                        "Camera helper returned no readiness result.",
                        None,
                    ),
                )
            }
        }
    }

    pub fn get_capture_confidence_snapshot(&self, session_id: &str) -> CaptureConfidenceSnapshot {
        CaptureConfidenceSnapshot {
            session_id: session_id.into(),
            revision: 0,
            updated_at: now_iso(),
            shoot_ends_at: (Utc::now() + Duration::minutes(50))
                .to_rfc3339_opts(SecondsFormat::Millis, true),
            active_preset: ActivePresetDto {
                preset_id: "preset-soft-noir".into(),
                label: "Soft Noir".into(),
            },
            latest_photo: LatestPhotoState::Empty,
        }
    }
}

pub fn build_capture_confidence_snapshot(
    manifest: &SessionManifest,
    updated_at: &str,
) -> CaptureConfidenceSnapshot {
    let active_preset = manifest
        .active_preset
        .as_ref()
        .map(|preset| ActivePresetDto {
            preset_id: preset.preset_id.clone(),
            label: preset.display_name.clone(),
        })
        .unwrap_or_else(|| ActivePresetDto {
            preset_id: "warm-tone".into(),
            label: "웜톤".into(),
        });

    CaptureConfidenceSnapshot {
        session_id: manifest.session_id.clone(),
        revision: manifest.capture_revision,
        updated_at: updated_at.into(),
        shoot_ends_at: manifest.timing.actual_shoot_end_at.clone(),
        active_preset,
        latest_photo: resolve_latest_photo_state(manifest),
    }
}

fn resolve_latest_photo_state(manifest: &SessionManifest) -> LatestPhotoState {
    let Some(latest_capture_id) = manifest.latest_capture_id.as_deref() else {
        return LatestPhotoState::Empty;
    };

    let Some((index, capture)) = manifest
        .captures
        .iter()
        .enumerate()
        .find(|(_, capture)| capture.capture_id == latest_capture_id)
    else {
        return LatestPhotoState::Empty;
    };

    let processed_path = normalize_path(Path::new(&manifest.processed_dir).join(&capture.processed_file_name));
    let session_root = normalize_path(&manifest.session_dir);
    let processed_root = normalize_path(&manifest.processed_dir);

    if !processed_path.starts_with(&session_root) || !processed_path.starts_with(&processed_root) {
        return LatestPhotoState::Empty;
    }

    if !processed_path.exists() {
        return LatestPhotoState::Empty;
    }

    LatestPhotoState::Ready {
        photo: crate::contracts::dto::LatestSessionPhoto {
            session_id: manifest.session_id.clone(),
            capture_id: capture.capture_id.clone(),
            sequence: (index + 1) as u32,
            asset_url: processed_path.to_string_lossy().replace('\\', "/"),
            captured_at: capture.captured_at.clone(),
        },
    }
}

fn normalize_path(path: impl AsRef<Path>) -> PathBuf {
    let mut normalized = PathBuf::new();

    for component in path.as_ref().components() {
        match component {
            Component::ParentDir => {
                normalized.pop();
            }
            Component::CurDir => {}
            other => normalized.push(other.as_os_str()),
        }
    }

    normalized
}

fn now_iso() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true)
}

fn degraded_status() -> CameraStatusSnapshot {
    CameraStatusSnapshot {
        connection_state: CameraConnectionState::Reconnecting,
        readiness: CameraReadiness::Degraded,
        last_updated_at: now_iso(),
    }
}

pub(crate) fn build_snapshot_request(session_id: &str) -> CameraReadinessRequest {
    CameraReadinessRequest {
        schema_version: crate::contracts::schema_version::PROTOCOL_SCHEMA_VERSION.into(),
        request_id: format!("snapshot-{session_id}"),
        correlation_id: session_id.into(),
        method: "camera.checkReadiness".into(),
        session_id: Some(session_id.into()),
        payload: crate::contracts::dto::CameraReadinessPayload {
            desired_camera_id: None,
            mock_scenario: None,
        },
    }
}

pub(crate) fn map_readiness_status(
    session_id: &str,
    status: &CameraStatusSnapshot,
    error: Option<&NormalizedErrorEnvelope>,
) -> CameraReadinessStatus {
    let is_ready = matches!(status.readiness, CameraReadiness::Ready)
        && matches!(status.connection_state, CameraConnectionState::Connected)
        && error.is_none();

    let connection_state = if is_ready {
        CustomerReadinessConnectionState::Ready
    } else if matches!(status.readiness, CameraReadiness::Pending)
        && matches!(status.connection_state, CameraConnectionState::Offline)
    {
        CustomerReadinessConnectionState::Preparing
    } else if let Some(error) = error {
        map_error_connection_state(error)
    } else {
        map_status_connection_state(status)
    };

    validated_readiness_status(CameraReadinessStatus {
        session_id: session_id.into(),
        connection_state,
        capture_enabled: is_ready,
        last_stable_customer_state: if is_ready {
            Some(crate::contracts::dto::LastSafeCustomerState::Ready)
        } else {
            None
        },
        error: error.cloned(),
        emitted_at: status.last_updated_at.clone(),
    })
}

pub(crate) fn unavailable_readiness_status(
    session_id: &str,
    code: &str,
    message: &str,
    details: Option<String>,
) -> CameraReadinessStatus {
    validated_readiness_status(CameraReadinessStatus {
        session_id: session_id.into(),
        connection_state: CustomerReadinessConnectionState::PhoneRequired,
        capture_enabled: false,
        last_stable_customer_state: None,
        error: Some(NormalizedErrorEnvelope::camera_unavailable(code, message, details)),
        emitted_at: now_iso(),
    })
}

pub(crate) fn error_readiness_status(
    session_id: &str,
    error: &NormalizedErrorEnvelope,
    emitted_at: String,
) -> CameraReadinessStatus {
    validated_readiness_status(CameraReadinessStatus {
        session_id: session_id.into(),
        connection_state: map_error_connection_state(error),
        capture_enabled: false,
        last_stable_customer_state: None,
        error: Some(error.clone()),
        emitted_at,
    })
}

fn map_error_connection_state(error: &NormalizedErrorEnvelope) -> CustomerReadinessConnectionState {
    match error.customer_state {
        crate::contracts::dto::CustomerState::CameraUnavailable => {
            CustomerReadinessConnectionState::PhoneRequired
        }
        crate::contracts::dto::CustomerState::CameraReconnectNeeded => {
            CustomerReadinessConnectionState::Waiting
        }
    }
}

fn map_status_connection_state(status: &CameraStatusSnapshot) -> CustomerReadinessConnectionState {
    match status.connection_state {
        CameraConnectionState::Connected | CameraConnectionState::Reconnecting => {
            CustomerReadinessConnectionState::Waiting
        }
        CameraConnectionState::Disconnected | CameraConnectionState::Offline => {
            CustomerReadinessConnectionState::PhoneRequired
        }
    }
}

fn validated_readiness_status(status: CameraReadinessStatus) -> CameraReadinessStatus {
    match status.validate() {
        Ok(()) => status,
        Err(error) => CameraReadinessStatus {
            session_id: status.session_id,
            connection_state: CustomerReadinessConnectionState::PhoneRequired,
            capture_enabled: false,
            last_stable_customer_state: None,
            error: Some(error),
            emitted_at: status.emitted_at,
        },
    }
}

#[derive(Default)]
pub struct VecProgressSink {
    pub events: Vec<CameraStatusChangedEvent>,
}

impl RecordingProgressSink for VecProgressSink {
    fn record_status(&mut self, event: CameraStatusChangedEvent) {
        self.events.push(event);
    }
}

#[cfg(test)]
mod tests {
    use super::{map_readiness_status, CameraHost, CameraHostConfig};
    use crate::contracts::dto::{
        CameraConnectionState, CameraReadiness, CameraStatusSnapshot,
        CustomerReadinessConnectionState, LastSafeCustomerState, NormalizedErrorEnvelope,
    };

    #[test]
    fn readiness_snapshot_uses_the_sidecar_result_instead_of_a_hardcoded_preparing_state() {
        let host = CameraHost::new(CameraHostConfig::default());

        let snapshot = host.get_readiness_snapshot("session-001");

        assert_eq!(
            snapshot.connection_state,
            CustomerReadinessConnectionState::Ready
        );
        assert!(snapshot.capture_enabled);
        assert_eq!(
            snapshot.last_stable_customer_state,
            Some(LastSafeCustomerState::Ready)
        );
        assert!(snapshot.error.is_none());
    }

    #[test]
    fn readiness_mapping_uses_waiting_for_retryable_degraded_states_and_keeps_capture_blocked() {
        let snapshot = map_readiness_status(
            "session-001",
            &CameraStatusSnapshot {
                connection_state: CameraConnectionState::Reconnecting,
                readiness: CameraReadiness::Degraded,
                last_updated_at: "2026-03-13T00:00:00.000Z".into(),
            },
            Some(&NormalizedErrorEnvelope::camera_reconnect_needed(
                "camera.reconnecting",
                "Camera connection is unstable.",
                None,
            )),
        );

        assert_eq!(
            snapshot.connection_state,
            CustomerReadinessConnectionState::Waiting
        );
        assert!(!snapshot.capture_enabled);
        assert_eq!(snapshot.last_stable_customer_state, None);
    }

    #[test]
    fn readiness_mapping_uses_phone_required_for_non_retryable_unavailable_states() {
        let snapshot = map_readiness_status(
            "session-001",
            &CameraStatusSnapshot {
                connection_state: CameraConnectionState::Offline,
                readiness: CameraReadiness::Degraded,
                last_updated_at: "2026-03-13T00:00:00.000Z".into(),
            },
            Some(&NormalizedErrorEnvelope::camera_unavailable(
                "camera.unavailable",
                "Camera helper stopped responding.",
                None,
            )),
        );

        assert_eq!(
            snapshot.connection_state,
            CustomerReadinessConnectionState::PhoneRequired
        );
        assert!(!snapshot.capture_enabled);
        assert_eq!(snapshot.last_stable_customer_state, None);
    }

    #[test]
    fn readiness_mapping_escalates_offline_degraded_states_without_an_error_envelope() {
        let snapshot = map_readiness_status(
            "session-001",
            &CameraStatusSnapshot {
                connection_state: CameraConnectionState::Offline,
                readiness: CameraReadiness::Degraded,
                last_updated_at: "2026-03-13T00:00:00.000Z".into(),
            },
            None,
        );

        assert_eq!(
            snapshot.connection_state,
            CustomerReadinessConnectionState::PhoneRequired
        );
        assert!(!snapshot.capture_enabled);
        assert_eq!(snapshot.last_stable_customer_state, None);
    }
}
