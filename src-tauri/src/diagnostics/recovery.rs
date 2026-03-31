use std::{
    fs,
    path::{Path, PathBuf},
    time::SystemTime,
};

use crate::{
    capture::{
        ingest_pipeline::complete_preview_render_in_dir,
        normalized_state::get_capture_readiness_in_dir,
    },
    contracts::dto::{
        validate_operator_recovery_action_input, CapabilitySnapshotDto, CaptureReadinessInputDto,
        HostErrorEnvelope, OperatorRecoveryActionInputDto, OperatorRecoveryActionResultDto,
        OperatorRecoveryDiagnosticsSummaryDto, OperatorRecoveryNextStateDto,
        OperatorRecoverySummaryDto, OperatorSessionSummaryDto,
    },
    diagnostics::{
        audit_log::{try_append_operator_audit_record, OperatorAuditRecordInput},
        load_operator_session_summary_in_dir,
    },
    handoff::sync_post_end_state_in_dir,
    session::{
        session_manifest::{
            current_timestamp, rfc3339_to_unix_seconds, unix_seconds_to_rfc3339, SessionManifest,
            SessionPostEnd, SESSION_POST_END_PHONE_REQUIRED, WARNING_LEAD_SECONDS,
        },
        session_paths::SessionPaths,
        session_repository::{read_session_manifest, write_session_manifest},
    },
    timing::{evaluate_phase, sync_session_timing_in_dir, TimingPhase},
};

const OPERATOR_RECOVERY_SUMMARY_SCHEMA_VERSION: &str = "operator-recovery-summary/v1";
const OPERATOR_RECOVERY_ACTION_RESULT_SCHEMA_VERSION: &str = "operator-recovery-action-result/v1";
const APPROVED_EXTENSION_MINUTES: u32 = 5;
const ROUTE_PHONE_REQUIRED_PRIMARY_ACTION: &str = "가까운 직원에게 알려 주세요.";
const ROUTE_PHONE_REQUIRED_SUPPORT_ACTION: &str = "직원에게 도움을 요청해 주세요.";
const ROUTE_PHONE_REQUIRED_WARNING: &str = "다시 찍기나 기기 조작은 잠시 멈춰 주세요.";

pub fn load_operator_recovery_summary_in_dir(
    base_dir: &Path,
    capability_snapshot: &CapabilitySnapshotDto,
) -> Result<OperatorRecoverySummaryDto, HostErrorEnvelope> {
    let session_summary = load_operator_session_summary_in_dir(base_dir, capability_snapshot)?;

    Ok(build_operator_recovery_summary(session_summary))
}

pub fn execute_operator_recovery_action_in_dir(
    base_dir: &Path,
    capability_snapshot: &CapabilitySnapshotDto,
    input: OperatorRecoveryActionInputDto,
) -> Result<OperatorRecoveryActionResultDto, HostErrorEnvelope> {
    validate_operator_recovery_action_input(&input)?;

    let current_summary = load_operator_recovery_summary_in_dir(base_dir, capability_snapshot)?;
    let requested_session_id = input.session_id.clone();
    let action = input.action.clone();

    if current_summary.session_id.as_deref() != Some(requested_session_id.as_str()) {
        return build_rejected_result_without_audit(
            base_dir,
            requested_session_id,
            action,
            "session-mismatch",
            "현재 operator 화면과 다른 세션 응답이 돌아와 액션을 실행하지 않았어요.",
            current_summary,
        );
    }

    if !current_summary
        .allowed_actions
        .iter()
        .any(|allowed_action| allowed_action == &input.action)
    {
        return build_rejected_result(
            base_dir,
            requested_session_id,
            action,
            "action-not-allowed",
            "이 세션 범주에서는 선택한 복구 액션을 실행할 수 없어요.",
            current_summary,
        );
    }

    match input.action.as_str() {
        "retry" => execute_retry(base_dir, capability_snapshot, input, current_summary),
        "approved-boundary-restart" => {
            execute_boundary_restart(base_dir, capability_snapshot, input, current_summary)
        }
        "approved-time-extension" => {
            execute_time_extension(base_dir, capability_snapshot, input, current_summary)
        }
        "route-phone-required" => {
            execute_phone_required_route(base_dir, capability_snapshot, input, current_summary)
        }
        _ => build_rejected_result(
            base_dir,
            requested_session_id,
            action,
            "action-not-allowed",
            "이 세션 범주에서는 선택한 복구 액션을 실행할 수 없어요.",
            current_summary,
        ),
    }
}

