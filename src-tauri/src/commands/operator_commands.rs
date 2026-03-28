use crate::{
    capture::helper_supervisor::try_ensure_helper_running,
    commands::runtime_commands::resolve_runtime_capability_snapshot,
    contracts::dto::{
        HostErrorEnvelope, OperatorAuditQueryFilterDto, OperatorAuditQueryResultDto,
        OperatorRecoveryActionInputDto, OperatorRecoveryActionResultDto,
        OperatorRecoverySummaryDto, OperatorSessionSummaryDto,
    },
    diagnostics::{
        audit_log::load_operator_audit_history_in_dir,
        ensure_operator_window_label, find_current_operator_session_id_in_dir,
        load_operator_session_summary_in_dir,
        recovery::{
            execute_operator_recovery_action_in_dir, load_operator_recovery_summary_in_dir,
        },
    },
    session::session_repository::resolve_app_session_base_dir,
};
use tauri::Manager;

#[tauri::command]
pub fn load_operator_session_summary(
    app: tauri::AppHandle,
    window: tauri::Window,
) -> Result<OperatorSessionSummaryDto, HostErrorEnvelope> {
    let app_local_data_dir = app.path().app_local_data_dir().map_err(|error| {
        HostErrorEnvelope::persistence(format!("앱 데이터 경로를 확인하지 못했어요: {error}"))
    })?;
    let base_dir = resolve_app_session_base_dir(app_local_data_dir);
    let capability_snapshot = resolve_runtime_capability_snapshot();
    ensure_operator_window_label(window.label())?;
    if let Some(session_id) = find_current_operator_session_id_in_dir(&base_dir)? {
        try_ensure_helper_running(&base_dir, &session_id);
    }

    load_operator_session_summary_in_dir(&base_dir, &capability_snapshot)
}

#[tauri::command]
pub fn load_operator_recovery_summary(
    app: tauri::AppHandle,
    window: tauri::Window,
) -> Result<OperatorRecoverySummaryDto, HostErrorEnvelope> {
    let app_local_data_dir = app.path().app_local_data_dir().map_err(|error| {
        HostErrorEnvelope::persistence(format!("앱 데이터 경로를 확인하지 못했어요: {error}"))
    })?;
    let base_dir = resolve_app_session_base_dir(app_local_data_dir);
    let capability_snapshot = resolve_runtime_capability_snapshot();
    ensure_operator_window_label(window.label())?;
    if let Some(session_id) = find_current_operator_session_id_in_dir(&base_dir)? {
        try_ensure_helper_running(&base_dir, &session_id);
    }

    load_operator_recovery_summary_in_dir(&base_dir, &capability_snapshot)
}

#[tauri::command]
pub fn load_operator_audit_history(
    app: tauri::AppHandle,
    window: tauri::Window,
    input: OperatorAuditQueryFilterDto,
) -> Result<OperatorAuditQueryResultDto, HostErrorEnvelope> {
    let app_local_data_dir = app.path().app_local_data_dir().map_err(|error| {
        HostErrorEnvelope::persistence(format!("앱 데이터 경로를 확인하지 못했어요: {error}"))
    })?;
    let base_dir = resolve_app_session_base_dir(app_local_data_dir);
    let capability_snapshot = resolve_runtime_capability_snapshot();
    ensure_operator_window_label(window.label())?;

    load_operator_audit_history_in_dir(&base_dir, &capability_snapshot, input)
}

#[tauri::command]
pub fn run_operator_recovery_action(
    app: tauri::AppHandle,
    window: tauri::Window,
    input: OperatorRecoveryActionInputDto,
) -> Result<OperatorRecoveryActionResultDto, HostErrorEnvelope> {
    let app_local_data_dir = app.path().app_local_data_dir().map_err(|error| {
        HostErrorEnvelope::persistence(format!("앱 데이터 경로를 확인하지 못했어요: {error}"))
    })?;
    let base_dir = resolve_app_session_base_dir(app_local_data_dir);
    let capability_snapshot = resolve_runtime_capability_snapshot();
    ensure_operator_window_label(window.label())?;
    try_ensure_helper_running(&base_dir, &input.session_id);

    execute_operator_recovery_action_in_dir(&base_dir, &capability_snapshot, input)
}
