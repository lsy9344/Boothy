use std::env;

use crate::contracts::dto::CapabilitySnapshotDto;

const RUNTIME_PROFILE_ENV: &str = "BOOTHY_RUNTIME_PROFILE";
const ADMIN_AUTHENTICATED_ENV: &str = "BOOTHY_ADMIN_AUTHENTICATED";

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
