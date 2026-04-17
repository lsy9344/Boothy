pub mod audit_log;
pub mod recovery;

use std::{
    cmp::Ordering,
    fs,
    path::{Path, PathBuf},
    time::SystemTime,
};

use crate::{
    capture::normalized_state::normalize_capture_readiness,
    contracts::dto::{
        CapabilitySnapshotDto, HostErrorEnvelope, LiveCaptureTruthDto, OperatorBoundarySummaryDto,
        OperatorCameraConnectionSummaryDto, OperatorPreviewArchitectureSummaryDto,
        OperatorRecentFailureSummaryDto, OperatorSessionSummaryDto,
    },
    handoff::project_post_end_state_in_dir,
    render::dedicated_renderer::dedicated_renderer_hardware_capability,
    session::{
        session_manifest::{
            rfc3339_to_unix_seconds, PreviewRendererRouteSnapshot,
            PreviewRendererWarmStateSnapshot, SessionCaptureRecord, SessionManifest,
            SESSION_POST_END_COMPLETED, SESSION_POST_END_EXPORT_WAITING,
            SESSION_POST_END_PHONE_REQUIRED,
        },
        session_paths::SessionPaths,
        session_repository::read_session_manifest,
    },
    timing::project_session_timing,
};

const OPERATOR_SESSION_SUMMARY_SCHEMA_VERSION: &str = "operator-session-summary/v1";
const OPERATOR_WINDOW_LABEL: &str = "operator-window";

#[derive(Debug, Default, Clone)]
struct DiagnosticsContext {
    observed_at: Option<String>,
    event_name: Option<String>,
    post_end_state: Option<String>,
    implementation_track: Option<String>,
    lane_owner: Option<String>,
    fallback_reason_code: Option<String>,
    route_stage: Option<String>,
    visible_owner: Option<String>,
    visible_owner_transition_at_ms: Option<u64>,
    warm_state: Option<String>,
    warm_state_observed_at: Option<String>,
    first_visible_ms: Option<u64>,
    replacement_ms: Option<u64>,
    original_visible_to_preset_applied_visible_ms: Option<u64>,
    malformed: bool,
}

#[derive(Debug, Default)]
struct PreviewTransitionSummaryContext {
    observed_at: Option<String>,
    implementation_track: Option<String>,
    lane_owner: Option<String>,
    fallback_reason_code: Option<String>,
    route_stage: Option<String>,
    visible_owner: Option<String>,
    visible_owner_transition_at_ms: Option<u64>,
    warm_state: Option<String>,
    first_visible_ms: Option<u64>,
    replacement_ms: Option<u64>,
    original_visible_to_preset_applied_visible_ms: Option<u64>,
    saw_lane_owner: bool,
    saw_fallback_reason: bool,
    saw_route_stage: bool,
}

#[derive(Debug, Default)]
struct CaptureClientVisibleContext {
    visible_at_ms: Option<u64>,
}

#[derive(Debug, Default)]
struct PreviewTimingMetrics {
    first_visible_ms: Option<u64>,
    same_capture_full_screen_visible_ms: Option<u64>,
    replacement_ms: Option<u64>,
    original_visible_to_preset_applied_visible_ms: Option<u64>,
}

#[derive(Debug)]
struct SessionManifestCandidate {
    session_name: String,
    manifest_path: PathBuf,
    modified_at: Option<SystemTime>,
}

