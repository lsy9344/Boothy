use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use tauri::{Emitter, Manager, PhysicalPosition, WebviewUrl, WebviewWindowBuilder, WindowEvent};

use crate::{
    contracts::dto::{
        validate_viewer_layout_report, HostErrorEnvelope, ViewerLayoutReportDto,
        ViewerListenerReportDto, ViewerReadinessSnapshotDto, ViewerReadinessUpdateDto,
    },
    display::sample_publisher::{current_sample_lane_mode, SampleLaneMode},
    viewer::{
        current_epoch_ms, current_monotonic_ms,
        display_profile::{
            select_customer_monitor, MonitorDescriptor, MonitorSelection,
            MONITOR_TARGETING_SINGLE_MONITOR_FALLBACK,
        },
        is_event_loop_started, is_main_thread, should_offload_window_creation,
        viewer_state::{
            VIEWER_REASON_LAYOUT_NOT_READY, VIEWER_REASON_LISTENER_NOT_READY,
            VIEWER_REASON_SESSION_MISMATCH, VIEWER_REASON_STALE_EPOCH, VIEWER_REASON_STALE_REPORT,
        },
        ViewerStateHandle,
    },
};

pub const VIEWER_WINDOW_LABEL: &str = "viewer-window";
pub const VIEWER_READINESS_UPDATE_EVENT: &str = "viewer-readiness-update";
pub const VIEWER_SESSION_BINDING_UPDATE_EVENT: &str = "viewer-session-binding-update";

/// 창 생성이 계속 실패할 때 재시도 사이의 최소 간격.
/// booth는 readiness를 주기적으로 조회하므로 이 간격이 없으면 실패 시 스레드를 계속 만든다.
pub const VIEWER_WINDOW_RETRY_INTERVAL_MS: u64 = 2_000;
pub const VIEWER_WINDOW_UNHEALTHY_TIMEOUT_MS: u64 = 10_000;

/// 동시에 두 번 창을 만들지 않게 하는 single-flight 게이트.
static VIEWER_WINDOW_CREATION_IN_FLIGHT: AtomicBool = AtomicBool::new(false);
static VIEWER_WINDOW_LAST_ATTEMPT_MS: AtomicU64 = AtomicU64::new(0);
static VIEWER_WINDOW_UNHEALTHY_SINCE_MS: AtomicU64 = AtomicU64::new(0);

/// hidden prewarm 노출을 이미 끝낸 viewer 세대. `u64::MAX`는 "아직 없음"이다.
///
/// 창은 세대마다 한 번만 드러나면 된다. generation마다 `set_position`·`show`·`set_fullscreen`을
/// 다시 걸면 측정 구간 한가운데에서 창 상태가 흔들리고, 그때 발생하는 layout 재보고가
/// viewer의 present 계측을 무너뜨린다 (HV-13B 2026-08-12 회차의 누락 표본).
static VIEWER_WINDOW_REVEALED_EPOCH: AtomicU64 = AtomicU64::new(u64::MAX);

/// 이 세대에서 노출을 수행해야 하는지 판정한다. 같은 세대에서는 정확히 한 번만 true다.
pub fn should_reveal_for_epoch(revealed_epoch: u64, current_epoch: u64) -> bool {
    revealed_epoch != current_epoch
}

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

pub fn should_recreate_unhealthy_window(
    reason_code: &str,
    unhealthy_since_ms: Option<u64>,
    now_ms: u64,
    timeout_ms: u64,
) -> bool {
    let recoverable = matches!(
        reason_code,
        VIEWER_REASON_LISTENER_NOT_READY
            | VIEWER_REASON_LAYOUT_NOT_READY
            | VIEWER_REASON_SESSION_MISMATCH
            | VIEWER_REASON_STALE_EPOCH
            | VIEWER_REASON_STALE_REPORT
    );

    recoverable
        && unhealthy_since_ms
            .map(|since_ms| now_ms.saturating_sub(since_ms) >= timeout_ms)
            .unwrap_or(false)
}

