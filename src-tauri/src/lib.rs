use tauri::Manager;

pub mod commands;
pub mod capture;
pub mod contracts;
pub mod db;
pub mod diagnostics;
pub mod export;
pub mod session;
pub mod timing;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_store::Builder::new().build())
        .setup(|app| {
            let operational_log_state = db::sqlite::initialize_operational_log(app.handle())?;
            app.manage(operational_log_state);
            app.manage(commands::capture_commands::CameraWatchRegistry::default());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            diagnostics::lifecycle_log::record_lifecycle_event,
            diagnostics::operator_log::record_operator_intervention,
            commands::capture_commands::load_session_gallery,
            commands::capture_commands::delete_session_photo,
            commands::capture_commands::camera_run_readiness_flow,
            commands::capture_commands::get_camera_readiness_snapshot,
            commands::capture_commands::watch_camera_readiness,
            commands::capture_commands::unwatch_camera_readiness,
            commands::capture_commands::get_capture_confidence_snapshot,
            commands::capture_commands::watch_capture_confidence,
            commands::capture_commands::unwatch_capture_confidence,
            commands::capture_commands::request_capture,
            commands::session_commands::start_session,
            commands::session_commands::initialize_session_timing,
            commands::session_commands::get_session_timing,
            commands::session_commands::get_post_end_outcome,
            commands::session_commands::select_session_preset,
            commands::operator_commands::extend_session_timing,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
