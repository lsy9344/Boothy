use std::{
    fs,
    path::Path,
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

use crate::{
    capture::{
        ingest_pipeline::persist_capture_in_dir,
        sidecar_client::{
            map_capture_round_trip_error, read_latest_status_message, wait_for_capture_round_trip,
            write_capture_request_message, CanonHelperCaptureRequestMessage,
            CanonHelperStatusMessage, SidecarClientError,
            CANON_HELPER_CAPTURE_REQUEST_SCHEMA_VERSION,
        },
        CAPTURE_PIPELINE_LOCK, IN_FLIGHT_CAPTURE_SESSIONS,
    },
    contracts::dto::{
        CaptureDeleteInputDto, CaptureDeleteResultDto, CaptureReadinessDto,
        CaptureReadinessInputDto, CaptureRequestInputDto, CaptureRequestResultDto,
        HostErrorEnvelope, LiveCaptureTruthDto,
    },
    handoff::sync_post_end_state_in_dir,
    preset::preset_catalog::{find_published_preset_summary, resolve_published_preset_catalog_dir},
    session::{
        session_manifest::{
            normalize_legacy_manifest, rfc3339_to_unix_seconds, ActivePresetBinding,
            SessionManifest, SESSION_POST_END_COMPLETED, SESSION_POST_END_PHONE_REQUIRED,
        },
        session_paths::SessionPaths,
        session_repository::write_session_manifest,
    },
    timing::{sync_session_timing_in_dir, TimingPhase},
};

const CAMERA_HELPER_STATUS_MAX_AGE_SECONDS: u64 = 5;
static CAPTURE_REQUEST_COUNTER: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LiveCameraGate {
    Ready,
    CameraPreparing,
    HelperPreparing,
    PhoneRequired,
}

#[derive(Debug, Clone)]
struct ProjectedLiveCaptureTruth {
    dto: LiveCaptureTruthDto,
    gate: LiveCameraGate,
}

pub fn get_capture_readiness_in_dir(
    base_dir: &Path,
    input: CaptureReadinessInputDto,
) -> Result<CaptureReadinessDto, HostErrorEnvelope> {
    let manifest = read_session_manifest_with_timing(base_dir, &input.session_id)?;

    Ok(normalize_capture_readiness(base_dir, &manifest))
}

pub fn request_capture_in_dir(
    base_dir: &Path,
    input: CaptureRequestInputDto,
) -> Result<CaptureRequestResultDto, HostErrorEnvelope> {
    let readiness = get_capture_readiness_in_dir(
        base_dir,
        CaptureReadinessInputDto {
            session_id: input.session_id.clone(),
        },
    )?;

    if !readiness.can_capture {
        return Err(HostErrorEnvelope::capture_not_ready(
            "지금은 촬영할 수 없어요.",
            readiness,
        ));
    }

    let _in_flight_guard = acquire_in_flight_capture_guard(base_dir, &input.session_id)?;
    let manifest = read_session_manifest_with_timing(base_dir, &input.session_id)?;
    let active_preset = manifest.active_preset.clone().ok_or_else(|| {
        HostErrorEnvelope::preset_not_available("촬영 전에 룩을 다시 골라 주세요.")
    })?;
    let request_id = generate_capture_request_id();
    let requested_at = crate::session::session_manifest::current_timestamp(SystemTime::now())?;
    let request_message = CanonHelperCaptureRequestMessage {
        schema_version: CANON_HELPER_CAPTURE_REQUEST_SCHEMA_VERSION.into(),
        message_type: "request-capture".into(),
        session_id: input.session_id.clone(),
        request_id: request_id.clone(),
        requested_at,
        active_preset_id: active_preset.preset_id.clone(),
        active_preset_version: active_preset.published_version.clone(),
    };

    write_capture_request_message(base_dir, &request_message)
        .map_err(|error| map_capture_round_trip_error(&input.session_id, error))?;

    let round_trip = wait_for_capture_round_trip(base_dir, &input.session_id, &request_id)
        .map_err(|error| map_capture_round_trip_error(&input.session_id, error))?;
    let (manifest, capture) = persist_capture_in_dir(
        base_dir,
        &input,
        round_trip.capture_id,
        request_id,
        round_trip.raw_path,
        round_trip.capture_accepted_at_ms,
        round_trip.persisted_at_ms,
    )?;

    Ok(CaptureRequestResultDto {
        schema_version: "capture-request-result/v1".into(),
        session_id: input.session_id,
        status: "capture-saved".into(),
        capture: capture.clone(),
        readiness: CaptureReadinessDto::capture_saved(manifest.session_id.clone(), capture)
            .with_timing(manifest.timing.clone())
            .with_live_capture_truth(project_live_capture_truth(base_dir, &manifest).dto),
    })
}

pub fn delete_capture_in_dir(
    base_dir: &Path,
    input: CaptureDeleteInputDto,
) -> Result<CaptureDeleteResultDto, HostErrorEnvelope> {
    let paths = SessionPaths::try_new(base_dir, &input.session_id)?;
    let _pipeline_guard = CAPTURE_PIPELINE_LOCK.lock().map_err(|_| {
        HostErrorEnvelope::persistence("촬영 상태를 잠그지 못했어요. 잠시 후 다시 시도해 주세요.")
    })?;
    let mut manifest = read_session_manifest_with_timing(base_dir, &input.session_id)?;
    let capture_index = manifest
        .captures
        .iter()
        .position(|capture| capture.capture_id == input.capture_id)
        .ok_or_else(|| {
            HostErrorEnvelope::capture_delete_blocked(
                "이 사진은 지금 정리할 수 없어요. 잠시 후 다시 확인해 주세요.",
                normalize_capture_readiness(base_dir, &manifest),
            )
        })?;
    let capture = manifest.captures[capture_index].clone();
    let is_finalized_post_end = matches!(
        manifest.lifecycle.stage.as_str(),
        "completed" | "phone-required"
    );

    if capture.session_id != input.session_id
        || !matches!(
            capture.render_status.as_str(),
            "previewReady" | "finalReady"
        )
        || capture.post_end_state != "activeSession"
        || is_finalized_post_end
    {
        return Err(HostErrorEnvelope::capture_delete_blocked(
            "이 사진은 지금 정리할 수 없어요. 잠시 후 다시 확인해 주세요.",
            normalize_capture_readiness(base_dir, &manifest),
        ));
    }

    let staged_assets = stage_capture_asset_deletions(&paths, &capture)?;

    manifest.captures.remove(capture_index);
    manifest.lifecycle.stage = derive_capture_lifecycle_stage(&manifest);
    manifest.updated_at =
        crate::session::session_manifest::current_timestamp(std::time::SystemTime::now())?;
    if let Err(error) = write_session_manifest(&paths.manifest_path, &manifest) {
        rollback_staged_asset_deletions(&staged_assets);
        return Err(error);
    }
    finalize_staged_asset_deletions(&staged_assets);

    Ok(CaptureDeleteResultDto {
        schema_version: "capture-delete-result/v1".into(),
        session_id: input.session_id,
        capture_id: input.capture_id,
        status: "capture-deleted".into(),
        readiness: normalize_capture_readiness(base_dir, &manifest),
        manifest,
    })
}

pub fn normalize_capture_readiness(
    base_dir: &Path,
    manifest: &SessionManifest,
) -> CaptureReadinessDto {
    let timing = manifest.timing.clone();
    let latest_capture = manifest.captures.last().cloned();
    let timing_phase = timing_phase(timing.as_ref());
    let live_capture_truth = project_live_capture_truth(base_dir, manifest);
    let live_camera_gate = live_capture_truth.gate;
    let post_end = if timing_phase == TimingPhase::Ended
        && matches!(
            manifest.lifecycle.stage.as_str(),
            "export-waiting" | "completed" | "phone-required"
        ) {
        manifest.post_end.clone()
    } else {
        None
    };

    if let Some(post_end_state) = post_end.clone() {
        let readiness = match post_end_state.state() {
            SESSION_POST_END_COMPLETED => {
                CaptureReadinessDto::completed(manifest.session_id.clone(), latest_capture)
            }
            SESSION_POST_END_PHONE_REQUIRED => {
                CaptureReadinessDto::phone_required(manifest.session_id.clone())
            }
            _ => CaptureReadinessDto::export_waiting(manifest.session_id.clone(), latest_capture),
        };

        return with_projected_live_capture_truth(
            readiness
                .with_post_end(Some(post_end_state))
                .with_timing(timing),
            &live_capture_truth,
        );
    }

    if timing_phase == TimingPhase::Ended {
        return with_projected_live_capture_truth(
            match manifest.lifecycle.stage.as_str() {
                "phone-required" | "blocked" => {
                    CaptureReadinessDto::phone_required(manifest.session_id.clone())
                        .with_timing(timing)
                }
                _ => {
                    CaptureReadinessDto::export_waiting(manifest.session_id.clone(), latest_capture)
                        .with_timing(timing)
                }
            },
            &live_capture_truth,
        );
    }

    if !has_valid_active_preset(base_dir, manifest.active_preset.as_ref()) {
        return with_projected_live_capture_truth(
            CaptureReadinessDto::preset_missing(manifest.session_id.clone()).with_timing(timing),
            &live_capture_truth,
        );
    }

    match manifest.lifecycle.stage.as_str() {
        _ if has_in_flight_capture(base_dir) => with_projected_live_capture_truth(
            CaptureReadinessDto::camera_preparing(manifest.session_id.clone())
                .with_latest_capture(latest_capture)
                .with_timing(timing),
            &live_capture_truth,
        ),
        "ready" | "capture-ready" | "preset-selected" => match latest_capture {
            Some(capture)
                if capture.render_status == "previewWaiting"
                    || capture.render_status == "captureSaved" =>
            {
                with_projected_live_capture_truth(
                    CaptureReadinessDto::preview_waiting(
                        manifest.session_id.clone(),
                        Some(capture),
                    )
                    .with_timing(timing),
                    &live_capture_truth,
                )
            }
            Some(capture) if capture.render_status == "renderFailed" => {
                with_projected_live_capture_truth(
                    CaptureReadinessDto::phone_required(manifest.session_id.clone())
                        .with_timing(timing),
                    &live_capture_truth,
                )
            }
            Some(capture) if capture.render_status == "previewReady" => match live_camera_gate {
                LiveCameraGate::Ready => with_projected_live_capture_truth(
                    CaptureReadinessDto::preview_ready(manifest.session_id.clone(), capture)
                        .with_timing(timing),
                    &live_capture_truth,
                ),
                _ => with_projected_live_capture_truth(
                    build_blocked_readiness_from_live_camera_gate(
                        manifest.session_id.clone(),
                        live_camera_gate,
                        &live_capture_truth,
                        Some(capture),
                    )
                    .with_timing(timing),
                    &live_capture_truth,
                ),
            },
            _ if timing_phase == TimingPhase::Warning => match live_camera_gate {
                LiveCameraGate::Ready => with_projected_live_capture_truth(
                    CaptureReadinessDto::warning(manifest.session_id.clone(), latest_capture)
                        .with_timing(timing),
                    &live_capture_truth,
                ),
                _ => with_projected_live_capture_truth(
                    build_blocked_readiness_from_live_camera_gate(
                        manifest.session_id.clone(),
                        live_camera_gate,
                        &live_capture_truth,
                        latest_capture,
                    )
                    .with_timing(timing),
                    &live_capture_truth,
                ),
            },
            _ => match live_camera_gate {
                LiveCameraGate::Ready => with_projected_live_capture_truth(
                    CaptureReadinessDto::ready(
                        manifest.session_id.clone(),
                        "captureReady",
                        latest_capture,
                    )
                    .with_timing(timing),
                    &live_capture_truth,
                ),
                _ => with_projected_live_capture_truth(
                    build_blocked_readiness_from_live_camera_gate(
                        manifest.session_id.clone(),
                        live_camera_gate,
                        &live_capture_truth,
                        latest_capture,
                    )
                    .with_timing(timing),
                    &live_capture_truth,
                ),
            },
        },
        "phone-required" | "blocked" => with_projected_live_capture_truth(
            CaptureReadinessDto::phone_required(manifest.session_id.clone())
                .with_post_end(post_end)
                .with_timing(timing),
            &live_capture_truth,
        ),
        "preview-waiting" => with_projected_live_capture_truth(
            CaptureReadinessDto::preview_waiting(manifest.session_id.clone(), latest_capture)
                .with_timing(timing),
            &live_capture_truth,
        ),
        "export-waiting" => with_projected_live_capture_truth(
            CaptureReadinessDto::export_waiting(manifest.session_id.clone(), latest_capture)
                .with_timing(timing),
            &live_capture_truth,
        ),
        "completed" => with_projected_live_capture_truth(
            CaptureReadinessDto::completed(manifest.session_id.clone(), latest_capture)
                .with_post_end(post_end)
                .with_timing(timing),
            &live_capture_truth,
        ),
        "warning" => with_projected_live_capture_truth(
            CaptureReadinessDto::warning(manifest.session_id.clone(), latest_capture)
                .with_timing(timing),
            &live_capture_truth,
        ),
        "helper-preparing" => with_projected_live_capture_truth(
            CaptureReadinessDto::helper_preparing(manifest.session_id.clone()).with_timing(timing),
            &live_capture_truth,
        ),
        "camera-preparing" | "preparing" => with_projected_live_capture_truth(
            CaptureReadinessDto::camera_preparing(manifest.session_id.clone()).with_timing(timing),
            &live_capture_truth,
        ),
        _ => with_projected_live_capture_truth(
            CaptureReadinessDto::camera_preparing(manifest.session_id.clone()).with_timing(timing),
            &live_capture_truth,
        ),
    }
}

fn has_valid_active_preset(base_dir: &Path, active_preset: Option<&ActivePresetBinding>) -> bool {
    let Some(active_preset) = active_preset else {
        return false;
    };

    let catalog_root = resolve_published_preset_catalog_dir(base_dir);

    find_published_preset_summary(
        &catalog_root,
        &active_preset.preset_id,
        &active_preset.published_version,
    )
    .is_some()
}

fn project_live_capture_truth(
    base_dir: &Path,
    manifest: &SessionManifest,
) -> ProjectedLiveCaptureTruth {
    match read_latest_status_message(base_dir, &manifest.session_id) {
        Ok(Some(status)) => project_live_capture_truth_from_status(manifest, status),
        Ok(None) => ProjectedLiveCaptureTruth {
            dto: LiveCaptureTruthDto {
                source: "canon-helper-sidecar".into(),
                freshness: "missing".into(),
                session_match: "unknown".into(),
                camera_state: "unknown".into(),
                helper_state: "unknown".into(),
                observed_at: None,
                sequence: None,
                detail_code: None,
            },
            gate: LiveCameraGate::CameraPreparing,
        },
        Err(SidecarClientError::InvalidStatus) => ProjectedLiveCaptureTruth {
            dto: LiveCaptureTruthDto {
                source: "canon-helper-sidecar".into(),
                freshness: "missing".into(),
                session_match: "unknown".into(),
                camera_state: "unknown".into(),
                helper_state: "unknown".into(),
                observed_at: None,
                sequence: None,
                detail_code: Some("invalid-status".into()),
            },
            gate: LiveCameraGate::HelperPreparing,
        },
        Err(SidecarClientError::StatusUnreadable) => ProjectedLiveCaptureTruth {
            dto: LiveCaptureTruthDto {
                source: "canon-helper-sidecar".into(),
                freshness: "missing".into(),
                session_match: "unknown".into(),
                camera_state: "unknown".into(),
                helper_state: "unknown".into(),
                observed_at: None,
                sequence: None,
                detail_code: Some("status-unreadable".into()),
            },
            gate: LiveCameraGate::CameraPreparing,
        },
        Err(_) => ProjectedLiveCaptureTruth {
            dto: LiveCaptureTruthDto {
                source: "canon-helper-sidecar".into(),
                freshness: "missing".into(),
                session_match: "unknown".into(),
                camera_state: "unknown".into(),
                helper_state: "unknown".into(),
                observed_at: None,
                sequence: None,
                detail_code: Some("helper-unavailable".into()),
            },
            gate: LiveCameraGate::HelperPreparing,
        },
    }
}

fn project_live_capture_truth_from_status(
    manifest: &SessionManifest,
    status: CanonHelperStatusMessage,
) -> ProjectedLiveCaptureTruth {
    let freshness = if is_fresh_helper_status(&status) {
        "fresh"
    } else {
        "stale"
    };
    let session_match = if status.session_id == manifest.session_id {
        "matched"
    } else {
        "mismatched"
    };
    let camera_state = normalize_live_camera_state(&status.camera_state);
    let helper_state = normalize_live_helper_state(&status.helper_state);
    let gate = if freshness != "fresh" || session_match != "matched" {
        LiveCameraGate::CameraPreparing
    } else {
        derive_live_camera_gate(camera_state, helper_state)
    };

    ProjectedLiveCaptureTruth {
        dto: LiveCaptureTruthDto {
            source: "canon-helper-sidecar".into(),
            freshness: freshness.into(),
            session_match: session_match.into(),
            camera_state: camera_state.into(),
            helper_state: helper_state.into(),
            observed_at: Some(status.observed_at),
            sequence: status.sequence,
            detail_code: status.detail_code,
        },
        gate,
    }
}

fn is_fresh_helper_status(status: &CanonHelperStatusMessage) -> bool {
    let Ok(observed_at_seconds) = rfc3339_to_unix_seconds(&status.observed_at) else {
        return false;
    };
    let Ok(now_duration) = SystemTime::now().duration_since(UNIX_EPOCH) else {
        return false;
    };

    now_duration.as_secs().saturating_sub(observed_at_seconds)
        <= CAMERA_HELPER_STATUS_MAX_AGE_SECONDS
}

fn normalize_live_camera_state(camera_state: &str) -> &'static str {
    match camera_state {
        "disconnected" => "disconnected",
        "connecting" => "connecting",
        "connected-idle" => "connected-idle",
        "ready" => "ready",
        "capturing" => "capturing",
        "recovering" => "recovering",
        "degraded" => "degraded",
        "error" => "error",
        _ => "unknown",
    }
}