pub fn load_operator_session_summary_in_dir(
    base_dir: &Path,
    capability_snapshot: &CapabilitySnapshotDto,
) -> Result<OperatorSessionSummaryDto, HostErrorEnvelope> {
    ensure_operator_access(capability_snapshot)?;

    let Some((_manifest_path, manifest)) = load_current_session_manifest(base_dir)? else {
        return Ok(build_no_session_summary());
    };

    if !is_current_operator_session(&manifest) {
        return Ok(build_no_session_summary());
    }

    let manifest = project_session_timing(manifest, SystemTime::now())?;
    let manifest = project_post_end_state_in_dir(base_dir, manifest, SystemTime::now())?;
    let diagnostics = read_diagnostics_context(base_dir, &manifest.session_id);
    let readiness = normalize_capture_readiness(base_dir, &manifest);
    let latest_capture = readiness
        .latest_capture
        .as_ref()
        .or_else(|| manifest.captures.last());
    let live_capture_truth = readiness.live_capture_truth.clone();
    let reason_code = readiness.reason_code.clone();
    let render_status = latest_capture.map(|capture| capture.render_status.as_str());
    let capture_boundary = build_capture_boundary(&manifest, readiness.reason_code.as_str());
    let preview_render_boundary = build_preview_render_boundary(render_status);
    let completion_boundary = build_completion_boundary(
        manifest.post_end.as_ref().map(|post_end| post_end.state()),
        manifest.timing.as_ref().map(|timing| timing.phase.as_str()),
    );
    let camera_connection = build_camera_connection_summary(
        &manifest,
        readiness.reason_code.as_str(),
        live_capture_truth.as_ref(),
    );
    let blocked_state_category = derive_blocked_state_category(
        &capture_boundary,
        &preview_render_boundary,
        &completion_boundary,
        manifest.post_end.as_ref().map(|post_end| post_end.state()),
    );
    let recent_failure = build_recent_failure_summary(
        reason_code.as_str(),
        render_status,
        manifest.post_end.as_ref().map(|post_end| post_end.state()),
        manifest.timing.as_ref().map(|timing| timing.phase.as_str()),
        &diagnostics,
    );
    let preview_architecture = build_preview_architecture_summary(
        latest_capture,
        latest_capture
            .and_then(|capture| capture.preview_renderer_route.as_ref())
            .or(manifest.active_preview_renderer_route.as_ref()),
        manifest.active_preview_renderer_warm_state.as_ref(),
        &diagnostics,
    );

    Ok(OperatorSessionSummaryDto {
        schema_version: OPERATOR_SESSION_SUMMARY_SCHEMA_VERSION.into(),
        state: "session-loaded".into(),
        blocked_state_category: blocked_state_category.into(),
        session_id: Some(manifest.session_id.clone()),
        booth_alias: Some(manifest.booth_alias.clone()),
        active_preset_id: manifest.active_preset_id.clone(),
        active_preset_display_name: manifest.active_preset_display_name.clone(),
        active_preset_version: manifest
            .active_preset
            .as_ref()
            .map(|preset| preset.published_version.clone()),
        lifecycle_stage: Some(manifest.lifecycle.stage.clone()),
        timing_phase: manifest.timing.as_ref().map(|timing| timing.phase.clone()),
        updated_at: Some(manifest.updated_at.clone()),
        post_end_state: manifest
            .post_end
            .as_ref()
            .map(|post_end| post_end.state().into()),
        recent_failure,
        camera_connection,
        capture_boundary,
        preview_render_boundary,
        completion_boundary,
        preview_architecture,
        live_capture_truth,
    })
}

pub(crate) fn find_current_operator_session_id_in_dir(
    base_dir: &Path,
) -> Result<Option<String>, HostErrorEnvelope> {
    let Some((_manifest_path, manifest)) = load_current_session_manifest(base_dir)? else {
        return Ok(None);
    };

    if !is_current_operator_session(&manifest) {
        return Ok(None);
    }

    Ok(Some(manifest.session_id))
}

pub fn ensure_operator_window_label(window_label: &str) -> Result<(), HostErrorEnvelope> {
    if window_label == OPERATOR_WINDOW_LABEL {
        return Ok(());
    }

    Err(HostErrorEnvelope::capability_denied(
        "operator diagnostics는 operator 전용 창에서만 열 수 있어요.",
    ))
}

pub(crate) fn ensure_operator_access(
    capability_snapshot: &CapabilitySnapshotDto,
) -> Result<(), HostErrorEnvelope> {
    if capability_snapshot.is_admin_authenticated
        && capability_snapshot
            .allowed_surfaces
            .iter()
            .any(|surface| surface == "operator")
    {
        return Ok(());
    }

    Err(HostErrorEnvelope::capability_denied(
        "승인된 운영자 세션에서만 현재 세션 진단을 볼 수 있어요.",
    ))
}

fn build_no_session_summary() -> OperatorSessionSummaryDto {
    OperatorSessionSummaryDto {
        schema_version: OPERATOR_SESSION_SUMMARY_SCHEMA_VERSION.into(),
        state: "no-session".into(),
        blocked_state_category: "not-blocked".into(),
        session_id: None,
        booth_alias: None,
        active_preset_id: None,
        active_preset_display_name: None,
        active_preset_version: None,
        lifecycle_stage: None,
        timing_phase: None,
        updated_at: None,
        post_end_state: None,
        recent_failure: None,
        camera_connection: OperatorCameraConnectionSummaryDto {
            state: "disconnected".into(),
            title: "세션 없음".into(),
            detail: "진행 중인 세션이 없어 카메라 연결 상태를 아직 판단하지 않았어요.".into(),
            observed_at: None,
        },
        capture_boundary: clear_boundary(
            "현재 세션 없음",
            "진행 중인 세션이 없어 capture 경계를 아직 판단하지 않았어요.",
        ),
        preview_render_boundary: clear_boundary(
            "최근 결과 없음",
            "진행 중인 세션이 없어 preview/render 경계도 아직 비어 있어요.",
        ),
        completion_boundary: clear_boundary(
            "후처리 경계 비어 있음",
            "현재 세션이 시작되면 completion 경계 진단을 함께 보여 드릴게요.",
        ),
        preview_architecture: build_preview_architecture_summary(
            None,
            None,
            None,
            &DiagnosticsContext::default(),
        ),
        live_capture_truth: None,
    }
}

