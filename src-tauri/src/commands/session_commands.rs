use std::fs;
use std::sync::OnceLock;

use chrono::{Local, SecondsFormat, Utc};
use rusqlite::Connection;
use serde::Deserialize;
use tauri::{AppHandle, Manager, Runtime, State};

use crate::{
    contracts::dto::{
        GetPostEndOutcomePayload, GetSessionTimingPayload, InitializeSessionTimingPayload, PostEndOutcomeEnvelope,
        PostEndOutcomeFailureEnvelope, PostEndOutcomeResult, PostEndOutcomeSuccessEnvelope, SessionStartEnvelope,
        SessionStartFailureEnvelope, SessionStartPayload, SessionStartResult, SessionStartSuccessEnvelope,
        SelectSessionPresetEnvelope, SelectSessionPresetFailureEnvelope, SelectSessionPresetPayload,
        SelectSessionPresetResult, SelectSessionPresetSuccessEnvelope, SessionActivePresetDto,
        SessionTimingEnvelope, SessionTimingFailureEnvelope, SessionTimingResult, SessionTimingSuccessEnvelope,
    },
    db::sqlite::{open_operational_log_connection, OperationalLogState},
    diagnostics::lifecycle_log::insert_lifecycle_event,
    session::{
        session_paths::resolve_booth_session_root,
        session_repository::{
            get_session_timing as get_session_timing_record,
            initialize_session_timing as initialize_session_timing_record,
            provision_session,
            resolve_post_end_outcome,
            select_session_preset as persist_session_preset,
            SessionProvisionRequest,
        },
        session_manifest::SessionActivePresetSelection,
    },
};

#[tauri::command]
pub fn start_session<R: Runtime>(
    app_handle: AppHandle<R>,
    state: State<'_, OperationalLogState>,
    payload: SessionStartPayload,
) -> SessionStartEnvelope {
    let session_name = payload.session_name.trim();
    let branch_id = payload.branch_id.trim();

    if session_name.is_empty() {
        return SessionStartEnvelope::Failure(SessionStartFailureEnvelope {
            ok: false,
            error_code: "session_name.required".into(),
            message: "Session name is required".into(),
        });
    }

    if branch_id.is_empty() {
        return provisioning_failed("branchId is required".into());
    }

    let created_at = Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true);
    let operational_date = Local::now().format("%F").to_string();
    let session_root = match app_handle.path().app_local_data_dir() {
        Ok(path) => resolve_booth_session_root(&path),
        Err(error) => {
            return provisioning_failed(format!("failed to resolve app-local session root: {error}"));
        }
    };

    let provisioned_session = match provision_session(
        &session_root,
        SessionProvisionRequest {
            session_name: session_name.into(),
            created_at,
            operational_date,
            reservation_start_at: payload.reservation_start_at,
            session_type: payload.session_type,
        },
    ) {
        Ok(session) => session,
        Err(error) => return provisioning_failed(error.to_string()),
    };

    let connection = match open_operational_log_connection(state.db_path()) {
        Ok(connection) => connection,
        Err(error) => return provisioning_failed_with_cleanup(&provisioned_session, error.to_string()),
    };

    let provisioned_session = match finalize_session_start(&connection, provisioned_session, branch_id) {
        Ok(session) => session,
        Err(error) => return provisioning_failed(error),
    };

    SessionStartEnvelope::Success(SessionStartSuccessEnvelope {
        ok: true,
        value: SessionStartResult {
            session_id: provisioned_session.session_id,
            session_name: provisioned_session.session_name,
            session_folder: provisioned_session.session_dir.to_string_lossy().replace('\\', "/"),
            manifest_path: provisioned_session
                .manifest_path
                .to_string_lossy()
                .replace('\\', "/"),
            created_at: provisioned_session.created_at,
            preparation_state: "preparing".into(),
        },
    })
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
struct ApprovedPresetCatalogEntry {
    id: String,
    name: String,
}

fn approved_preset_catalog() -> &'static [ApprovedPresetCatalogEntry] {
    static APPROVED_PRESET_CATALOG: OnceLock<Vec<ApprovedPresetCatalogEntry>> = OnceLock::new();

    APPROVED_PRESET_CATALOG.get_or_init(|| {
        serde_json::from_str(include_str!("../../../src/shared-contracts/presets/presetCatalog.json"))
            .expect("approved preset catalog asset should deserialize")
    })
}

