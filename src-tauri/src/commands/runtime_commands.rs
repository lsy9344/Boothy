use std::env;

use serde::{Deserialize, Serialize};
use tauri::Manager;

use crate::{
    contracts::dto::CapabilitySnapshotDto,
    render::dedicated_renderer::{
        append_capture_visibility_evidence_update_in_dir, resolve_capture_visibility_owner_in_dir,
    },
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
    let enriched_detail =
        enrich_capture_client_timing_detail(base_dir, session_id, input.label.as_str(), input.message.as_deref());
    let detail = enriched_detail.as_deref().or(input.message.as_deref());
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
    append_capture_visibility_evidence_if_needed(base_dir, session_id, input.label.as_str(), detail);
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
        "fast-preview-ready"
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

fn append_capture_visibility_evidence_if_needed(
    base_dir: &std::path::Path,
    session_id: &str,
    label: &str,
    detail: Option<&str>,
) {
    if !matches!(
        label,
        "current-session-preview-visible" | "recent-session-visible"
    ) {
        return;
    }

    let Some(capture_id) = extract_capture_client_detail_value(detail, "captureId") else {
        return;
    };
    let Some(request_id) = extract_capture_client_detail_value(detail, "requestId") else {
        return;
    };
    let Some(ready_at_ms) = extract_capture_client_detail_value(detail, "readyAtMs")
        .and_then(|value| value.parse::<u64>().ok())
    else {
        return;
    };
    let Some(ui_lag_ms) = extract_capture_client_detail_value(detail, "uiLagMs")
        .and_then(|value| value.parse::<u64>().ok())
    else {
        return;
    };

    let visible_at_ms = ready_at_ms.saturating_add(ui_lag_ms);
    let _ = append_capture_visibility_evidence_update_in_dir(
        base_dir,
        session_id,
        capture_id,
        request_id,
        visible_at_ms,
        extract_capture_client_detail_value(detail, "visibleOwner"),
    );
}

fn enrich_capture_client_timing_detail(
    base_dir: &std::path::Path,
    session_id: &str,
    label: &str,
    detail: Option<&str>,
) -> Option<String> {
    let detail = detail?;
    if !matches!(
        label,
        "current-session-preview-visible" | "recent-session-visible"
    ) {
        return Some(detail.to_string());
    }

    let capture_id = extract_capture_client_detail_value(Some(detail), "captureId")?;
    let request_id = extract_capture_client_detail_value(Some(detail), "requestId")?;
    let ready_at_ms = extract_capture_client_detail_value(Some(detail), "readyAtMs")
        .and_then(|value| value.parse::<u64>().ok())?;
    let ui_lag_ms = extract_capture_client_detail_value(Some(detail), "uiLagMs")
        .and_then(|value| value.parse::<u64>().ok())?;
    let visible_at_ms = ready_at_ms.saturating_add(ui_lag_ms);
    let visible_owner = extract_capture_client_detail_value(Some(detail), "visibleOwner")
        .map(str::to_string)
        .or_else(|| {
            resolve_capture_visibility_owner_in_dir(base_dir, session_id, capture_id, request_id)
                .ok()
                .flatten()
        })?;

    let mut parts = detail
        .split(';')
        .map(str::to_string)
        .collect::<Vec<_>>();
    if extract_capture_client_detail_value(Some(detail), "visibleOwner").is_none() {
        parts.push(format!("visibleOwner={visible_owner}"));
    }
    if extract_capture_client_detail_value(Some(detail), "visibleOwnerTransitionAtMs").is_none() {
        parts.push(format!("visibleOwnerTransitionAtMs={visible_at_ms}"));
    }

    Some(parts.join(";"))
}