fn existing_window_needs_recreation(snapshot: &ViewerReadinessSnapshotDto, now_ms: u64) -> bool {
    if !matches!(
        snapshot.reason_code.as_str(),
        VIEWER_REASON_LISTENER_NOT_READY
            | VIEWER_REASON_LAYOUT_NOT_READY
            | VIEWER_REASON_SESSION_MISMATCH
            | VIEWER_REASON_STALE_EPOCH
            | VIEWER_REASON_STALE_REPORT
    ) {
        VIEWER_WINDOW_UNHEALTHY_SINCE_MS.store(0, Ordering::SeqCst);
        return false;
    }

    let unhealthy_since_ms = match VIEWER_WINDOW_UNHEALTHY_SINCE_MS.load(Ordering::SeqCst) {
        0 => {
            let started_at_ms = now_ms.max(1);
            let _ = VIEWER_WINDOW_UNHEALTHY_SINCE_MS.compare_exchange(
                0,
                started_at_ms,
                Ordering::SeqCst,
                Ordering::SeqCst,
            );
            started_at_ms
        }
        value => value,
    };

    should_recreate_unhealthy_window(
        &snapshot.reason_code,
        Some(unhealthy_since_ms),
        now_ms,
        VIEWER_WINDOW_UNHEALTHY_TIMEOUT_MS,
    )
}

fn emit_viewer_readiness(app: &tauri::AppHandle, snapshot: ViewerReadinessSnapshotDto) {
    let _ = app.emit(
        VIEWER_READINESS_UPDATE_EVENT,
        ViewerReadinessUpdateDto::new(snapshot),
    );
}

fn should_refresh_capture_readiness(
    previous_viewer_ready: bool,
    next_viewer_ready: bool,
    next_session_id: Option<&str>,
) -> bool {
    !previous_viewer_ready && next_viewer_ready && next_session_id.is_some()
}

fn refresh_capture_readiness_if_viewer_became_ready(
    app: &tauri::AppHandle,
    previous_viewer_ready: bool,
    snapshot: &ViewerReadinessSnapshotDto,
) {
    if !should_refresh_capture_readiness(
        previous_viewer_ready,
        snapshot.viewer_ready,
        snapshot.session_id.as_deref(),
    ) {
        return;
    }

    if let Some(session_id) = snapshot.session_id.as_deref() {
        crate::commands::capture_commands::emit_current_capture_readiness(app, session_id);
    }
}

fn read_snapshot(app: &tauri::AppHandle) -> ViewerReadinessSnapshotDto {
    let handle = app.state::<ViewerStateHandle>();
    let state = handle.0.lock().expect("viewer state lock poisoned");

    state.snapshot_with_clock(current_epoch_ms(), current_monotonic_ms())
}

pub const HIDDEN_PREWARM_STRATEGY_ENV: &str = "BOOTHY_DISPLAY_HIDDEN_PREWARM_STRATEGY";

/// hidden prewarm 변형의 은닉 방식. A/B에서 둘 다 시험해야 한다.
///
/// **알려진 충돌:** WebView2는 숨김/가려진 창의 타이머와 `requestAnimationFrame`을 throttle한다.
/// Story 7.1의 liveness는 1초 heartbeat + 5초 staleness 임계이므로, `Invisible`은
/// `stale-report`를 만들어 촬영을 막을 수 있다. 이는 버그가 아니라 Story 7.1이 의도한
/// 정직한 차단이며, **임계값을 늘려 회피하지 않는다.** 관측 결과 자체가 A/B의 산출물이다.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HiddenPrewarmStrategy {
    /// `visible(false)`. throttling 영향이 가장 크다.
    Invisible,
    /// 승인 모니터 밖 좌표에 배치. 창은 살아 있지만 고객에게 보이지 않는다.
    Offscreen,
}

