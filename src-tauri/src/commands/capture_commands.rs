use crate::{
    capture::{
        camera_host::{
            build_capture_confidence_snapshot, build_snapshot_request, error_readiness_status,
            map_readiness_status, unavailable_readiness_status, CameraHost, CameraHostConfig,
            RecordingProgressSink,
        },
        sidecar_client::{
            run_mock_capture_sidecar, watch_mock_readiness_sidecar, SidecarCaptureOutcome,
            SidecarReadinessWatchMessage,
        },
    },
    contracts::{
        dto::{
            CameraCommandResult, CameraReadinessRequest, CameraReadinessStatus,
            CaptureCommandRequest, CaptureCommandResult, CaptureConfidenceSnapshot,
            CaptureProgressEvent, CaptureProgressPayload, CaptureProgressStage, DeleteSessionPhotoRequest,
            DeleteSessionPhotoResponse, SessionGalleryRequest, SessionGallerySnapshot,
        },
        error_envelope::HostErrorEnvelope,
        schema_version::{CONTRACT_SCHEMA_VERSION, PROTOCOL_SCHEMA_VERSION},
    },
    export::thumbnail_guard::ThumbnailGuard,
    session::{
        session_paths::{resolve_booth_session_root, resolve_session_paths},
        session_repository::{append_session_capture, SessionRepository},
    },
};
use std::{
    collections::HashMap,
    path::Path,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    thread,
    time::Duration,
};
use tauri::{ipc::Channel, AppHandle, Manager, Runtime, State};

const WATCH_POLL_INTERVAL: Duration = Duration::from_millis(750);
const MAX_CAPTURE_CONFIDENCE_WATCH_FAILURES: u8 = 3;

#[derive(Clone, Default)]
pub struct CameraWatchRegistry {
    readiness: Arc<Mutex<HashMap<String, Arc<AtomicBool>>>>,
    capture_confidence: Arc<Mutex<HashMap<String, Arc<AtomicBool>>>>,
}

impl CameraWatchRegistry {
    fn register_readiness(&self, watch_id: &str) -> Arc<AtomicBool> {
        register_watch_flag(&self.readiness, watch_id)
    }

    fn cancel_readiness(&self, watch_id: &str) {
        cancel_watch_flag(&self.readiness, watch_id);
    }

    fn finish_readiness(&self, watch_id: &str) {
        finish_watch_flag(&self.readiness, watch_id);
    }

    fn register_capture_confidence(&self, watch_id: &str) -> Arc<AtomicBool> {
        register_watch_flag(&self.capture_confidence, watch_id)
    }

    fn cancel_capture_confidence(&self, watch_id: &str) {
        cancel_watch_flag(&self.capture_confidence, watch_id);
    }

    fn finish_capture_confidence(&self, watch_id: &str) {
        finish_watch_flag(&self.capture_confidence, watch_id);
    }
}

fn register_watch_flag(
    watch_map: &Arc<Mutex<HashMap<String, Arc<AtomicBool>>>>,
    watch_id: &str,
) -> Arc<AtomicBool> {
    let flag = Arc::new(AtomicBool::new(false));

    if let Some(previous) = watch_map
        .lock()
        .expect("camera watch registry lock should not be poisoned")
        .insert(watch_id.into(), flag.clone())
    {
        previous.store(true, Ordering::Relaxed);
    }

    flag
}

fn cancel_watch_flag(
    watch_map: &Arc<Mutex<HashMap<String, Arc<AtomicBool>>>>,
    watch_id: &str,
) {
    if let Some(flag) = watch_map
        .lock()
        .expect("camera watch registry lock should not be poisoned")
        .remove(watch_id)
    {
        flag.store(true, Ordering::Relaxed);
    }
}

fn finish_watch_flag(
    watch_map: &Arc<Mutex<HashMap<String, Arc<AtomicBool>>>>,
    watch_id: &str,
) {
    watch_map
        .lock()
        .expect("camera watch registry lock should not be poisoned")
        .remove(watch_id);
}