fn normalize_live_helper_state(helper_state: &str) -> &'static str {
    match helper_state {
        "starting" => "starting",
        "connecting" => "connecting",
        "healthy" => "healthy",
        "recovering" => "recovering",
        "degraded" => "degraded",
        "error" => "error",
        _ => "unknown",
    }
}

fn derive_live_camera_gate(camera_state: &str, helper_state: &str) -> LiveCameraGate {
    if matches!(helper_state, "degraded" | "error") || matches!(camera_state, "degraded" | "error")
    {
        return LiveCameraGate::PhoneRequired;
    }

    if matches!(helper_state, "starting" | "connecting" | "recovering") {
        return LiveCameraGate::HelperPreparing;
    }

    match camera_state {
        "ready" if helper_state == "healthy" => LiveCameraGate::Ready,
        "recovering" => LiveCameraGate::HelperPreparing,
        "disconnected" | "connecting" | "connected-idle" | "capturing" => {
            LiveCameraGate::CameraPreparing
        }
        _ => LiveCameraGate::CameraPreparing,
    }
}

fn with_projected_live_capture_truth(
    readiness: CaptureReadinessDto,
    live_capture_truth: &ProjectedLiveCaptureTruth,
) -> CaptureReadinessDto {
    readiness.with_live_capture_truth(live_capture_truth.dto.clone())
}

