use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::Path,
    time::SystemTime,
};

use serde::Deserialize;

use crate::{
    capture::ingest_pipeline::{complete_final_render_in_dir, mark_final_render_failed_in_dir},
    contracts::dto::HostErrorEnvelope,
    diagnostics::audit_log::{try_append_operator_audit_record, OperatorAuditRecordInput},
    session::{
        session_manifest::{
            current_timestamp, CompletedPostEnd, ExportWaitingPostEnd, PhoneRequiredPostEnd,
            SessionManifest, SessionPostEnd, SESSION_POST_END_COMPLETED,
            SESSION_POST_END_EXPORT_WAITING, SESSION_POST_END_HANDOFF_READY,
            SESSION_POST_END_LOCAL_DELIVERABLE_READY, SESSION_POST_END_PHONE_REQUIRED,
        },
        session_paths::SessionPaths,
        session_repository::{read_session_manifest, write_session_manifest},
    },
};

const POST_END_PENDING_CAPTURE_STATE: &str = "postEndPending";
const HANDOFF_GUIDANCE_FILE: &str = "customer-guidance.json";

#[derive(Debug, Clone, PartialEq, Eq)]
struct PostEndEvaluation {
    state: String,
    completion_variant: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct HandoffGuidanceFile {
    #[serde(default)]
    approved_recipient_label: Option<String>,
    #[serde(default)]
    next_location_label: Option<String>,
    #[serde(default)]
    primary_action_label: Option<String>,
    #[serde(default)]
    support_action_label: Option<String>,
    #[serde(default)]
    show_booth_alias: Option<bool>,
}

fn normalize_optional_label(value: Option<String>) -> Option<String> {
    value.and_then(|label| {
        let trimmed = label.trim();

        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    })
}

fn has_handoff_destination_in_guidance(guidance: &HandoffGuidanceFile) -> bool {
    guidance.approved_recipient_label.is_some() || guidance.next_location_label.is_some()
}

fn has_handoff_destination_in_completed(record: &CompletedPostEnd) -> bool {
    record.approved_recipient_label.is_some() || record.next_location_label.is_some()
}

fn build_local_deliverable_ready_post_end(
    evaluated_at: &str,
    guidance: Option<&HandoffGuidanceFile>,
    existing_completed: Option<CompletedPostEnd>,
) -> CompletedPostEnd {
    if let Some(existing) = existing_completed {
        if existing.completion_variant == SESSION_POST_END_LOCAL_DELIVERABLE_READY {
            return CompletedPostEnd {
                evaluated_at: evaluated_at.into(),
                ..existing
            };
        }
    }

    CompletedPostEnd {
        state: SESSION_POST_END_COMPLETED.into(),
        evaluated_at: evaluated_at.into(),
        completion_variant: SESSION_POST_END_LOCAL_DELIVERABLE_READY.into(),
        approved_recipient_label: None,
        next_location_label: None,
        primary_action_label: guidance
            .and_then(|value| value.primary_action_label.clone())
            .unwrap_or_else(|| "안내가 끝났어요. 천천히 이동해 주세요.".into()),
        support_action_label: guidance.and_then(|value| value.support_action_label.clone()),
        show_booth_alias: guidance
            .and_then(|value| value.show_booth_alias)
            .unwrap_or(false),
        handoff: None,
    }
}

pub fn sync_post_end_state_in_dir(
    base_dir: &Path,
    manifest_path: &Path,
    mut manifest: SessionManifest,
    now: SystemTime,
) -> Result<SessionManifest, HostErrorEnvelope> {
    let timing_phase = manifest
        .timing
        .as_ref()
        .map(|timing| timing.phase.as_str())
        .unwrap_or("active");

    if timing_phase != "ended" {
        return Ok(manifest);
    }

    manifest = attempt_final_render_if_needed(base_dir, manifest_path, manifest)?;

    let Some(evaluation) = resolve_explicit_post_end(base_dir, &manifest)? else {
        return Ok(manifest);
    };
    let current = manifest.post_end.as_ref().map(|record| PostEndEvaluation {
        state: record.state().to_string(),
        completion_variant: record.completion_variant().map(str::to_string),
    });
    let next_capture_state = capture_post_end_state_for(&evaluation);
    let captures_need_update = manifest
        .captures
        .iter()
        .any(|capture| capture.post_end_state != next_capture_state);
    let lifecycle_needs_update = manifest.lifecycle.stage != evaluation.state;
    let state_changed = current.as_ref() != Some(&evaluation);

    if !captures_need_update && !lifecycle_needs_update && !state_changed {
        return Ok(manifest);
    }

    let evaluated_at = current_timestamp(now)?;
    let existing_completed = manifest.post_end.as_ref().and_then(|record| match record {
        SessionPostEnd::Completed(value) => Some(value.clone()),
        _ => None,
    });
    let existing_phone_required = manifest.post_end.as_ref().and_then(|record| match record {
        SessionPostEnd::PhoneRequired(value) => Some(value.clone()),
        _ => None,
    });

    manifest.post_end = Some(build_post_end_record(
        base_dir,
        &manifest.session_id,
        &evaluation,
        &evaluated_at,
        existing_completed,
        existing_phone_required,
    )?);
    manifest.lifecycle.stage = evaluation.state.clone();

    for capture in &mut manifest.captures {
        capture.post_end_state = next_capture_state.to_string();
    }

    manifest.updated_at = evaluated_at.clone();
    write_session_manifest(manifest_path, &manifest)?;
    append_post_end_log(base_dir, &manifest.session_id, &evaluation, &evaluated_at)?;
    append_post_end_audit_record(base_dir, &manifest, &evaluation, &evaluated_at);

    Ok(manifest)
}

pub fn project_post_end_state_in_dir(
    base_dir: &Path,
    mut manifest: SessionManifest,
    now: SystemTime,
) -> Result<SessionManifest, HostErrorEnvelope> {
    let timing_phase = manifest
        .timing
        .as_ref()
        .map(|timing| timing.phase.as_str())
        .unwrap_or("active");

    if timing_phase != "ended" {
        return Ok(manifest);
    }

    let Some(evaluation) = resolve_explicit_post_end(base_dir, &manifest)? else {
        return Ok(manifest);
    };
    let current = manifest.post_end.as_ref().map(|record| PostEndEvaluation {
        state: record.state().to_string(),
        completion_variant: record.completion_variant().map(str::to_string),
    });
    let next_capture_state = capture_post_end_state_for(&evaluation);
    let captures_need_update = manifest
        .captures
        .iter()
        .any(|capture| capture.post_end_state != next_capture_state);
    let lifecycle_needs_update = manifest.lifecycle.stage != evaluation.state;
    let state_changed = current.as_ref() != Some(&evaluation);

    if !captures_need_update && !lifecycle_needs_update && !state_changed {
        return Ok(manifest);
    }

    let evaluated_at = current_timestamp(now)?;
    let existing_completed = manifest.post_end.as_ref().and_then(|record| match record {
        SessionPostEnd::Completed(value) => Some(value.clone()),
        _ => None,
    });
    let existing_phone_required = manifest.post_end.as_ref().and_then(|record| match record {
        SessionPostEnd::PhoneRequired(value) => Some(value.clone()),
        _ => None,
    });

    manifest.post_end = Some(build_post_end_record(
        base_dir,
        &manifest.session_id,
        &evaluation,
        &evaluated_at,
        existing_completed,
        existing_phone_required,
    )?);
    manifest.lifecycle.stage = evaluation.state.clone();

    for capture in &mut manifest.captures {
        capture.post_end_state = next_capture_state.to_string();
    }

    Ok(manifest)
}

fn resolve_explicit_post_end(
    base_dir: &Path,
    manifest: &SessionManifest,
) -> Result<Option<PostEndEvaluation>, HostErrorEnvelope> {
    let evaluation = evaluate_post_end(base_dir, manifest)?;
    let explicit_state =
        manifest.post_end.as_ref().map(|record| record.state()).or(
            match manifest.lifecycle.stage.as_str() {
                SESSION_POST_END_EXPORT_WAITING => Some(SESSION_POST_END_EXPORT_WAITING),
                SESSION_POST_END_COMPLETED => Some(SESSION_POST_END_COMPLETED),
                SESSION_POST_END_PHONE_REQUIRED => Some(SESSION_POST_END_PHONE_REQUIRED),
                _ => None,
            },
        );

    match explicit_state {
        Some(SESSION_POST_END_COMPLETED) | Some(SESSION_POST_END_PHONE_REQUIRED) => Ok(Some(
            resolve_locked_post_end(manifest, evaluation, explicit_state.unwrap()),
        )),
        Some(SESSION_POST_END_EXPORT_WAITING) => match evaluation.state.as_str() {
            SESSION_POST_END_COMPLETED | SESSION_POST_END_PHONE_REQUIRED => Ok(Some(evaluation)),
            _ => Ok(Some(PostEndEvaluation {
                state: SESSION_POST_END_EXPORT_WAITING.into(),
                completion_variant: None,
            })),
        },
        None => Ok(Some(evaluation)),
        _ => Ok(None),
    }
}

fn resolve_locked_post_end(
    manifest: &SessionManifest,
    mut evaluation: PostEndEvaluation,
    explicit_state: &str,
) -> PostEndEvaluation {
    evaluation.state = explicit_state.into();

    if explicit_state != SESSION_POST_END_COMPLETED {
        evaluation.completion_variant = None;
    } else if evaluation.completion_variant.is_none() {
        evaluation.completion_variant = manifest
            .post_end
            .as_ref()
            .and_then(|record| record.completion_variant().map(str::to_string))
            .or(Some(SESSION_POST_END_LOCAL_DELIVERABLE_READY.into()));
    }

    evaluation
}

fn evaluate_post_end(
    base_dir: &Path,
    manifest: &SessionManifest,
) -> Result<PostEndEvaluation, HostErrorEnvelope> {
    let Some(latest_capture) = manifest.captures.last() else {
        return Ok(PostEndEvaluation {
            state: SESSION_POST_END_EXPORT_WAITING.into(),
            completion_variant: None,
        });
    };

    let evaluation = match latest_capture.render_status.as_str() {
        "previewWaiting" | "captureSaved" => PostEndEvaluation {
            state: SESSION_POST_END_EXPORT_WAITING.into(),
            completion_variant: None,
        },
        "previewReady" => PostEndEvaluation {
            state: SESSION_POST_END_EXPORT_WAITING.into(),
            completion_variant: None,
        },
        "finalReady" => PostEndEvaluation {
            state: SESSION_POST_END_COMPLETED.into(),
            completion_variant: Some(resolve_completed_variant(base_dir, manifest)?),
        },
        "renderFailed" => PostEndEvaluation {
            state: SESSION_POST_END_PHONE_REQUIRED.into(),
            completion_variant: None,
        },
        _ => PostEndEvaluation {
            state: SESSION_POST_END_EXPORT_WAITING.into(),
            completion_variant: None,
        },
    };

    Ok(evaluation)
}

fn resolve_completed_variant(
    base_dir: &Path,
    manifest: &SessionManifest,
) -> Result<String, HostErrorEnvelope> {
    let file_guidance = read_handoff_guidance_file(base_dir, &manifest.session_id)?;
    let has_guidance_destination = file_guidance
        .as_ref()
        .map(has_handoff_destination_in_guidance)
        .unwrap_or(false);
    let has_existing_handoff_destination = manifest
        .post_end
        .as_ref()
        .and_then(|record| match record {
            SessionPostEnd::Completed(value) => Some(value),
            _ => None,
        })
        .filter(|record| record.completion_variant == SESSION_POST_END_HANDOFF_READY)
        .map(has_handoff_destination_in_completed)
        .unwrap_or(false);

    Ok(
        if has_guidance_destination || has_existing_handoff_destination {
            SESSION_POST_END_HANDOFF_READY.into()
        } else {
            SESSION_POST_END_LOCAL_DELIVERABLE_READY.into()
        },
    )
}

fn attempt_final_render_if_needed(
    base_dir: &Path,
    manifest_path: &Path,
    manifest: SessionManifest,
) -> Result<SessionManifest, HostErrorEnvelope> {
    let Some(latest_capture) = manifest.captures.last() else {
        return Ok(manifest);
    };

    if latest_capture.render_status != "previewReady"
        || latest_capture.final_asset.asset_path.is_some()
    {
        return Ok(manifest);
    }

    let capture_id = latest_capture.capture_id.clone();
    let session_id = manifest.session_id.clone();

    match complete_final_render_in_dir(base_dir, &session_id, &capture_id) {
        Ok(_) => read_session_manifest(manifest_path),
        Err(_) => {
            let _ = mark_final_render_failed_in_dir(base_dir, &session_id, &capture_id);
            read_session_manifest(manifest_path)
        }
    }
}

fn capture_post_end_state_for(evaluation: &PostEndEvaluation) -> &'static str {
    match evaluation.completion_variant.as_deref() {
        Some(SESSION_POST_END_LOCAL_DELIVERABLE_READY) => SESSION_POST_END_LOCAL_DELIVERABLE_READY,
        Some(SESSION_POST_END_HANDOFF_READY) => SESSION_POST_END_HANDOFF_READY,
        _ => POST_END_PENDING_CAPTURE_STATE,
    }
}

