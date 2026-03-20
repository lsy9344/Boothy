use tauri::Manager;

use crate::{
    contracts::dto::{
        HostErrorEnvelope, LoadPresetCatalogInputDto, PresetCatalogResultDto,
        PresetSelectionInputDto, PresetSelectionResultDto,
    },
    preset::preset_catalog::load_preset_catalog_in_dir,
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

    select_active_preset_in_dir(&base_dir, input)
}
