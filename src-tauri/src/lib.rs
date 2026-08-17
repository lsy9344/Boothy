use tauri::{Manager, RunEvent, WebviewUrl, WebviewWindowBuilder};

pub mod branch_config;
pub mod capture;
pub mod commands;
pub mod contracts;
pub mod diagnostics;
pub mod display;
pub mod handoff;
pub mod preset;
pub mod release;
pub mod render;
pub mod session;
pub mod timing;
pub mod viewer;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Story 7.7: `--self-check`는 **Tauri 빌더를 세우기 전에** 끝난다.
    // 창을 만들면 clean offline VM 자동화가 그 창을 닫을 방법이 없다.
    // 결과는 stdout이 아니라 보고서 파일과 종료 코드로 나간다 (릴리스 빌드에는 콘솔이 없다).
    let arguments = std::env::args().collect::<Vec<_>>();
    if let Some(request) = release::self_check::parse_self_check_request(&arguments) {
        std::process::exit(release::self_check::run_self_check(&request));
    }

    let app = tauri::Builder::default()
        .manage(viewer::ViewerStateHandle::default())
        .manage(display::DisplayStateHandle::default())
        .setup(|app| {
            // Story 7.1: window 생성이 event loop 스레드에서 일어나는지 판별하기 위한 기준점.
            viewer::record_main_thread();

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

            // Story 7.1: 관람 창은 촬영 입력 이후가 아니라 앱 준비 중에 미리 만든다.
            // 실패해도 앱 부팅을 막지 않고 host-normalized readiness truth로만 보고한다.
            commands::viewer_commands::ensure_viewer_window_state(app.handle());

            // Story 7.2: 계측 lane이 켜져 있으면 표시 이미지는 제품 결과가 아니다. 부팅 시 경고한다.
            commands::display_commands::log_sample_lane_mode();

            // Story 7.5: 상주 renderer lane은 spike다. 켜져 있으면 부팅 시 분명히 남긴다.
            commands::resident_renderer_commands::log_resident_renderer_mode();

            // Story 7.7: 설치본이 자기 인벤토리와 어긋나면 **운영자 진단으로만** 투영한다.
            // 고객 문구를 만들지 않고, 세션·촬영·렌더 경로를 새로 막지도 않는다.
            release::self_check::run_startup_release_governance_check(&runtime_base_dir);

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
            commands::preset_commands::repair_invalid_draft,
            commands::preset_commands::publish_validated_preset,
            commands::preset_commands::load_preset_catalog_state,
            commands::preset_commands::rollback_preset_catalog,
            commands::preset_commands::select_active_preset,
            commands::session_commands::start_session,
            commands::viewer_commands::get_viewer_readiness,
            commands::viewer_commands::ensure_viewer_window,
            commands::viewer_commands::report_viewer_listener_ready,
            commands::viewer_commands::report_viewer_layout,
            commands::display_commands::get_viewer_display_state,
            commands::display_commands::publish_display_sample,
            commands::display_commands::report_display_present,
            commands::display_commands::report_trusted_capture_input,
            commands::display_commands::stamp_clock_probe,
            commands::resident_renderer_commands::get_resident_renderer_plan
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application");

    // 이 시점부터 main thread는 event loop에 점유된다. 이후 window 생성은 background 스레드에서만 한다.
    viewer::mark_event_loop_started();

    app.run(|_app_handle, event| {
        if matches!(event, RunEvent::ExitRequested { .. } | RunEvent::Exit) {
            capture::helper_supervisor::shutdown_helper_process();
        }
    });
}