fn build_post_end_record(
    base_dir: &Path,
    session_id: &str,
    evaluation: &PostEndEvaluation,
    evaluated_at: &str,
    existing_completed: Option<CompletedPostEnd>,
    existing_phone_required: Option<PhoneRequiredPostEnd>,
) -> Result<SessionPostEnd, HostErrorEnvelope> {
    match evaluation.state.as_str() {
        SESSION_POST_END_EXPORT_WAITING => {
            Ok(SessionPostEnd::ExportWaiting(ExportWaitingPostEnd {
                state: SESSION_POST_END_EXPORT_WAITING.into(),
                evaluated_at: evaluated_at.into(),
            }))
        }
        SESSION_POST_END_COMPLETED => build_completed_post_end(
            base_dir,
            session_id,
            evaluation.completion_variant.as_deref(),
            evaluated_at,
            existing_completed,
        )
        .map(SessionPostEnd::Completed),
        SESSION_POST_END_PHONE_REQUIRED => Ok(SessionPostEnd::PhoneRequired(
            existing_phone_required.unwrap_or_else(|| PhoneRequiredPostEnd {
                state: SESSION_POST_END_PHONE_REQUIRED.into(),
                evaluated_at: evaluated_at.into(),
                primary_action_label: "가까운 직원에게 알려 주세요.".into(),
                support_action_label: Some("직원에게 도움을 요청해 주세요.".into()),
                unsafe_action_warning: "다시 찍기나 기기 조작은 잠시 멈춰 주세요.".into(),
                show_booth_alias: false,
            }),
        )),
        _ => Ok(SessionPostEnd::ExportWaiting(ExportWaitingPostEnd {
            state: SESSION_POST_END_EXPORT_WAITING.into(),
            evaluated_at: evaluated_at.into(),
        })),
    }
}

