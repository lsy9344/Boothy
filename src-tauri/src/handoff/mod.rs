use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::Path,
    time::SystemTime,
};

use serde::Deserialize;

use crate::{
    capture::CAPTURE_PIPELINE_LOCK,
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
const POST_END_LABEL_MAX_CHARS: usize = 80;

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

        if trimmed.is_empty() || trimmed.chars().count() > POST_END_LABEL_MAX_CHARS {
            None
        } else {
            Some(trimmed.to_string())
        }
    })
}

pub fn sync_post_end_state_in_dir(
    base_dir: &Path,
    manifest_path: &Path,
    _manifest: SessionManifest,
    now: SystemTime,
) -> Result<SessionManifest, HostErrorEnvelope> {
    let _guard = CAPTURE_PIPELINE_LOCK.lock().map_err(|_| {
        HostErrorEnvelope::persistence("촬영 상태를 잠그지 못했어요. 잠시 후 다시 확인해 주세요.")
    })?;
    let manifest = read_session_manifest(manifest_path)?;
    sync_post_end_state_locked(base_dir, manifest_path, manifest, now)
}

fn sync_post_end_state_locked(
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

    let Some(evaluation) = resolve_explicit_post_end(base_dir, &manifest) else {
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

    let Some(evaluation) = resolve_explicit_post_end(base_dir, &manifest) else {
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
) -> Option<PostEndEvaluation> {
    if let Some(evaluation) = reusable_terminal_post_end_evaluation(manifest) {
        return Some(evaluation);
    }

    let evaluation = evaluate_post_end(base_dir, manifest);
    let explicit_state =
        manifest.post_end.as_ref().map(|record| record.state()).or(
            match manifest.lifecycle.stage.as_str() {
                SESSION_POST_END_EXPORT_WAITING => Some(SESSION_POST_END_EXPORT_WAITING),
                SESSION_POST_END_PHONE_REQUIRED => Some(SESSION_POST_END_PHONE_REQUIRED),
                _ => None,
            },
        );

    match explicit_state {
        Some(SESSION_POST_END_COMPLETED) | Some(SESSION_POST_END_PHONE_REQUIRED) => Some(
            resolve_locked_post_end(manifest, evaluation, explicit_state.unwrap()),
        ),
        Some(SESSION_POST_END_EXPORT_WAITING) => match evaluation.state.as_str() {
            SESSION_POST_END_COMPLETED | SESSION_POST_END_PHONE_REQUIRED => Some(evaluation),
            _ => Some(PostEndEvaluation {
                state: SESSION_POST_END_EXPORT_WAITING.into(),
                completion_variant: None,
            }),
        },
        None => Some(evaluation),
        _ => None,
    }
}

fn reusable_terminal_post_end_evaluation(manifest: &SessionManifest) -> Option<PostEndEvaluation> {
    let post_end = manifest.post_end.as_ref()?;

    match post_end {
        SessionPostEnd::Completed(value)
            if value.completion_variant == SESSION_POST_END_LOCAL_DELIVERABLE_READY =>
        {
            Some(PostEndEvaluation {
                state: SESSION_POST_END_COMPLETED.into(),
                completion_variant: Some(SESSION_POST_END_LOCAL_DELIVERABLE_READY.into()),
            })
        }
        SessionPostEnd::Completed(value)
            if value.completion_variant == SESSION_POST_END_HANDOFF_READY
                && (value.approved_recipient_label.is_some()
                    || value.next_location_label.is_some()) =>
        {
            Some(PostEndEvaluation {
                state: SESSION_POST_END_COMPLETED.into(),
                completion_variant: Some(SESSION_POST_END_HANDOFF_READY.into()),
            })
        }
        SessionPostEnd::PhoneRequired(_) => Some(PostEndEvaluation {
            state: SESSION_POST_END_PHONE_REQUIRED.into(),
            completion_variant: None,
        }),
        _ => None,
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

fn evaluate_post_end(base_dir: &Path, manifest: &SessionManifest) -> PostEndEvaluation {
    if manifest.captures.is_empty() {
        return PostEndEvaluation {
            state: SESSION_POST_END_EXPORT_WAITING.into(),
            completion_variant: None,
        };
    }

    if manifest
        .captures
        .iter()
        .any(|capture| capture.render_status == "renderFailed")
    {
        return PostEndEvaluation {
            state: SESSION_POST_END_PHONE_REQUIRED.into(),
            completion_variant: None,
        };
    }

    if manifest
        .captures
        .iter()
        .all(|capture| capture_has_final_truth(base_dir, &manifest.session_id, capture))
    {
        let completion_variant = if has_handoff_destination(base_dir, manifest) {
            SESSION_POST_END_HANDOFF_READY
        } else {
            SESSION_POST_END_LOCAL_DELIVERABLE_READY
        };

        PostEndEvaluation {
            state: SESSION_POST_END_COMPLETED.into(),
            completion_variant: Some(completion_variant.into()),
        }
    } else {
        PostEndEvaluation {
            state: SESSION_POST_END_EXPORT_WAITING.into(),
            completion_variant: None,
        }
    }
}

fn capture_has_final_truth(
    base_dir: &Path,
    session_id: &str,
    capture: &crate::session::session_manifest::SessionCaptureRecord,
) -> bool {
    capture.render_status == "finalReady"
        && capture
            .final_asset
            .asset_path
            .as_deref()
            .map(|asset_path| is_existing_session_scoped_asset(base_dir, session_id, asset_path))
            .unwrap_or(false)
        && capture.final_asset.ready_at_ms.is_some()
}

fn is_existing_session_scoped_asset(base_dir: &Path, session_id: &str, asset_path: &str) -> bool {
    let Ok(paths) = SessionPaths::try_new(base_dir, session_id) else {
        return false;
    };
    let asset_path = Path::new(asset_path);

    if !asset_path.is_file() {
        return false;
    }

    let Ok(canonical_asset_path) = fs::canonicalize(asset_path) else {
        return false;
    };
    let Ok(canonical_session_root) = fs::canonicalize(paths.session_root) else {
        return false;
    };

    canonical_asset_path.starts_with(canonical_session_root)
}

fn has_handoff_destination(base_dir: &Path, manifest: &SessionManifest) -> bool {
    if read_handoff_guidance_file(base_dir, &manifest.session_id)
        .ok()
        .flatten()
        .map(|guidance| {
            guidance.approved_recipient_label.is_some() || guidance.next_location_label.is_some()
        })
        .unwrap_or(false)
    {
        return true;
    }

    manifest
        .post_end
        .as_ref()
        .and_then(|record| match record {
            SessionPostEnd::Completed(value)
                if value.completion_variant == SESSION_POST_END_HANDOFF_READY =>
            {
                Some(value)
            }
            _ => None,
        })
        .map(|record| {
            record.approved_recipient_label.is_some() || record.next_location_label.is_some()
        })
        .unwrap_or(false)
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
    let mut variant = completion_variant.unwrap_or(SESSION_POST_END_LOCAL_DELIVERABLE_READY);

    if variant == SESSION_POST_END_HANDOFF_READY {
        let file_guidance = read_handoff_guidance_file(base_dir, session_id)?;

        if let Some(file_guidance) = file_guidance.as_ref() {
            if file_guidance.approved_recipient_label.is_some()
                || file_guidance.next_location_label.is_some()
            {
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
                && (existing.approved_recipient_label.is_some()
                    || existing.next_location_label.is_some())
            {
                return Ok(CompletedPostEnd {
                    evaluated_at: evaluated_at.into(),
                    ..existing.clone()
                });
            }
        }

        variant = SESSION_POST_END_LOCAL_DELIVERABLE_READY;
    }

    if let Some(existing) = existing_completed {
        if existing.completion_variant == variant {
            return Ok(CompletedPostEnd {
                evaluated_at: evaluated_at.into(),
                ..existing
            });
        }
    }

    Ok(CompletedPostEnd {
        state: SESSION_POST_END_COMPLETED.into(),
        evaluated_at: evaluated_at.into(),
        completion_variant: SESSION_POST_END_LOCAL_DELIVERABLE_READY.into(),
        approved_recipient_label: None,
        next_location_label: None,
        primary_action_label: "안내가 끝났어요. 천천히 이동해 주세요.".into(),
        support_action_label: None,
        show_booth_alias: false,
        handoff: None,
    })
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session::session_manifest::{
        CompletedPostEnd, SessionLifecycle, SessionPostEnd, SessionTiming,
        SESSION_MANIFEST_SCHEMA_VERSION, SESSION_TIMING_SCHEMA_VERSION,
    };

    fn temp_dir(test_name: &str) -> std::path::PathBuf {
        let unique = SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("test clock should be valid")
            .as_nanos();
        std::env::temp_dir().join(format!("boothy-{test_name}-{unique}"))
    }

    fn manifest_with_post_end(post_end: SessionPostEnd) -> SessionManifest {
        SessionManifest {
            schema_version: SESSION_MANIFEST_SCHEMA_VERSION.into(),
            session_id: "session_terminal".into(),
            booth_alias: "Booth".into(),
            customer: crate::session::session_manifest::SessionCustomer {
                name: "Kim".into(),
                phone_last_four: "4821".into(),
            },
            lifecycle: SessionLifecycle {
                status: "closed".into(),
                stage: post_end.state().into(),
            },
            active_preset_id: None,
            active_preset_display_name: None,
            active_preset: None,
            catalog_revision: None,
            catalog_snapshot: None,
            timing: None,
            post_end: Some(post_end),
            captures: vec![],
            created_at: "2026-04-30T00:00:00Z".into(),
            updated_at: "2026-04-30T00:00:00Z".into(),
        }
    }

    #[test]
    fn reusable_terminal_completed_post_end_returns_cached_evaluation() {
        let manifest = manifest_with_post_end(SessionPostEnd::Completed(CompletedPostEnd {
            state: SESSION_POST_END_COMPLETED.into(),
            evaluated_at: "2026-04-30T00:00:00Z".into(),
            completion_variant: SESSION_POST_END_LOCAL_DELIVERABLE_READY.into(),
            approved_recipient_label: None,
            next_location_label: None,
            primary_action_label: "안내가 끝났어요. 천천히 이동해 주세요.".into(),
            support_action_label: None,
            show_booth_alias: false,
            handoff: None,
        }));

        let evaluation = reusable_terminal_post_end_evaluation(&manifest)
            .expect("local deliverable completion should be reusable");

        assert_eq!(evaluation.state, SESSION_POST_END_COMPLETED);
        assert_eq!(
            evaluation.completion_variant.as_deref(),
            Some(SESSION_POST_END_LOCAL_DELIVERABLE_READY)
        );
    }

    #[test]
    fn post_end_sync_reloads_manifest_before_persisting() {
        let base_dir = temp_dir("post-end-reload-before-write");
        std::fs::create_dir_all(&base_dir).expect("test dir should exist");
        let manifest_path = base_dir.join("manifest.json");
        let stale_manifest = SessionManifest {
            schema_version: SESSION_MANIFEST_SCHEMA_VERSION.into(),
            session_id: "session_terminal".into(),
            booth_alias: "Booth".into(),
            customer: crate::session::session_manifest::SessionCustomer {
                name: "Kim".into(),
                phone_last_four: "4821".into(),
            },
            lifecycle: SessionLifecycle {
                status: "closed".into(),
                stage: "ended".into(),
            },
            active_preset_id: None,
            active_preset_display_name: None,
            active_preset: None,
            catalog_revision: None,
            catalog_snapshot: None,
            timing: Some(SessionTiming {
                schema_version: SESSION_TIMING_SCHEMA_VERSION.into(),
                session_id: "session_terminal".into(),
                adjusted_end_at: "2000-01-01T00:00:00Z".into(),
                warning_at: "1999-12-31T23:55:00Z".into(),
                phase: "ended".into(),
                capture_allowed: false,
                approved_extension_minutes: 0,
                approved_extension_audit_ref: None,
                warning_triggered_at: None,
                ended_triggered_at: Some("2026-04-30T00:00:00Z".into()),
            }),
            post_end: None,
            captures: vec![],
            created_at: "2026-04-30T00:00:00Z".into(),
            updated_at: "2026-04-30T00:00:00Z".into(),
        };
        let current_manifest =
            manifest_with_post_end(SessionPostEnd::Completed(CompletedPostEnd {
                state: SESSION_POST_END_COMPLETED.into(),
                evaluated_at: "2026-04-30T00:00:00Z".into(),
                completion_variant: SESSION_POST_END_LOCAL_DELIVERABLE_READY.into(),
                approved_recipient_label: None,
                next_location_label: None,
                primary_action_label: "안내가 끝났어요. 천천히 이동해 주세요.".into(),
                support_action_label: None,
                show_booth_alias: false,
                handoff: None,
            }));
        std::fs::write(
            &manifest_path,
            serde_json::to_vec_pretty(&current_manifest).expect("manifest should serialize"),
        )
        .expect("manifest should be writable");

        let projected = sync_post_end_state_in_dir(
            &base_dir,
            &manifest_path,
            stale_manifest,
            SystemTime::now(),
        )
        .expect("post-end projection should succeed");

        assert_eq!(projected.lifecycle.stage, SESSION_POST_END_COMPLETED);
        assert_eq!(
            projected.post_end.as_ref().map(|post_end| post_end.state()),
            Some(SESSION_POST_END_COMPLETED)
        );

        let persisted: SessionManifest = serde_json::from_slice(
            &std::fs::read(&manifest_path).expect("manifest should remain readable"),
        )
        .expect("manifest should deserialize");
        assert_eq!(
            persisted.post_end.as_ref().map(|post_end| post_end.state()),
            Some(SESSION_POST_END_COMPLETED)
        );
        assert_eq!(persisted.lifecycle.stage, SESSION_POST_END_COMPLETED);

        let _ = std::fs::remove_dir_all(base_dir);
    }

    #[test]
    fn post_end_sync_does_not_overwrite_when_manifest_reload_fails() {
        let base_dir = temp_dir("post-end-reload-failure");
        std::fs::create_dir_all(&base_dir).expect("test dir should exist");
        let manifest_path = base_dir.join("manifest.json");
        let stale_manifest = manifest_with_post_end(SessionPostEnd::Completed(CompletedPostEnd {
            state: SESSION_POST_END_COMPLETED.into(),
            evaluated_at: "2026-04-30T00:00:00Z".into(),
            completion_variant: SESSION_POST_END_LOCAL_DELIVERABLE_READY.into(),
            approved_recipient_label: None,
            next_location_label: None,
            primary_action_label: "안내가 끝났어요. 천천히 이동해 주세요.".into(),
            support_action_label: None,
            show_booth_alias: false,
            handoff: None,
        }));
        let corrupted_manifest = "{ this is not valid session json";
        std::fs::write(&manifest_path, corrupted_manifest)
            .expect("corrupted manifest should be writable");

        let result = sync_post_end_state_in_dir(
            &base_dir,
            &manifest_path,
            stale_manifest,
            SystemTime::now(),
        );

        assert!(
            result.is_err(),
            "post-end sync should fail closed when live manifest cannot be reloaded"
        );
        assert_eq!(
            std::fs::read_to_string(&manifest_path).expect("manifest should remain readable"),
            corrupted_manifest,
            "stale in-memory post-end truth must not overwrite the live manifest"
        );

        let _ = std::fs::remove_dir_all(base_dir);
    }
}
