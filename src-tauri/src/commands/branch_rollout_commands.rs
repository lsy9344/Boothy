use tauri::Manager;

use crate::{
    branch_config::{
        apply_branch_rollback_in_dir, apply_branch_rollout_in_dir,
        load_branch_rollout_overview_in_dir,
    },
    commands::runtime_commands::resolve_runtime_capability_snapshot,
    contracts::dto::{
        BranchRollbackInputDto, BranchRolloutActionResultDto, BranchRolloutInputDto,
        BranchRolloutOverviewResultDto, HostErrorEnvelope,
    },
    session::session_repository::resolve_app_session_base_dir,
};

#[tauri::command]
pub fn load_branch_rollout_overview(
    app: tauri::AppHandle,
) -> Result<BranchRolloutOverviewResultDto, HostErrorEnvelope> {
    let app_local_data_dir = app.path().app_local_data_dir().map_err(|error| {
        HostErrorEnvelope::persistence(format!("앱 데이터 경로를 확인하지 못했어요: {error}"))
    })?;
    let base_dir = resolve_app_session_base_dir(app_local_data_dir);
    let capability_snapshot = resolve_runtime_capability_snapshot();

    load_branch_rollout_overview_in_dir(&base_dir, &capability_snapshot)
}

#[tauri::command]
pub fn apply_branch_rollout(
    app: tauri::AppHandle,
    input: BranchRolloutInputDto,
) -> Result<BranchRolloutActionResultDto, HostErrorEnvelope> {
    let app_local_data_dir = app.path().app_local_data_dir().map_err(|error| {
        HostErrorEnvelope::persistence(format!("앱 데이터 경로를 확인하지 못했어요: {error}"))
    })?;
    let base_dir = resolve_app_session_base_dir(app_local_data_dir);
    let capability_snapshot = resolve_runtime_capability_snapshot();

    apply_branch_rollout_in_dir(&base_dir, &capability_snapshot, input)
}

#[tauri::command]
pub fn apply_branch_rollback(
    app: tauri::AppHandle,
    input: BranchRollbackInputDto,
) -> Result<BranchRolloutActionResultDto, HostErrorEnvelope> {
    let app_local_data_dir = app.path().app_local_data_dir().map_err(|error| {
        HostErrorEnvelope::persistence(format!("앱 데이터 경로를 확인하지 못했어요: {error}"))
    })?;
    let base_dir = resolve_app_session_base_dir(app_local_data_dir);
    let capability_snapshot = resolve_runtime_capability_snapshot();

    apply_branch_rollback_in_dir(&base_dir, &capability_snapshot, input)
}