#[tauri::command]
pub fn load_session_gallery(
    request: SessionGalleryRequest,
) -> Result<SessionGallerySnapshot, HostErrorEnvelope> {
    let guard = ThumbnailGuard::new(SessionRepository::new());
    guard.load_session_gallery(request)
}

#[tauri::command]
pub fn delete_session_photo(
    request: DeleteSessionPhotoRequest,
) -> Result<DeleteSessionPhotoResponse, HostErrorEnvelope> {
    let guard = ThumbnailGuard::new(SessionRepository::new());
    guard.delete_session_capture(request)
}

#[tauri::command]
pub fn camera_run_readiness_flow(
    payload: CameraReadinessRequest,
    channel: Channel<crate::contracts::dto::CameraStatusChangedEvent>,
) -> CameraCommandResult {
    let host = CameraHost::new(CameraHostConfig::default());
    let mut sink = TauriProgressSink { channel };
    host.run_readiness_flow(payload, &mut sink)
}

#[tauri::command]
pub fn get_camera_readiness_snapshot(session_id: String) -> CameraReadinessStatus {
    CameraHost::new(CameraHostConfig::default()).get_readiness_snapshot(&session_id)
}

#[tauri::command]
pub fn watch_camera_readiness(
    session_id: String,
    watch_id: String,
    status_channel: Channel<crate::contracts::dto::CameraReadinessStatus>,
    watch_registry: State<'_, CameraWatchRegistry>,
) {
    let cancellation = watch_registry.register_readiness(&watch_id);
    let registry = watch_registry.inner().clone();

    thread::spawn(move || loop {
        let mut watch = match watch_mock_readiness_sidecar(&CameraHostConfig::default().sidecar, &build_snapshot_request(&session_id))
        {
            Ok(watch) => watch,
            Err(error) => {
                let _ = status_channel.send(unavailable_readiness_status(
                    &session_id,
                    "camera.sidecar.unavailable",
                    "Camera helper could not be reached.",
                    Some(error),
                ));
                registry.finish_readiness(&watch_id);
                return;
            }
        };

        loop {
            if cancellation.load(Ordering::Relaxed) {
                break;
            }

            let snapshot = match watch.next_message() {
                Ok(Some(SidecarReadinessWatchMessage::Success(status))) => {
                    map_readiness_status(&session_id, &status, None)
                }
                Ok(Some(SidecarReadinessWatchMessage::Error(error))) => {
                    error_readiness_status(&session_id, &error, chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true))
                }
                Ok(None) => break,
                Err(error) => {
                    let _ = status_channel.send(unavailable_readiness_status(
                        &session_id,
                        "camera.sidecar.unavailable",
                        "Camera helper could not be reached.",
                        Some(error),
                    ));
                    break;
                }
            };

            if status_channel.send(snapshot).is_err() {
                break;
            }
        }

        let _ = watch.stop();
        registry.finish_readiness(&watch_id);
        break;
    });
}

#[tauri::command]
pub fn unwatch_camera_readiness(watch_id: String, watch_registry: State<'_, CameraWatchRegistry>) {
    watch_registry.cancel_readiness(&watch_id);
}

#[tauri::command]
pub fn get_capture_confidence_snapshot<R: Runtime>(
    app_handle: AppHandle<R>,
    session_id: String,
) -> Result<CaptureConfidenceSnapshot, HostErrorEnvelope> {
    let manifest = load_session_manifest(&app_handle, &session_id)?;

    Ok(build_capture_confidence_snapshot(
        &manifest,
        &manifest.timing.last_timing_update_at,
    ))
}

