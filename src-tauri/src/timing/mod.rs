use std::{fs::OpenOptions, io::Write, path::Path, time::SystemTime};

use crate::{
    contracts::dto::HostErrorEnvelope,
    diagnostics::audit_log::{try_append_operator_audit_record, OperatorAuditRecordInput},
    session::{
        session_manifest::{
            current_timestamp, rfc3339_to_unix_seconds, SessionManifest, SessionTiming,
        },
        session_paths::SessionPaths,
        session_repository::{read_session_manifest, write_session_manifest},
    },
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimingPhase {
    Active,
    Warning,
    Ended,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SessionTimingEventInput<'a> {
    pub session_id: &'a str,
    pub event: &'a str,
    pub capture_id: Option<&'a str>,
    pub request_id: Option<&'a str>,
    pub detail: Option<&'a str>,
}

impl TimingPhase {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Warning => "warning",
            Self::Ended => "ended",
        }
    }
}

pub fn sync_session_timing_in_dir(
    base_dir: &Path,
    manifest_path: &Path,
    _manifest: SessionManifest,
    now: SystemTime,
) -> Result<SessionManifest, HostErrorEnvelope> {
    let manifest = read_session_manifest(manifest_path)?;
    sync_session_timing_locked(base_dir, manifest_path, manifest, now)
}

fn sync_session_timing_locked(
    base_dir: &Path,
    manifest_path: &Path,
    mut manifest: SessionManifest,
    now: SystemTime,
) -> Result<SessionManifest, HostErrorEnvelope> {
    let Some(current_timing) = manifest.timing.clone() else {
        return Ok(manifest);
    };

    let evaluated_phase = evaluate_phase(&current_timing, now)?;
    let mut next_timing = current_timing.clone();
    let mut should_persist = false;

    if current_timing.phase != evaluated_phase.as_str() {
        next_timing.phase = evaluated_phase.as_str().into();
        should_persist = true;
    }

    let capture_allowed = evaluated_phase != TimingPhase::Ended;

    if current_timing.capture_allowed != capture_allowed {
        next_timing.capture_allowed = capture_allowed;
        should_persist = true;
    }

    let event_timestamp = current_timestamp(now)?;

    if evaluated_phase == TimingPhase::Warning && current_timing.warning_triggered_at.is_none() {
        next_timing.warning_triggered_at = Some(event_timestamp.clone());
        append_timing_log(base_dir, &manifest.session_id, "warning", &event_timestamp)?;
        try_append_operator_audit_record(
            base_dir,
            OperatorAuditRecordInput {
                occurred_at: event_timestamp.clone(),
                session_id: Some(manifest.session_id.clone()),
                event_category: "timing-transition",
                event_type: "warning-triggered",
                summary: "세션이 종료 경고 구간에 들어갔어요.".into(),
                detail: "남은 시간을 기준으로 warning transition이 최종 확정되었어요.".into(),
                actor_id: None,
                source: "timing-policy",
                capture_id: None,
                preset_id: manifest.active_preset_id.clone(),
                published_version: manifest
                    .active_preset
                    .as_ref()
                    .map(|preset| preset.published_version.clone()),
                reason_code: Some("warning".into()),
            },
        );
        should_persist = true;
    }

    if evaluated_phase == TimingPhase::Ended && current_timing.ended_triggered_at.is_none() {
        next_timing.ended_triggered_at = Some(event_timestamp.clone());
        append_timing_log(base_dir, &manifest.session_id, "ended", &event_timestamp)?;
        append_timing_log(
            base_dir,
            &manifest.session_id,
            "extension-hook-reserved",
            &event_timestamp,
        )?;
        try_append_operator_audit_record(
            base_dir,
            OperatorAuditRecordInput {
                occurred_at: event_timestamp.clone(),
                session_id: Some(manifest.session_id.clone()),
                event_category: "timing-transition",
                event_type: "session-ended",
                summary: "세션 종료 시각이 확정되었어요.".into(),
                detail: "추가 촬영 없이 종료 후 결과 판정 단계로 넘어갈 timing transition이 기록되었어요."
                    .into(),
                actor_id: None,
                source: "timing-policy",
                capture_id: None,
                preset_id: manifest.active_preset_id.clone(),
                published_version: manifest
                    .active_preset
                    .as_ref()
                    .map(|preset| preset.published_version.clone()),
                reason_code: Some("ended".into()),
            },
        );
        should_persist = true;
    }

    let next_stage = derive_lifecycle_stage(manifest.lifecycle.stage.as_str(), evaluated_phase);

    if manifest.lifecycle.stage != next_stage {
        manifest.lifecycle.stage = next_stage;
        should_persist = true;
    }

    if !should_persist {
        return Ok(manifest);
    }

    manifest.timing = Some(next_timing);
    manifest.updated_at = event_timestamp;
    write_session_manifest(manifest_path, &manifest)?;

    Ok(manifest)
}

pub fn project_session_timing(
    mut manifest: SessionManifest,
    now: SystemTime,
) -> Result<SessionManifest, HostErrorEnvelope> {
    let Some(current_timing) = manifest.timing.clone() else {
        return Ok(manifest);
    };

    let evaluated_phase = evaluate_phase(&current_timing, now)?;
    let mut next_timing = current_timing.clone();

    if current_timing.phase != evaluated_phase.as_str() {
        next_timing.phase = evaluated_phase.as_str().into();
    }

    let capture_allowed = evaluated_phase != TimingPhase::Ended;

    if current_timing.capture_allowed != capture_allowed {
        next_timing.capture_allowed = capture_allowed;
    }

    let next_stage = derive_lifecycle_stage(manifest.lifecycle.stage.as_str(), evaluated_phase);

    if manifest.lifecycle.stage != next_stage {
        manifest.lifecycle.stage = next_stage;
    }

    manifest.timing = Some(next_timing);

    Ok(manifest)
}

