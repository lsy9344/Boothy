use tauri::Manager;

use crate::{
    capture::helper_supervisor::try_ensure_helper_running,
    commands::runtime_commands::resolve_runtime_capability_snapshot,
    contracts::dto::{
        AuthoringWorkspaceResultDto, DraftPresetEditPayloadDto, DraftPresetSummaryDto,
        HostErrorEnvelope, LoadPresetCatalogInputDto, PresetCatalogResultDto,
        PresetCatalogStateResultDto, PresetSelectionInputDto, PresetSelectionResultDto,
        PublishValidatedPresetInputDto, PublishValidatedPresetResultDto,
        RepairInvalidDraftInputDto, RollbackPresetCatalogInputDto, RollbackPresetCatalogResultDto,
        ValidateDraftPresetInputDto, ValidateDraftPresetResultDto,
    },
    preset::{
        authoring_pipeline::{
            create_draft_preset_in_dir, load_authoring_workspace_in_dir,
            publish_validated_preset_in_dir, repair_invalid_draft_in_dir, save_draft_preset_in_dir,
            validate_draft_preset_in_dir,
        },
        preset_catalog::load_preset_catalog_in_dir,
        preset_catalog_state::{load_preset_catalog_state_in_dir, rollback_preset_catalog_in_dir},
    },
    session::session_repository::{resolve_app_session_base_dir, select_active_preset_in_dir},
};

#[tauri::command]
pub fn load_preset_catalog(
    app: tauri::AppHandle,
    input: LoadPresetCatalogInputDto,
) -> Result<PresetCatalogResultDto, HostErrorEnvelope> {
    let app_local_data_dir = app.path().app_local_data_dir().map_err(|error| {
        HostErrorEnvelope::persistence(format!("앱 데이터 경로를 확인하지 못했어요: {error}"))
    })?;
    let base_dir = resolve_app_session_base_dir(app_local_data_dir);

    load_preset_catalog_in_dir(&base_dir, input)
}

#[tauri::command]
pub fn select_active_preset(
    app: tauri::AppHandle,
    input: PresetSelectionInputDto,
) -> Result<PresetSelectionResultDto, HostErrorEnvelope> {
    let app_local_data_dir = app.path().app_local_data_dir().map_err(|error| {
        HostErrorEnvelope::persistence(format!("앱 데이터 경로를 확인하지 못했어요: {error}"))
    })?;
    let base_dir = resolve_app_session_base_dir(app_local_data_dir);
    let session_id = input.session_id.clone();
    let result = select_active_preset_in_dir(&base_dir, input)?;
    try_ensure_helper_running(&base_dir, &session_id);

    Ok(result)
}

#[tauri::command]
pub fn load_authoring_workspace(
    app: tauri::AppHandle,
    window: tauri::Window,
) -> Result<AuthoringWorkspaceResultDto, HostErrorEnvelope> {
    let app_local_data_dir = app.path().app_local_data_dir().map_err(|error| {
        HostErrorEnvelope::persistence(format!("앱 데이터 경로를 확인하지 못했어요: {error}"))
    })?;
    let base_dir = resolve_app_session_base_dir(app_local_data_dir);
    let capability_snapshot = resolve_runtime_capability_snapshot();
    crate::preset::authoring_pipeline::ensure_authoring_window_label(window.label())?;

    load_authoring_workspace_in_dir(&base_dir, &capability_snapshot)
}

#[tauri::command]
pub fn create_draft_preset(
    app: tauri::AppHandle,
    window: tauri::Window,
    input: DraftPresetEditPayloadDto,
) -> Result<DraftPresetSummaryDto, HostErrorEnvelope> {
    let app_local_data_dir = app.path().app_local_data_dir().map_err(|error| {
        HostErrorEnvelope::persistence(format!("앱 데이터 경로를 확인하지 못했어요: {error}"))
    })?;
    let base_dir = resolve_app_session_base_dir(app_local_data_dir);
    let capability_snapshot = resolve_runtime_capability_snapshot();
    crate::preset::authoring_pipeline::ensure_authoring_window_label(window.label())?;

    create_draft_preset_in_dir(&base_dir, &capability_snapshot, input)
}