fn execute_retry(
    base_dir: &Path,
    capability_snapshot: &CapabilitySnapshotDto,
    input: OperatorRecoveryActionInputDto,
    current_summary: OperatorRecoverySummaryDto,
) -> Result<OperatorRecoveryActionResultDto, HostErrorEnvelope> {
    match current_summary.blocked_category.as_deref() {
        Some("preview-or-render") => {
            let manifest = read_live_manifest(base_dir, &input.session_id)?;
            let Some(latest_capture) = manifest.captures.last() else {
                return build_rejected_result(
                    base_dir,
                    input.session_id,
                    input.action,
                    "recovery-unavailable",
                    "다시 시도할 최근 촬영 문맥이 없어 안전한 render 재시도를 시작하지 않았어요.",
                    current_summary,
                );
            };

            let _ = complete_preview_render_in_dir(
                base_dir,
                &manifest.session_id,
                &latest_capture.capture_id,
            )?;

            let refreshed_summary =
                load_operator_recovery_summary_in_dir(base_dir, capability_snapshot)?;

            build_applied_result(
                base_dir,
                input.session_id,
                input.action,
                "현재 막힌 preview/render 경계를 다시 시도했어요.",
                refreshed_summary,
            )
        }
        Some("timing-or-post-end") => {
            let _ = read_live_manifest(base_dir, &input.session_id)?;
            let refreshed_summary =
                load_operator_recovery_summary_in_dir(base_dir, capability_snapshot)?;

            if refreshed_summary.blocked_category.is_none() {
                return build_applied_result(
                    base_dir,
                    input.session_id,
                    input.action,
                    "종료 후 완료 판정을 다시 확인해 다음 상태를 갱신했어요.",
                    refreshed_summary,
                );
            }

            build_rejected_result(
                base_dir,
                input.session_id,
                input.action,
                "recovery-unavailable",
                "종료 후 상태를 다시 확인했지만 아직 안전 복구를 계속할 만큼 정리되지 않았어요.",
                refreshed_summary,
            )
        }
        Some("capture") => {
            let _ = read_live_manifest(base_dir, &input.session_id)?;
            let refreshed_summary =
                load_operator_recovery_summary_in_dir(base_dir, capability_snapshot)?;

            if refreshed_summary.blocked_category.is_none() {
                return build_applied_result(
                    base_dir,
                    input.session_id,
                    input.action,
                    "캡처 경계를 다시 확인해 현재 세션 상태를 새로 반영했어요.",
                    refreshed_summary,
                );
            }

            build_rejected_result(
                base_dir,
                input.session_id,
                input.action,
                "recovery-unavailable",
                "캡처 경계를 다시 확인했지만 아직 안전하게 촬영을 다시 열 수 없어요.",
                refreshed_summary,
            )
        }
        _ => build_rejected_result(
            base_dir,
            input.session_id,
            input.action,
            "not-blocked",
            "지금은 막힌 세션 범주가 없어 복구 재시도가 필요하지 않아요.",
            current_summary,
        ),
    }
}