fn provisioning_failed(message: String) -> SessionStartEnvelope {
    let is_validation_failure = message.starts_with("diagnostics.invalidPayload:")
        || message.contains("diagnostics.invalidPayload")
        || message.contains("branchId is required");

    let (error_code, safe_message) = if is_validation_failure {
        ("session.validation_failed", "Session details are invalid.")
    } else {
        ("session.provisioning_failed", "Unable to start the session right now.")
    };

    SessionStartEnvelope::Failure(SessionStartFailureEnvelope {
        ok: false,
        error_code: error_code.into(),
        message: safe_message.into(),
    })
}

fn finalize_session_start(
    connection: &Connection,
    provisioned_session: crate::session::session_repository::ProvisionedSession,
    branch_id: &str,
) -> Result<crate::session::session_repository::ProvisionedSession, String> {
    if let Err(error) = insert_lifecycle_event(
        connection,
        &provisioned_session.as_lifecycle_event(branch_id),
    ) {
        return Err(rollback_provisioned_session(&provisioned_session, error.to_string()));
    }

    Ok(provisioned_session)
}

fn provisioning_failed_with_cleanup(
    provisioned_session: &crate::session::session_repository::ProvisionedSession,
    message: String,
) -> SessionStartEnvelope {
    provisioning_failed(rollback_provisioned_session(provisioned_session, message))
}

fn rollback_provisioned_session(
    provisioned_session: &crate::session::session_repository::ProvisionedSession,
    message: String,
) -> String {
    match cleanup_provisioned_session(provisioned_session) {
        Ok(()) => message,
        Err(cleanup_error) => format!("{message}; cleanup failed: {cleanup_error}"),
    }
}

fn cleanup_provisioned_session(
    provisioned_session: &crate::session::session_repository::ProvisionedSession,
) -> Result<(), String> {
    if provisioned_session.session_dir.exists() {
        fs::remove_dir_all(&provisioned_session.session_dir)
            .map_err(|error| format!("failed to remove session directory: {error}"))?;
    }

    if let Some(day_root) = provisioned_session.session_dir.parent() {
        if day_root.exists()
            && day_root
                .read_dir()
                .map_err(|error| format!("failed to inspect session day root: {error}"))?
                .next()
                .is_none()
        {
            fs::remove_dir(day_root)
                .map_err(|error| format!("failed to remove empty session day root: {error}"))?;
        }
    }

    Ok(())
}

#[tauri::command]
pub fn initialize_session_timing(payload: InitializeSessionTimingPayload) -> SessionTimingEnvelope {
    match initialize_session_timing_record(
        std::path::Path::new(&payload.manifest_path),
        &payload.session_id,
        &payload.reservation_start_at,
        &payload.session_type,
        &payload.updated_at,
    ) {
        Ok(record) => SessionTimingEnvelope::Success(SessionTimingSuccessEnvelope {
            ok: true,
            value: SessionTimingResult {
                session_id: record.session_id,
                manifest_path: record.manifest_path,
                timing: record.timing,
            },
        }),
        Err(error) => session_timing_failed(error.code.as_str(), error.message),
    }
}

#[tauri::command]
pub fn get_session_timing(payload: GetSessionTimingPayload) -> SessionTimingEnvelope {
    match get_session_timing_record(std::path::Path::new(&payload.manifest_path), &payload.session_id) {
        Ok(record) => SessionTimingEnvelope::Success(SessionTimingSuccessEnvelope {
            ok: true,
            value: SessionTimingResult {
                session_id: record.session_id,
                manifest_path: record.manifest_path,
                timing: record.timing,
            },
        }),
        Err(error) => session_timing_failed(error.code.as_str(), error.message),
    }
}

#[tauri::command]
pub fn get_post_end_outcome(payload: GetPostEndOutcomePayload) -> PostEndOutcomeEnvelope {
    match resolve_post_end_outcome(std::path::Path::new(&payload.manifest_path), &payload.session_id) {
        Ok(record) => PostEndOutcomeEnvelope::Success(PostEndOutcomeSuccessEnvelope {
            ok: true,
            value: PostEndOutcomeResult {
                session_id: record.session_id,
                actual_shoot_end_at: record.actual_shoot_end_at,
                outcome_kind: record.outcome_kind,
                guidance_mode: record.guidance_mode,
                session_name: record.session_name,
                show_session_name: record.show_session_name,
                handoff_target_label: record.handoff_target_label,
            },
        }),
        Err(error) => post_end_outcome_failed(error.code.as_str(), error.message),
    }
}

#[tauri::command]
pub fn select_session_preset<R: Runtime>(
    app_handle: AppHandle<R>,
    payload: SelectSessionPresetPayload,
) -> SelectSessionPresetEnvelope {
    build_select_session_preset_response(payload, |session_id| {
        resolve_manifest_path(&app_handle, session_id)
    })
}

