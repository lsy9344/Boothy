use std::{
    fs,
    path::Path,
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

use crate::{
    capture::{
        ingest_pipeline::{
            complete_preview_render_in_dir, persist_capture_in_dir,
            promote_pending_fast_preview_in_dir, sync_completed_speculative_preview_in_dir,
        },
        sidecar_client::{
            is_retryable_capture_helper_error, map_capture_round_trip_error,
            read_capture_event_count, read_capture_request_messages,
            read_latest_helper_error_message, read_latest_status_message,
            read_processed_capture_request_ids, wait_for_capture_round_trip,
            write_capture_request_message, CanonHelperCaptureRequestMessage,
            CanonHelperStatusMessage, FastPreviewReadyUpdate, SidecarClientError,
            CANON_HELPER_CAPTURE_REQUEST_SCHEMA_VERSION,
        },
        CAPTURE_PIPELINE_LOCK, IN_FLIGHT_CAPTURE_SESSIONS,
    },
    contracts::dto::{
        CaptureDeleteInputDto, CaptureDeleteResultDto, CaptureReadinessDto,
        CaptureReadinessInputDto, CaptureRequestInputDto, CaptureRequestResultDto,
        HostErrorEnvelope, LiveCaptureTruthDto,
    },
    diagnostics::audit_log::{try_append_operator_audit_record, OperatorAuditRecordInput},
    handoff::sync_post_end_state_in_dir,
    preset::preset_catalog::{find_published_preset_summary, resolve_published_preset_catalog_dir},
    render::{is_valid_render_preview_asset, schedule_preview_renderer_warmup_in_dir},
    session::{
        session_manifest::{
            current_timestamp, rfc3339_to_unix_seconds, ActivePresetBinding, SessionCaptureRecord,
            SessionManifest, SESSION_POST_END_COMPLETED, SESSION_POST_END_PHONE_REQUIRED,
        },
        session_paths::SessionPaths,
        session_repository::{read_session_manifest, write_session_manifest},
    },
    timing::{
        append_session_timing_event_in_dir, sync_session_timing_in_dir, SessionTimingEventInput,
        TimingPhase,
    },
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
    let mut manifest = read_session_manifest_with_timing(base_dir, &input.session_id)?;
    let repaired_invalid_preview = sync_invalid_preview_truth_in_manifest(base_dir, &mut manifest)?;
    let repaired_render_failure =
        sync_recoverable_render_failure_in_manifest(base_dir, &mut manifest)?;
    let repaired_finished_speculative_preview =
        sync_finished_speculative_preview_in_manifest(base_dir, &mut manifest)?;
    sync_better_preview_assets_in_manifest(base_dir, &mut manifest)?;
    sync_retryable_capture_failure_recovery_in_manifest(base_dir, &mut manifest)?;
    if repaired_invalid_preview || repaired_render_failure || repaired_finished_speculative_preview
    {
        manifest = read_session_manifest_with_timing(base_dir, &input.session_id)?;
        sync_better_preview_assets_in_manifest(base_dir, &mut manifest)?;
        sync_retryable_capture_failure_recovery_in_manifest(base_dir, &mut manifest)?;
    }

    Ok(normalize_capture_readiness(base_dir, &manifest))
}

pub fn request_capture_in_dir(
    base_dir: &Path,
    input: CaptureRequestInputDto,
) -> Result<CaptureRequestResultDto, HostErrorEnvelope> {
    request_capture_in_dir_with_fast_preview(base_dir, input, |_| {})
}

pub fn request_capture_in_dir_with_fast_preview<F>(
    base_dir: &Path,
    input: CaptureRequestInputDto,
    mut on_fast_preview_ready: F,
) -> Result<CaptureRequestResultDto, HostErrorEnvelope>
where
    F: FnMut(FastPreviewReadyUpdate),
{
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

    let in_flight_guard = acquire_in_flight_capture_guard(base_dir, &input.session_id)?;
    let manifest = read_session_manifest_with_timing(base_dir, &input.session_id)?;
    let active_preset = manifest.active_preset.clone().ok_or_else(|| {
        HostErrorEnvelope::preset_not_available("촬영 전에 룩을 다시 골라 주세요.")
    })?;
    let request_id = input
        .request_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .unwrap_or_else(generate_capture_request_id);
    ensure_capture_request_id_is_fresh(base_dir, &input.session_id, &request_id, &readiness)?;
    let requested_at = current_timestamp(SystemTime::now())?;
    let request_message = CanonHelperCaptureRequestMessage {
        schema_version: CANON_HELPER_CAPTURE_REQUEST_SCHEMA_VERSION.into(),
        message_type: "request-capture".into(),
        session_id: input.session_id.clone(),
        request_id: request_id.clone(),
        requested_at,
        active_preset_id: active_preset.preset_id.clone(),
        active_preset_version: active_preset.published_version.clone(),
    };
    let starting_event_count = read_capture_event_count(base_dir, &input.session_id)
        .map_err(|error| map_capture_round_trip_error(&input.session_id, error))?;
    let fast_preview_base_dir = base_dir.to_path_buf();
    let fast_preview_session_id = input.session_id.clone();
    let fast_preview_request_id = request_id.clone();
    let mut early_fast_preview_update = None;

    schedule_preview_renderer_warmup_in_dir(
        base_dir,
        &input.session_id,
        &active_preset.preset_id,
        &active_preset.published_version,
    );

    write_capture_request_message(base_dir, &request_message)
        .map_err(|error| map_capture_round_trip_error(&input.session_id, error))?;
    let _ = append_session_timing_event_in_dir(
        base_dir,
        SessionTimingEventInput {
            session_id: &input.session_id,
            event: "request-capture",
            capture_id: None,
            request_id: Some(&request_id),
            detail: Some(&format!(
                "activePresetId={};activePresetVersion={}",
                active_preset.preset_id, active_preset.published_version
            )),
        },
    );

    let round_trip = match wait_for_capture_round_trip(
        base_dir,
        &input.session_id,
        &request_id,
        starting_event_count,
        |fast_preview| {
            if early_fast_preview_update.is_none() {
                if let Some(update) = promote_pending_fast_preview_in_dir(
                    &fast_preview_base_dir,
                    &fast_preview_session_id,
                    &fast_preview_request_id,
                    &fast_preview.capture_id,
                    &fast_preview.fast_preview_path,
                    fast_preview.fast_preview_kind.as_deref(),
                ) {
                    on_fast_preview_ready(update.clone());
                    early_fast_preview_update = Some(update);
                }
            }
        },
    ) {
        Ok(round_trip) => round_trip,
        Err(error) => {
            if should_persist_capture_round_trip_failure(&error) {
                persist_capture_round_trip_failure(base_dir, &input.session_id, &error)?;
            }

            drop(in_flight_guard);

            let readiness = get_capture_readiness_in_dir(
                base_dir,
                CaptureReadinessInputDto {
                    session_id: input.session_id.clone(),
                },
            )
            .ok()
            .map(|readiness| {
                capture_round_trip_failure_readiness(base_dir, &manifest, &error, readiness)
            })
            .unwrap_or_else(|| match error {
                SidecarClientError::CaptureTriggerRetryRequired => {
                    build_capture_retry_readiness(base_dir, &manifest)
                }
                _ => CaptureReadinessDto::phone_required(input.session_id.clone()),
            });

            return Err(HostErrorEnvelope::capture_not_ready(
                capture_round_trip_failure_message(&error),
                readiness,
            ));
        }
    };
    let file_arrived_detail = format!(
        "rawPath={};persistedAtMs={};fastPreview={};fastPreviewKind={}",
        round_trip.raw_path,
        round_trip.persisted_at_ms,
        round_trip
            .fast_preview
            .as_ref()
            .map(|preview| preview.asset_path.as_str())
            .unwrap_or("none"),
        round_trip
            .fast_preview
            .as_ref()
            .and_then(|preview| preview.kind.as_deref())
            .unwrap_or("none")
    );
    let _ = append_session_timing_event_in_dir(
        base_dir,
        SessionTimingEventInput {
            session_id: &input.session_id,
            event: "file-arrived",
            capture_id: Some(&round_trip.capture_id),
            request_id: Some(&request_id),
            detail: Some(&file_arrived_detail),
        },
    );
    let (manifest, capture, fast_preview_update) = persist_capture_in_dir(
        base_dir,
        &input,
        round_trip.capture_id,
        request_id.clone(),
        round_trip.raw_path,
        round_trip.fast_preview,
        round_trip.capture_accepted_at_ms,
        round_trip.persisted_at_ms,
    )
    .map_err(|error| {
        log::warn!(
            "capture_persist_failed session={} request_id={} code={} message={}",
            input.session_id,
            request_id,
            error.code,
            error.message
        );

        HostErrorEnvelope::capture_not_ready(
            "방금 사진 저장을 확인하지 못했어요. 가까운 직원에게 알려 주세요.",
            CaptureReadinessDto::phone_required(input.session_id.clone())
                .with_timing(manifest.timing.clone())
                .with_live_capture_truth(project_live_capture_truth(base_dir, &manifest).dto),
        )
    })?;
    if let Some(update) = fast_preview_update {
        let should_emit = early_fast_preview_update
            .as_ref()
            .map(|early_update: &FastPreviewReadyUpdate| {
                early_update.request_id != update.request_id
                    || early_update.capture_id != update.capture_id
                    || early_update.asset_path != update.asset_path
            })
            .unwrap_or(true);
        if should_emit {
            on_fast_preview_ready(update);
        }
    }

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

fn should_persist_capture_round_trip_failure(error: &SidecarClientError) -> bool {
    matches!(
        error,
        SidecarClientError::CaptureTimedOut
            | SidecarClientError::CaptureRejected
            | SidecarClientError::RecoveryRequired
            | SidecarClientError::CaptureSessionMismatch
            | SidecarClientError::CaptureFileMissing
            | SidecarClientError::CaptureFileEmpty
            | SidecarClientError::CaptureFileUnscoped
            | SidecarClientError::CaptureProtocolViolation
    )
}

fn ensure_capture_request_id_is_fresh(
    base_dir: &Path,
    session_id: &str,
    request_id: &str,
    readiness: &CaptureReadinessDto,
) -> Result<(), HostErrorEnvelope> {
    let existing_requests = read_capture_request_messages(base_dir, session_id).map_err(|_| {
        HostErrorEnvelope::persistence(
            "촬영 요청 기록을 확인하지 못했어요. 가까운 직원에게 알려 주세요.",
        )
    })?;
    let processed_request_ids =
        read_processed_capture_request_ids(base_dir, session_id).map_err(|_| {
            HostErrorEnvelope::persistence(
                "촬영 요청 기록을 확인하지 못했어요. 가까운 직원에게 알려 주세요.",
            )
        })?;

    if existing_requests
        .iter()
        .any(|request| request.request_id == request_id)
        || processed_request_ids
            .iter()
            .any(|processed| processed == request_id)
    {
        return Err(HostErrorEnvelope::capture_not_ready(
            "촬영 요청을 다시 보내 주세요.",
            readiness.clone(),
        ));
    }

    Ok(())
}

fn capture_round_trip_failure_message(error: &SidecarClientError) -> &'static str {
    match error {
        SidecarClientError::CaptureTriggerRetryRequired => {
            "사진을 아직 찍지 못했어요. 초점을 다시 맞춘 뒤 한 번 더 시도해 주세요."
        }
        SidecarClientError::CaptureTimedOut => {
            "사진 저장을 끝내지 못했어요. 가까운 직원에게 알려 주세요."
        }
        SidecarClientError::CaptureRejected
        | SidecarClientError::RecoveryRequired
        | SidecarClientError::CaptureSessionMismatch
        | SidecarClientError::CaptureFileMissing
        | SidecarClientError::CaptureFileEmpty
        | SidecarClientError::CaptureFileUnscoped
        | SidecarClientError::CaptureProtocolViolation => {
            "사진 저장을 확인하지 못했어요. 가까운 직원에게 알려 주세요."
        }
        SidecarClientError::RequestWriteFailed
        | SidecarClientError::EventsUnreadable
        | SidecarClientError::InvalidEvents
        | SidecarClientError::StatusUnreadable
        | SidecarClientError::InvalidStatus => {
            "카메라 연결 상태를 확인하지 못했어요. 가까운 직원에게 알려 주세요."
        }
    }
}

fn capture_round_trip_failure_reason_code(error: &SidecarClientError) -> &'static str {
    match error {
        SidecarClientError::CaptureTriggerRetryRequired => "capture-trigger-retry-required",
        SidecarClientError::CaptureTimedOut => "capture-timeout",
        SidecarClientError::CaptureRejected => "capture-rejected",
        SidecarClientError::RecoveryRequired => "capture-recovery-required",
        SidecarClientError::CaptureSessionMismatch => "capture-session-mismatch",
        SidecarClientError::CaptureFileMissing => "capture-file-missing",
        SidecarClientError::CaptureFileEmpty => "capture-file-empty",
        SidecarClientError::CaptureFileUnscoped => "capture-file-unscoped",
        SidecarClientError::CaptureProtocolViolation => "capture-protocol-violation",
        SidecarClientError::RequestWriteFailed => "capture-request-write-failed",
        SidecarClientError::EventsUnreadable => "capture-events-unreadable",
        SidecarClientError::InvalidEvents => "capture-events-invalid",
        SidecarClientError::StatusUnreadable => "capture-status-unreadable",
        SidecarClientError::InvalidStatus => "capture-status-invalid",
    }
}