fn execute_boundary_restart(
    base_dir: &Path,
    capability_snapshot: &CapabilitySnapshotDto,
    input: OperatorRecoveryActionInputDto,
    current_summary: OperatorRecoverySummaryDto,
) -> Result<OperatorRecoveryActionResultDto, HostErrorEnvelope> {
    match current_summary.blocked_category.as_deref() {
        Some("capture") => {
            let paths = SessionPaths::try_new(base_dir, &input.session_id)?;
            let mut manifest = read_live_manifest(base_dir, &input.session_id)?;

            if manifest.active_preset.is_none() {
                return build_rejected_result(
                    base_dir,
                    input.session_id,
                    input.action,
                    "recovery-unavailable",
                    "활성 preset 바인딩이 없어 캡처 경계를 안전하게 다시 시작할 수 없어요.",
                    current_summary,
                );
            }

            manifest.post_end = None;
            manifest.lifecycle.stage = derive_active_lifecycle_stage(&manifest);
            manifest.updated_at = current_timestamp(SystemTime::now())?;
            write_session_manifest(&paths.manifest_path, &manifest)?;

            let refreshed_summary =
                load_operator_recovery_summary_in_dir(base_dir, capability_snapshot)?;

            build_applied_result(
                base_dir,
                input.session_id,
                input.action,
                "캡처 경계를 승인된 범위 안에서 다시 시작했어요.",
                refreshed_summary,
            )
        }
        Some("preview-or-render") => {
            let paths = SessionPaths::try_new(base_dir, &input.session_id)?;
            let mut manifest = read_live_manifest(base_dir, &input.session_id)?;
            let Some(latest_capture) = manifest.captures.last_mut() else {
                return build_rejected_result(
                    base_dir,
                    input.session_id,
                    input.action,
                    "recovery-unavailable",
                    "다시 시작할 최근 촬영 문맥이 없어 render 경계를 재시작하지 않았어요.",
                    current_summary,
                );
            };

            latest_capture.preview.asset_path = None;
            latest_capture.preview.ready_at_ms = None;
            latest_capture.final_asset.asset_path = None;
            latest_capture.final_asset.ready_at_ms = None;
            latest_capture.render_status = "previewWaiting".into();
            latest_capture.post_end_state = "activeSession".into();
            latest_capture.timing.preview_visible_at_ms = None;
            latest_capture.timing.preview_budget_state = "pending".into();
            manifest.post_end = None;
            manifest.lifecycle.stage = "preview-waiting".into();
            manifest.updated_at = current_timestamp(SystemTime::now())?;
            write_session_manifest(&paths.manifest_path, &manifest)?;

            let refreshed_summary =
                load_operator_recovery_summary_in_dir(base_dir, capability_snapshot)?;

            build_applied_result(
                base_dir,
                input.session_id,
                input.action,
                "preview/render 경계를 승인된 범위 안에서 다시 시작했어요.",
                refreshed_summary,
            )
        }
        _ => build_rejected_result(
            base_dir,
            input.session_id,
            input.action,
            "action-not-allowed",
            "현재 세션 범주에는 boundary restart가 허용되지 않아요.",
            current_summary,
        ),
    }
}