fn build_camera_connection_summary(
    manifest: &SessionManifest,
    reason_code: &str,
    live_capture_truth: Option<&LiveCaptureTruthDto>,
) -> OperatorCameraConnectionSummaryDto {
    let observed_at = live_capture_truth.and_then(|truth| truth.observed_at.clone());
    let connection_state =
        derive_camera_connection_state(manifest, reason_code, live_capture_truth);

    match connection_state {
        "disconnected" => OperatorCameraConnectionSummaryDto {
            state: connection_state.into(),
            title: "카메라 연결이 아직 확인되지 않았어요.".into(),
            detail: "카메라 전원과 연결 상태를 먼저 점검해 주세요.".into(),
            observed_at,
        },
        "connecting" => OperatorCameraConnectionSummaryDto {
            state: connection_state.into(),
            title: "카메라 연결을 확인하는 중이에요.".into(),
            detail: "helper와 카메라 연결 상태를 정규화하는 동안 잠시만 기다려 주세요.".into(),
            observed_at,
        },
        "connected" => OperatorCameraConnectionSummaryDto {
            state: connection_state.into(),
            title: "카메라와 helper 연결이 확인됐어요.".into(),
            detail: "카메라와 helper가 현재 세션 기준으로 연결된 상태예요.".into(),
            observed_at,
        },
        _ => {
            let detail = match live_capture_truth {
                Some(truth)
                    if truth.freshness != "fresh" || truth.session_match != "matched" =>
                {
                    "최근 helper 상태가 현재 세션과 맞지 않거나 오래돼 연결 진실을 다시 확인해야 해요."
                }
                Some(truth)
                    if matches!(
                        truth.camera_state.as_str(),
                        "recovering" | "degraded" | "error"
                    ) || matches!(
                        truth.helper_state.as_str(),
                        "recovering" | "degraded" | "error"
                    ) =>
                {
                    "카메라 또는 helper 연결이 흔들려 복구가 필요한 상태예요."
                }
                _ => "카메라 연결 신호가 기대한 준비 상태와 달라 복구 여부를 확인해 주세요.",
            };

            OperatorCameraConnectionSummaryDto {
                state: "recovery-required".into(),
                title: "카메라 연결 복구가 필요해요.".into(),
                detail: detail.into(),
                observed_at,
            }
        }
    }
}

fn derive_camera_connection_state(
    manifest: &SessionManifest,
    reason_code: &str,
    live_capture_truth: Option<&LiveCaptureTruthDto>,
) -> &'static str {
    let has_connected_context = has_connected_camera_context(manifest);
    let Some(live_capture_truth) = live_capture_truth else {
        return if has_connected_context {
            "recovery-required"
        } else {
            "connecting"
        };
    };

    if live_capture_truth.freshness != "fresh" || live_capture_truth.session_match != "matched" {
        return if has_connected_context {
            "recovery-required"
        } else {
            "connecting"
        };
    }

    if let Some(connection_state) = live_capture_truth
        .detail_code
        .as_deref()
        .and_then(map_camera_connection_state_from_detail_code)
    {
        return connection_state;
    }

    if matches!(
        live_capture_truth.camera_state.as_str(),
        "degraded" | "error" | "recovering"
    ) || matches!(
        live_capture_truth.helper_state.as_str(),
        "degraded" | "error" | "recovering"
    ) {
        return "recovery-required";
    }

    if live_capture_truth.camera_state == "disconnected" {
        return "disconnected";
    }

    if matches!(
        (
            live_capture_truth.camera_state.as_str(),
            live_capture_truth.helper_state.as_str()
        ),
        ("ready", "healthy") | ("connected-idle", "healthy") | ("capturing", "healthy")
    ) {
        return "connected";
    }

    if reason_code == "phone-required" && has_connected_context {
        return "recovery-required";
    }

    if matches!(reason_code, "camera-preparing" | "helper-preparing")
        || matches!(
            live_capture_truth.camera_state.as_str(),
            "connecting" | "connected-idle" | "unknown"
        )
        || matches!(
            live_capture_truth.helper_state.as_str(),
            "starting" | "connecting" | "unknown"
        )
    {
        return "connecting";
    }

    if has_connected_context {
        "recovery-required"
    } else {
        "disconnected"
    }
}

fn map_camera_connection_state_from_detail_code(detail_code: &str) -> Option<&'static str> {
    match detail_code {
        "camera-not-found" | "usb-disconnected" | "unsupported-camera" => Some("disconnected"),
        "sdk-initializing" | "session-opening" => Some("connecting"),
        "connected-idle" | "camera-ready" => Some("connected"),
        "reconnect-pending" | "sdk-init-failed" => Some("recovery-required"),
        _ => None,
    }
}

fn has_connected_camera_context(manifest: &SessionManifest) -> bool {
    if !manifest.captures.is_empty() {
        return true;
    }

    matches!(
        manifest.lifecycle.stage.as_str(),
        "ready"
            | "capture-ready"
            | "preview-waiting"
            | "warning"
            | "ended"
            | "export-waiting"
            | "completed"
            | "phone-required"
    )
}

