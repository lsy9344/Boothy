pub mod capture;
pub mod commands;
pub mod contracts;
pub mod preset;
pub mod session;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::capture_commands::get_capture_readiness,
            commands::capture_commands::delete_capture,
            commands::capture_commands::request_capture,
            commands::preset_commands::load_preset_catalog,
            commands::preset_commands::select_active_preset,
            commands::session_commands::start_session
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