fn execute_time_extension(
    base_dir: &Path,
    capability_snapshot: &CapabilitySnapshotDto,
    input: OperatorRecoveryActionInputDto,
    current_summary: OperatorRecoverySummaryDto,
) -> Result<OperatorRecoveryActionResultDto, HostErrorEnvelope> {
    if current_summary.blocked_category.as_deref() != Some("timing-or-post-end") {
        return build_rejected_result(
            base_dir,
            input.session_id,
            input.action,
            "action-not-allowed",
            "현재 세션 범주에는 시간 연장이 허용되지 않아요.",
            current_summary,
        );
    }

    if current_summary.post_end_state.as_deref() == Some(SESSION_POST_END_PHONE_REQUIRED) {
        return build_rejected_result(
            base_dir,
            input.session_id,
            input.action,
            "recovery-unavailable",
            "이미 Phone Required로 보호 전환된 세션에는 추가 시간 연장을 적용하지 않아요.",
            current_summary,
        );
    }

    let paths = SessionPaths::try_new(base_dir, &input.session_id)?;
    let mut manifest = read_live_manifest(base_dir, &input.session_id)?;
    let audit_session_id = manifest.session_id.clone();
    let Some(timing) = manifest.timing.as_mut() else {
        return build_rejected_result(
            base_dir,
            input.session_id,
            input.action,
            "recovery-unavailable",
            "현재 세션 타이밍 정보를 확인할 수 없어 승인된 시간 연장을 적용하지 않았어요.",
            current_summary,
        );
    };

    if timing.approved_extension_minutes >= APPROVED_EXTENSION_MINUTES {
        return build_rejected_result(
            base_dir,
            input.session_id,
            input.action,
            "extension-limit-reached",
            "이 세션에는 이미 허용된 최대 시간 연장이 적용되어 추가 연장을 열지 않았어요.",
            current_summary,
        );
    }

    let new_adjusted_end_at = unix_seconds_to_rfc3339(
        rfc3339_to_unix_seconds(&timing.adjusted_end_at)?
            .saturating_add((APPROVED_EXTENSION_MINUTES as u64) * 60),
    );
    let new_warning_at = unix_seconds_to_rfc3339(
        rfc3339_to_unix_seconds(&new_adjusted_end_at)?.saturating_sub(WARNING_LEAD_SECONDS),
    );
    let event_timestamp = current_timestamp(SystemTime::now())?;
    let mut next_timing = timing.clone();

    next_timing.adjusted_end_at = new_adjusted_end_at.clone();
    next_timing.warning_at = new_warning_at.clone();
    next_timing.approved_extension_minutes += APPROVED_EXTENSION_MINUTES;
    next_timing.approved_extension_audit_ref = Some(format!(
        "operator-recovery:{}:approved-time-extension",
        audit_session_id
    ));
    next_timing.ended_triggered_at = None;

    let evaluated_phase = evaluate_phase(&next_timing, SystemTime::now())?;

    if evaluated_phase == TimingPhase::Ended {
        return build_rejected_result(
            base_dir,
            input.session_id,
            input.action,
            "recovery-unavailable",
            "승인된 시간 연장 범위 안에서도 이미 종료 시각을 지나 세션을 다시 열 수 없어요.",
            current_summary,
        );
    }

    timing.adjusted_end_at = new_adjusted_end_at;
    timing.warning_at = new_warning_at;
    timing.approved_extension_minutes = next_timing.approved_extension_minutes;
    timing.approved_extension_audit_ref = next_timing.approved_extension_audit_ref;
    timing.warning_triggered_at = if evaluated_phase == TimingPhase::Warning {
        Some(event_timestamp.clone())
    } else {
        None
    };
    timing.ended_triggered_at = None;
    timing.phase = evaluated_phase.as_str().into();
    timing.capture_allowed = evaluated_phase != TimingPhase::Ended;
    manifest.post_end = None;

    for capture in &mut manifest.captures {
        if capture.post_end_state != "activeSession" {
            capture.post_end_state = "activeSession".into();
        }
    }

    manifest.lifecycle.stage = derive_active_lifecycle_stage(&manifest);
    manifest.updated_at = event_timestamp;
    write_session_manifest(&paths.manifest_path, &manifest)?;

    let refreshed_summary = load_operator_recovery_summary_in_dir(base_dir, capability_snapshot)?;

    build_applied_result(
        base_dir,
        input.session_id,
        input.action,
        "승인된 범위 안에서 현재 세션 종료 시각을 한 번 연장했어요.",
        refreshed_summary,
    )
}

fn execute_phone_required_route(
    base_dir: &Path,
    capability_snapshot: &CapabilitySnapshotDto,
    input: OperatorRecoveryActionInputDto,
    _current_summary: OperatorRecoverySummaryDto,
) -> Result<OperatorRecoveryActionResultDto, HostErrorEnvelope> {
    let paths = SessionPaths::try_new(base_dir, &input.session_id)?;
    let mut manifest = read_live_manifest(base_dir, &input.session_id)?;
    let event_timestamp = current_timestamp(SystemTime::now())?;

    manifest.post_end = Some(SessionPostEnd::phone_required(
        event_timestamp.clone(),
        ROUTE_PHONE_REQUIRED_PRIMARY_ACTION.into(),
        Some(ROUTE_PHONE_REQUIRED_SUPPORT_ACTION.into()),
        ROUTE_PHONE_REQUIRED_WARNING.into(),
        false,
    ));
    manifest.lifecycle.stage = "phone-required".into();
    manifest.updated_at = event_timestamp;
    write_session_manifest(&paths.manifest_path, &manifest)?;

    let refreshed_summary = load_operator_recovery_summary_in_dir(base_dir, capability_snapshot)?;

    build_applied_result(
        base_dir,
        input.session_id,
        input.action,
        "현재 세션을 고객 보호를 위한 Phone Required 상태로 전환했어요.",
        refreshed_summary,
    )
}