#[tauri::command]
pub fn watch_capture_confidence<R: Runtime>(
    app_handle: AppHandle<R>,
    session_id: String,
    watch_id: String,
    capture_channel: Channel<CaptureConfidenceSnapshot>,
    watch_registry: State<'_, CameraWatchRegistry>,
) -> Result<(), HostErrorEnvelope> {
    load_capture_confidence_snapshot(&app_handle, &session_id)?;

    let mut consecutive_failures = 0_u8;
    let cancellation = watch_registry.register_capture_confidence(&watch_id);
    let registry = watch_registry.inner().clone();
    thread::spawn(move || loop {
        if cancellation.load(Ordering::Relaxed) {
            registry.finish_capture_confidence(&watch_id);
            break;
        }

        let snapshot = match load_capture_confidence_snapshot(&app_handle, &session_id) {
            Ok(snapshot) => {
                consecutive_failures = next_capture_confidence_watch_failures(consecutive_failures, true);
                snapshot
            }
            Err(error) => {
                consecutive_failures = next_capture_confidence_watch_failures(consecutive_failures, false);
                if consecutive_failures == MAX_CAPTURE_CONFIDENCE_WATCH_FAILURES {
                    eprintln!(
                        "capture confidence watch is retrying after repeated snapshot read failures for session {session_id}: {}",
                        error.message
                    );
                }

                thread::sleep(WATCH_POLL_INTERVAL);
                continue;
            }
        };

        if capture_channel.send(snapshot).is_err() {
            registry.finish_capture_confidence(&watch_id);
            break;
        }

        thread::sleep(WATCH_POLL_INTERVAL);
    });

    Ok(())
}

#[tauri::command]
pub fn unwatch_capture_confidence(
    watch_id: String,
    watch_registry: State<'_, CameraWatchRegistry>,
) {
    watch_registry.cancel_capture_confidence(&watch_id);
}