fn load_current_session_manifest(
    base_dir: &Path,
) -> Result<Option<(PathBuf, SessionManifest)>, HostErrorEnvelope> {
    let sessions_root = base_dir.join("sessions");

    if !sessions_root.exists() {
        return Ok(None);
    }

    let session_dirs = fs::read_dir(&sessions_root).map_err(|error| {
        HostErrorEnvelope::persistence(format!("현재 세션 목록을 읽지 못했어요: {error}"))
    })?;
    let mut candidates = Vec::new();

    for entry in session_dirs {
        let session_root = match entry {
            Ok(entry) => entry.path(),
            Err(_) => continue,
        };

        if !session_root.is_dir() {
            continue;
        }

        let Some(session_name) = session_root
            .file_name()
            .and_then(|value| value.to_str())
            .map(str::to_string)
        else {
            continue;
        };

        if session_name.starts_with(".creating-") {
            continue;
        }

        let manifest_path = session_root.join("session.json");
        if !manifest_path.is_file() {
            continue;
        }

        let modified_at = fs::metadata(&manifest_path)
            .ok()
            .and_then(|metadata| metadata.modified().ok());
        candidates.push(SessionManifestCandidate {
            session_name,
            manifest_path,
            modified_at,
        });
    }

    candidates.sort_by(compare_session_manifest_candidates);

    for candidate in candidates {
        let manifest = read_session_manifest(&candidate.manifest_path)?;

        return Ok(Some((candidate.manifest_path, manifest)));
    }

    Ok(None)
}

fn compare_session_manifest_candidates(
    left: &SessionManifestCandidate,
    right: &SessionManifestCandidate,
) -> Ordering {
    match (left.modified_at, right.modified_at) {
        (Some(left_modified_at), Some(right_modified_at)) => right_modified_at
            .cmp(&left_modified_at)
            .then_with(|| right.session_name.cmp(&left.session_name)),
        (Some(_), None) => Ordering::Less,
        (None, Some(_)) => Ordering::Greater,
        (None, None) => right.session_name.cmp(&left.session_name),
    }
}

fn is_current_operator_session(manifest: &SessionManifest) -> bool {
    !matches!(
        manifest.post_end.as_ref().map(|post_end| post_end.state()),
        Some(SESSION_POST_END_COMPLETED)
    ) && manifest.lifecycle.stage != SESSION_POST_END_COMPLETED
}

fn read_diagnostics_context(base_dir: &Path, session_id: &str) -> DiagnosticsContext {
    let log_path = SessionPaths::new(base_dir, session_id)
        .diagnostics_dir
        .join("timing-events.log");

    if !log_path.is_file() {
        return DiagnosticsContext::default();
    }

    let Ok(contents) = fs::read_to_string(log_path) else {
        return DiagnosticsContext {
            malformed: true,
            ..DiagnosticsContext::default()
        };
    };
    let lines = contents
        .lines()
        .rev()
        .filter(|line| !line.trim().is_empty())
        .collect::<Vec<_>>();
    let Some(last_line) = lines.first().copied() else {
        return DiagnosticsContext::default();
    };
    let parts: Vec<_> = last_line.split('\t').collect();
    if parts.is_empty() {
        return DiagnosticsContext {
            malformed: true,
            ..DiagnosticsContext::default()
        };
    }
    if rfc3339_to_unix_seconds(parts[0]).is_err() {
        return DiagnosticsContext {
            malformed: true,
            ..DiagnosticsContext::default()
        };
    }

    let mut context = DiagnosticsContext {
        observed_at: Some(parts[0].to_string()),
        ..DiagnosticsContext::default()
    };

    for part in parts.into_iter().skip(1) {
        let Some((key, value)) = part.split_once('=') else {
            context.malformed = true;
            return context;
        };

        match key {
            "event" => context.event_name = Some(value.to_string()),
            "state" => context.post_end_state = Some(value.to_string()),
            _ => {}
        }
    }

    for line in &lines {
        let Some(visible_context) = parse_capture_client_visible_context(line) else {
            continue;
        };

        if visible_context.visible_at_ms.is_some() {
            context.visible_owner_transition_at_ms = visible_context.visible_at_ms;
            break;
        }
    }

    for line in &lines {
        let Some(summary) = parse_preview_transition_summary_context(line) else {
            continue;
        };

        if summary.saw_lane_owner && summary.saw_fallback_reason && summary.saw_route_stage {
            context.implementation_track = summary.implementation_track;
            context.lane_owner = summary.lane_owner;
            context.fallback_reason_code = summary.fallback_reason_code;
            context.route_stage = summary.route_stage;
            context.visible_owner = summary.visible_owner;
            context.visible_owner_transition_at_ms = context
                .visible_owner_transition_at_ms
                .or(summary.visible_owner_transition_at_ms);
            context.warm_state = summary.warm_state;
            context.warm_state_observed_at = summary.observed_at;
            context.first_visible_ms = summary.first_visible_ms;
            context.replacement_ms = summary.replacement_ms;
            context.original_visible_to_preset_applied_visible_ms =
                summary.original_visible_to_preset_applied_visible_ms;
            break;
        }
    }

    context
}