fn build_operator_recovery_summary(
    session_summary: OperatorSessionSummaryDto,
) -> OperatorRecoverySummaryDto {
    let blocked_category = map_blocked_category(session_summary.blocked_state_category.as_str());
    let diagnostics_summary =
        build_diagnostics_summary(&session_summary, blocked_category.as_deref());
    let allowed_actions = build_allowed_actions(
        blocked_category.as_deref(),
        session_summary.post_end_state.as_deref(),
    );

    OperatorRecoverySummaryDto {
        schema_version: OPERATOR_RECOVERY_SUMMARY_SCHEMA_VERSION.into(),
        state: session_summary.state,
        blocked_state_category: session_summary.blocked_state_category,
        blocked_category,
        diagnostics_summary,
        allowed_actions,
        session_id: session_summary.session_id,
        booth_alias: session_summary.booth_alias,
        active_preset_id: session_summary.active_preset_id,
        active_preset_display_name: session_summary.active_preset_display_name,
        active_preset_version: session_summary.active_preset_version,
        lifecycle_stage: session_summary.lifecycle_stage,
        timing_phase: session_summary.timing_phase,
        updated_at: session_summary.updated_at,
        post_end_state: session_summary.post_end_state,
        recent_failure: session_summary.recent_failure,
        camera_connection: session_summary.camera_connection,
        capture_boundary: session_summary.capture_boundary,
        preview_render_boundary: session_summary.preview_render_boundary,
        completion_boundary: session_summary.completion_boundary,
        live_capture_truth: session_summary.live_capture_truth,
    }
}

fn build_diagnostics_summary(
    session_summary: &OperatorSessionSummaryDto,
    blocked_category: Option<&str>,
) -> Option<OperatorRecoveryDiagnosticsSummaryDto> {
    if let Some(recent_failure) = session_summary.recent_failure.as_ref() {
        return Some(OperatorRecoveryDiagnosticsSummaryDto {
            title: recent_failure.title.clone(),
            detail: recent_failure.detail.clone(),
            observed_at: recent_failure.observed_at.clone(),
        });
    }

    match blocked_category {
        Some("capture") => Some(OperatorRecoveryDiagnosticsSummaryDto {
            title: session_summary.capture_boundary.title.clone(),
            detail: session_summary.capture_boundary.detail.clone(),
            observed_at: None,
        }),
        Some("preview-or-render") => Some(OperatorRecoveryDiagnosticsSummaryDto {
            title: session_summary.preview_render_boundary.title.clone(),
            detail: session_summary.preview_render_boundary.detail.clone(),
            observed_at: None,
        }),
        Some("timing-or-post-end") => Some(OperatorRecoveryDiagnosticsSummaryDto {
            title: session_summary.completion_boundary.title.clone(),
            detail: session_summary.completion_boundary.detail.clone(),
            observed_at: None,
        }),
        _ => None,
    }
}

fn build_allowed_actions(
    blocked_category: Option<&str>,
    post_end_state: Option<&str>,
) -> Vec<String> {
    match blocked_category {
        Some("capture") => vec![
            "retry".into(),
            "approved-boundary-restart".into(),
            "route-phone-required".into(),
        ],
        Some("preview-or-render") => vec![
            "retry".into(),
            "approved-boundary-restart".into(),
            "route-phone-required".into(),
        ],
        Some("timing-or-post-end") => {
            let mut actions = vec!["retry".into()];

            if post_end_state != Some(SESSION_POST_END_PHONE_REQUIRED) {
                actions.push("approved-time-extension".into());
            }

            actions.push("route-phone-required".into());
            actions
        }
        _ => Vec::new(),
    }
}

fn map_blocked_category(blocked_state_category: &str) -> Option<String> {
    match blocked_state_category {
        "capture-blocked" => Some("capture".into()),
        "preview-render-blocked" => Some("preview-or-render".into()),
        "timing-post-end-blocked" => Some("timing-or-post-end".into()),
        _ => None,
    }
}