#[tauri::command]
pub fn request_capture<R: Runtime>(
    app_handle: AppHandle<R>,
    payload: CaptureCommandRequest,
    channel: Channel<CaptureProgressEvent>,
) -> Result<CaptureCommandResult, HostErrorEnvelope> {
    if let Err(error) = payload.validate() {
        return Err(HostErrorEnvelope::invalid_payload(error.message));
    }

    let manifest_path = resolve_manifest_path(&app_handle, &payload.session_id)?;
    let manifest = SessionRepository::new().load_manifest(&manifest_path)?;
    let active_preset = validate_capture_request_active_preset(&payload, &manifest)?;
    let capture_id = next_capture_id(&manifest.captures);
    let original_file_name = format!("originals/{capture_id}.nef");
    let processed_file_name = format!("{capture_id}.png");
    let original_output_path = Path::new(&manifest.session_dir).join(&original_file_name);
    let processed_output_path = Path::new(&manifest.processed_dir).join(&processed_file_name);
    let sidecar_capture = validate_sidecar_capture_success(
        &payload,
        &manifest_path,
        &capture_id,
        &original_file_name,
        &processed_file_name,
        run_mock_capture_sidecar(
            &CameraHostConfig::default().sidecar,
            &payload,
            &capture_id,
            &original_file_name,
            &processed_file_name,
            &original_output_path,
            &processed_output_path,
        )
        .map_err(HostErrorEnvelope::provisioning_failed)?,
    )?;

    for event in &sidecar_capture.progress_events {
        if !matches!(event.payload.stage, CaptureProgressStage::CaptureStarted) {
            continue;
        }

        let _ = channel.send(rebase_capture_progress_event(
            &payload,
            event,
            &sidecar_capture.capture_id,
            &event.payload.last_updated_at,
        ));
    }

    let persisted_capture = append_session_capture(
        &manifest_path,
        &payload.session_id,
        active_preset,
        &sidecar_capture.capture_id,
        &sidecar_capture.original_file_name,
        &sidecar_capture.processed_file_name,
        &sidecar_capture.captured_at,
    )?;

    for event in &sidecar_capture.progress_events {
        if !matches!(event.payload.stage, CaptureProgressStage::CaptureCompleted) {
            continue;
        }

        let _ = channel.send(rebase_capture_progress_event(
            &payload,
            event,
            &sidecar_capture.capture_id,
            &sidecar_capture.captured_at,
        ));
    }

    Ok(CaptureCommandResult {
        schema_version: CONTRACT_SCHEMA_VERSION.into(),
        request_id: payload.request_id,
        correlation_id: payload.correlation_id,
        ok: true,
        session_id: persisted_capture.session_id,
        capture_id: persisted_capture.capture_id,
        original_file_name: persisted_capture.original_file_name,
        processed_file_name: persisted_capture.processed_file_name,
        captured_at: persisted_capture.captured_at,
        manifest_path: persisted_capture.manifest_path,
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ValidatedSidecarCapture {
    capture_id: String,
    original_file_name: String,
    processed_file_name: String,
    captured_at: String,
    progress_events: Vec<CaptureProgressEvent>,
}

fn validate_sidecar_capture_success(
    payload: &CaptureCommandRequest,
    manifest_path: &std::path::Path,
    expected_capture_id: &str,
    expected_original_file_name: &str,
    expected_processed_file_name: &str,
    sidecar_outcome: SidecarCaptureOutcome,
) -> Result<ValidatedSidecarCapture, HostErrorEnvelope> {
    let SidecarCaptureOutcome {
        progress_events,
        success,
        error,
    } = sidecar_outcome;

    if let Some(error) = error {
        return Err(HostErrorEnvelope::provisioning_failed(error.message));
    }

    let success = success.ok_or_else(|| {
        HostErrorEnvelope::provisioning_failed("Camera helper returned no capture result.")
    })?;

    if success.session_id != payload.session_id {
        return Err(HostErrorEnvelope::provisioning_failed(
            "Camera helper returned a capture for an unexpected session.",
        ));
    }

    let expected_manifest_path = normalize_wire_path(manifest_path.to_string_lossy().as_ref());
    if normalize_wire_path(&success.manifest_path) != expected_manifest_path {
        return Err(HostErrorEnvelope::provisioning_failed(
            "Camera helper returned an unexpected manifest path.",
        ));
    }

    if success.capture_id != expected_capture_id
        || success.original_file_name != expected_original_file_name
        || success.processed_file_name != expected_processed_file_name
    {
        return Err(HostErrorEnvelope::provisioning_failed(
            "Camera helper returned an unexpected capture identity.",
        ));
    }

    Ok(ValidatedSidecarCapture {
        capture_id: success.capture_id,
        original_file_name: success.original_file_name,
        processed_file_name: success.processed_file_name,
        captured_at: success.captured_at,
        progress_events,
    })
}

fn next_capture_id(captures: &[crate::session::session_manifest::ManifestCaptureRecord]) -> String {
    let next_sequence = captures
        .iter()
        .filter_map(|capture| {
            capture
                .capture_id
                .strip_prefix("capture-")
                .and_then(|suffix| suffix.parse::<u32>().ok())
        })
        .max()
        .unwrap_or(0)
        .saturating_add(1);

    format!("capture-{next_sequence:03}")
}

fn next_capture_confidence_watch_failures(previous: u8, load_succeeded: bool) -> u8 {
    if load_succeeded {
        0
    } else {
        previous.saturating_add(1).min(MAX_CAPTURE_CONFIDENCE_WATCH_FAILURES)
    }
}

fn validate_capture_request_active_preset(
    payload: &CaptureCommandRequest,
    manifest: &crate::session::session_manifest::SessionManifest,
) -> Result<crate::session::session_manifest::SessionActivePresetSelection, HostErrorEnvelope> {
    let manifest_active_preset = manifest.active_preset.clone().ok_or_else(|| {
        HostErrorEnvelope::session_manifest_invalid(
            "Active session manifest is missing the active preset required for capture.",
        )
    })?;

    if manifest
        .active_preset_name
        .as_deref()
        .is_some_and(|display_name| display_name != manifest_active_preset.display_name)
    {
        return Err(HostErrorEnvelope::session_manifest_invalid(
            "Active session manifest preset fields are out of sync.",
        ));
    }

    if payload.payload.active_preset.preset_id != manifest_active_preset.preset_id
        || payload.payload.active_preset.label != manifest_active_preset.display_name
    {
        return Err(HostErrorEnvelope::invalid_payload(
            "Capture request preset does not match the active session preset.",
        ));
    }

    Ok(manifest_active_preset)
}

fn normalize_wire_path(path: &str) -> String {
    path.replace('\\', "/")
}

fn rebase_capture_progress_event(
    payload: &CaptureCommandRequest,
    event: &CaptureProgressEvent,
    capture_id: &str,
    last_updated_at: &str,
) -> CaptureProgressEvent {
    CaptureProgressEvent {
        schema_version: PROTOCOL_SCHEMA_VERSION.into(),
        request_id: payload.request_id.clone(),
        correlation_id: payload.correlation_id.clone(),
        event: "capture.progress".into(),
        session_id: Some(payload.session_id.clone()),
        payload: CaptureProgressPayload {
            stage: event.payload.stage.clone(),
            capture_id: capture_id.into(),
            percent_complete: event.payload.percent_complete,
            last_updated_at: last_updated_at.into(),
        },
    }
}

struct TauriProgressSink {
    channel: Channel<crate::contracts::dto::CameraStatusChangedEvent>,
}

impl RecordingProgressSink for TauriProgressSink {
    fn record_status(&mut self, event: crate::contracts::dto::CameraStatusChangedEvent) {
        let _ = self.channel.send(event);
    }
}

fn load_session_manifest<R: Runtime>(
    app_handle: &AppHandle<R>,
    session_id: &str,
) -> Result<crate::session::session_manifest::SessionManifest, HostErrorEnvelope> {
    let (operational_date, session_name) = session_id
        .split_once(':')
        .ok_or_else(|| HostErrorEnvelope::invalid_payload("Invalid sessionId format"))?;

    let app_local_data_dir = app_handle
        .path()
        .app_local_data_dir()
        .map_err(|error| HostErrorEnvelope::provisioning_failed(format!(
            "failed to resolve app-local session root: {error}"
        )))?;
    let session_root = resolve_booth_session_root(&app_local_data_dir);
    let manifest_path = resolve_session_paths(
        &session_root,
        std::path::Path::new(operational_date).join(session_name),
    )
    .manifest_path;

    SessionRepository::new().load_manifest(manifest_path)
}

fn load_capture_confidence_snapshot<R: Runtime>(
    app_handle: &AppHandle<R>,
    session_id: &str,
) -> Result<CaptureConfidenceSnapshot, HostErrorEnvelope> {
    let manifest = load_session_manifest(app_handle, session_id)?;

    Ok(build_capture_confidence_snapshot(
        &manifest,
        &manifest.timing.last_timing_update_at,
    ))
}

fn resolve_manifest_path<R: Runtime>(
    app_handle: &AppHandle<R>,
    session_id: &str,
) -> Result<std::path::PathBuf, HostErrorEnvelope> {
    let (operational_date, session_name) = session_id
        .split_once(':')
        .ok_or_else(|| HostErrorEnvelope::invalid_payload("Invalid sessionId format"))?;

    let app_local_data_dir = app_handle
        .path()
        .app_local_data_dir()
        .map_err(|error| HostErrorEnvelope::provisioning_failed(format!(
            "failed to resolve app-local session root: {error}"
        )))?;
    let session_root = resolve_booth_session_root(&app_local_data_dir);

    Ok(resolve_session_paths(
        &session_root,
        std::path::Path::new(operational_date).join(session_name),
    )
    .manifest_path)
}

#[cfg(test)]
mod tests {
    use super::{
        next_capture_confidence_watch_failures, next_capture_id, rebase_capture_progress_event,
        validate_capture_request_active_preset, validate_sidecar_capture_success,
    };
    use crate::{
        capture::sidecar_client::SidecarCaptureOutcome,
        contracts::{
            dto::{
                CaptureActivePresetDto, CaptureCommandPayload, CaptureCommandRequest,
                CaptureProgressEvent, CaptureProgressPayload, CaptureProgressStage,
                SidecarCaptureSuccessResponse,
            },
            error_envelope::HostErrorCode,
            schema_version::{CONTRACT_SCHEMA_VERSION, PROTOCOL_SCHEMA_VERSION},
        },
        session::session_manifest::{
            ManifestCaptureRecord, SessionActivePresetSelection, SessionManifest, SessionTiming,
        },
    };
    use std::path::Path;

    #[test]
    fn validate_sidecar_capture_success_uses_helper_result_as_truth() {
        let payload = build_capture_request("2026-03-13:kim");
        let sidecar_outcome = SidecarCaptureOutcome {
            progress_events: vec![build_progress_event(CaptureProgressStage::CaptureStarted, "capture-sidecar", 10)],
            success: Some(SidecarCaptureSuccessResponse {
                schema_version: CONTRACT_SCHEMA_VERSION.into(),
                request_id: payload.request_id.clone(),
                correlation_id: payload.correlation_id.clone(),
                ok: true,
                session_id: payload.session_id.clone(),
                capture_id: "capture-sidecar".into(),
                original_file_name: "originals/capture-sidecar.nef".into(),
                processed_file_name: "capture-sidecar.png".into(),
                captured_at: "2026-03-13T10:07:02.000Z".into(),
                manifest_path: "C:/sessions/kim/session.json".into(),
            }),
            error: None,
        };

        let validated =
            validate_sidecar_capture_success(
                &payload,
                Path::new("C:/sessions/kim/session.json"),
                "capture-sidecar",
                "originals/capture-sidecar.nef",
                "capture-sidecar.png",
                sidecar_outcome,
            )
            .expect("expected helper capture to validate");

        assert_eq!(validated.capture_id, "capture-sidecar");
        assert_eq!(validated.original_file_name, "originals/capture-sidecar.nef");
        assert_eq!(validated.processed_file_name, "capture-sidecar.png");
        assert_eq!(validated.captured_at, "2026-03-13T10:07:02.000Z");
        assert_eq!(validated.progress_events.len(), 1);
    }

    #[test]
    fn validate_sidecar_capture_success_rejects_manifest_path_drift() {
        let payload = build_capture_request("2026-03-13:kim");
        let sidecar_outcome = SidecarCaptureOutcome {
            progress_events: vec![],
            success: Some(SidecarCaptureSuccessResponse {
                schema_version: CONTRACT_SCHEMA_VERSION.into(),
                request_id: payload.request_id.clone(),
                correlation_id: payload.correlation_id.clone(),
                ok: true,
                session_id: payload.session_id.clone(),
                capture_id: "capture-sidecar".into(),
                original_file_name: "originals/capture-sidecar.nef".into(),
                processed_file_name: "capture-sidecar.png".into(),
                captured_at: "2026-03-13T10:07:02.000Z".into(),
                manifest_path: "C:/sessions/other/session.json".into(),
            }),
            error: None,
        };

        let error =
            validate_sidecar_capture_success(
                &payload,
                Path::new("C:/sessions/kim/session.json"),
                "capture-sidecar",
                "originals/capture-sidecar.nef",
                "capture-sidecar.png",
                sidecar_outcome,
            )
            .expect_err("expected manifest path drift to fail");

        assert_eq!(error.code, HostErrorCode::ProvisioningFailed);
        assert_eq!(error.message, "Camera helper returned an unexpected manifest path.");
    }

    #[test]
    fn validate_sidecar_capture_success_accepts_windows_manifest_separator_drift() {
        let payload = build_capture_request("2026-03-13:kim");
        let sidecar_outcome = SidecarCaptureOutcome {
            progress_events: vec![],
            success: Some(SidecarCaptureSuccessResponse {
                schema_version: CONTRACT_SCHEMA_VERSION.into(),
                request_id: payload.request_id.clone(),
                correlation_id: payload.correlation_id.clone(),
                ok: true,
                session_id: payload.session_id.clone(),
                capture_id: "capture-sidecar".into(),
                original_file_name: "originals/capture-sidecar.nef".into(),
                processed_file_name: "capture-sidecar.png".into(),
                captured_at: "2026-03-13T10:07:02.000Z".into(),
                manifest_path: "C:\\sessions\\kim\\session.json".into(),
            }),
            error: None,
        };

        let validated = validate_sidecar_capture_success(
            &payload,
            Path::new("C:/sessions/kim/session.json"),
            "capture-sidecar",
            "originals/capture-sidecar.nef",
            "capture-sidecar.png",
            sidecar_outcome,
        )
        .expect("expected separator-only drift to validate");

        assert_eq!(validated.capture_id, "capture-sidecar");
    }

    #[test]
    fn validate_sidecar_capture_success_rejects_capture_identity_drift() {
        let payload = build_capture_request("2026-03-13:kim");
        let sidecar_outcome = SidecarCaptureOutcome {
            progress_events: vec![],
            success: Some(SidecarCaptureSuccessResponse {
                schema_version: CONTRACT_SCHEMA_VERSION.into(),
                request_id: payload.request_id.clone(),
                correlation_id: payload.correlation_id.clone(),
                ok: true,
                session_id: payload.session_id.clone(),
                capture_id: "capture-999".into(),
                original_file_name: "originals/capture-999.nef".into(),
                processed_file_name: "capture-999.png".into(),
                captured_at: "2026-03-13T10:07:02.000Z".into(),
                manifest_path: "C:/sessions/kim/session.json".into(),
            }),
            error: None,
        };

        let error =
            validate_sidecar_capture_success(
                &payload,
                Path::new("C:/sessions/kim/session.json"),
                "capture-001",
                "originals/capture-001.nef",
                "capture-001.png",
                sidecar_outcome,
            )
            .expect_err("expected helper capture drift to fail");

        assert_eq!(error.code, HostErrorCode::ProvisioningFailed);
        assert_eq!(error.message, "Camera helper returned an unexpected capture identity.");
    }

    #[test]
    fn capture_confidence_watch_failure_budget_recovers_after_a_successful_read() {
        assert_eq!(next_capture_confidence_watch_failures(0, false), 1);
        assert_eq!(next_capture_confidence_watch_failures(1, false), 2);
        assert_eq!(next_capture_confidence_watch_failures(2, true), 0);
    }

    #[test]
    fn capture_confidence_watch_failure_budget_saturates_instead_of_requesting_termination() {
        assert_eq!(next_capture_confidence_watch_failures(2, false), 3);
        assert_eq!(next_capture_confidence_watch_failures(3, false), 3);
        assert_eq!(next_capture_confidence_watch_failures(9, false), 3);
    }

    #[test]
    fn validate_capture_request_active_preset_rejects_stale_payload_truth() {
        let payload = build_capture_request("2026-03-13:kim");
        let mut manifest = build_manifest_with_active_preset("background-warm", "웜톤");
        manifest.active_preset_name = Some("웜톤".into());

        let error = validate_capture_request_active_preset(&payload, &manifest)
            .expect_err("expected stale payload preset to be rejected");

        assert_eq!(error.code, HostErrorCode::InvalidPayload);
        assert_eq!(
            error.message,
            "Capture request preset does not match the active session preset."
        );
    }

    #[test]
    fn validate_capture_request_active_preset_accepts_matching_manifest_truth() {
        let payload = build_capture_request("2026-03-13:kim");
        let manifest = build_manifest_with_active_preset("background-pink", "배경지 - 핑크");

        let active_preset = validate_capture_request_active_preset(&payload, &manifest)
            .expect("expected matching preset truth to validate");

        assert_eq!(active_preset.preset_id, "background-pink");
        assert_eq!(active_preset.display_name, "배경지 - 핑크");
    }

    #[test]
    fn rebase_capture_progress_event_keeps_host_request_identity_and_sidecar_stage() {
        let payload = build_capture_request("2026-03-13:kim");
        let event = build_progress_event(CaptureProgressStage::CaptureCompleted, "capture-sidecar", 100);

        let rebased =
            rebase_capture_progress_event(&payload, &event, "capture-007", "2026-03-13T10:07:02.000Z");

        assert_eq!(rebased.schema_version, PROTOCOL_SCHEMA_VERSION);
        assert_eq!(rebased.request_id, payload.request_id);
        assert_eq!(rebased.correlation_id, payload.correlation_id);
        assert_eq!(rebased.session_id, Some(payload.session_id));
        assert_eq!(rebased.payload.stage, CaptureProgressStage::CaptureCompleted);
        assert_eq!(rebased.payload.capture_id, "capture-007");
        assert_eq!(rebased.payload.last_updated_at, "2026-03-13T10:07:02.000Z");
    }

    #[test]
    fn next_capture_id_uses_the_highest_existing_capture_sequence_instead_of_reusing_deleted_slots() {
        let next_id = next_capture_id(&[
            ManifestCaptureRecord {
                capture_id: "capture-001".into(),
                original_file_name: "originals/capture-001.nef".into(),
                processed_file_name: "capture-001.png".into(),
                captured_at: "2026-03-08T09:00:00.000Z".into(),
            },
            ManifestCaptureRecord {
                capture_id: "capture-003".into(),
                original_file_name: "originals/capture-003.nef".into(),
                processed_file_name: "capture-003.png".into(),
                captured_at: "2026-03-08T09:02:00.000Z".into(),
            },
        ]);

        assert_eq!(next_id, "capture-004");
    }

    fn build_capture_request(session_id: &str) -> CaptureCommandRequest {
        CaptureCommandRequest {
            schema_version: CONTRACT_SCHEMA_VERSION.into(),
            request_id: "req-123".into(),
            correlation_id: "corr-456".into(),
            method: "camera.capture".into(),
            session_id: session_id.into(),
            payload: CaptureCommandPayload {
                active_preset: CaptureActivePresetDto {
                    preset_id: "background-pink".into(),
                    label: "배경지 - 핑크".into(),
                },
            },
        }
    }

    fn build_progress_event(stage: CaptureProgressStage, capture_id: &str, percent_complete: u8) -> CaptureProgressEvent {
        CaptureProgressEvent {
            schema_version: PROTOCOL_SCHEMA_VERSION.into(),
            request_id: "req-sidecar".into(),
            correlation_id: "corr-sidecar".into(),
            event: "capture.progress".into(),
            session_id: Some("2026-03-13:kim".into()),
            payload: CaptureProgressPayload {
                stage,
                capture_id: capture_id.into(),
                percent_complete,
                last_updated_at: "2026-03-13T10:07:00.000Z".into(),
            },
        }
    }

    fn build_manifest_with_active_preset(
        preset_id: &str,
        display_name: &str,
    ) -> SessionManifest {
        SessionManifest {
            schema_version: 1,
            session_id: "2026-03-13:kim".into(),
            session_name: "kim".into(),
            operational_date: "2026-03-13".into(),
            created_at: "2026-03-13T10:00:00.000Z".into(),
            session_dir: "C:/sessions/kim".into(),
            manifest_path: "C:/sessions/kim/session.json".into(),
            events_path: "C:/sessions/kim/events.ndjson".into(),
            export_status_path: "C:/sessions/kim/export-status.json".into(),
            processed_dir: "C:/sessions/kim/processed".into(),
            capture_revision: 0,
            latest_capture_id: None,
            active_preset_name: Some(display_name.into()),
            active_preset: Some(SessionActivePresetSelection {
                preset_id: preset_id.into(),
                display_name: display_name.into(),
            }),
            captures: vec![],
            camera_state: crate::session::session_manifest::CameraState {
                connection_state: "connected".into(),
            },
            timing: SessionTiming {
                reservation_start_at: "2026-03-13T10:00:00.000Z".into(),
                actual_shoot_end_at: "2026-03-13T10:50:00.000Z".into(),
                session_type: "standard".into(),
                operator_extension_count: 0,
                last_timing_update_at: "2026-03-13T10:00:00.000Z".into(),
            },
            export_state: crate::session::session_manifest::ExportState {
                status: "notStarted".into(),
            },
        }
    }
}