fn build_preview_architecture_summary(
    latest_capture: Option<&SessionCaptureRecord>,
    route_snapshot: Option<&PreviewRendererRouteSnapshot>,
    warm_state_snapshot: Option<&PreviewRendererWarmStateSnapshot>,
    diagnostics: &DiagnosticsContext,
) -> OperatorPreviewArchitectureSummaryDto {
    let prefer_snapshot_warm_state = warm_state_snapshot
        .map(|snapshot| {
            let Some(snapshot_sort_key) = rfc3339_sort_key_lossy(&snapshot.observed_at) else {
                return diagnostics.warm_state.is_none();
            };
            let Some(diagnostics_sort_key) = diagnostics
                .warm_state_observed_at
                .as_deref()
                .and_then(rfc3339_sort_key_lossy)
            else {
                return true;
            };
            snapshot_sort_key > diagnostics_sort_key
        })
        .unwrap_or(false);
    let warm_state = if prefer_snapshot_warm_state {
        warm_state_snapshot.map(|snapshot| snapshot.state.clone())
    } else {
        diagnostics
            .warm_state
            .clone()
            .or_else(|| warm_state_snapshot.map(|snapshot| snapshot.state.clone()))
    };
    let warm_state_observed_at = if prefer_snapshot_warm_state {
        warm_state_snapshot.map(|snapshot| snapshot.observed_at.clone())
    } else {
        diagnostics
            .warm_state_observed_at
            .clone()
            .or_else(|| warm_state_snapshot.map(|snapshot| snapshot.observed_at.clone()))
    };
    let timing_metrics = build_preview_timing_metrics(latest_capture, diagnostics);
    let implementation_track = diagnostics
        .implementation_track
        .clone()
        .or_else(|| route_snapshot.and_then(|snapshot| snapshot.implementation_track.clone()));
    let normalized_route = route_snapshot.map(|snapshot| {
        normalize_preview_architecture_route(
            snapshot.route.as_str(),
            implementation_track.as_deref(),
        )
        .to_string()
    });

    OperatorPreviewArchitectureSummaryDto {
        route: normalized_route,
        route_stage: diagnostics
            .route_stage
            .clone()
            .or_else(|| route_snapshot.map(|snapshot| snapshot.route_stage.clone())),
        implementation_track,
        lane_owner: diagnostics.lane_owner.clone(),
        fallback_reason_code: diagnostics
            .fallback_reason_code
            .clone()
            .or_else(|| route_snapshot.and_then(|snapshot| snapshot.fallback_reason_code.clone())),
        capture_id: latest_capture.map(|capture| capture.capture_id.clone()),
        request_id: latest_capture.map(|capture| capture.request_id.clone()),
        visible_owner: diagnostics
            .visible_owner
            .clone()
            .or_else(|| diagnostics.lane_owner.clone()),
        visible_owner_transition_at_ms: diagnostics
            .visible_owner_transition_at_ms
            .or_else(|| {
                latest_capture.and_then(|capture| {
                    capture
                        .timing
                        .preview_visible_at_ms
                        .or(capture.timing.xmp_preview_ready_at_ms)
                })
            }),
        warm_state,
        warm_state_observed_at,
        first_visible_ms: timing_metrics.first_visible_ms,
        same_capture_full_screen_visible_ms: timing_metrics.same_capture_full_screen_visible_ms,
        replacement_ms: timing_metrics.replacement_ms,
        original_visible_to_preset_applied_visible_ms: timing_metrics
            .original_visible_to_preset_applied_visible_ms,
        hardware_capability: dedicated_renderer_hardware_capability().into(),
    }
}

fn normalize_preview_architecture_route<'a>(
    route: &'a str,
    implementation_track: Option<&str>,
) -> &'a str {
    match implementation_track {
        Some("actual-primary-lane") => "actual-primary-lane",
        Some("prototype-track") if route == "actual-primary-lane" => "local-renderer-sidecar",
        _ => route,
    }
}

fn build_preview_timing_metrics(
    latest_capture: Option<&SessionCaptureRecord>,
    diagnostics: &DiagnosticsContext,
) -> PreviewTimingMetrics {
    let capture_metrics = latest_capture
        .map(build_preview_timing_metrics_from_capture)
        .unwrap_or_default();
    let diagnostics_same_capture_full_screen_visible_ms = latest_capture.and_then(|capture| {
        diagnostics
            .visible_owner_transition_at_ms
            .map(|visible_at_ms| {
                visible_at_ms.saturating_sub(capture.timing.capture_acknowledged_at_ms)
            })
    });

    PreviewTimingMetrics {
        first_visible_ms: capture_metrics
            .first_visible_ms
            .or(diagnostics.first_visible_ms),
        same_capture_full_screen_visible_ms: diagnostics_same_capture_full_screen_visible_ms
            .or(capture_metrics.same_capture_full_screen_visible_ms)
            .or(diagnostics.replacement_ms),
        replacement_ms: diagnostics_same_capture_full_screen_visible_ms
            .or(capture_metrics.replacement_ms)
            .or(diagnostics.replacement_ms),
        original_visible_to_preset_applied_visible_ms: capture_metrics
            .original_visible_to_preset_applied_visible_ms
            .or(diagnostics.original_visible_to_preset_applied_visible_ms),
    }
}

