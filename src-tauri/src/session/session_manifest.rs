use serde::{Deserialize, Serialize};

use crate::contracts::schema_version::MANIFEST_SCHEMA_VERSION;
use crate::diagnostics::error::OperationalLogError;
use crate::timing::shoot_end::create_session_timing_state;

use super::session_paths::SessionPaths;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CameraState {
    pub connection_state: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ExportState {
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SessionTiming {
    pub reservation_start_at: String,
    pub actual_shoot_end_at: String,
    pub session_type: String,
    pub operator_extension_count: u32,
    pub last_timing_update_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ManifestCaptureRecord {
    pub capture_id: String,
    pub original_file_name: String,
    pub processed_file_name: String,
    pub captured_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SessionActivePresetSelection {
    pub preset_id: String,
    pub display_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SessionManifest {
    pub schema_version: i64,
    pub session_id: String,
    pub session_name: String,
    pub operational_date: String,
    pub created_at: String,
    pub session_dir: String,
    pub manifest_path: String,
    pub events_path: String,
    pub export_status_path: String,
    pub processed_dir: String,
    #[serde(default)]
    pub capture_revision: u32,
    pub latest_capture_id: Option<String>,
    pub active_preset_name: Option<String>,
    pub active_preset: Option<SessionActivePresetSelection>,
    pub captures: Vec<ManifestCaptureRecord>,
    pub camera_state: CameraState,
    pub timing: SessionTiming,
    pub export_state: ExportState,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionManifestDraft {
    pub session_id: String,
    pub session_name: String,
    pub operational_date: String,
    pub created_at: String,
    pub reservation_start_at: String,
    pub session_type: String,
    pub capture_revision: u32,
    pub active_preset_name: Option<String>,
    pub active_preset: Option<SessionActivePresetSelection>,
    pub latest_capture_id: Option<String>,
    pub captures: Vec<ManifestCaptureRecord>,
    pub paths: SessionPaths,
}

pub fn create_session_manifest(
    draft: SessionManifestDraft,
) -> Result<SessionManifest, OperationalLogError> {
    let timing = create_session_timing_state(
        &draft.reservation_start_at,
        &draft.session_type,
        &draft.created_at,
    )
    ?;

    Ok(SessionManifest {
        schema_version: MANIFEST_SCHEMA_VERSION,
        session_id: draft.session_id,
        session_name: draft.session_name,
        operational_date: draft.operational_date,
        created_at: draft.created_at,
        session_dir: to_wire_path(&draft.paths.session_dir),
        manifest_path: to_wire_path(&draft.paths.manifest_path),
        events_path: to_wire_path(&draft.paths.events_path),
        export_status_path: to_wire_path(&draft.paths.export_status_path),
        processed_dir: to_wire_path(&draft.paths.processed_dir),
        capture_revision: draft.capture_revision,
        latest_capture_id: draft.latest_capture_id,
        active_preset_name: draft.active_preset_name,
        active_preset: draft.active_preset,
        captures: draft.captures,
        camera_state: CameraState {
            connection_state: "offline".into(),
        },
        timing,
        export_state: ExportState {
            status: "notStarted".into(),
        },
    })
}

fn to_wire_path(path: &std::path::Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}
