use tauri::{Manager, RunEvent, WebviewUrl, WebviewWindowBuilder};

pub mod branch_config;
pub mod capture;
pub mod commands;
pub mod contracts;
pub mod diagnostics;
pub mod handoff;
pub mod preset;
pub mod session;
pub mod timing;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let app = tauri::Builder::default()
        .setup(|app| {
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }

            let app_local_data_dir = app
                .path()
                .app_local_data_dir()
                .map_err(|error| format!("앱 데이터 경로를 확인하지 못했어요: {error}"))?;
            let runtime_base_dir =
                session::session_repository::resolve_app_session_base_dir(app_local_data_dir);
            preset::default_catalog::ensure_default_preset_catalog_in_dir(&runtime_base_dir)
                .map_err(|error| error.message.clone())?;

            let capability_snapshot =
                commands::runtime_commands::resolve_runtime_capability_snapshot();
            let should_open_authoring_window = capability_snapshot.is_admin_authenticated
                && capability_snapshot
                    .allowed_surfaces
                    .iter()
                    .any(|surface| surface == "authoring");
            let should_open_operator_window = capability_snapshot.is_admin_authenticated
                && capability_snapshot
                    .allowed_surfaces
                    .iter()
                    .any(|surface| surface == "operator");

            if should_open_authoring_window && app.get_webview_window("authoring-window").is_none()
            {
                WebviewWindowBuilder::new(
                    app,
                    "authoring-window",
                    WebviewUrl::App("index.html".into()),
                )
                .title("Boothy Authoring")
                .inner_size(1280.0, 840.0)
                .resizable(true)
                .build()?;
            }

            if should_open_operator_window && app.get_webview_window("operator-window").is_none() {
                WebviewWindowBuilder::new(
                    app,
                    "operator-window",
                    WebviewUrl::App("index.html".into()),
                )
                .title("Boothy Operator")
                .inner_size(1280.0, 840.0)
                .resizable(true)
                .build()?;
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::branch_rollout_commands::load_branch_rollout_overview,
            commands::branch_rollout_commands::apply_branch_rollout,
            commands::branch_rollout_commands::apply_branch_rollback,
            commands::capture_commands::get_capture_readiness,
            commands::capture_commands::delete_capture,
            commands::capture_commands::request_capture,
            commands::operator_commands::load_operator_session_summary,
            commands::operator_commands::load_operator_recovery_summary,
            commands::operator_commands::load_operator_audit_history,
            commands::operator_commands::run_operator_recovery_action,
            commands::runtime_commands::get_capability_snapshot,
            commands::runtime_commands::log_capture_client_state,
            commands::preset_commands::load_preset_catalog,
            commands::preset_commands::load_authoring_workspace,
            commands::preset_commands::create_draft_preset,
            commands::preset_commands::save_draft_preset,
            commands::preset_commands::validate_draft_preset,
            commands::preset_commands::publish_validated_preset,
            commands::preset_commands::load_preset_catalog_state,
            commands::preset_commands::rollback_preset_catalog,
            commands::preset_commands::select_active_preset,
            commands::session_commands::start_session
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application");

    app.run(|_app_handle, event| {
        if matches!(event, RunEvent::ExitRequested { .. } | RunEvent::Exit) {
            capture::helper_supervisor::shutdown_helper_process();
        }
    });
}