fn build_applied_result(
    base_dir: &Path,
    session_id: String,
    action: String,
    message: &str,
    summary: OperatorRecoverySummaryDto,
) -> Result<OperatorRecoveryActionResultDto, HostErrorEnvelope> {
    let next_state = build_next_state(base_dir, &session_id, &summary)?;
    try_append_operator_audit_record(
        base_dir,
        OperatorAuditRecordInput {
            occurred_at: current_timestamp(SystemTime::now())?,
            session_id: Some(session_id.clone()),
            event_category: "operator-intervention",
            event_type: map_action_to_event_type(action.as_str()),
            summary: message.into(),
            detail: summary
                .diagnostics_summary
                .as_ref()
                .map(|diagnostics| diagnostics.detail.clone())
                .unwrap_or_else(|| "운영자가 허용된 복구 액션을 실행했어요.".into()),
            actor_id: None,
            source: "operator-console",
            capture_id: None,
            preset_id: summary.active_preset_id.clone(),
            published_version: summary.active_preset_version.clone(),
            reason_code: None,
        },
    );

    Ok(OperatorRecoveryActionResultDto {
        schema_version: OPERATOR_RECOVERY_ACTION_RESULT_SCHEMA_VERSION.into(),
        session_id,
        action,
        status: "applied".into(),
        message: message.into(),
        rejection_reason: None,
        diagnostics_summary: summary.diagnostics_summary.clone(),
        next_state,
        summary,
    })
}

fn build_rejected_result(
    base_dir: &Path,
    session_id: String,
    action: String,
    rejection_reason: &str,
    message: &str,
    summary: OperatorRecoverySummaryDto,
) -> Result<OperatorRecoveryActionResultDto, HostErrorEnvelope> {
    build_rejected_result_internal(
        base_dir,
        session_id,
        action,
        rejection_reason,
        message,
        summary,
        true,
    )
}

fn build_rejected_result_without_audit(
    base_dir: &Path,
    session_id: String,
    action: String,
    rejection_reason: &str,
    message: &str,
    summary: OperatorRecoverySummaryDto,
) -> Result<OperatorRecoveryActionResultDto, HostErrorEnvelope> {
    build_rejected_result_internal(
        base_dir,
        session_id,
        action,
        rejection_reason,
        message,
        summary,
        false,
    )
}

fn build_rejected_result_internal(
    base_dir: &Path,
    session_id: String,
    action: String,
    rejection_reason: &str,
    message: &str,
    summary: OperatorRecoverySummaryDto,
    should_audit: bool,
) -> Result<OperatorRecoveryActionResultDto, HostErrorEnvelope> {
    let next_state = match summary.session_id.as_deref() {
        Some(summary_session_id) => build_next_state(base_dir, summary_session_id, &summary)?,
        None => build_next_state_from_summary(&summary),
    };

    if should_audit {
        try_append_operator_audit_record(
            base_dir,
            OperatorAuditRecordInput {
                occurred_at: current_timestamp(SystemTime::now())?,
                session_id: Some(session_id.clone()),
                event_category: "operator-intervention",
                event_type: map_action_to_event_type(action.as_str()),
                summary: message.into(),
                detail: summary
                    .diagnostics_summary
                    .as_ref()
                    .map(|diagnostics| diagnostics.detail.clone())
                    .unwrap_or_else(|| "운영자가 요청한 복구 액션을 적용하지 않았어요.".into()),
                actor_id: None,
                source: "operator-console",
                capture_id: None,
                preset_id: summary.active_preset_id.clone(),
                published_version: summary.active_preset_version.clone(),
                reason_code: Some(rejection_reason.into()),
            },
        );
    }

    Ok(OperatorRecoveryActionResultDto {
        schema_version: OPERATOR_RECOVERY_ACTION_RESULT_SCHEMA_VERSION.into(),
        session_id,
        action,
        status: "rejected".into(),
        message: message.into(),
        rejection_reason: Some(rejection_reason.into()),
        diagnostics_summary: summary.diagnostics_summary.clone(),
        next_state,
        summary,
    })
}