fn capture_round_trip_failure_detail(error: &SidecarClientError) -> &'static str {
    match error {
        SidecarClientError::CaptureTriggerRetryRequired => {
            "초점 또는 셔터 준비 단계에서 촬영이 시작되지 않아 현재 촬영을 재시도 대기 상태로 되돌렸어요."
        }
        SidecarClientError::CaptureTimedOut => {
            "셔터 요청 뒤 file-arrived 경계를 확인하지 못해 세션을 phone-required로 고정했어요."
        }
        SidecarClientError::CaptureRejected => {
            "helper가 캡처 요청을 끝까지 수락하지 못해 세션을 phone-required로 고정했어요."
        }
        SidecarClientError::RecoveryRequired => {
            "캡처 중 helper recovery가 필요해 세션을 phone-required로 고정했어요."
        }
        SidecarClientError::CaptureSessionMismatch => {
            "다른 세션으로 보이는 촬영 이벤트를 감지해 세션을 phone-required로 고정했어요."
        }
        SidecarClientError::CaptureFileMissing => {
            "촬영 완료 이벤트는 도착했지만 RAW 파일을 찾지 못해 세션을 phone-required로 고정했어요."
        }
        SidecarClientError::CaptureFileEmpty => {
            "촬영 RAW 파일이 비어 있어 세션을 phone-required로 고정했어요."
        }
        SidecarClientError::CaptureFileUnscoped => {
            "세션 범위를 벗어난 RAW 파일 경로가 감지돼 세션을 phone-required로 고정했어요."
        }
        SidecarClientError::CaptureProtocolViolation => {
            "capture-accepted 없이 file-arrived가 도착해 세션을 phone-required로 고정했어요."
        }
        SidecarClientError::RequestWriteFailed
        | SidecarClientError::EventsUnreadable
        | SidecarClientError::InvalidEvents
        | SidecarClientError::StatusUnreadable
        | SidecarClientError::InvalidStatus => "캡처 경계 진단을 읽지 못했어요.",
    }
}