pub fn parse_hidden_prewarm_strategy(raw: Option<&str>) -> HiddenPrewarmStrategy {
    match raw.map(str::trim) {
        Some("offscreen") => HiddenPrewarmStrategy::Offscreen,
        _ => HiddenPrewarmStrategy::Invisible,
    }
}

pub fn current_hidden_prewarm_strategy() -> HiddenPrewarmStrategy {
    parse_hidden_prewarm_strategy(std::env::var(HIDDEN_PREWARM_STRATEGY_ENV).ok().as_deref())
}

/// 관람 창을 숨긴 채로 만들어야 하는지 판정한다.
/// 기본 lane(`off`)과 `visible-standby`에서는 항상 false다 — Story 7.1 동작 그대로다.
pub fn should_prewarm_hidden(lane_mode: SampleLaneMode) -> bool {
    matches!(lane_mode, SampleLaneMode::HiddenPrewarm)
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
        let snapshot = read_snapshot(app);
        let now_ms = current_monotonic_ms();

        if !existing_window_needs_recreation(&snapshot, now_ms) {
            return snapshot;
        }

        let last_attempt_ms = match VIEWER_WINDOW_LAST_ATTEMPT_MS.load(Ordering::SeqCst) {
            0 => None,
            value => Some(value),
        };

        if !should_attempt_window_creation(
            VIEWER_WINDOW_CREATION_IN_FLIGHT.load(Ordering::SeqCst),
            last_attempt_ms,
            now_ms,
            VIEWER_WINDOW_RETRY_INTERVAL_MS,
        ) || VIEWER_WINDOW_CREATION_IN_FLIGHT
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .is_err()
        {
            return snapshot;
        }

        VIEWER_WINDOW_LAST_ATTEMPT_MS.store(now_ms.max(1), Ordering::SeqCst);
        spawn_viewer_window_recreation(app);
        return snapshot;
    }

    VIEWER_WINDOW_UNHEALTHY_SINCE_MS.store(0, Ordering::SeqCst);

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

