use std::env;

use serde::{Deserialize, Serialize};
use tauri::Manager;

use crate::{
    contracts::dto::CapabilitySnapshotDto,
    session::session_repository::resolve_app_session_base_dir,
    timing::{append_session_timing_event_in_dir, SessionTimingEventInput},
};

const RUNTIME_PROFILE_ENV: &str = "BOOTHY_RUNTIME_PROFILE";
const ADMIN_AUTHENTICATED_ENV: &str = "BOOTHY_ADMIN_AUTHENTICATED";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CaptureClientDebugLogInputDto {
    pub label: String,
    pub session_id: Option<String>,
    pub runtime_mode: Option<String>,
    pub customer_state: Option<String>,
    pub reason_code: Option<String>,
    pub can_capture: Option<bool>,
    pub message: Option<String>,
}

fn read_bool_env(name: &str) -> bool {
    env::var(name)
        .ok()
        .map(|value| {
            matches!(
                value.trim().to_ascii_lowercase().as_str(),
                "1" | "true" | "yes"
            )
        })
        .unwrap_or(false)
}

pub fn capability_snapshot_for_profile(
    profile: &str,
    is_admin_authenticated: bool,
) -> CapabilitySnapshotDto {
    let mut allowed_surfaces = vec!["booth".to_string()];

    match profile {
        "operator-enabled" => {
            allowed_surfaces.push("operator".into());
            allowed_surfaces.push("settings".into());
        }
        "authoring-enabled" => {
            allowed_surfaces.push("operator".into());
            allowed_surfaces.push("authoring".into());
            allowed_surfaces.push("settings".into());
        }
        _ => {}
    }

    CapabilitySnapshotDto {
        is_admin_authenticated,
        allowed_surfaces,
    }
}

pub fn resolve_runtime_capability_snapshot() -> CapabilitySnapshotDto {
    let profile = env::var(RUNTIME_PROFILE_ENV).unwrap_or_else(|_| "booth".into());
    let is_admin_authenticated = read_bool_env(ADMIN_AUTHENTICATED_ENV);

    capability_snapshot_for_profile(&profile, is_admin_authenticated)
}

#[tauri::command]
pub fn get_capability_snapshot() -> CapabilitySnapshotDto {
    resolve_runtime_capability_snapshot()
}

pub fn append_capture_client_timing_event_in_dir(
    base_dir: &std::path::Path,
    input: &CaptureClientDebugLogInputDto,
) {
    let Some(session_id) = input
        .session_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    else {
        return;
    };

    if !should_append_capture_client_timing_event(&input.label) {
        return;
    }

    // Mirror the key first-visible latency seam into the per-session log so
    // time-reduction analysis does not depend on the global app log alone.
    let detail = input.message.as_deref();
    let _ = append_session_timing_event_in_dir(
        base_dir,
        SessionTimingEventInput {
            session_id,
            event: input.label.as_str(),
            capture_id: extract_capture_client_detail_value(detail, "captureId"),
            request_id: extract_capture_client_detail_value(detail, "requestId"),
            detail,
        },
    );
}

#[tauri::command]
pub fn log_capture_client_state(app: tauri::AppHandle, input: CaptureClientDebugLogInputDto) {
    let session_id = input.session_id.clone().unwrap_or_else(|| "none".into());
    let runtime_mode = input.runtime_mode.clone().unwrap_or_else(|| "none".into());
    let customer_state = input
        .customer_state
        .clone()
        .unwrap_or_else(|| "none".into());
    let reason_code = input.reason_code.clone().unwrap_or_else(|| "none".into());
    let can_capture = input
        .can_capture
        .map(|value| value.to_string())
        .unwrap_or_else(|| "none".into());
    let message = input.message.clone().unwrap_or_else(|| "none".into());

    log::info!(
        "capture_client_state label={} session={} runtime={} customer_state={} reason_code={} can_capture={} message={}",
        input.label,
        session_id,
        runtime_mode,
        customer_state,
        reason_code,
        can_capture,
        message
    );

    let Ok(app_local_data_dir) = app.path().app_local_data_dir() else {
        return;
    };
    let base_dir = resolve_app_session_base_dir(app_local_data_dir);
    append_capture_client_timing_event_in_dir(&base_dir, &input);
}

fn should_append_capture_client_timing_event(label: &str) -> bool {
    matches!(
        label,
        "button-pressed"
            | "fast-preview-ready"
            | "current-session-preview-visible"
            | "current-session-preview-pending-visible"
            | "recent-session-visible"
            | "recent-session-pending-visible"
    )
}

fn extract_capture_client_detail_value<'a>(detail: Option<&'a str>, key: &str) -> Option<&'a str> {
    detail.and_then(|value| {
        value
            .split(';')
            .filter_map(|entry| entry.split_once('='))
            .find_map(|(entry_key, entry_value)| {
                (entry_key.trim() == key)
                    .then_some(entry_value.trim())
                    .filter(|entry_value| !entry_value.is_empty() && *entry_value != "unknown")
            })
    })
}