fn build_blocked_readiness_from_live_camera_gate(
    session_id: String,
    live_camera_gate: LiveCameraGate,
    live_capture_truth: &ProjectedLiveCaptureTruth,
    latest_capture: Option<crate::session::session_manifest::SessionCaptureRecord>,
) -> CaptureReadinessDto {
    match live_camera_gate {
        LiveCameraGate::Ready => {
            CaptureReadinessDto::ready(session_id, "captureReady", latest_capture)
        }
        LiveCameraGate::CameraPreparing => match live_capture_truth.dto.camera_state.as_str() {
            "disconnected" => CaptureReadinessDto::camera_waiting_for_power(session_id)
                .with_latest_capture(latest_capture),
            "connecting" | "connected-idle" => CaptureReadinessDto::camera_connecting(session_id)
                .with_latest_capture(latest_capture),
            _ => CaptureReadinessDto::camera_preparing(session_id).with_latest_capture(latest_capture),
        },
        LiveCameraGate::HelperPreparing => {
            CaptureReadinessDto::helper_preparing(session_id).with_latest_capture(latest_capture)
        }
        LiveCameraGate::PhoneRequired => {
            CaptureReadinessDto::phone_required(session_id).with_latest_capture(latest_capture)
        }
    }
}

fn derive_capture_lifecycle_stage(manifest: &SessionManifest) -> String {
    match manifest.captures.last() {
        Some(capture)
            if capture.render_status == "previewWaiting"
                || capture.render_status == "captureSaved" =>
        {
            "preview-waiting".into()
        }
        Some(capture) if capture.render_status == "renderFailed" => "phone-required".into(),
        Some(_) => "capture-ready".into(),
        None if manifest.active_preset.is_some() => "capture-ready".into(),
        None => "session-started".into(),
    }
}