fn build_completed_post_end(
    base_dir: &Path,
    session_id: &str,
    completion_variant: Option<&str>,
    evaluated_at: &str,
    existing_completed: Option<CompletedPostEnd>,
) -> Result<CompletedPostEnd, HostErrorEnvelope> {
    let variant = completion_variant.unwrap_or(SESSION_POST_END_LOCAL_DELIVERABLE_READY);

    if variant == SESSION_POST_END_HANDOFF_READY {
        let file_guidance = read_handoff_guidance_file(base_dir, session_id)?;

        if let Some(file_guidance) = file_guidance.as_ref() {
            if has_handoff_destination_in_guidance(file_guidance) {
                return Ok(CompletedPostEnd {
                    state: SESSION_POST_END_COMPLETED.into(),
                    evaluated_at: evaluated_at.into(),
                    completion_variant: SESSION_POST_END_HANDOFF_READY.into(),
                    approved_recipient_label: file_guidance.approved_recipient_label.clone(),
                    next_location_label: file_guidance.next_location_label.clone(),
                    primary_action_label: file_guidance
                        .primary_action_label
                        .clone()
                        .unwrap_or_else(|| "안내된 곳으로 이동해 주세요.".into()),
                    support_action_label: file_guidance.support_action_label.clone(),
                    show_booth_alias: file_guidance.show_booth_alias.unwrap_or(false),
                    handoff: None,
                });
            }
        }

        if let Some(existing) = existing_completed.as_ref() {
            if existing.completion_variant == variant
                && has_handoff_destination_in_completed(existing)
            {
                return Ok(CompletedPostEnd {
                    evaluated_at: evaluated_at.into(),
                    ..existing.clone()
                });
            }
        }

        return Ok(build_local_deliverable_ready_post_end(
            evaluated_at,
            file_guidance.as_ref(),
            existing_completed,
        ));
    }

    let file_guidance = read_handoff_guidance_file(base_dir, session_id)?;

    Ok(build_local_deliverable_ready_post_end(
        evaluated_at,
        file_guidance.as_ref(),
        existing_completed,
    ))
}

