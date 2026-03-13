use std::{
    fs,
    path::{Component, Path, PathBuf},
};

use chrono::{SecondsFormat, Utc};
use serde::Deserialize;
use serde_json::json;

use crate::{
    contracts::error_envelope::HostErrorEnvelope,
    diagnostics::{
        error::OperationalLogError,
        lifecycle_log::{LifecycleEventKind, LifecycleEventWrite},
    },
    session::{
        session_manifest::{
            create_session_manifest, SessionActivePresetSelection, SessionManifest,
            SessionManifestDraft, SessionTiming,
        },
        session_paths::{resolve_session_paths, SessionPaths},
    },
    timing::{
        extension_rules::apply_operator_session_extension,
        shoot_end::{create_session_timing_state, resolve_reservation_start_at},
    },
};

#[derive(Debug, Clone, Default)]
pub struct SessionRepository;

impl SessionRepository {
    pub fn new() -> Self {
        Self
    }

    pub fn load_manifest<P: AsRef<Path>>(
        &self,
        manifest_path: P,
    ) -> Result<SessionManifest, HostErrorEnvelope> {
        if !manifest_path.as_ref().exists() {
            return Err(HostErrorEnvelope::session_manifest_invalid(format!(
                "manifest not found: {}",
                manifest_path.as_ref().display()
            )));
        }

        let manifest_json = fs::read_to_string(manifest_path.as_ref())
            .map_err(|error| HostErrorEnvelope::session_manifest_invalid(error.to_string()))?;

        serde_json::from_str(&manifest_json)
            .map_err(|error| HostErrorEnvelope::session_manifest_invalid(error.to_string()))
    }

