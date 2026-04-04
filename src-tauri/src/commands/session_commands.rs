use tauri::Manager;

use crate::{
    capture::helper_supervisor::try_ensure_helper_running,
    contracts::dto::{HostErrorEnvelope, SessionStartInputDto},
    render::prime_preview_worker_runtime_in_dir,
    session::session_repository::{
        resolve_app_session_base_dir, start_session_in_dir, SessionStartResultDto,
    },
};

#[tauri::command]
pub fn start_session(
    app: tauri::AppHandle,
    input: SessionStartInputDto,
) -> Result<SessionStartResultDto, HostErrorEnvelope> {
    let app_local_data_dir = app.path().app_local_data_dir().map_err(|error| {
        HostErrorEnvelope::persistence(format!("앱 데이터 경로를 확인하지 못했어요: {error}"))
    })?;
    let base_dir = resolve_app_session_base_dir(app_local_data_dir);

    let result = start_session_in_dir(&base_dir, input)?;
    prime_preview_worker_runtime_in_dir(&base_dir, &result.session_id);
    try_ensure_helper_running(&base_dir, &result.session_id);

    Ok(result)
}
