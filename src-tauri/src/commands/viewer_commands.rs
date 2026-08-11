use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use tauri::{Emitter, Manager, PhysicalPosition, WebviewUrl, WebviewWindowBuilder, WindowEvent};

use crate::{
    contracts::dto::{
        validate_viewer_layout_report, HostErrorEnvelope, ViewerLayoutReportDto,
        ViewerListenerReportDto, ViewerReadinessSnapshotDto, ViewerReadinessUpdateDto,
    },
    viewer::{
        current_epoch_ms, current_monotonic_ms,
        display_profile::{
            select_customer_monitor, MonitorDescriptor, MonitorSelection,
            MONITOR_TARGETING_SINGLE_MONITOR_FALLBACK,
        },
        is_event_loop_started, is_main_thread, should_offload_window_creation, ViewerStateHandle,
    },
};

pub const VIEWER_WINDOW_LABEL: &str = "viewer-window";
pub const VIEWER_READINESS_UPDATE_EVENT: &str = "viewer-readiness-update";
pub const VIEWER_SESSION_BINDING_UPDATE_EVENT: &str = "viewer-session-binding-update";

/// 창 생성이 계속 실패할 때 재시도 사이의 최소 간격.
/// booth는 readiness를 주기적으로 조회하므로 이 간격이 없으면 실패 시 스레드를 계속 만든다.
pub const VIEWER_WINDOW_RETRY_INTERVAL_MS: u64 = 2_000;

/// 동시에 두 번 창을 만들지 않게 하는 single-flight 게이트.
static VIEWER_WINDOW_CREATION_IN_FLIGHT: AtomicBool = AtomicBool::new(false);
static VIEWER_WINDOW_LAST_ATTEMPT_MS: AtomicU64 = AtomicU64::new(0);

/// 지금 창 생성을 다시 시도해도 되는지 판정한다.
pub fn should_attempt_window_creation(
    creation_in_flight: bool,
    last_attempt_ms: Option<u64>,
    now_ms: u64,
    retry_interval_ms: u64,
) -> bool {
    if creation_in_flight {
        return false;
    }

    match last_attempt_ms {
        None => true,
        Some(last_attempt_ms) => now_ms.saturating_sub(last_attempt_ms) >= retry_interval_ms,
    }
}

fn emit_viewer_readiness(app: &tauri::AppHandle, snapshot: ViewerReadinessSnapshotDto) {
    let _ = app.emit(
        VIEWER_READINESS_UPDATE_EVENT,
        ViewerReadinessUpdateDto::new(snapshot),
    );
}

fn read_snapshot(app: &tauri::AppHandle) -> ViewerReadinessSnapshotDto {
    let handle = app.state::<ViewerStateHandle>();
    let state = handle.0.lock().expect("viewer state lock poisoned");

    state.snapshot_with_clock(current_epoch_ms(), current_monotonic_ms())
}