#[derive(Debug, Clone)]
struct StagedAssetDeletion {
    original_path: String,
    staged_path: String,
}

fn stage_capture_asset_deletions(
    paths: &SessionPaths,
    capture: &crate::session::session_manifest::SessionCaptureRecord,
) -> Result<Vec<StagedAssetDeletion>, HostErrorEnvelope> {
    let mut staged_assets = Vec::new();

    stage_session_scoped_asset_if_present(
        paths,
        &capture.raw.asset_path,
        &capture.capture_id,
        "raw",
        &mut staged_assets,
    )?;

    if let Some(preview_path) = capture.preview.asset_path.as_deref() {
        stage_session_scoped_asset_if_present(
            paths,
            preview_path,
            &capture.capture_id,
            "preview",
            &mut staged_assets,
        )?;
    }

    if let Some(final_path) = capture.final_asset.asset_path.as_deref() {
        stage_session_scoped_asset_if_present(
            paths,
            final_path,
            &capture.capture_id,
            "final",
            &mut staged_assets,
        )?;
    }

    Ok(staged_assets)
}

fn stage_session_scoped_asset_if_present(
    paths: &SessionPaths,
    asset_path: &str,
    capture_id: &str,
    asset_kind: &str,
    staged_assets: &mut Vec<StagedAssetDeletion>,
) -> Result<(), HostErrorEnvelope> {
    if !is_session_scoped_asset_path(paths, asset_path) {
        return Ok(());
    }

    let asset = Path::new(asset_path);

    if !asset.exists() {
        return Ok(());
    }

    let metadata = fs::metadata(asset).map_err(|error| {
        HostErrorEnvelope::persistence(format!("세션 파일을 정리하지 못했어요: {error}"))
    })?;

    if !metadata.is_file() {
        return Err(HostErrorEnvelope::persistence(
            "세션 파일을 정리하지 못했어요. 잠시 후 다시 시도해 주세요.",
        ));
    }

    let staged_path = format!("{asset_path}.delete-{capture_id}-{asset_kind}");

    if Path::new(&staged_path).exists() {
        fs::remove_file(&staged_path).map_err(|error| {
            HostErrorEnvelope::persistence(format!("세션 파일을 정리하지 못했어요: {error}"))
        })?;
    }

    fs::rename(asset_path, &staged_path).map_err(|error| {
        HostErrorEnvelope::persistence(format!("세션 파일을 정리하지 못했어요: {error}"))
    })?;

    staged_assets.push(StagedAssetDeletion {
        original_path: asset_path.into(),
        staged_path,
    });

    Ok(())
}

