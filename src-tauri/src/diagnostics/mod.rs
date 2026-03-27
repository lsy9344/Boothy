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
        CapabilitySnapshotDto, HostErrorEnvelope, OperatorBoundarySummaryDto,
        OperatorRecentFailureSummaryDto, OperatorSessionSummaryDto,
    },
    handoff::project_post_end_state_in_dir,
    session::{
        session_manifest::{
            rfc3339_to_unix_seconds, SessionManifest, SESSION_POST_END_COMPLETED,
            SESSION_POST_END_EXPORT_WAITING, SESSION_POST_END_PHONE_REQUIRED,
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
    malformed: bool,
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
    let reason_code = readiness.reason_code.clone();
    let render_status = readiness
        .latest_capture
        .as_ref()
        .map(|capture| capture.render_status.as_str());
    let capture_boundary = build_capture_boundary(&manifest, readiness.reason_code.as_str());
    let preview_render_boundary = build_preview_render_boundary(render_status);
    let completion_boundary = build_completion_boundary(
        manifest.post_end.as_ref().map(|post_end| post_end.state()),
        manifest.timing.as_ref().map(|timing| timing.phase.as_str()),
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
        capture_boundary,
        preview_render_boundary,
        completion_boundary,
    })
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
    }
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
    let log_path = match SessionPaths::try_new(base_dir, session_id) {
        Ok(paths) => paths.diagnostics_dir.join("timing-events.log"),
        Err(_) => return DiagnosticsContext::default(),
    };

    if !log_path.is_file() {
        return DiagnosticsContext::default();
    }

    let Ok(contents) = fs::read_to_string(log_path) else {
        return DiagnosticsContext {
            malformed: true,
            ..DiagnosticsContext::default()
        };
    };
    let Some(last_line) = contents.lines().rev().find(|line| !line.trim().is_empty()) else {
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

    context
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