fn read_handoff_guidance_file(
    base_dir: &Path,
    session_id: &str,
) -> Result<Option<HandoffGuidanceFile>, HostErrorEnvelope> {
    let guidance_path = SessionPaths::try_new(base_dir, session_id)?
        .handoff_dir
        .join(HANDOFF_GUIDANCE_FILE);

    if !guidance_path.is_file() {
        return Ok(None);
    }

    let guidance_bytes = fs::read_to_string(&guidance_path).map_err(|error| {
        HostErrorEnvelope::persistence(format!("handoff 안내를 읽지 못했어요: {error}"))
    })?;
    let mut guidance: HandoffGuidanceFile = match serde_json::from_str(&guidance_bytes) {
        Ok(value) => value,
        Err(_) => return Ok(None),
    };
    guidance.approved_recipient_label = normalize_optional_label(guidance.approved_recipient_label);
    guidance.next_location_label = normalize_optional_label(guidance.next_location_label);
    guidance.primary_action_label = normalize_optional_label(guidance.primary_action_label);
    guidance.support_action_label = normalize_optional_label(guidance.support_action_label);

    Ok(Some(guidance))
}

fn append_post_end_log(
    base_dir: &Path,
    session_id: &str,
    evaluation: &PostEndEvaluation,
    occurred_at: &str,
) -> Result<(), HostErrorEnvelope> {
    let diagnostics_dir = SessionPaths::try_new(base_dir, session_id)?.diagnostics_dir;
    std::fs::create_dir_all(&diagnostics_dir).map_err(|error| {
        HostErrorEnvelope::persistence(format!("진단 로그를 남기지 못했어요: {error}"))
    })?;
    let log_path = diagnostics_dir.join("timing-events.log");
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_path)
        .map_err(|error| {
            HostErrorEnvelope::persistence(format!("진단 로그를 남기지 못했어요: {error}"))
        })?;

    let variant = evaluation.completion_variant.as_deref().unwrap_or("none");

    writeln!(
        file,
        "{occurred_at}\tsession={session_id}\tevent=post-end-evaluated\tstate={}\tvariant={variant}",
        evaluation.state
    )
    .map_err(|error| {
        HostErrorEnvelope::persistence(format!("진단 로그를 남기지 못했어요: {error}"))
    })?;

    Ok(())
}