fn map_action_to_event_type(action: &str) -> &'static str {
    match action {
        "approved-boundary-restart" => "approved-boundary-restart",
        "approved-time-extension" => "approved-time-extension",
        "route-phone-required" => "route-phone-required",
        _ => "retry",
    }
}

fn build_next_state(
    base_dir: &Path,
    session_id: &str,
    summary: &OperatorRecoverySummaryDto,
) -> Result<OperatorRecoveryNextStateDto, HostErrorEnvelope> {
    let readiness = get_capture_readiness_in_dir(
        base_dir,
        CaptureReadinessInputDto {
            session_id: session_id.into(),
        },
    )?;

    Ok(OperatorRecoveryNextStateDto {
        customer_state: readiness.customer_state,
        reason_code: readiness.reason_code,
        lifecycle_stage: summary.lifecycle_stage.clone(),
        timing_phase: summary
            .timing_phase
            .clone()
            .or_else(|| readiness.timing.as_ref().map(|timing| timing.phase.clone())),
        post_end_state: summary.post_end_state.clone().or_else(|| {
            readiness
                .post_end
                .as_ref()
                .map(|post_end| post_end.state().into())
        }),
    })
}

fn build_next_state_from_summary(
    summary: &OperatorRecoverySummaryDto,
) -> OperatorRecoveryNextStateDto {
    let (customer_state, reason_code) = match summary.post_end_state.as_deref() {
        Some("phone-required") => ("Phone Required", "phone-required"),
        Some("completed") => ("Completed", "completed"),
        Some("export-waiting") => ("Export Waiting", "export-waiting"),
        _ => match summary.blocked_state_category.as_str() {
            "capture-blocked" => ("Preparing", "camera-preparing"),
            "preview-render-blocked" => ("Preview Waiting", "preview-waiting"),
            "timing-post-end-blocked" => ("Export Waiting", "export-waiting"),
            _ => ("Ready", "ready"),
        },
    };

    OperatorRecoveryNextStateDto {
        customer_state: customer_state.into(),
        reason_code: reason_code.into(),
        lifecycle_stage: summary.lifecycle_stage.clone(),
        timing_phase: summary.timing_phase.clone(),
        post_end_state: summary.post_end_state.clone(),
    }
}

fn read_live_manifest(
    base_dir: &Path,
    session_id: &str,
) -> Result<SessionManifest, HostErrorEnvelope> {
    let paths = SessionPaths::try_new(base_dir, session_id)?;
    let manifest = read_session_manifest(&paths.manifest_path)?;
    let manifest =
        sync_session_timing_in_dir(base_dir, &paths.manifest_path, manifest, SystemTime::now())?;

    sync_post_end_state_in_dir(base_dir, &paths.manifest_path, manifest, SystemTime::now())
}

fn derive_active_lifecycle_stage(manifest: &SessionManifest) -> String {
    match manifest.captures.last() {
        Some(latest_capture)
            if latest_capture.render_status == "previewWaiting"
                || latest_capture.render_status == "captureSaved" =>
        {
            "preview-waiting".into()
        }
        Some(latest_capture) if latest_capture.render_status == "renderFailed" => {
            "phone-required".into()
        }
        Some(_) if manifest.active_preset.is_some() => "capture-ready".into(),
        None if manifest.active_preset.is_some() => "capture-ready".into(),
        _ => "session-started".into(),
    }
}

#[derive(Debug)]
struct SessionManifestCandidate {
    session_name: String,
    manifest_path: PathBuf,
    modified_at: Option<SystemTime>,
}

#[allow(dead_code)]
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

    candidates.sort_by(|left, right| match (left.modified_at, right.modified_at) {
        (Some(left_modified_at), Some(right_modified_at)) => right_modified_at
            .cmp(&left_modified_at)
            .then_with(|| right.session_name.cmp(&left.session_name)),
        (Some(_), None) => std::cmp::Ordering::Less,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (None, None) => right.session_name.cmp(&left.session_name),
    });

    for candidate in candidates {
        let manifest = read_session_manifest(&candidate.manifest_path)?;

        return Ok(Some((candidate.manifest_path, manifest)));
    }

    Ok(None)
}