fn rollback_staged_asset_deletions(staged_assets: &[StagedAssetDeletion]) {
    for staged_asset in staged_assets.iter().rev() {
        if Path::new(&staged_asset.staged_path).exists() {
            let _ = fs::rename(&staged_asset.staged_path, &staged_asset.original_path);
        }
    }
}

fn finalize_staged_asset_deletions(staged_assets: &[StagedAssetDeletion]) {
    for staged_asset in staged_assets {
        if Path::new(&staged_asset.staged_path).exists() {
            let _ = fs::remove_file(&staged_asset.staged_path);
        }
    }
}

fn is_session_scoped_asset_path(paths: &SessionPaths, asset_path: &str) -> bool {
    let normalized_asset_path = asset_path.replace('\\', "/").to_lowercase();
    let normalized_session_root = format!(
        "{}/",
        paths
            .session_root
            .to_string_lossy()
            .replace('\\', "/")
            .to_lowercase()
    );

    if normalized_asset_path.starts_with("//") {
        return false;
    }

    if normalized_asset_path
        .split('/')
        .any(|segment| segment == "..")
    {
        return false;
    }

    let is_absolute_path = normalized_asset_path.starts_with('/')
        || normalized_asset_path
            .chars()
            .nth(1)
            .map(|character| character == ':')
            .unwrap_or(false);

    if !is_absolute_path {
        return false;
    }

    normalized_asset_path.starts_with(&normalized_session_root)
}