fn build_preview_timing_metrics_from_capture(
    latest_capture: &SessionCaptureRecord,
) -> PreviewTimingMetrics {
    let acknowledged_at_ms = latest_capture.timing.capture_acknowledged_at_ms;
    let first_visible_at_ms = latest_capture
        .timing
        .fast_preview_visible_at_ms
        .or(latest_capture.timing.preview_visible_at_ms);
    let replacement_visible_at_ms = latest_capture
        .timing
        .preview_visible_at_ms
        .or(latest_capture.timing.xmp_preview_ready_at_ms);
    let first_visible_ms =
        first_visible_at_ms.map(|visible_at_ms| visible_at_ms.saturating_sub(acknowledged_at_ms));
    let replacement_ms = replacement_visible_at_ms
        .map(|visible_at_ms| visible_at_ms.saturating_sub(acknowledged_at_ms));

    PreviewTimingMetrics {
        first_visible_ms,
        same_capture_full_screen_visible_ms: replacement_ms,
        replacement_ms,
        original_visible_to_preset_applied_visible_ms: match (first_visible_ms, replacement_ms) {
            (Some(first_visible_ms), Some(replacement_ms)) => {
                Some(replacement_ms.saturating_sub(first_visible_ms))
            }
            _ => None,
        },
    }
}

fn rfc3339_sort_key_lossy(timestamp: &str) -> Option<(u64, u32)> {
    let seconds = rfc3339_to_unix_seconds(timestamp).ok()?;
    let time = timestamp.trim().split_once('T')?.1;
    let fraction = time
        .split_once('.')
        .and_then(|(_, fraction_and_offset)| {
            let digits = fraction_and_offset
                .chars()
                .take_while(|char| char.is_ascii_digit())
                .collect::<String>();
            if digits.is_empty() {
                None
            } else {
                Some(digits)
            }
        })
        .map(|digits| {
            let truncated = if digits.len() > 9 {
                &digits[..9]
            } else {
                digits.as_str()
            };
            let parsed = truncated.parse::<u32>().ok()?;
            let scale = 10_u32.pow(9_u32.saturating_sub(truncated.len() as u32));
            Some(parsed.saturating_mul(scale))
        })
        .unwrap_or(Some(0))?;

    Some((seconds, fraction))
}

fn normalize_diagnostics_value(value: &str) -> Option<String> {
    match value.trim() {
        "" | "none" => None,
        normalized => Some(normalized.to_string()),
    }
}

fn parse_preview_metric(value: &str) -> Option<u64> {
    value.trim().parse::<u64>().ok()
}

fn parse_preview_transition_summary_context(line: &str) -> Option<PreviewTransitionSummaryContext> {
    if !line.contains("\tevent=capture_preview_transition_summary") {
        return None;
    }

    let observed_at = line.split('\t').next()?.to_string();
    if rfc3339_to_unix_seconds(&observed_at).is_err() {
        return None;
    }
    let detail = line.split("\tdetail=").nth(1)?;
    let mut summary = PreviewTransitionSummaryContext {
        observed_at: Some(observed_at),
        ..PreviewTransitionSummaryContext::default()
    };

    for fragment in detail.split(';') {
        let Some((key, value)) = fragment.split_once('=') else {
            continue;
        };

        match key {
            "laneOwner" => {
                summary.saw_lane_owner = true;
                summary.lane_owner = normalize_diagnostics_value(value);
            }
            "implementationTrack" => {
                summary.implementation_track = normalize_diagnostics_value(value);
            }
            "fallbackReason" => {
                summary.saw_fallback_reason = true;
                summary.fallback_reason_code = normalize_diagnostics_value(value);
            }
            "routeStage" => {
                summary.saw_route_stage = true;
                summary.route_stage = normalize_diagnostics_value(value);
            }
            "visibleOwner" => {
                summary.visible_owner = normalize_diagnostics_value(value);
            }
            "visibleOwnerTransitionAtMs" => {
                summary.visible_owner_transition_at_ms = parse_preview_metric(value);
            }
            "warmState" => {
                summary.warm_state = normalize_diagnostics_value(value);
            }
            "firstVisibleMs" => {
                summary.first_visible_ms = parse_preview_metric(value);
            }
            "replacementMs" => {
                summary.replacement_ms = parse_preview_metric(value);
            }
            "originalVisibleToPresetAppliedVisibleMs" => {
                summary.original_visible_to_preset_applied_visible_ms = parse_preview_metric(value);
            }
            _ => {}
        }
    }

    Some(summary)
}