fn persist_capture_round_trip_failure(
    base_dir: &Path,
    session_id: &str,
    error: &SidecarClientError,
) -> Result<(), HostErrorEnvelope> {
    let paths = SessionPaths::try_new(base_dir, session_id)?;
    let mut manifest = read_session_manifest_with_timing(base_dir, session_id)?;
    let occurred_at = current_timestamp(SystemTime::now())?;

    manifest.lifecycle.stage = "phone-required".into();
    manifest.post_end = None;
    manifest.updated_at = occurred_at.clone();
    write_session_manifest(&paths.manifest_path, &manifest)?;

    try_append_operator_audit_record(
        base_dir,
        OperatorAuditRecordInput {
            occurred_at,
            session_id: Some(session_id.to_string()),
            event_category: "critical-failure",
            event_type: "capture-round-trip-failed",
            summary: "촬영 결과를 세션에 저장하지 못했어요.".into(),
            detail: capture_round_trip_failure_detail(error).into(),
            actor_id: None,
            source: "capture-boundary",
            capture_id: None,
            preset_id: manifest.active_preset_id.clone(),
            published_version: manifest
                .active_preset
                .as_ref()
                .map(|preset| preset.published_version.clone()),
            reason_code: Some(capture_round_trip_failure_reason_code(error).into()),
        },
    );

    Ok(())
}