fn spawn_viewer_window_recreation(app: &tauri::AppHandle) {
    let recreation_app = app.clone();

    std::thread::spawn(move || {
        if let Some(window) = recreation_app.get_webview_window(VIEWER_WINDOW_LABEL) {
            if let Err(error) = window.destroy() {
                log::error!("viewer_window_destroy_for_recovery_failed error={error}");
                VIEWER_WINDOW_CREATION_IN_FLIGHT.store(false, Ordering::SeqCst);
                return;
            }
        }

        create_viewer_window_blocking(&recreation_app);
        VIEWER_WINDOW_UNHEALTHY_SINCE_MS.store(0, Ordering::SeqCst);
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
    if selection.monitor.is_none() {
        {
            let handle = app.state::<ViewerStateHandle>();
            let mut state = handle.0.lock().expect("viewer state lock poisoned");
            state.mark_window_creation_blocked(&selection);
        }

        log::warn!(
            "viewer_window_creation_blocked targeting={}",
            selection.targeting
        );
        let snapshot = read_snapshot(app);
        emit_viewer_readiness(app, snapshot.clone());
        return snapshot;
    }
    // 전용 고객 모니터가 없으면 fullscreen으로 부스 조작 화면을 덮지 않는다.
    // readiness는 그대로 계산되지만 창은 창모드로 열려 개발/단일 모니터 환경을 막지 않는다.
    let is_dedicated_customer_monitor =
        selection.targeting != MONITOR_TARGETING_SINGLE_MONITOR_FALLBACK;
    // Story 7.2 A/B: hidden prewarm 변형에서만 숨긴 채로 만든다. 기본값은 Story 7.1 그대로다.
    let prewarm_hidden = should_prewarm_hidden(current_sample_lane_mode());
    let hidden_strategy = current_hidden_prewarm_strategy();
    let mut builder = WebviewWindowBuilder::new(
        app,
        VIEWER_WINDOW_LABEL,
        WebviewUrl::App("index.html".into()),
    )
    .title("Boothy Viewer")
    .decorations(!is_dedicated_customer_monitor)
    .skip_taskbar(is_dedicated_customer_monitor)
    .visible(!(prewarm_hidden && hidden_strategy == HiddenPrewarmStrategy::Invisible))
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
                    // offscreen 변형은 승인 모니터 아래쪽 화면 밖에 배치한다.
                    // 창은 계속 살아 있으므로 throttling 영향을 `visible(false)`와 비교할 수 있다.
                    let position =
                        if prewarm_hidden && hidden_strategy == HiddenPrewarmStrategy::Offscreen {
                            PhysicalPosition::new(
                                monitor.position_x,
                                monitor.position_y + monitor.height_px as i32,
                            )
                        } else {
                            PhysicalPosition::new(monitor.position_x, monitor.position_y)
                        };

                    // 위치를 먼저 옮긴 뒤 fullscreen을 적용해야 대상 모니터를 채운다.
                    // 실패를 조용히 삼키면 창이 잘못된 크기로 열려도 알 수 없으므로 로그로 남긴다.
                    if let Err(error) = window.set_position(position) {
                        log::warn!("viewer_window_set_position_failed error={error}");
                    }

                    if !prewarm_hidden {
                        if let Err(error) = window.set_fullscreen(true) {
                            log::warn!("viewer_window_set_fullscreen_failed error={error}");
                        }
                    }
                }
            }

            if prewarm_hidden {
                log::warn!(
                    "viewer_window_prewarmed_hidden strategy={:?} — Story 7.2 A/B 변형",
                    hidden_strategy
                );
            }

            let window_epoch = {
                let handle = app.state::<ViewerStateHandle>();
                let mut state = handle.0.lock().expect("viewer state lock poisoned");
                state.mark_window_open(&selection);
                state.viewer_epoch()
            };

            // Story 7.6. 새 epoch가 발급됐다. 이전 세대에 묶인 display lane 렌더는
            // 어차피 `stale-epoch`로 거부되므로 계속 돌리는 것은 순수 낭비다.
            // **P2는 건드리지 않는다** — final은 관람 창 세대와 무관한 산출물이다.
            crate::commands::display_commands::cancel_stale_epoch_display_renders(
                &app,
                window_epoch,
            );

            let close_app = app.clone();
            window.on_window_event(move |event| {
                if matches!(
                    event,
                    WindowEvent::Destroyed | WindowEvent::CloseRequested { .. }
                ) {
                    let did_close_current_epoch = {
                        let handle = close_app.state::<ViewerStateHandle>();
                        let mut state = handle.0.lock().expect("viewer state lock poisoned");
                        state.mark_window_closed_if_epoch(window_epoch)
                    };

                    if !did_close_current_epoch {
                        return;
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

            // Story 7.2 AC 4: trusted input 이후의 창 생성은 실패 표본이다. 계측을 위해 센다.
            crate::viewer::present_telemetry::record_viewer_window_event();

            log::info!(
                "viewer_window_opened targeting={} profile={} at_host_micros={}",
                selection.targeting,
                selection.profile_id.unwrap_or("unapproved"),
                crate::viewer::current_monotonic_micros()
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

/// hidden prewarm 변형에서 첫 qualifying generation이 커밋된 뒤 관람 창을 드러낸다.
///
/// **이 시각은 present 계측에 포함된다.** hidden 변형의 비용이 여기에 나타나야 A/B가 정직하다.
/// `visible-standby`와 `off`에서는 아무 일도 하지 않는다.
pub fn reveal_prewarmed_viewer_window(app: &tauri::AppHandle) {
    if !should_prewarm_hidden(current_sample_lane_mode()) {
        return;
    }

    let Some(window) = app.get_webview_window(VIEWER_WINDOW_LABEL) else {
        return;
    };

    // 한 세대에 한 번만 드러낸다. 두 번째부터는 창 상태를 건드리지 않는다.
    let current_epoch = {
        let handle = app.state::<ViewerStateHandle>();
        let state = handle.0.lock().expect("viewer state lock poisoned");
        state.viewer_epoch()
    };

    if !should_reveal_for_epoch(
        VIEWER_WINDOW_REVEALED_EPOCH.load(Ordering::SeqCst),
        current_epoch,
    ) {
        return;
    }

    VIEWER_WINDOW_REVEALED_EPOCH.store(current_epoch, Ordering::SeqCst);

    let selection = resolve_monitor_selection(app);

    if let Some(monitor) = selection.monitor.as_ref() {
        if let Err(error) = window.set_position(PhysicalPosition::new(
            monitor.position_x,
            monitor.position_y,
        )) {
            log::warn!("viewer_window_reveal_position_failed error={error}");
        }
    }

    if let Err(error) = window.show() {
        log::warn!("viewer_window_reveal_show_failed error={error}");
    }

    let should_fullscreen = selection.targeting != MONITOR_TARGETING_SINGLE_MONITOR_FALLBACK;
    if let Err(error) = window.set_fullscreen(should_fullscreen) {
        log::warn!("viewer_window_reveal_fullscreen_failed error={error}");
    }

    log::info!(
        "viewer_window_revealed epoch={} at_host_micros={}",
        current_epoch,
        crate::viewer::current_monotonic_micros()
    );
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
    let (outcome, previous_viewer_ready) = {
        let handle = app.state::<ViewerStateHandle>();
        let mut state = handle.0.lock().expect("viewer state lock poisoned");
        let previous_viewer_ready = state
            .snapshot_with_clock(current_epoch_ms(), current_monotonic_ms())
            .viewer_ready;
        let outcome = state.apply_listener_report_with_clock(
            input.viewer_epoch,
            current_epoch_ms(),
            current_monotonic_ms(),
        );
        (outcome, previous_viewer_ready)
    };
    let snapshot = read_snapshot(&app);

    if matches!(outcome, crate::viewer::viewer_state::ReportOutcome::Applied) {
        emit_viewer_readiness(&app, snapshot.clone());
        refresh_capture_readiness_if_viewer_became_ready(&app, previous_viewer_ready, &snapshot);
    }

    Ok(snapshot)
}

#[tauri::command]
pub fn report_viewer_layout(
    app: tauri::AppHandle,
    input: ViewerLayoutReportDto,
) -> Result<ViewerReadinessSnapshotDto, HostErrorEnvelope> {
    validate_viewer_layout_report(&input)?;

    let (outcome, previous_viewer_ready) = {
        let handle = app.state::<ViewerStateHandle>();
        let mut state = handle.0.lock().expect("viewer state lock poisoned");
        let previous_viewer_ready = state
            .snapshot_with_clock(current_epoch_ms(), current_monotonic_ms())
            .viewer_ready;
        let outcome = state.apply_layout_report_with_clock(
            &input,
            current_epoch_ms(),
            current_monotonic_ms(),
        );
        (outcome, previous_viewer_ready)
    };
    let snapshot = read_snapshot(&app);

    if matches!(outcome, crate::viewer::viewer_state::ReportOutcome::Applied) {
        emit_viewer_readiness(&app, snapshot.clone());
        refresh_capture_readiness_if_viewer_became_ready(&app, previous_viewer_ready, &snapshot);
    }

    Ok(snapshot)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capture_readiness_refreshes_only_when_a_bound_viewer_becomes_ready() {
        assert!(should_refresh_capture_readiness(
            false,
            true,
            Some("session_01hzzzzzzzzzzzzzzzzzzzzz")
        ));
        assert!(!should_refresh_capture_readiness(
            true,
            true,
            Some("session_01hzzzzzzzzzzzzzzzzzzzzz")
        ));
        assert!(!should_refresh_capture_readiness(false, true, None));
        assert!(!should_refresh_capture_readiness(
            false,
            false,
            Some("session_01hzzzzzzzzzzzzzzzzzzzzz")
        ));
    }
}