fn parse_capture_client_visible_context(line: &str) -> Option<CaptureClientVisibleContext> {
    if !line.contains("\tevent=recent-session-visible")
        && !line.contains("\tevent=current-session-preview-visible")
    {
        return None;
    }

    let detail = line
        .split('\t')
        .find_map(|fragment| fragment.strip_prefix("detail="))?;
    let ready_at_ms = detail
        .split(';')
        .filter_map(|fragment| fragment.split_once('='))
        .find_map(|(key, value)| (key == "readyAtMs").then(|| value.parse::<u64>().ok()))
        .flatten();
    let ui_lag_ms = detail
        .split(';')
        .filter_map(|fragment| fragment.split_once('='))
        .find_map(|(key, value)| (key == "uiLagMs").then(|| value.parse::<u64>().ok()))
        .flatten();

    Some(CaptureClientVisibleContext {
        visible_at_ms: match (ready_at_ms, ui_lag_ms) {
            (Some(ready_at_ms), Some(ui_lag_ms)) => Some(ready_at_ms.saturating_add(ui_lag_ms)),
            _ => None,
        },
    })
}

fn build_capture_boundary(
    manifest: &SessionManifest,
    reason_code: &str,
) -> OperatorBoundarySummaryDto {
    match reason_code {
        "preset-missing" => blocked_boundary(
            "캡처 경계 준비 안 됨",
            "활성 preset이 없어 아직 촬영을 받을 수 없어요.",
        ),
        "camera-preparing" | "helper-preparing" => blocked_boundary(
            "캡처 준비 중",
            "촬영 경계가 아직 열리지 않았어요. 준비 상태를 다시 확인해 주세요.",
        ),
        _ if manifest.active_preset.is_some() => clear_boundary(
            "캡처 경계 정상",
            "활성 preset이 선택돼 있어 capture 경계는 열려 있어요.",
        ),
        _ => clear_boundary(
            "캡처 대기 중",
            "아직 최근 세션 문맥만 확인 중이라 capture 경계는 막히지 않았어요.",
        ),
    }
}

fn build_preview_render_boundary(render_status: Option<&str>) -> OperatorBoundarySummaryDto {
    match render_status {
        Some("captureSaved") | Some("previewWaiting") => blocked_boundary(
            "프리뷰/렌더 결과 준비 지연",
            "가장 최근 촬영본은 저장되었지만 preview/render 결과가 아직 준비되지 않았어요.",
        ),
        Some("renderFailed") => blocked_boundary(
            "프리뷰/렌더 결과 준비 실패",
            "가장 최근 촬영본의 결과를 만들지 못해 직원 확인이 필요해요.",
        ),
        Some("previewReady") | Some("finalReady") => clear_boundary(
            "프리뷰/렌더 경계 정상",
            "가장 최근 촬영본의 결과 준비가 끝나 있어요.",
        ),
        _ => clear_boundary(
            "최근 렌더 없음",
            "아직 확인할 최근 촬영 기록이 없어 preview/render 경계는 비어 있어요.",
        ),
    }
}

fn build_completion_boundary(
    post_end_state: Option<&str>,
    timing_phase: Option<&str>,
) -> OperatorBoundarySummaryDto {
    match post_end_state {
        Some(SESSION_POST_END_EXPORT_WAITING) => blocked_boundary(
            "완료 판정 대기",
            "촬영은 끝났고 결과와 다음 안내를 정리하는 중이에요.",
        ),
        Some(SESSION_POST_END_PHONE_REQUIRED) => blocked_boundary(
            "종료 후 직원 확인 필요",
            "종료 후 처리 단계에서 자동 완료로 넘어가지 못했어요.",
        ),
        Some(SESSION_POST_END_COMPLETED) => clear_boundary(
            "완료 경계 정상",
            "종료 후 안내가 확정돼 있어 completion 경계는 정리된 상태예요.",
        ),
        _ if timing_phase == Some("ended") => blocked_boundary(
            "종료 후 상태 확인 필요",
            "세션 시간은 끝났지만 종료 후 안내 상태가 아직 확정되지 않았어요.",
        ),
        _ => clear_boundary(
            "완료 경계 대기 전",
            "아직 종료 후 완료 경계로 들어가지 않았어요.",
        ),
    }
}

fn derive_blocked_state_category(
    capture_boundary: &OperatorBoundarySummaryDto,
    preview_render_boundary: &OperatorBoundarySummaryDto,
    completion_boundary: &OperatorBoundarySummaryDto,
    post_end_state: Option<&str>,
) -> &'static str {
    if completion_boundary.status == "blocked"
        || matches!(
            post_end_state,
            Some(SESSION_POST_END_EXPORT_WAITING) | Some(SESSION_POST_END_PHONE_REQUIRED)
        )
    {
        return "timing-post-end-blocked";
    }

    if capture_boundary.status == "blocked" {
        return "capture-blocked";
    }

    if preview_render_boundary.status == "blocked" {
        return "preview-render-blocked";
    }

    "not-blocked"
}