fn capture_round_trip_failure_readiness(
    base_dir: &Path,
    manifest: &SessionManifest,
    error: &SidecarClientError,
    fallback: CaptureReadinessDto,
) -> CaptureReadinessDto {
    match error {
        SidecarClientError::CaptureTriggerRetryRequired => {
            build_capture_retry_readiness(base_dir, manifest)
        }
        _ => fallback,
    }
}

fn build_capture_retry_readiness(
    base_dir: &Path,
    manifest: &SessionManifest,
) -> CaptureReadinessDto {
    let latest_capture = manifest.captures.last().cloned();
    let timing = manifest.timing.clone();
    let projected_live_capture_truth = project_live_capture_truth(base_dir, manifest);

    with_projected_live_capture_truth(
        CaptureReadinessDto::capture_retry_required(manifest.session_id.clone(), latest_capture)
            .with_timing(timing),
        &projected_live_capture_truth,
    )
}

fn sync_retryable_capture_failure_recovery_in_manifest(
    base_dir: &Path,
    manifest: &mut SessionManifest,
) -> Result<(), HostErrorEnvelope> {
    if !matches!(
        manifest.lifecycle.stage.as_str(),
        "phone-required" | "blocked"
    ) {
        return Ok(());
    }

    if manifest.post_end.is_some() || timing_phase(manifest.timing.as_ref()) == TimingPhase::Ended {
        return Ok(());
    }

    let projected_live_capture_truth = project_live_capture_truth(base_dir, manifest);

    if projected_live_capture_truth.dto.freshness != "fresh"
        || projected_live_capture_truth.dto.session_match != "matched"
        || projected_live_capture_truth.gate != LiveCameraGate::Ready
    {
        return Ok(());
    }

    let Some(latest_helper_error) =
        read_latest_helper_error_message(base_dir, &manifest.session_id)
            .ok()
            .flatten()
    else {
        return Ok(());
    };

    if !helper_error_allows_session_recovery_after_ready(&latest_helper_error) {
        return Ok(());
    }

    let next_stage = derive_capture_lifecycle_stage(manifest);
    if next_stage == manifest.lifecycle.stage {
        return Ok(());
    }

    let previous_stage = manifest.lifecycle.stage.clone();
    let paths = SessionPaths::try_new(base_dir, &manifest.session_id)?;
    manifest.lifecycle.stage = next_stage.clone();
    manifest.updated_at = current_timestamp(SystemTime::now())?;
    write_session_manifest(&paths.manifest_path, manifest)?;

    log::info!(
        "capture_retry_recovered session={} previous_stage={} next_stage={} detail_code={} live_truth=fresh:matched:ready:healthy",
        manifest.session_id,
        previous_stage,
        next_stage,
        latest_helper_error.detail_code
    );

    Ok(())
}