pub fn evaluate_phase(
    timing: &SessionTiming,
    now: SystemTime,
) -> Result<TimingPhase, HostErrorEnvelope> {
    let now_seconds = now
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|_| {
            HostErrorEnvelope::persistence(
                "세션 타이밍을 읽지 못했어요. 잠시 후 다시 확인해 주세요.",
            )
        })?
        .as_secs();
    let warning_at_seconds = rfc3339_to_unix_seconds(&timing.warning_at)?;
    let adjusted_end_at_seconds = rfc3339_to_unix_seconds(&timing.adjusted_end_at)?;

    if now_seconds >= adjusted_end_at_seconds {
        return Ok(TimingPhase::Ended);
    }

    if now_seconds >= warning_at_seconds {
        return Ok(TimingPhase::Warning);
    }

    Ok(TimingPhase::Active)
}

pub fn append_session_timing_event_in_dir(
    base_dir: &Path,
    input: SessionTimingEventInput<'_>,
) -> Result<(), HostErrorEnvelope> {
    let diagnostics_dir = SessionPaths::try_new(base_dir, input.session_id)?.diagnostics_dir;
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
    let occurred_at = current_timestamp(SystemTime::now())?;
    let capture_id = input.capture_id.unwrap_or("none");
    let request_id = input.request_id.unwrap_or("none");
    let detail = input.detail.unwrap_or("none");

    writeln!(
        file,
        "{occurred_at}\tsession={}\tcapture={capture_id}\trequest={request_id}\tevent={}\tdetail={detail}",
        input.session_id, input.event
    )
    .map_err(|error| {
        HostErrorEnvelope::persistence(format!("진단 로그를 남기지 못했어요: {error}"))
    })?;

    Ok(())
}

fn derive_lifecycle_stage(current_stage: &str, phase: TimingPhase) -> String {
    match phase {
        TimingPhase::Active => current_stage.into(),
        TimingPhase::Warning => match current_stage {
            "session-started" | "preset-selected" | "ready" | "capture-ready" | "warning" => {
                "warning".into()
            }
            _ => current_stage.into(),
        },
        TimingPhase::Ended => match current_stage {
            "session-started" | "preset-selected" | "ready" | "capture-ready"
            | "preview-waiting" | "warning" | "ended" => "ended".into(),
            _ => current_stage.into(),
        },
    }
}

fn append_timing_log(
    base_dir: &Path,
    session_id: &str,
    event_name: &str,
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

    writeln!(
        file,
        "{occurred_at}\tsession={session_id}\tevent={event_name}"
    )
    .map_err(|error| {
        HostErrorEnvelope::persistence(format!("진단 로그를 남기지 못했어요: {error}"))
    })?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session::session_manifest::{
        SessionCustomer, SessionLifecycle, SESSION_MANIFEST_SCHEMA_VERSION,
        SESSION_TIMING_SCHEMA_VERSION,
    };
    use crate::session::session_paths::SessionPaths;

    fn temp_dir(test_name: &str) -> std::path::PathBuf {
        let unique = SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("test clock should be valid")
            .as_nanos();
        std::env::temp_dir().join(format!("boothy-timing-{test_name}-{unique}"))
    }

    fn stale_manifest() -> SessionManifest {
        let session_id = "session_00000000000000000000000001".to_string();

        SessionManifest {
            schema_version: SESSION_MANIFEST_SCHEMA_VERSION.into(),
            session_id: session_id.clone(),
            booth_alias: "Kim 4821".into(),
            customer: SessionCustomer {
                name: "Kim".into(),
                phone_last_four: "4821".into(),
            },
            created_at: "2026-04-30T00:00:00Z".into(),
            updated_at: "2026-04-30T00:00:00Z".into(),
            lifecycle: SessionLifecycle {
                status: "active".into(),
                stage: "capture-ready".into(),
            },
            catalog_revision: None,
            catalog_snapshot: None,
            active_preset: None,
            active_preset_id: None,
            active_preset_display_name: None,
            timing: Some(SessionTiming {
                schema_version: SESSION_TIMING_SCHEMA_VERSION.into(),
                session_id,
                adjusted_end_at: "2000-01-01T00:00:00Z".into(),
                warning_at: "1999-12-31T23:55:00Z".into(),
                phase: "active".into(),
                capture_allowed: true,
                approved_extension_minutes: 0,
                approved_extension_audit_ref: None,
                warning_triggered_at: None,
                ended_triggered_at: None,
            }),
            captures: vec![],
            post_end: None,
        }
    }

    #[test]
    fn timing_sync_fails_closed_when_live_manifest_cannot_be_reloaded() {
        let base_dir = temp_dir("reload-failure");
        let paths = SessionPaths::new(&base_dir, "session_00000000000000000000000001");
        std::fs::create_dir_all(&paths.diagnostics_dir).expect("diagnostics dir should exist");
        let manifest_path = paths.manifest_path.clone();
        let corrupted_manifest = "{ this is not valid session json";
        std::fs::write(&manifest_path, corrupted_manifest)
            .expect("corrupted manifest should be writable");

        let result = sync_session_timing_in_dir(
            &base_dir,
            &manifest_path,
            stale_manifest(),
            SystemTime::now(),
        );

        assert!(
            result.is_err(),
            "timing sync should fail closed when live manifest cannot be reloaded"
        );
        assert_eq!(
            std::fs::read_to_string(&manifest_path).expect("manifest should remain readable"),
            corrupted_manifest,
            "stale in-memory timing state must not overwrite the live manifest"
        );

        let _ = std::fs::remove_dir_all(base_dir);
    }
}