fn build_recent_failure_summary(
    reason_code: &str,
    render_status: Option<&str>,
    post_end_state: Option<&str>,
    timing_phase: Option<&str>,
    diagnostics: &DiagnosticsContext,
) -> Option<OperatorRecentFailureSummaryDto> {
    if diagnostics.malformed {
        return Some(OperatorRecentFailureSummaryDto {
            title: "최근 진단 로그를 복원하지 못했어요.".into(),
            detail:
                "로그 형식이 올바르지 않아 현재 세션 요약만 기준으로 운영자 진단을 보여 주고 있어요."
                    .into(),
            observed_at: None,
        });
    }

    if matches!(
        post_end_state,
        Some(SESSION_POST_END_EXPORT_WAITING) | Some(SESSION_POST_END_PHONE_REQUIRED)
    ) || timing_phase == Some("ended")
    {
        return Some(build_completion_recent_failure(
            post_end_state,
            timing_phase,
            diagnostics,
        ));
    }

    match reason_code {
        "preset-missing" => {
            return Some(OperatorRecentFailureSummaryDto {
                title: "활성 preset이 아직 선택되지 않았어요.".into(),
                detail: "세션은 열려 있지만 현재 촬영을 시작할 preset 바인딩이 비어 있어요.".into(),
                observed_at: None,
            })
        }
        "camera-preparing" | "helper-preparing" => {
            return Some(OperatorRecentFailureSummaryDto {
                title: "촬영 준비가 아직 끝나지 않았어요.".into(),
                detail: "capture 경계가 아직 열리지 않아 booth가 준비 상태를 기다리고 있어요."
                    .into(),
                observed_at: None,
            })
        }
        "phone-required" if render_status.is_none() && post_end_state.is_none() => {
            return Some(OperatorRecentFailureSummaryDto {
                title: "최근 촬영을 세션에 저장하지 못했어요.".into(),
                detail: "셔터 동작 뒤 RAW handoff를 확인하지 못해 운영자 확인이 필요한 상태예요."
                    .into(),
                observed_at: None,
            })
        }
        _ => {}
    }

    match render_status {
        Some("captureSaved") | Some("previewWaiting") => Some(OperatorRecentFailureSummaryDto {
            title: "가장 최근 촬영본 결과 준비가 지연되고 있어요.".into(),
            detail:
                "원본 저장은 끝났지만 preview/render 결과가 아직 준비되지 않아 다음 안내가 멈춰 있어요."
                    .into(),
            observed_at: None,
        }),
        Some("renderFailed") => Some(OperatorRecentFailureSummaryDto {
            title: "가장 최근 촬영본 결과 준비가 실패했어요.".into(),
            detail: "preview/render 결과를 만들지 못해 직원 확인이 필요한 상태예요.".into(),
            observed_at: None,
        }),
        _ => None,
    }
}

fn build_completion_recent_failure(
    post_end_state: Option<&str>,
    timing_phase: Option<&str>,
    diagnostics: &DiagnosticsContext,
) -> OperatorRecentFailureSummaryDto {
    let observed_at = match diagnostics.event_name.as_deref() {
        Some("ended") | Some("post-end-evaluated") => diagnostics.observed_at.clone(),
        _ => None,
    };

    match post_end_state {
        Some(SESSION_POST_END_EXPORT_WAITING) => OperatorRecentFailureSummaryDto {
            title: "종료 후 완료 판정이 아직 보류돼 있어요.".into(),
            detail: "세션 종료는 감지됐지만 결과와 다음 안내가 아직 completed로 확정되지 않았어요."
                .into(),
            observed_at,
        },
        Some(SESSION_POST_END_PHONE_REQUIRED) => OperatorRecentFailureSummaryDto {
            title: "종료 후 자동 완료가 멈춰 직원 확인이 필요해요.".into(),
            detail: "종료 후 처리 단계에서 phone-required로 분류돼 운영자 확인이 필요한 상태예요."
                .into(),
            observed_at,
        },
        _ if timing_phase == Some("ended") => OperatorRecentFailureSummaryDto {
            title: "세션 종료 후 상태 확정이 아직 남아 있어요.".into(),
            detail: "세션 시간은 끝났지만 종료 후 안내 상태가 아직 export-waiting 또는 completed로 확정되지 않았어요."
                .into(),
            observed_at,
        },
        _ => OperatorRecentFailureSummaryDto {
            title: "종료 후 상태를 다시 확인하고 있어요.".into(),
            detail: "최근 종료 후 진단을 바탕으로 completion 경계를 다시 계산하는 중이에요."
                .into(),
            observed_at,
        },
    }
}

fn clear_boundary(title: &str, detail: &str) -> OperatorBoundarySummaryDto {
    OperatorBoundarySummaryDto {
        status: "clear".into(),
        title: title.into(),
        detail: detail.into(),
    }
}

fn blocked_boundary(title: &str, detail: &str) -> OperatorBoundarySummaryDto {
    OperatorBoundarySummaryDto {
        status: "blocked".into(),
        title: title.into(),
        detail: detail.into(),
    }
}