fn helper_error_allows_session_recovery_after_ready(
    message: &crate::capture::sidecar_client::CanonHelperErrorMessage,
) -> bool {
    if is_retryable_capture_helper_error(message) {
        return true;
    }

    matches!(
        message.detail_code.as_str(),
        "capture-download-timeout" | "capture-transfer-start-timeout"
    )
}

fn sync_recoverable_render_failure_in_manifest(
    base_dir: &Path,
    manifest: &mut SessionManifest,
) -> Result<bool, HostErrorEnvelope> {
    if !matches!(
        manifest.lifecycle.stage.as_str(),
        "phone-required" | "blocked"
    ) {
        return Ok(false);
    }

    if manifest.post_end.is_some() || timing_phase(manifest.timing.as_ref()) == TimingPhase::Ended {
        return Ok(false);
    }

    let Some(latest_capture) = manifest.captures.last() else {
        return Ok(false);
    };

    if latest_capture.render_status != "renderFailed" {
        return Ok(false);
    }

    let projected_live_capture_truth = project_live_capture_truth(base_dir, manifest);
    if projected_live_capture_truth.dto.freshness != "fresh"
        || projected_live_capture_truth.dto.session_match != "matched"
        || projected_live_capture_truth.gate != LiveCameraGate::Ready
    {
        return Ok(false);
    }

    match complete_preview_render_in_dir(base_dir, &manifest.session_id, &latest_capture.capture_id)
    {
        Ok(_) => {
            log::info!(
                "capture_render_recovered session={} capture_id={} recovery=preview-rerendered",
                manifest.session_id,
                latest_capture.capture_id
            );
            Ok(true)
        }
        Err(error) => {
            log::warn!(
                "capture_render_recovery_failed session={} capture_id={} code={} message={}",
                manifest.session_id,
                latest_capture.capture_id,
                error.code,
                error.message
            );
            Ok(false)
        }
    }
}

