use tauri::Manager;

use crate::{
  contracts::dto::{HostErrorEnvelope, SessionStartInputDto},
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

  start_session_in_dir(&base_dir, input)
}
