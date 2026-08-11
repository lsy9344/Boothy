//! Story 7.1: 촬영 전 전용 관람 창 준비와 화면 크기 계약.
//!
//! 이 모듈은 viewer readiness truth만 소유한다. immutable display generation,
//! display pointer, double-buffer swap, actual-present telemetry는 Story 7.2 이후가 소유한다.

pub mod display_profile;
pub mod readiness_gate;
pub mod viewer_state;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::sync::OnceLock;
use std::thread::ThreadId;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

pub use viewer_state::ViewerState;

/// Tauri `manage`로 등록되는 프로세스 전역 viewer truth.
#[derive(Default)]
pub struct ViewerStateHandle(pub Mutex<ViewerState>);

static MAIN_THREAD_ID: OnceLock<ThreadId> = OnceLock::new();
static EVENT_LOOP_STARTED: AtomicBool = AtomicBool::new(false);

/// `setup`에서 호출한다. 이후 window 생성이 event loop 스레드에서 일어나는지 판별하는 기준이 된다.
pub fn record_main_thread() {
    let _ = MAIN_THREAD_ID.set(std::thread::current().id());
}

/// `app.run` 직전에 호출한다. 이 시점부터 main thread는 event loop에 점유된다.
pub fn mark_event_loop_started() {
    EVENT_LOOP_STARTED.store(true, Ordering::SeqCst);
}

pub fn is_main_thread() -> bool {
    MAIN_THREAD_ID
        .get()
        .is_some_and(|main_thread_id| *main_thread_id == std::thread::current().id())
}

pub fn is_event_loop_started() -> bool {
    EVENT_LOOP_STARTED.load(Ordering::SeqCst)
}

/// Window 생성을 background 스레드로 넘겨야 하는지 판정한다.
///
/// `WebviewWindowBuilder::build()`는 window 생성 메시지를 event loop에 보내고 응답을 기다린다.
/// event loop가 도는 main thread에서 이걸 호출하면 자기 자신을 기다리는 교착이 되고,
/// 창은 뜨지만 절대 그려지지 않으며 호출도 반환되지 않는다.
/// (HV-13A 2026-08-11 No-Go의 실제 원인. `#[tauri::command]`는 기본이 blocking 실행이라
/// main thread에서 돈다 — `tauri-macros` `ExecutionContext::Blocking`.)
///
/// `setup` 단계는 event loop가 아직 시작되지 않아 main thread에서 직접 생성해도 안전하다.
pub fn should_offload_window_creation(is_main_thread: bool, event_loop_started: bool) -> bool {
    is_main_thread && event_loop_started
}

pub fn current_epoch_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_millis() as u64)
        .unwrap_or(0)
}

/// 프로세스 수명 동안 wall-clock 보정의 영향을 받지 않는 liveness 기준.
pub fn current_monotonic_ms() -> u64 {
    static PROCESS_START: OnceLock<Instant> = OnceLock::new();

    PROCESS_START
        .get_or_init(Instant::now)
        .elapsed()
        .as_millis() as u64
}