fn read_session_manifest(
    base_dir: &Path,
    session_id: &str,
) -> Result<SessionManifest, HostErrorEnvelope> {
    let manifest_path = SessionPaths::try_new(base_dir, session_id)?.manifest_path;

    if !manifest_path.is_file() {
        return Err(HostErrorEnvelope::session_not_found(
            "진행 중인 세션을 찾지 못했어요. 처음 화면에서 다시 시작해 주세요.",
        ));
    }

    let manifest_bytes = fs::read_to_string(manifest_path).map_err(|error| {
        HostErrorEnvelope::persistence(format!("세션 매니페스트를 읽지 못했어요: {error}"))
    })?;

    let mut manifest: SessionManifest = serde_json::from_str(&manifest_bytes).map_err(|error| {
        HostErrorEnvelope::persistence(format!("세션 매니페스트를 읽지 못했어요: {error}"))
    })?;

    normalize_legacy_manifest(&mut manifest);

    Ok(manifest)
}

fn read_session_manifest_with_timing(
    base_dir: &Path,
    session_id: &str,
) -> Result<SessionManifest, HostErrorEnvelope> {
    let paths = SessionPaths::try_new(base_dir, session_id)?;
    let manifest = read_session_manifest(base_dir, session_id)?;

    sync_session_timing_in_dir(
        base_dir,
        &paths.manifest_path,
        manifest,
        std::time::SystemTime::now(),
    )
    .and_then(|manifest| {
        sync_post_end_state_in_dir(
            base_dir,
            &paths.manifest_path,
            manifest,
            std::time::SystemTime::now(),
        )
    })
}