/// 승인된 고객 모니터 이름. 미설정이면 결정적 fallback 규칙을 사용한다.
fn approved_customer_monitor_name() -> Option<String> {
    std::env::var("BOOTHY_APPROVED_CUSTOMER_MONITOR")
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn read_available_monitors(app: &tauri::AppHandle) -> Vec<MonitorDescriptor> {
    let primary_name = app
        .primary_monitor()
        .ok()
        .flatten()
        .and_then(|monitor| monitor.name().cloned());

    app.available_monitors()
        .unwrap_or_default()
        .into_iter()
        .map(|monitor| {
            let name = monitor.name().cloned();
            let size = monitor.size();
            let position = monitor.position();

            MonitorDescriptor {
                is_primary: name.is_some() && name == primary_name,
                name,
                width_px: size.width,
                height_px: size.height,
                scale_factor: monitor.scale_factor(),
                position_x: position.x,
                position_y: position.y,
            }
        })
        .collect()
}

fn resolve_monitor_selection(app: &tauri::AppHandle) -> MonitorSelection {
    let monitors = read_available_monitors(app);

    select_customer_monitor(&monitors, approved_customer_monitor_name().as_deref())
}

/// app-lifetime viewer window를 보장한다. 촬영 입력 이후가 아니라 앱/세션 준비 중에 호출한다.
///
/// main thread(event loop)에서 호출되면 창을 직접 만들지 않고 background 스레드로 넘긴다.
/// `WebviewWindowBuilder::build()`가 event loop 응답을 기다리기 때문에, event loop 스레드에서
/// 직접 호출하면 교착이 발생해 창이 흰 화면으로 멈춘다.
pub fn ensure_viewer_window_state(app: &tauri::AppHandle) -> ViewerReadinessSnapshotDto {
    if app.get_webview_window(VIEWER_WINDOW_LABEL).is_some() {
        return read_snapshot(app);
    }

    let now_ms = current_monotonic_ms();
    let last_attempt_ms = match VIEWER_WINDOW_LAST_ATTEMPT_MS.load(Ordering::SeqCst) {
        0 => None,
        value => Some(value),
    };

    if !should_attempt_window_creation(
        VIEWER_WINDOW_CREATION_IN_FLIGHT.load(Ordering::SeqCst),
        last_attempt_ms,
        now_ms,
        VIEWER_WINDOW_RETRY_INTERVAL_MS,
    ) {
        return read_snapshot(app);
    }

    if VIEWER_WINDOW_CREATION_IN_FLIGHT
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_err()
    {
        return read_snapshot(app);
    }

    VIEWER_WINDOW_LAST_ATTEMPT_MS.store(now_ms.max(1), Ordering::SeqCst);

    if should_offload_window_creation(is_main_thread(), is_event_loop_started()) {
        spawn_viewer_window_creation(app);

        return read_snapshot(app);
    }

    let snapshot = create_viewer_window_blocking(app);
    VIEWER_WINDOW_CREATION_IN_FLIGHT.store(false, Ordering::SeqCst);

    snapshot
}

/// 창 생성을 background 스레드에서 수행한다. 호출자는 즉시 현재 snapshot을 받고,
/// 준비 완료는 `viewer-readiness-update` 이벤트로 도착한다.
fn spawn_viewer_window_creation(app: &tauri::AppHandle) {
    let creation_app = app.clone();

    std::thread::spawn(move || {
        create_viewer_window_blocking(&creation_app);
        VIEWER_WINDOW_CREATION_IN_FLIGHT.store(false, Ordering::SeqCst);
    });
}

fn create_viewer_window_blocking(app: &tauri::AppHandle) -> ViewerReadinessSnapshotDto {
    if app.get_webview_window(VIEWER_WINDOW_LABEL).is_some() {
        return read_snapshot(app);
    }

    {
        let handle = app.state::<ViewerStateHandle>();
        let mut state = handle.0.lock().expect("viewer state lock poisoned");
        state.mark_window_creating();
    }

    let selection = resolve_monitor_selection(app);
    // 전용 고객 모니터가 없으면 fullscreen으로 부스 조작 화면을 덮지 않는다.
    // readiness는 그대로 계산되지만 창은 창모드로 열려 개발/단일 모니터 환경을 막지 않는다.
    let is_dedicated_customer_monitor =
        selection.targeting != MONITOR_TARGETING_SINGLE_MONITOR_FALLBACK;
    let mut builder = WebviewWindowBuilder::new(
        app,
        VIEWER_WINDOW_LABEL,
        WebviewUrl::App("index.html".into()),
    )
    .title("Boothy Viewer")
    .decorations(!is_dedicated_customer_monitor)
    .skip_taskbar(is_dedicated_customer_monitor)
    .focused(false);

    if let Some(monitor) = selection.monitor.as_ref() {
        builder = if is_dedicated_customer_monitor {
            builder.inner_size(
                monitor.width_px as f64 / monitor.scale_factor,
                monitor.height_px as f64 / monitor.scale_factor,
            )
        } else {
            builder.inner_size(
                monitor.width_px as f64 / monitor.scale_factor / 2.0,
                monitor.height_px as f64 / monitor.scale_factor / 2.0,
            )
        };
    }

    match builder.build() {
        Ok(window) => {
            if let Some(monitor) = selection.monitor.as_ref() {
                if is_dedicated_customer_monitor {
                    // 위치를 먼저 옮긴 뒤 fullscreen을 적용해야 대상 모니터를 채운다.
                    // 실패를 조용히 삼키면 창이 잘못된 크기로 열려도 알 수 없으므로 로그로 남긴다.
                    if let Err(error) = window.set_position(PhysicalPosition::new(
                        monitor.position_x,
                        monitor.position_y,
                    )) {
                        log::warn!("viewer_window_set_position_failed error={error}");
                    }

                    if let Err(error) = window.set_fullscreen(true) {
                        log::warn!("viewer_window_set_fullscreen_failed error={error}");
                    }
                }
            }

            {
                let handle = app.state::<ViewerStateHandle>();
                let mut state = handle.0.lock().expect("viewer state lock poisoned");
                state.mark_window_open(&selection);
            }

            let close_app = app.clone();
            window.on_window_event(move |event| {
                if matches!(
                    event,
                    WindowEvent::Destroyed | WindowEvent::CloseRequested { .. }
                ) {
                    {
                        let handle = close_app.state::<ViewerStateHandle>();
                        let mut state = handle.0.lock().expect("viewer state lock poisoned");
                        state.mark_window_closed();
                    }

                    let snapshot = read_snapshot(&close_app);
                    log::warn!(
                        "viewer_window_lost epoch={} revision={} reason={}",
                        snapshot.viewer_epoch,
                        snapshot.revision,
                        snapshot.reason_code
                    );
                    emit_viewer_readiness(&close_app, snapshot);
                }
            });

            log::info!(
                "viewer_window_opened targeting={} profile={}",
                selection.targeting,
                selection.profile_id.unwrap_or("unapproved")
            );
        }
        Err(error) => {
            let handle = app.state::<ViewerStateHandle>();
            let mut state = handle.0.lock().expect("viewer state lock poisoned");
            state.mark_window_closed();
            log::error!("viewer_window_create_failed error={error}");
        }
    }

    let snapshot = read_snapshot(app);
    emit_viewer_readiness(app, snapshot.clone());

    snapshot
}

/// 현재 세션을 host truth로 고정하고 viewer가 준비되어 있는지 확인한다.
///
/// `start_session`은 blocking command라 main thread에서 실행된다. 따라서 여기서 창 생성을
/// 기다리면 세션 시작 자체가 멈춘다. binding은 즉시 반영하고 창 보장은 비동기로 넘긴다.
/// 촬영은 viewer가 준비될 때까지 `viewer-preparing`으로 정직하게 막힌다.
pub fn bind_viewer_session(app: &tauri::AppHandle, session_id: &str) {
    {
        let handle = app.state::<ViewerStateHandle>();
        let mut state = handle.0.lock().expect("viewer state lock poisoned");
        state.bind_session(session_id);
    }

    let snapshot = read_snapshot(app);
    emit_viewer_readiness(app, snapshot.clone());
    let _ = app.emit(
        VIEWER_SESSION_BINDING_UPDATE_EVENT,
        ViewerReadinessUpdateDto::new(snapshot),
    );

    ensure_viewer_window_state(app);
}

#[tauri::command]
pub fn get_viewer_readiness(
    app: tauri::AppHandle,
) -> Result<ViewerReadinessSnapshotDto, HostErrorEnvelope> {
    Ok(read_snapshot(&app))
}

/// `async` 속성이 필수다. 기본 `#[tauri::command]`는 blocking 실행이라 main thread(event loop)에서
/// 돌고, 그 안에서 window를 만들면 event loop가 자기 응답을 기다리는 교착이 된다.
/// `async` 속성은 이 함수를 sync threadpool로 보내 main thread를 비워 둔다.
#[tauri::command(async)]
pub fn ensure_viewer_window(
    app: tauri::AppHandle,
) -> Result<ViewerReadinessSnapshotDto, HostErrorEnvelope> {
    Ok(ensure_viewer_window_state(&app))
}

#[tauri::command]
pub fn report_viewer_listener_ready(
    app: tauri::AppHandle,
    input: ViewerListenerReportDto,
) -> Result<ViewerReadinessSnapshotDto, HostErrorEnvelope> {
    let outcome = {
        let handle = app.state::<ViewerStateHandle>();
        let mut state = handle.0.lock().expect("viewer state lock poisoned");
        state.apply_listener_report_with_clock(
            input.viewer_epoch,
            current_epoch_ms(),
            current_monotonic_ms(),
        )
    };
    let snapshot = read_snapshot(&app);

    if matches!(outcome, crate::viewer::viewer_state::ReportOutcome::Applied) {
        emit_viewer_readiness(&app, snapshot.clone());
    }

    Ok(snapshot)
}

#[tauri::command]
pub fn report_viewer_layout(
    app: tauri::AppHandle,
    input: ViewerLayoutReportDto,
) -> Result<ViewerReadinessSnapshotDto, HostErrorEnvelope> {
    validate_viewer_layout_report(&input)?;

    let outcome = {
        let handle = app.state::<ViewerStateHandle>();
        let mut state = handle.0.lock().expect("viewer state lock poisoned");
        state.apply_layout_report_with_clock(&input, current_epoch_ms(), current_monotonic_ms())
    };
    let snapshot = read_snapshot(&app);

    if matches!(outcome, crate::viewer::viewer_state::ReportOutcome::Applied) {
        emit_viewer_readiness(&app, snapshot.clone());
    }

    Ok(snapshot)
}