fn session_timing_failed(source_code: &str, message: String) -> SessionTimingEnvelope {
    let error_code = if source_code == "diagnostics.invalidPayload" && message.contains("not found") {
        "session_timing.not_found"
    } else if source_code == "diagnostics.invalidPayload" {
        "session_timing.invalid_payload"
    } else {
        "session_timing.persistence_failed"
    };

    SessionTimingEnvelope::Failure(SessionTimingFailureEnvelope {
        ok: false,
        error_code: error_code.into(),
        message,
    })
}

fn post_end_outcome_failed(source_code: &str, message: String) -> PostEndOutcomeEnvelope {
    let error_code = if source_code == "diagnostics.invalidPayload" && message.contains("before actualShootEndAt") {
        "post_end.not_ready"
    } else if source_code == "diagnostics.invalidPayload" {
        "post_end.invalid_payload"
    } else {
        "post_end.persistence_failed"
    };

    PostEndOutcomeEnvelope::Failure(PostEndOutcomeFailureEnvelope {
        ok: false,
        error_code: error_code.into(),
        message,
    })
}

fn select_session_preset_failed(error_code: &str, message: String) -> SelectSessionPresetEnvelope {
    SelectSessionPresetEnvelope::Failure(SelectSessionPresetFailureEnvelope {
        ok: false,
        error_code: error_code.into(),
        message,
    })
}

fn classify_select_session_preset_failure(source_code: &str, message: &str) -> &'static str {
    let _ = message;

    if source_code == "session.manifestNotFound" {
        return "session.preset_selection_session_not_found";
    }

    if source_code == "session.manifestSessionMismatch"
        || source_code == "session.preset_selection_invalid_session"
    {
        return "session.preset_selection_invalid_session";
    }

    "session.preset_selection_failed"
}

fn resolve_manifest_path<R: Runtime>(
    app_handle: &AppHandle<R>,
    session_id: &str,
) -> Result<std::path::PathBuf, String> {
    let (operational_date, session_name) = session_id
        .split_once(':')
        .ok_or_else(|| "Invalid sessionId format".to_string())?;

    let app_local_data_dir = app_handle
        .path()
        .app_local_data_dir()
        .map_err(|error| format!("failed to resolve app-local session root: {error}"))?;
    let session_root = resolve_booth_session_root(&app_local_data_dir);

    Ok(crate::session::session_paths::resolve_session_paths(
        &session_root,
        std::path::Path::new(operational_date).join(session_name),
    )
    .manifest_path)
}

fn resolve_active_preset(preset_id: &str) -> Option<SessionActivePresetDto> {
    approved_preset_catalog()
        .iter()
        .find(|preset| preset.id == preset_id)
        .map(|preset| SessionActivePresetDto {
            preset_id: preset.id.clone(),
            display_name: preset.name.clone(),
        })
}

