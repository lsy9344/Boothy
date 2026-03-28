use std::env;

use serde::{Deserialize, Serialize};

use crate::contracts::dto::CapabilitySnapshotDto;

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

#[tauri::command]
pub fn log_capture_client_state(input: CaptureClientDebugLogInputDto) {
    log::info!(
        "capture_client_state label={} session={} runtime={} customer_state={} reason_code={} can_capture={} message={}",
        input.label,
        input.session_id.unwrap_or_else(|| "none".into()),
        input.runtime_mode.unwrap_or_else(|| "none".into()),
        input.customer_state.unwrap_or_else(|| "none".into()),
        input.reason_code.unwrap_or_else(|| "none".into()),
        input
            .can_capture
            .map(|value| value.to_string())
            .unwrap_or_else(|| "none".into()),
        input.message.unwrap_or_else(|| "none".into())
    );
}