fn append_post_end_audit_record(
    base_dir: &Path,
    manifest: &SessionManifest,
    evaluation: &PostEndEvaluation,
    occurred_at: &str,
) {
    let (event_category, event_type, summary, detail, reason_code) = match evaluation.state.as_str()
    {
        SESSION_POST_END_EXPORT_WAITING => (
            "post-end-outcome",
            "post-end-export-waiting",
            "종료 후 결과 정리 상태가 확정되었어요.".to_string(),
            "후처리 결과가 아직 완료되지 않아 export-waiting 상태를 유지해요.".to_string(),
            Some("export-waiting".to_string()),
        ),
        SESSION_POST_END_COMPLETED => (
            "post-end-outcome",
            "post-end-completed",
            "종료 후 완료 결과가 확정되었어요.".to_string(),
            "운영자가 회고에 사용할 최종 outcome이 completed로 정리되었어요.".to_string(),
            evaluation.completion_variant.clone(),
        ),
        SESSION_POST_END_PHONE_REQUIRED => (
            "critical-failure",
            "post-end-phone-required",
            "종료 후 자동 완료를 멈추고 직원 확인 상태로 전환했어요.".to_string(),
            "후처리 결과가 안전 기준을 벗어나 Phone Required 보호 상태로 잠겼어요.".to_string(),
            Some("render-failed".to_string()),
        ),
        _ => (
            "post-end-outcome",
            "post-end-export-waiting",
            "종료 후 결과 정리 상태가 확정되었어요.".to_string(),
            "후처리 결과를 다시 정리하고 있어요.".to_string(),
            None,
        ),
    };

    try_append_operator_audit_record(
        base_dir,
        OperatorAuditRecordInput {
            occurred_at: occurred_at.into(),
            session_id: Some(manifest.session_id.clone()),
            event_category,
            event_type,
            summary,
            detail,
            actor_id: None,
            source: "post-end-evaluator",
            capture_id: manifest
                .captures
                .last()
                .map(|capture| capture.capture_id.clone()),
            preset_id: manifest.active_preset_id.clone(),
            published_version: manifest
                .active_preset
                .as_ref()
                .map(|preset| preset.published_version.clone()),
            reason_code,
        },
    );
}