#[tauri::command]
pub fn save_draft_preset(
    app: tauri::AppHandle,
    window: tauri::Window,
    input: DraftPresetEditPayloadDto,
) -> Result<DraftPresetSummaryDto, HostErrorEnvelope> {
    let app_local_data_dir = app.path().app_local_data_dir().map_err(|error| {
        HostErrorEnvelope::persistence(format!("앱 데이터 경로를 확인하지 못했어요: {error}"))
    })?;
    let base_dir = resolve_app_session_base_dir(app_local_data_dir);
    let capability_snapshot = resolve_runtime_capability_snapshot();
    crate::preset::authoring_pipeline::ensure_authoring_window_label(window.label())?;

    save_draft_preset_in_dir(&base_dir, &capability_snapshot, input)
}

#[tauri::command]
pub fn validate_draft_preset(
    app: tauri::AppHandle,
    window: tauri::Window,
    input: ValidateDraftPresetInputDto,
) -> Result<ValidateDraftPresetResultDto, HostErrorEnvelope> {
    let app_local_data_dir = app.path().app_local_data_dir().map_err(|error| {
        HostErrorEnvelope::persistence(format!("앱 데이터 경로를 확인하지 못했어요: {error}"))
    })?;
    let base_dir = resolve_app_session_base_dir(app_local_data_dir);
    let capability_snapshot = resolve_runtime_capability_snapshot();
    crate::preset::authoring_pipeline::ensure_authoring_window_label(window.label())?;

    validate_draft_preset_in_dir(&base_dir, &capability_snapshot, input)
}

#[tauri::command]
pub fn repair_invalid_draft(
    app: tauri::AppHandle,
    window: tauri::Window,
    input: RepairInvalidDraftInputDto,
) -> Result<(), HostErrorEnvelope> {
    let app_local_data_dir = app.path().app_local_data_dir().map_err(|error| {
        HostErrorEnvelope::persistence(format!("앱 데이터 경로를 확인하지 못했어요: {error}"))
    })?;
    let base_dir = resolve_app_session_base_dir(app_local_data_dir);
    let capability_snapshot = resolve_runtime_capability_snapshot();
    crate::preset::authoring_pipeline::ensure_authoring_window_label(window.label())?;

    repair_invalid_draft_in_dir(&base_dir, &capability_snapshot, input)
}

#[tauri::command]
pub fn publish_validated_preset(
    app: tauri::AppHandle,
    window: tauri::Window,
    input: PublishValidatedPresetInputDto,
) -> Result<PublishValidatedPresetResultDto, HostErrorEnvelope> {
    let app_local_data_dir = app.path().app_local_data_dir().map_err(|error| {
        HostErrorEnvelope::persistence(format!("앱 데이터 경로를 확인하지 못했어요: {error}"))
    })?;
    let base_dir = resolve_app_session_base_dir(app_local_data_dir);
    let capability_snapshot = resolve_runtime_capability_snapshot();
    crate::preset::authoring_pipeline::ensure_authoring_window_label(window.label())?;

    publish_validated_preset_in_dir(&base_dir, &capability_snapshot, input)
}

#[tauri::command]
pub fn load_preset_catalog_state(
    app: tauri::AppHandle,
    window: tauri::Window,
) -> Result<PresetCatalogStateResultDto, HostErrorEnvelope> {
    let app_local_data_dir = app.path().app_local_data_dir().map_err(|error| {
        HostErrorEnvelope::persistence(format!("앱 데이터 경로를 확인하지 못했어요: {error}"))
    })?;
    let base_dir = resolve_app_session_base_dir(app_local_data_dir);
    let capability_snapshot = resolve_runtime_capability_snapshot();
    crate::preset::authoring_pipeline::ensure_authoring_window_label(window.label())?;

    load_preset_catalog_state_in_dir(&base_dir, &capability_snapshot)
}

#[tauri::command]
pub fn rollback_preset_catalog(
    app: tauri::AppHandle,
    window: tauri::Window,
    input: RollbackPresetCatalogInputDto,
) -> Result<RollbackPresetCatalogResultDto, HostErrorEnvelope> {
    let app_local_data_dir = app.path().app_local_data_dir().map_err(|error| {
        HostErrorEnvelope::persistence(format!("앱 데이터 경로를 확인하지 못했어요: {error}"))
    })?;
    let base_dir = resolve_app_session_base_dir(app_local_data_dir);
    let capability_snapshot = resolve_runtime_capability_snapshot();
    crate::preset::authoring_pipeline::ensure_authoring_window_label(window.label())?;

    rollback_preset_catalog_in_dir(&base_dir, &capability_snapshot, input)
}