    pub fn save_manifest<P: AsRef<Path>>(
        &self,
        manifest_path: P,
        manifest: &SessionManifest,
    ) -> Result<(), HostErrorEnvelope> {
        if let Some(parent) = manifest_path.as_ref().parent() {
            fs::create_dir_all(parent)
                .map_err(|error| HostErrorEnvelope::session_manifest_invalid(error.to_string()))?;
        }

        fs::write(
            manifest_path.as_ref(),
            serde_json::to_string_pretty(manifest)
                .map_err(|error| HostErrorEnvelope::session_manifest_invalid(error.to_string()))?,
        )
        .map_err(|error| HostErrorEnvelope::session_manifest_invalid(error.to_string()))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionProvisionRequest {
    pub session_name: String,
    pub created_at: String,
    pub operational_date: String,
    pub reservation_start_at: Option<String>,
    pub session_type: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProvisionedSession {
    pub session_id: String,
    pub session_name: String,
    pub created_at: String,
    pub operational_date: String,
    pub session_dir: PathBuf,
    pub manifest_path: PathBuf,
    pub events_path: PathBuf,
    pub export_status_path: PathBuf,
    pub processed_dir: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionTimingRecord {
    pub session_id: String,
    pub manifest_path: String,
    pub timing: SessionTiming,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionPresetSelectionRecord {
    pub session_id: String,
    pub manifest_path: String,
    pub updated_at: String,
    pub active_preset: SessionActivePresetSelection,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionCaptureRecord {
    pub session_id: String,
    pub manifest_path: String,
    pub original_file_name: String,
    pub processed_path: String,
    pub processed_file_name: String,
    pub capture_id: String,
    pub captured_at: String,
    pub active_preset: SessionActivePresetSelection,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PostEndOutcomeRecord {
    pub session_id: String,
    pub actual_shoot_end_at: String,
    pub outcome_kind: String,
    pub guidance_mode: String,
    pub session_name: Option<String>,
    pub show_session_name: bool,
    pub handoff_target_label: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ExportStatusArtifact {
    status: String,
}

impl ProvisionedSession {
    pub fn as_lifecycle_event(&self, branch_id: &str) -> LifecycleEventWrite {
        LifecycleEventWrite {
            payload_version: 1,
            event_type: LifecycleEventKind::SessionCreated,
            occurred_at: self.created_at.clone(),
            branch_id: branch_id.into(),
            session_id: Some(self.session_id.clone()),
            session_name: Some(self.session_name.clone()),
            current_stage: "preparing".into(),
            actual_shoot_end_at: None,
            catalog_fallback_reason: None,
            extension_status: None,
            recent_fault_category: None,
        }
    }
}

pub fn provision_session(
    session_root_base: &Path,
    request: SessionProvisionRequest,
) -> Result<ProvisionedSession, OperationalLogError> {
    let session_name = sanitize_session_name(request.session_name.trim());
    let created_at = request.created_at.trim();
    let operational_date = request.operational_date.trim();
    let reservation_start_at = request.reservation_start_at.as_deref().map(str::trim);
    let session_type = request.session_type.as_deref().map(str::trim);

    validate_required_text("sessionName", &session_name, 160)?;
    validate_required_text("createdAt", created_at, 64)?;
    validate_operational_date(operational_date)?;

    let session_day_root = session_root_base.join(operational_date);
    fs::create_dir_all(&session_day_root)?;

    let resolved_session_name = resolve_unique_session_name(&session_day_root, &session_name);
    let relative_session_path = Path::new(operational_date).join(&resolved_session_name);
    let paths = resolve_session_paths(session_root_base, &relative_session_path);
    let reservation_start_at = match reservation_start_at {
        Some(value) => {
            validate_required_text("reservationStartAt", value, 64)?;
            value.to_string()
        }
        None => resolve_reservation_start_at(created_at)?,
    };
    let session_type = match session_type {
        Some("standard") => "standard",
        Some("couponExtended") => "couponExtended",
        Some(other) => {
            return Err(OperationalLogError::invalid_payload(format!(
                "sessionType must be one of: standard, couponExtended; got {other}"
            )));
        }
        None => "standard",
    };

    create_artifacts(&paths, created_at)?;

    let session_id = format!("{operational_date}:{resolved_session_name}");
    let manifest = create_session_manifest(SessionManifestDraft {
        session_id: session_id.clone(),
        session_name: resolved_session_name.clone(),
        operational_date: operational_date.into(),
        created_at: created_at.into(),
        reservation_start_at,
        session_type: session_type.into(),
        capture_revision: 0,
        active_preset_name: None,
        active_preset: None,
        latest_capture_id: None,
        captures: Vec::new(),
        paths: paths.clone(),
    })?;

    fs::write(
        &paths.manifest_path,
        serde_json::to_string_pretty(&manifest).map_err(OperationalLogError::from)?,
    )?;

    Ok(ProvisionedSession {
        session_id,
        session_name: resolved_session_name,
        created_at: created_at.into(),
        operational_date: operational_date.into(),
        session_dir: paths.session_dir,
        manifest_path: paths.manifest_path,
        events_path: paths.events_path,
        export_status_path: paths.export_status_path,
        processed_dir: paths.processed_dir,
    })
}

pub fn initialize_session_timing(
    manifest_path: &Path,
    session_id: &str,
    reservation_start_at: &str,
    session_type: &str,
    updated_at: &str,
) -> Result<SessionTimingRecord, OperationalLogError> {
    let mut manifest = load_session_manifest(manifest_path)?;
    validate_session_identity(&manifest, session_id)?;
    manifest.timing = create_session_timing_state(reservation_start_at, session_type, updated_at)?;
    persist_session_manifest(manifest_path, &manifest)?;

    Ok(to_timing_record(&manifest, manifest_path))
}

pub fn get_session_timing(
    manifest_path: &Path,
    session_id: &str,
) -> Result<SessionTimingRecord, OperationalLogError> {
    let manifest = load_session_manifest(manifest_path)?;
    validate_session_identity(&manifest, session_id)?;

    Ok(to_timing_record(&manifest, manifest_path))
}

pub fn resolve_post_end_outcome(
    manifest_path: &Path,
    session_id: &str,
) -> Result<PostEndOutcomeRecord, OperationalLogError> {
    let observed_at = Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true);
    resolve_post_end_outcome_at(manifest_path, session_id, &observed_at)
}

pub fn resolve_post_end_outcome_at(
    manifest_path: &Path,
    session_id: &str,
    observed_at: &str,
) -> Result<PostEndOutcomeRecord, OperationalLogError> {
    let manifest = load_session_manifest(manifest_path)?;
    validate_session_identity(&manifest, session_id)?;

    let observed_at = chrono::DateTime::parse_from_rfc3339(observed_at)
        .map_err(|_| OperationalLogError::invalid_payload("observedAt must be an ISO-8601 timestamp"))?;
    let actual_shoot_end_at = chrono::DateTime::parse_from_rfc3339(&manifest.timing.actual_shoot_end_at)
        .map_err(|_| OperationalLogError::invalid_payload("actualShootEndAt must be an ISO-8601 timestamp"))?;

    if observed_at < actual_shoot_end_at {
        return Err(OperationalLogError::invalid_payload(
            "post-end outcome cannot resolve before actualShootEndAt",
        ));
    }

    let (raw_export_status, unresolved_guidance) =
        resolve_export_status(&manifest.export_status_path, &manifest.export_state.status);
    let (outcome_kind, guidance_mode, show_session_name) = match raw_export_status.as_str() {
        "completed" => ("completed", "standard", false),
        "failed" => ("handoff", "wait-or-call", true),
        "notStarted" | "queued" | "processing" => (
            "export-waiting",
            if unresolved_guidance { "wait-or-call" } else { "standard" },
            false,
        ),
        _ => ("export-waiting", "wait-or-call", false),
    };

    Ok(PostEndOutcomeRecord {
        session_id: manifest.session_id,
        actual_shoot_end_at: manifest.timing.actual_shoot_end_at,
        outcome_kind: outcome_kind.into(),
        guidance_mode: guidance_mode.into(),
        session_name: if show_session_name {
            Some(manifest.session_name)
        } else {
            None
        },
        show_session_name,
        handoff_target_label: None,
    })
}

pub fn extend_session_timing(
    manifest_path: &Path,
    session_id: &str,
    updated_at: &str,
) -> Result<SessionTimingRecord, OperationalLogError> {
    let mut manifest = load_session_manifest(manifest_path)?;
    validate_session_identity(&manifest, session_id)?;
    manifest.timing = apply_operator_session_extension(&manifest.timing, updated_at)?;
    persist_session_manifest(manifest_path, &manifest)?;

    Ok(to_timing_record(&manifest, manifest_path))
}

pub fn select_session_preset(
    manifest_path: &Path,
    session_id: &str,
    active_preset: SessionActivePresetSelection,
    updated_at: &str,
) -> Result<SessionPresetSelectionRecord, OperationalLogError> {
    let mut manifest = load_session_manifest(manifest_path)?;
    validate_session_identity(&manifest, session_id)?;
    validate_required_text("updatedAt", updated_at, 64)?;
    validate_required_text("presetId", &active_preset.preset_id, 120)?;
    validate_required_text("displayName", &active_preset.display_name, 160)?;

    manifest.active_preset_name = Some(active_preset.display_name.clone());
    manifest.active_preset = Some(active_preset.clone());
    persist_session_manifest(manifest_path, &manifest)?;

    Ok(SessionPresetSelectionRecord {
        session_id: manifest.session_id,
        manifest_path: manifest_path.to_string_lossy().replace('\\', "/"),
        updated_at: updated_at.into(),
        active_preset,
    })
}

pub fn append_session_capture(
    manifest_path: &Path,
    session_id: &str,
    active_preset: SessionActivePresetSelection,
    capture_id: &str,
    original_file_name: &str,
    processed_file_name: &str,
    captured_at: &str,
) -> Result<SessionCaptureRecord, HostErrorEnvelope> {
    validate_capture_relative_file_name(original_file_name)?;
    validate_capture_relative_file_name(processed_file_name)?;
    validate_capture_timestamp(captured_at)?;

    let repository = SessionRepository::new();
    let mut manifest = repository.load_manifest(manifest_path)?;
    validate_capture_session_identity(&manifest, session_id)?;

    let original_path = resolve_capture_asset_path(Path::new(&manifest.session_dir), original_file_name)?;
    let processed_path = resolve_capture_asset_path(Path::new(&manifest.processed_dir), processed_file_name)?;

    if !original_path.exists() {
        return Err(HostErrorEnvelope::session_capture_not_found(format!(
            "original capture file not found: {capture_id}"
        )));
    }
    if !processed_path.exists() {
        return Err(HostErrorEnvelope::session_capture_not_found(format!(
            "processed capture file not found: {capture_id}"
        )));
    }

    manifest.capture_revision = manifest.capture_revision.saturating_add(1);
    manifest.captures.push(crate::session::session_manifest::ManifestCaptureRecord {
        capture_id: capture_id.into(),
        original_file_name: original_file_name.into(),
        processed_file_name: processed_file_name.into(),
        captured_at: captured_at.into(),
    });
    manifest.latest_capture_id = Some(capture_id.into());
    manifest.active_preset_name = Some(active_preset.display_name.clone());
    manifest.active_preset = Some(active_preset.clone());

    if let Err(error) = repository.save_manifest(manifest_path, &manifest) {
        let _ = fs::remove_file(&original_path);
        let _ = fs::remove_file(&processed_path);
        return Err(error);
    }

    Ok(SessionCaptureRecord {
        session_id: manifest.session_id,
        manifest_path: manifest_path.to_string_lossy().replace('\\', "/"),
        original_file_name: original_file_name.into(),
        processed_path: processed_path.to_string_lossy().replace('\\', "/"),
        processed_file_name: processed_file_name.into(),
        capture_id: capture_id.into(),
        captured_at: captured_at.into(),
        active_preset,
    })
}

fn create_artifacts(paths: &SessionPaths, created_at: &str) -> Result<(), OperationalLogError> {
    fs::create_dir_all(&paths.session_dir)?;
    fs::create_dir_all(&paths.processed_dir)?;
    fs::write(&paths.events_path, "")?;
    fs::write(
        &paths.export_status_path,
        serde_json::to_string_pretty(&json!({
            "schemaVersion": 1,
            "status": "notStarted",
            "updatedAt": created_at,
        }))
        .map_err(OperationalLogError::from)?,
    )?;

    Ok(())
}

fn load_session_manifest(manifest_path: &Path) -> Result<SessionManifest, OperationalLogError> {
    if !manifest_path.exists() {
        return Err(OperationalLogError::session_manifest_not_found(format!(
            "manifest not found: {}",
            manifest_path.display()
        )));
    }

    let manifest_json = fs::read_to_string(manifest_path)?;

    serde_json::from_str(&manifest_json).map_err(OperationalLogError::from)
}

fn resolve_export_status(export_status_path: &str, fallback_status: &str) -> (String, bool) {
    let parsed_status = fs::read_to_string(export_status_path)
        .ok()
        .and_then(|contents| serde_json::from_str::<ExportStatusArtifact>(&contents).ok())
        .and_then(|artifact| normalize_export_status(&artifact.status).map(str::to_string));

    match parsed_status {
        Some(status) => (status, false),
        None => (
            normalize_export_status(fallback_status)
                .unwrap_or("notStarted")
                .to_string(),
            true,
        ),
    }
}

fn normalize_export_status(status: &str) -> Option<&'static str> {
    match status.trim() {
        "notStarted" => Some("notStarted"),
        "queued" => Some("queued"),
        "processing" => Some("processing"),
        "completed" => Some("completed"),
        "failed" => Some("failed"),
        _ => None,
    }
}

fn persist_session_manifest(
    manifest_path: &Path,
    manifest: &SessionManifest,
) -> Result<(), OperationalLogError> {
    fs::write(
        manifest_path,
        serde_json::to_string_pretty(manifest).map_err(OperationalLogError::from)?,
    )?;

    Ok(())
}

fn validate_session_identity(
    manifest: &SessionManifest,
    session_id: &str,
) -> Result<(), OperationalLogError> {
    if manifest.session_id != session_id {
        return Err(OperationalLogError::session_manifest_session_mismatch(format!(
            "manifest sessionId does not match requested sessionId: {}",
            session_id
        )));
    }

    Ok(())
}

fn validate_capture_session_identity(
    manifest: &SessionManifest,
    session_id: &str,
) -> Result<(), HostErrorEnvelope> {
    if manifest.session_id != session_id {
        return Err(HostErrorEnvelope::session_capture_wrong_session(format!(
            "manifest session does not match request: {session_id}"
        )));
    }

    Ok(())
}

fn to_timing_record(manifest: &SessionManifest, manifest_path: &Path) -> SessionTimingRecord {
    SessionTimingRecord {
        session_id: manifest.session_id.clone(),
        manifest_path: manifest_path.to_string_lossy().replace('\\', "/"),
        timing: manifest.timing.clone(),
    }
}

fn resolve_unique_session_name(session_day_root: &Path, base_session_name: &str) -> String {
    if !session_day_root.join(base_session_name).exists() {
        return base_session_name.into();
    }

    let mut suffix = 2_u32;
    loop {
        let candidate = format!("{base_session_name}_{suffix}");
        if !session_day_root.join(&candidate).exists() {
            return candidate;
        }
        suffix += 1;
    }
}

fn sanitize_session_name(value: &str) -> String {
    let normalized_whitespace = value.split_whitespace().collect::<Vec<_>>().join(" ");
    let mut sanitized = String::with_capacity(normalized_whitespace.len());

    for ch in normalized_whitespace.chars() {
        if is_windows_forbidden_character(ch) || ch.is_control() {
            if !sanitized.ends_with('_') {
                sanitized.push('_');
            }
            continue;
        }

        sanitized.push(ch);
    }

    let mut sanitized = sanitized
        .trim_matches(|ch: char| matches!(ch, ' ' | '.' | '_'))
        .to_string();

    if sanitized.is_empty() {
        sanitized = "session".into();
    }

    if is_windows_reserved_name(&sanitized) {
        sanitized.push_str("_session");
    }

    sanitized
}

fn is_windows_forbidden_character(ch: char) -> bool {
    matches!(ch, '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*')
}

fn is_windows_reserved_name(value: &str) -> bool {
    let normalized = value.trim_end_matches([' ', '.']).to_ascii_uppercase();

    matches!(
        normalized.as_str(),
        "CON"
            | "PRN"
            | "AUX"
            | "NUL"
            | "COM1"
            | "COM2"
            | "COM3"
            | "COM4"
            | "COM5"
            | "COM6"
            | "COM7"
            | "COM8"
            | "COM9"
            | "LPT1"
            | "LPT2"
            | "LPT3"
            | "LPT4"
            | "LPT5"
            | "LPT6"
            | "LPT7"
            | "LPT8"
            | "LPT9"
    )
}

fn validate_required_text(
    field_name: &str,
    value: &str,
    max_length: usize,
) -> Result<(), OperationalLogError> {
    if value.is_empty() {
        return Err(OperationalLogError::invalid_payload(format!("{field_name} is required")));
    }
    if value.len() > max_length {
        return Err(OperationalLogError::invalid_payload(format!(
            "{field_name} must be at most {max_length} characters"
        )));
    }

    Ok(())
}

fn validate_capture_relative_file_name(file_name: &str) -> Result<(), HostErrorEnvelope> {
    if file_name.is_empty() {
        return Err(HostErrorEnvelope::invalid_payload("capture asset path is required"));
    }

    let path = Path::new(file_name);
    let has_normal_component = path
        .components()
        .any(|component| matches!(component, Component::Normal(_)));
    let has_invalid_component = path.components().any(|component| {
        matches!(
            component,
            Component::ParentDir | Component::RootDir | Component::Prefix(_)
        )
    });

    if !has_normal_component || has_invalid_component {
        return Err(HostErrorEnvelope::session_capture_out_of_root(format!(
            "capture resolved outside active session root: {file_name}"
        )));
    }

    Ok(())
}

fn validate_capture_timestamp(captured_at: &str) -> Result<(), HostErrorEnvelope> {
    if chrono::DateTime::parse_from_rfc3339(captured_at).is_err() {
        return Err(HostErrorEnvelope::invalid_payload(
            "capturedAt must be a non-empty ISO-8601 timestamp",
        ));
    }

    Ok(())
}

fn resolve_capture_asset_path(root: &Path, file_name: &str) -> Result<PathBuf, HostErrorEnvelope> {
    let normalized_root = normalize_path(root);
    let normalized_path = normalize_path(root.join(file_name));

    if !normalized_path.starts_with(&normalized_root) {
        return Err(HostErrorEnvelope::session_capture_out_of_root(format!(
            "capture resolved outside active session root: {file_name}"
        )));
    }

    Ok(normalized_path)
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

fn validate_operational_date(value: &str) -> Result<(), OperationalLogError> {
    let bytes = value.as_bytes();
    let is_date = bytes.len() == 10
        && bytes[4] == b'-'
        && bytes[7] == b'-'
        && bytes
            .iter()
            .enumerate()
            .all(|(index, byte)| matches!(index, 4 | 7) || byte.is_ascii_digit());

    if !is_date {
        return Err(OperationalLogError::invalid_payload(
            "operationalDate must be a YYYY-MM-DD local date",
        ));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use tempfile::tempdir;

    use crate::session::session_manifest::ManifestCaptureRecord;

    use super::*;

    #[test]
    fn select_session_preset_persists_active_preset_in_the_current_session_manifest() {
        let session_root = tempdir().expect("tempdir");
        let created_at = "2026-03-08T00:00:00.000Z";
        let session = provision_session(
            session_root.path(),
            SessionProvisionRequest {
                session_name: "김보라 오후 세션".into(),
                created_at: created_at.into(),
                operational_date: "2026-03-08".into(),
                reservation_start_at: None,
                session_type: None,
            },
        )
        .expect("session should provision");

        let selection = select_session_preset(
            &session.manifest_path,
            &session.session_id,
            SessionActivePresetSelection {
                preset_id: "background-pink".into(),
                display_name: "배경지 - 핑크".into(),
            },
            "2026-03-08T00:00:06.000Z",
        )
        .expect("preset selection should persist");

        let manifest = load_session_manifest(&session.manifest_path).expect("manifest should load");

        assert_eq!(selection.session_id, session.session_id);
        assert_eq!(
            selection.active_preset,
            SessionActivePresetSelection {
                preset_id: "background-pink".into(),
                display_name: "배경지 - 핑크".into(),
            }
        );
        assert_eq!(
            manifest.active_preset,
            Some(SessionActivePresetSelection {
                preset_id: "background-pink".into(),
                display_name: "배경지 - 핑크".into(),
            })
        );
        assert_eq!(manifest.active_preset_name.as_deref(), Some("배경지 - 핑크"));
    }

    #[test]
    fn select_session_preset_rejects_a_mismatched_session_id() {
        let session_root = tempdir().expect("tempdir");
        let created_at = "2026-03-08T00:00:00.000Z";
        let session = provision_session(
            session_root.path(),
            SessionProvisionRequest {
                session_name: "김보라 오후 세션".into(),
                created_at: created_at.into(),
                operational_date: "2026-03-08".into(),
                reservation_start_at: None,
                session_type: None,
            },
        )
        .expect("session should provision");

        let error = select_session_preset(
            &session.manifest_path,
            "2026-03-08:다른 세션",
            SessionActivePresetSelection {
                preset_id: "background-pink".into(),
                display_name: "배경지 - 핑크".into(),
            },
            "2026-03-08T00:00:06.000Z",
        )
        .expect_err("mismatched session id should fail");

        assert_eq!(error.code, "session.manifestSessionMismatch");
        assert!(error.message.contains("manifest sessionId does not match requested sessionId"));
    }

    #[test]
    fn select_session_preset_preserves_existing_captures_and_latest_capture_identity() {
        let session_root = tempdir().expect("tempdir");
        let created_at = "2026-03-08T00:00:00.000Z";
        let session = provision_session(
            session_root.path(),
            SessionProvisionRequest {
                session_name: "김보라 오후 세션".into(),
                created_at: created_at.into(),
                operational_date: "2026-03-08".into(),
                reservation_start_at: None,
                session_type: None,
            },
        )
        .expect("session should provision");

        let mut manifest = load_session_manifest(&session.manifest_path).expect("manifest should load");
        manifest.latest_capture_id = Some("capture-002".into());
        manifest.captures = vec![
            ManifestCaptureRecord {
                capture_id: "capture-001".into(),
                original_file_name: "originals/capture-001.nef".into(),
                processed_file_name: "capture-001.jpg".into(),
                captured_at: "2026-03-08T00:00:02.000Z".into(),
            },
            ManifestCaptureRecord {
                capture_id: "capture-002".into(),
                original_file_name: "originals/capture-002.nef".into(),
                processed_file_name: "capture-002.jpg".into(),
                captured_at: "2026-03-08T00:00:04.000Z".into(),
            },
        ];
        persist_session_manifest(&session.manifest_path, &manifest).expect("manifest should persist");

        select_session_preset(
            &session.manifest_path,
            &session.session_id,
            SessionActivePresetSelection {
                preset_id: "background-pink".into(),
                display_name: "배경지 - 핑크".into(),
            },
            "2026-03-08T00:00:06.000Z",
        )
        .expect("preset selection should persist");

        let updated_manifest = load_session_manifest(&session.manifest_path).expect("manifest should load");

        assert_eq!(updated_manifest.latest_capture_id.as_deref(), Some("capture-002"));
        assert_eq!(
            updated_manifest
                .captures
                .iter()
                .map(|capture| capture.capture_id.as_str())
                .collect::<Vec<_>>(),
            vec!["capture-001", "capture-002"]
        );
        assert_eq!(
            updated_manifest.active_preset,
            Some(SessionActivePresetSelection {
                preset_id: "background-pink".into(),
                display_name: "배경지 - 핑크".into(),
            })
        );
    }

    #[test]
    fn resolve_post_end_outcome_translates_host_export_states_into_customer_safe_outcomes() {
        let session_root = tempdir().expect("tempdir");
        let created_at = "2026-03-08T00:00:00.000Z";
        let session = provision_session(
            session_root.path(),
            SessionProvisionRequest {
                session_name: "김보라 오후 세션".into(),
                created_at: created_at.into(),
                operational_date: "2026-03-08".into(),
                reservation_start_at: None,
                session_type: None,
            },
        )
        .expect("session should provision");

        fs::write(
            &session.export_status_path,
            serde_json::to_string_pretty(&json!({
                "schemaVersion": 1,
                "status": "failed",
                "updatedAt": "2026-03-08T00:50:01.000Z",
            }))
            .expect("export status should serialize"),
        )
        .expect("export status should persist");

        let outcome = resolve_post_end_outcome_at(
            &session.manifest_path,
            &session.session_id,
            "2026-03-08T00:50:05.000Z",
        )
        .expect("post-end outcome should resolve");

        assert_eq!(outcome.outcome_kind, "handoff");
        assert_eq!(outcome.guidance_mode, "wait-or-call");
        assert_eq!(outcome.session_name.as_deref(), Some("김보라 오후 세션"));
        assert!(outcome.show_session_name);
    }

    #[test]
    fn resolve_post_end_outcome_rejects_resolution_before_the_authoritative_shoot_end() {
        let session_root = tempdir().expect("tempdir");
        let created_at = "2026-03-08T00:00:00.000Z";
        let session = provision_session(
            session_root.path(),
            SessionProvisionRequest {
                session_name: "김보라 오후 세션".into(),
                created_at: created_at.into(),
                operational_date: "2026-03-08".into(),
                reservation_start_at: None,
                session_type: None,
            },
        )
        .expect("session should provision");

        let error = resolve_post_end_outcome_at(
            &session.manifest_path,
            &session.session_id,
            "2026-03-08T00:49:55.000Z",
        )
        .expect_err("resolution should stay blocked before actualShootEndAt");

        assert_eq!(error.code, "diagnostics.invalidPayload");
        assert!(error.message.contains("actualShootEndAt"));
    }

}