fn build_select_session_preset_response<F>(
    payload: SelectSessionPresetPayload,
    resolve_manifest_path_fn: F,
) -> SelectSessionPresetEnvelope
where
    F: FnOnce(&str) -> Result<std::path::PathBuf, String>,
{
    let preset = match resolve_active_preset(&payload.preset_id) {
        Some(preset) => preset,
        None => {
            return select_session_preset_failed(
                "session.preset_selection_invalid_preset",
                "Unknown presetId".into(),
            )
        }
    };

    let manifest_path = match resolve_manifest_path_fn(&payload.session_id) {
        Ok(path) => path,
        Err(message) => {
            return select_session_preset_failed(
                "session.preset_selection_invalid_session",
                message,
            )
        }
    };
    let updated_at = Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true);

    match persist_session_preset(
        &manifest_path,
        &payload.session_id,
        SessionActivePresetSelection {
            preset_id: preset.preset_id.clone(),
            display_name: preset.display_name.clone(),
        },
        &updated_at,
    ) {
        Ok(record) => SelectSessionPresetEnvelope::Success(SelectSessionPresetSuccessEnvelope {
            ok: true,
            value: SelectSessionPresetResult {
                manifest_path: record.manifest_path,
                updated_at: record.updated_at,
                active_preset: SessionActivePresetDto {
                    preset_id: record.active_preset.preset_id,
                    display_name: record.active_preset.display_name,
                },
            },
        }),
        Err(error) => select_session_preset_failed(
            classify_select_session_preset_failure(error.code.as_str(), &error.message),
            error.message,
        ),
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::tempdir;

    use super::*;
    use crate::contracts::dto::{SelectSessionPresetEnvelope, SelectSessionPresetPayload};

    #[test]
    fn provisioning_failed_never_echoes_internal_diagnostics_to_the_frontend_contract() {
        let envelope =
            provisioning_failed("failed to resolve app-local session root: Access is denied.".into());

        match envelope {
            SessionStartEnvelope::Failure(failure) => {
                assert_eq!(failure.error_code, "session.provisioning_failed");
                assert_eq!(failure.message, "Unable to start the session right now.");
                assert!(!failure.message.contains("Access is denied"));
            }
            SessionStartEnvelope::Success(_) => panic!("expected failure envelope"),
        }
    }

    #[test]
    fn provisioning_failed_maps_host_validation_messages_to_a_typed_validation_code() {
        let envelope = provisioning_failed("diagnostics.invalidPayload: branchId is required".into());

        match envelope {
            SessionStartEnvelope::Failure(failure) => {
                assert_eq!(failure.error_code, "session.validation_failed");
                assert_eq!(failure.message, "Session details are invalid.");
            }
            SessionStartEnvelope::Success(_) => panic!("expected failure envelope"),
        }
    }

    #[test]
    fn provisioning_failed_with_cleanup_removes_partial_session_artifacts_before_returning_a_safe_failure() {
        let temp_dir = tempdir().expect("tempdir");
        let day_root = temp_dir.path().join("2026-03-13");
        let session_dir = day_root.join("김보라 오후 세션");
        let processed_dir = session_dir.join("processed");
        let manifest_path = session_dir.join("session.json");
        let events_path = session_dir.join("events.ndjson");
        let export_status_path = session_dir.join("export-status.json");

        fs::create_dir_all(&processed_dir).expect("processed dir should exist");
        fs::write(&manifest_path, "{}").expect("manifest should exist");
        fs::write(&events_path, "").expect("events file should exist");
        fs::write(&export_status_path, "{}").expect("export status should exist");

        let envelope = provisioning_failed_with_cleanup(
            &crate::session::session_repository::ProvisionedSession {
                session_id: "2026-03-13:김보라 오후 세션".into(),
                session_name: "김보라 오후 세션".into(),
                created_at: "2026-03-13T09:00:00.000Z".into(),
                operational_date: "2026-03-13".into(),
                session_dir: session_dir.clone(),
                manifest_path,
                events_path,
                export_status_path,
                processed_dir,
            },
            "diagnostics.storageFailure: access denied".into(),
        );

        match envelope {
            SessionStartEnvelope::Failure(failure) => {
                assert_eq!(failure.error_code, "session.provisioning_failed");
                assert_eq!(failure.message, "Unable to start the session right now.");
            }
            SessionStartEnvelope::Success(_) => panic!("expected failure envelope"),
        }

        assert!(!session_dir.exists());
        assert!(!day_root.exists());
    }

    #[test]
    fn select_session_preset_returns_typed_failure_when_the_preset_is_unknown() {
        let envelope = build_select_session_preset_response(
            SelectSessionPresetPayload {
                session_id: "2026-03-08:kim".into(),
                preset_id: "unknown-preset".into(),
            },
            |_session_id| Ok(std::path::PathBuf::from("ignored/session.json")),
        );

        assert_eq!(
            envelope,
            SelectSessionPresetEnvelope::Failure(SelectSessionPresetFailureEnvelope {
                ok: false,
                error_code: "session.preset_selection_invalid_preset".into(),
                message: "Unknown presetId".into(),
            })
        );
    }

    #[test]
    fn select_session_preset_returns_typed_failure_when_the_session_id_is_invalid() {
        let envelope = build_select_session_preset_response(
            SelectSessionPresetPayload {
                session_id: "invalid-session-id".into(),
                preset_id: "background-pink".into(),
            },
            |session_id: &str| {
                if session_id.contains(':') {
                    Ok(std::path::PathBuf::from("ignored/session.json"))
                } else {
                    Err("Invalid sessionId format".into())
                }
            },
        );

        assert_eq!(
            envelope,
            SelectSessionPresetEnvelope::Failure(SelectSessionPresetFailureEnvelope {
                ok: false,
                error_code: "session.preset_selection_invalid_session".into(),
                message: "Invalid sessionId format".into(),
            })
        );
    }

    #[test]
    fn select_session_preset_returns_typed_failure_when_the_manifest_is_missing() {
        let envelope = build_select_session_preset_response(
            SelectSessionPresetPayload {
                session_id: "2026-03-08:kim".into(),
                preset_id: "background-pink".into(),
            },
            |_session_id| Ok(std::path::PathBuf::from("missing/session.json")),
        );

        assert_eq!(
            envelope,
            SelectSessionPresetEnvelope::Failure(SelectSessionPresetFailureEnvelope {
                ok: false,
                error_code: "session.preset_selection_session_not_found".into(),
                message: "manifest not found: missing/session.json".into(),
            })
        );
    }
}