fn sync_finished_speculative_preview_in_manifest(
    base_dir: &Path,
    manifest: &mut SessionManifest,
) -> Result<bool, HostErrorEnvelope> {
    let Some(latest_capture) = manifest.captures.last() else {
        return Ok(false);
    };

    if latest_capture.render_status != "previewWaiting"
        || latest_capture.preview.ready_at_ms.is_some()
        || latest_capture.preview.asset_path.is_none()
    {
        return Ok(false);
    }

    match sync_completed_speculative_preview_in_dir(
        base_dir,
        &manifest.session_id,
        &latest_capture.capture_id,
    )? {
        Some(capture)
            if matches!(
                capture.render_status.as_str(),
                "previewReady" | "finalReady"
            ) && capture.preview.ready_at_ms.is_some() =>
        {
            log::info!(
                "capture_preview_promoted_from_finished_speculative_close session={} capture_id={}",
                manifest.session_id,
                latest_capture.capture_id
            );
            Ok(true)
        }
        _ => Ok(false),
    }
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
                if capture_has_resumable_fast_preview(&capture) {
                    match live_camera_gate {
                        LiveCameraGate::Ready => with_projected_live_capture_truth(
                            CaptureReadinessDto::ready(
                                manifest.session_id.clone(),
                                "captureReady",
                                Some(capture),
                            )
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
                    }
                } else {
                    with_projected_live_capture_truth(
                        CaptureReadinessDto::preview_waiting(
                            manifest.session_id.clone(),
                            Some(capture),
                        )
                        .with_timing(timing),
                        &live_capture_truth,
                    )
                }
            }
            Some(capture) if capture.render_status == "renderFailed" => {
                with_projected_live_capture_truth(
                    CaptureReadinessDto::phone_required(manifest.session_id.clone())
                        .with_timing(timing),
                    &live_capture_truth,
                )
            }
            Some(capture)
                if capture.render_status == "previewReady"
                    || capture.render_status == "finalReady" =>
            {
                match live_camera_gate {
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
                }
            }
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
        "preview-waiting" => {
            let resumable_capture = latest_capture
                .as_ref()
                .filter(|capture| capture_has_resumable_fast_preview(capture))
                .cloned();
            if let Some(capture) = resumable_capture {
                match live_camera_gate {
                    LiveCameraGate::Ready => with_projected_live_capture_truth(
                        CaptureReadinessDto::ready(
                            manifest.session_id.clone(),
                            "captureReady",
                            Some(capture),
                        )
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
                }
            } else {
                with_projected_live_capture_truth(
                    CaptureReadinessDto::preview_waiting(
                        manifest.session_id.clone(),
                        latest_capture,
                    )
                    .with_timing(timing),
                    &live_capture_truth,
                )
            }
        }
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
            _ => CaptureReadinessDto::camera_preparing(session_id)
                .with_latest_capture(latest_capture),
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

fn read_session_manifest_with_timing(
    base_dir: &Path,
    session_id: &str,
) -> Result<SessionManifest, HostErrorEnvelope> {
    let paths = SessionPaths::try_new(base_dir, session_id)?;
    let manifest = read_session_manifest(&paths.manifest_path)?;

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

fn sync_better_preview_assets_in_manifest(
    base_dir: &Path,
    manifest: &mut SessionManifest,
) -> Result<(), HostErrorEnvelope> {
    let paths = SessionPaths::try_new(base_dir, &manifest.session_id)?;
    let mut updated = false;

    for capture in &mut manifest.captures {
        let Some(better_preview_path) =
            find_better_session_preview_asset(&paths, &capture.capture_id)
        else {
            continue;
        };

        match capture.render_status.as_str() {
            "previewReady" | "finalReady" => {
                let Some(current_preview_path) = capture.preview.asset_path.as_deref() else {
                    continue;
                };

                if !is_session_scoped_asset_path(&paths, current_preview_path) {
                    continue;
                }

                if better_preview_path == current_preview_path {
                    continue;
                }

                capture.preview.asset_path = Some(better_preview_path);
                updated = true;
            }
            "captureSaved" | "previewWaiting" => {
                let current_preview_matches = capture
                    .preview
                    .asset_path
                    .as_deref()
                    .map(|asset_path| asset_path == better_preview_path.as_str())
                    .unwrap_or(false);
                if current_preview_matches {
                    continue;
                }

                capture.preview.asset_path = Some(better_preview_path.clone());
                if capture.timing.fast_preview_visible_at_ms.is_none() {
                    capture.timing.fast_preview_visible_at_ms =
                        preview_asset_visible_at_ms(Path::new(&better_preview_path))
                            .or_else(|| system_time_to_ms(SystemTime::now()));
                }
                updated = true;
            }
            _ => continue,
        }
    }

    if updated {
        manifest.updated_at = current_timestamp(SystemTime::now())?;
        write_session_manifest(&paths.manifest_path, manifest)?;
    }

    Ok(())
}

fn find_better_session_preview_asset(paths: &SessionPaths, capture_id: &str) -> Option<String> {
    let preferred_extensions = ["jpg", "jpeg", "png", "webp", "gif", "bmp"];
    preferred_extensions
        .iter()
        .map(|extension| {
            paths
                .renders_previews_dir
                .join(format!("{capture_id}.{extension}"))
        })
        .find(|path| is_valid_render_preview_asset(path))
        .map(|path| path.to_string_lossy().into_owned())
}

fn capture_has_resumable_fast_preview(capture: &SessionCaptureRecord) -> bool {
    matches!(
        capture.render_status.as_str(),
        "captureSaved" | "previewWaiting"
    ) && capture.preview.asset_path.is_some()
        && capture.preview.ready_at_ms.is_none()
        && capture.timing.fast_preview_visible_at_ms.is_some()
}

fn preview_asset_visible_at_ms(path: &Path) -> Option<u64> {
    fs::metadata(path)
        .ok()
        .and_then(|metadata| metadata.modified().ok())
        .and_then(system_time_to_ms)
}

fn system_time_to_ms(value: SystemTime) -> Option<u64> {
    value
        .duration_since(UNIX_EPOCH)
        .ok()
        .map(|duration| duration.as_millis() as u64)
}

fn sync_invalid_preview_truth_in_manifest(
    base_dir: &Path,
    manifest: &mut SessionManifest,
) -> Result<bool, HostErrorEnvelope> {
    if timing_phase(manifest.timing.as_ref()) == TimingPhase::Ended {
        return Ok(false);
    }

    let paths = SessionPaths::try_new(base_dir, &manifest.session_id)?;
    let mut updated = false;
    let mut repair_targets = Vec::new();

    for capture in &mut manifest.captures {
        if capture.post_end_state != "activeSession"
            || !matches!(
                capture.render_status.as_str(),
                "previewReady" | "finalReady"
            )
        {
            continue;
        }

        let preview_is_valid = capture
            .preview
            .asset_path
            .as_deref()
            .map(|asset_path| {
                is_session_scoped_asset_path(&paths, asset_path)
                    && is_valid_render_preview_asset(Path::new(asset_path))
            })
            .unwrap_or(false);

        if preview_is_valid {
            continue;
        }

        let Some(better_preview_path) =
            find_better_session_preview_asset(&paths, &capture.capture_id)
        else {
            capture.preview.asset_path = None;
            capture.preview.ready_at_ms = None;
            capture.final_asset.asset_path = None;
            capture.final_asset.ready_at_ms = None;
            capture.render_status = "previewWaiting".into();
            updated = true;
            repair_targets.push(capture.capture_id.clone());
            continue;
        };

        capture.preview.asset_path = Some(better_preview_path);
        updated = true;
    }

    if !updated {
        return Ok(false);
    }

    manifest.lifecycle.stage = derive_capture_lifecycle_stage(manifest);
    manifest.updated_at = current_timestamp(SystemTime::now())?;
    write_session_manifest(&paths.manifest_path, manifest)?;

    for capture_id in repair_targets {
        match complete_preview_render_in_dir(base_dir, &manifest.session_id, &capture_id) {
            Ok(_) => {
                log::info!(
                    "capture_preview_repaired session={} capture_id={} repair=rerendered",
                    manifest.session_id,
                    capture_id
                );
            }
            Err(error) => {
                log::warn!(
                    "capture_preview_repair_failed session={} capture_id={} code={} message={}",
                    manifest.session_id,
                    capture_id,
                    error.code,
                    error.message
                );
            }
        }
    }

    Ok(true)
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