fn timing_phase(timing: Option<&crate::session::session_manifest::SessionTiming>) -> TimingPhase {
    match timing.map(|value| value.phase.as_str()) {
        Some("warning") => TimingPhase::Warning,
        Some("ended") => TimingPhase::Ended,
        _ => TimingPhase::Active,
    }
}

fn generate_capture_request_id() -> String {
    let unix_nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let counter = CAPTURE_REQUEST_COUNTER.fetch_add(1, Ordering::Relaxed) as u128;
    let value = unix_nanos ^ (counter << 16);

    format!("request_{value:026x}")
}

fn has_in_flight_capture(base_dir: &Path) -> bool {
    let runtime_key = runtime_capture_guard_key(base_dir);

    IN_FLIGHT_CAPTURE_SESSIONS
        .lock()
        .map(|sessions| sessions.contains_key(&runtime_key))
        .unwrap_or(false)
}

fn acquire_in_flight_capture_guard(
    base_dir: &Path,
    session_id: &str,
) -> Result<InFlightCaptureGuard, HostErrorEnvelope> {
    let runtime_key = runtime_capture_guard_key(base_dir);
    let mut sessions = IN_FLIGHT_CAPTURE_SESSIONS.lock().map_err(|_| {
        HostErrorEnvelope::persistence("촬영 상태를 잠그지 못했어요. 잠시 후 다시 시도해 주세요.")
    })?;

    if sessions.contains_key(&runtime_key) {
        return Err(HostErrorEnvelope::capture_not_ready(
            "지금은 촬영을 마무리하는 중이에요.",
            CaptureReadinessDto::camera_preparing(session_id.to_string()),
        ));
    }

    sessions.insert(runtime_key.clone(), session_id.to_string());

    Ok(InFlightCaptureGuard {
        runtime_key,
        session_id: session_id.to_string(),
    })
}

struct InFlightCaptureGuard {
    runtime_key: String,
    session_id: String,
}

impl Drop for InFlightCaptureGuard {
    fn drop(&mut self) {
        if let Ok(mut sessions) = IN_FLIGHT_CAPTURE_SESSIONS.lock() {
            if sessions
                .get(&self.runtime_key)
                .map(|session_id| session_id == &self.session_id)
                .unwrap_or(false)
            {
                sessions.remove(&self.runtime_key);
            }
        }
    }
}

fn runtime_capture_guard_key(base_dir: &Path) -> String {
    base_dir.to_string_lossy().into_owned()
}
