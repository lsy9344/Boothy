use crate::file_management;
use chrono::{DateTime, Local, Timelike};
use log::{info, warn};
use serde::Serialize;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Duration;
use tauri::async_runtime::JoinHandle;
use tauri::{AppHandle, Emitter, Runtime};

const DEFAULT_T_MINUS_5_MESSAGE: &str = "이용시간이 5분 남았습니다.";
const WALL_CLOCK_JUMP_THRESHOLD_SECONDS: i64 = 5;

#[derive(Debug, Clone)]
pub struct SessionWindow {
    pub end: DateTime<Local>,
    pub t_minus_5_at: DateTime<Local>,
    pub reset_at: DateTime<Local>,
}

#[derive(Serialize, Clone)]
struct TimerTickPayload {
    remaining_seconds: i64,
}

#[derive(Serialize, Clone)]
struct TMinus5Payload {
    message: String,
}

pub struct SessionTimer {
    handles: Arc<Mutex<Option<TimerHandles>>>,
    generation: Arc<AtomicU64>,
}

struct TimerHandles {
    tick_handle: JoinHandle<()>,
    t_minus_5_handle: Option<JoinHandle<()>>,
    t_zero_handle: Option<JoinHandle<()>>,
    reset_handle: Option<JoinHandle<()>>,
}

enum TimedEvent {
    TMinus5 {
        message: String,
        emitted: Arc<AtomicBool>,
    },
    TZero {
        emitted: Arc<AtomicBool>,
    },
    Reset,
}

impl SessionTimer {
    pub fn new() -> Self {
        Self {
            handles: Arc::new(Mutex::new(None)),
            generation: Arc::new(AtomicU64::new(0)),
        }
    }

    pub fn start_for_session<R: Runtime>(&self, app_handle: AppHandle<R>) {
        self.stop();

        let generation = self.generation.fetch_add(1, Ordering::SeqCst) + 1;
        let start_time = Local::now();
        let window = compute_session_window(start_time);
        let initial_remaining = remaining_seconds(start_time, window.end);
        let message = load_t_minus_5_message(&app_handle);
        let t_minus_5_emitted = Arc::new(AtomicBool::new(false));
        let t_zero_emitted = Arc::new(AtomicBool::new(false));
        let reset_emitted = Arc::new(AtomicBool::new(false));

        info!(
            "Session timer started: end={}, t-5={}, reset={}",
            window.end.format("%H:%M:%S"),
            window.t_minus_5_at.format("%H:%M:%S"),
            window.reset_at.format("%H:%M:%S")
        );

        let tick_handle = spawn_tick_loop(
            app_handle.clone(),
            window.clone(),
            Arc::clone(&self.generation),
            generation,
            Arc::clone(&t_minus_5_emitted),
            Arc::clone(&t_zero_emitted),
            Arc::clone(&reset_emitted),
            message.clone(),
        );

        let should_emit_t_minus_5_now = should_emit_t_minus_5(start_time, &window);
        if should_emit_t_minus_5_now {
            emit_t_minus_5_once(&app_handle, &message, &t_minus_5_emitted);
        }

        let t_minus_5_handle = if should_emit_t_minus_5_now {
            None
        } else {
            duration_until(window.t_minus_5_at, start_time).map(|delay| {
                spawn_timed_event(
                    app_handle.clone(),
                    Arc::clone(&self.generation),
                    generation,
                    delay,
                    TimedEvent::TMinus5 {
                        message: message.clone(),
                        emitted: Arc::clone(&t_minus_5_emitted),
                    },
                )
            })
        };

        let t_zero_handle = if initial_remaining <= 0 {
            None
        } else {
            duration_until(window.end, start_time).map(|delay| {
                spawn_timed_event(
                    app_handle.clone(),
                    Arc::clone(&self.generation),
                    generation,
                    delay,
                    TimedEvent::TZero {
                        emitted: Arc::clone(&t_zero_emitted),
                    },
                )
            })
        };

        let reset_handle = duration_until(window.reset_at, start_time).map(|delay| {
            spawn_timed_event(
                app_handle.clone(),
                Arc::clone(&self.generation),
                generation,
                delay,
                TimedEvent::Reset,
            )
        });

        *lock_or_recover(&self.handles, "session_timer_handles") = Some(TimerHandles {
            tick_handle,
            t_minus_5_handle,
            t_zero_handle,
            reset_handle,
        });
    }

    pub fn stop(&self) {
        if let Some(handles) = lock_or_recover(&self.handles, "session_timer_handles").take() {
            handles.abort();
        }
    }
}

impl Default for SessionTimer {
    fn default() -> Self {
        Self::new()
    }
}

impl TimerHandles {
    fn abort(self) {
        self.tick_handle.abort();
        if let Some(handle) = self.t_minus_5_handle {
            handle.abort();
        }
        if let Some(handle) = self.t_zero_handle {
            handle.abort();
        }
        if let Some(handle) = self.reset_handle {
            handle.abort();
        }
    }
}

fn lock_or_recover<'a, T>(mutex: &'a Mutex<T>, label: &str) -> MutexGuard<'a, T> {
    match mutex.lock() {
        Ok(guard) => guard,
        Err(poisoned) => {
            warn!("Mutex poisoned for {}. Recovering inner state.", label);
            poisoned.into_inner()
        }
    }
}

fn load_t_minus_5_message<R: Runtime>(app_handle: &AppHandle<R>) -> String {
    file_management::load_settings_for_handle(app_handle)
        .ok()
        .and_then(|settings| settings.boothy_t_minus_5_warning_message)
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| DEFAULT_T_MINUS_5_MESSAGE.to_string())
}

pub fn compute_session_window(start: DateTime<Local>) -> SessionWindow {
    let end = start
        .with_minute(50)
        .and_then(|t| t.with_second(0))
        .and_then(|t| t.with_nanosecond(0))
        .unwrap_or(start);
    let t_minus_5_at = end - chrono::Duration::minutes(5);
    let reset_at = start
        .with_minute(59)
        .and_then(|t| t.with_second(0))
        .and_then(|t| t.with_nanosecond(0))
        .unwrap_or(start);

    SessionWindow {
        end,
        t_minus_5_at,
        reset_at,
    }
}

pub fn remaining_seconds(now: DateTime<Local>, end: DateTime<Local>) -> i64 {
    let diff = end.signed_duration_since(now).num_seconds();
    diff.max(0)
}

fn should_emit_t_minus_5(now: DateTime<Local>, window: &SessionWindow) -> bool {
    let remaining = remaining_seconds(now, window.end);
    remaining > 0 && now >= window.t_minus_5_at
}

fn duration_until(target: DateTime<Local>, now: DateTime<Local>) -> Option<Duration> {
    let diff = target.signed_duration_since(now);
    if diff <= chrono::Duration::zero() {
        return None;
    }
    diff.num_nanoseconds()
        .and_then(|nanos| nanos.try_into().ok())
        .map(Duration::from_nanos)
        .or_else(|| {
            diff.num_milliseconds()
                .try_into()
                .ok()
                .map(Duration::from_millis)
        })
}

fn spawn_tick_loop<R: Runtime>(
    app_handle: AppHandle<R>,
    window: SessionWindow,
    generation: Arc<AtomicU64>,
    generation_value: u64,
    t_minus_5_emitted: Arc<AtomicBool>,
    t_zero_emitted: Arc<AtomicBool>,
    reset_emitted: Arc<AtomicBool>,
    t_minus_5_message: String,
) -> JoinHandle<()> {
    tauri::async_runtime::spawn(async move {
        let mut last_wall_clock = Local::now();
        let initial_remaining = remaining_seconds(last_wall_clock, window.end);
        emit_tick(&app_handle, initial_remaining);

        // Check T-5 on startup
        if should_emit_t_minus_5(last_wall_clock, &window) {
            emit_t_minus_5_once(&app_handle, &t_minus_5_message, &t_minus_5_emitted);
        }

        // Check T-Zero on startup (if already past end time)
        if initial_remaining <= 0 {
            emit_t_zero_once(&app_handle, &t_zero_emitted);
        }

        // Check Reset on startup (if already past reset time)
        if last_wall_clock >= window.reset_at && !reset_emitted.swap(true, Ordering::SeqCst) {
            emit_reset(&app_handle);
        }

        let now = Local::now();
        let nanos_until_next = 1_000_000_000u64.saturating_sub(now.nanosecond() as u64);
        let start_instant = tokio::time::Instant::now() + Duration::from_nanos(nanos_until_next);
        let mut interval = tokio::time::interval_at(start_instant, Duration::from_secs(1));

        loop {
            interval.tick().await;
            if generation.load(Ordering::SeqCst) != generation_value {
                break;
            }

            let now = Local::now();
            let jump = now.signed_duration_since(last_wall_clock).num_seconds();
            if jump.abs() >= WALL_CLOCK_JUMP_THRESHOLD_SECONDS {
                warn!("Session timer wall-clock jump detected: {}s", jump);
            }

            let remaining = remaining_seconds(now, window.end);
            emit_tick(&app_handle, remaining);

            // Check T-5 in real-time
            if !t_minus_5_emitted.load(Ordering::SeqCst)
                && remaining > 0
                && now >= window.t_minus_5_at
            {
                emit_t_minus_5_once(&app_handle, &t_minus_5_message, &t_minus_5_emitted);
            }

            // Check T-Zero in real-time (when remaining hits 0)
            if remaining <= 0 && !t_zero_emitted.load(Ordering::SeqCst) {
                emit_t_zero_once(&app_handle, &t_zero_emitted);
            }

            // Check Reset in real-time
            if now >= window.reset_at && !reset_emitted.swap(true, Ordering::SeqCst) {
                info!("Reset time reached, emitting event");
                emit_reset(&app_handle);
            }

            last_wall_clock = now;
        }
    })
}

fn spawn_timed_event<R: Runtime>(
    app_handle: AppHandle<R>,
    generation: Arc<AtomicU64>,
    generation_value: u64,
    delay: Duration,
    event: TimedEvent,
) -> JoinHandle<()> {
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(delay).await;
        if generation.load(Ordering::SeqCst) != generation_value {
            return;
        }
        match event {
            TimedEvent::TMinus5 { message, emitted } => {
                emit_t_minus_5_once(&app_handle, &message, emitted.as_ref());
            }
            TimedEvent::TZero { emitted } => emit_t_zero_once(&app_handle, emitted.as_ref()),
            TimedEvent::Reset => emit_reset(&app_handle),
        }
    })
}

fn emit_tick<R: Runtime>(app_handle: &AppHandle<R>, remaining_seconds: i64) {
    let payload = TimerTickPayload { remaining_seconds };
    if let Err(err) = app_handle.emit("boothy-session-timer-tick", payload) {
        warn!("Failed to emit boothy-session-timer-tick: {}", err);
    }
}

fn emit_t_minus_5<R: Runtime>(app_handle: &AppHandle<R>, message: String) {
    let payload = TMinus5Payload { message };
    if let Err(err) = app_handle.emit("boothy-session-t-minus-5", payload) {
        warn!("Failed to emit boothy-session-t-minus-5: {}", err);
    }
}

fn emit_t_minus_5_once<R: Runtime>(app_handle: &AppHandle<R>, message: &str, emitted: &AtomicBool) {
    if emitted.swap(true, Ordering::SeqCst) {
        return;
    }
    emit_t_minus_5(app_handle, message.to_string());
}

fn emit_t_zero<R: Runtime>(app_handle: &AppHandle<R>) {
    if let Err(err) = app_handle.emit("boothy-session-t-zero", serde_json::json!({})) {
        warn!("Failed to emit boothy-session-t-zero: {}", err);
    }
}

fn emit_t_zero_once<R: Runtime>(app_handle: &AppHandle<R>, emitted: &AtomicBool) {
    if emitted.swap(true, Ordering::SeqCst) {
        return;
    }
    info!("T-Zero reached, emitting event");
    emit_t_zero(app_handle);
}

fn emit_reset<R: Runtime>(app_handle: &AppHandle<R>) {
    if let Err(err) = app_handle.emit("boothy-session-reset", serde_json::json!({})) {
        warn!("Failed to emit boothy-session-reset: {}", err);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn computes_end_boundary_at_minute_50() {
        let start = Local.with_ymd_and_hms(2025, 1, 1, 10, 0, 0).unwrap();
        let window = compute_session_window(start);

        assert_eq!(window.end.minute(), 50);
        assert_eq!(window.end.hour(), 10);
        assert_eq!(remaining_seconds(start, window.end), 50 * 60);
    }

    #[test]
    fn computes_late_entry_remaining_time() {
        let start = Local.with_ymd_and_hms(2025, 1, 1, 10, 42, 0).unwrap();
        let window = compute_session_window(start);

        assert_eq!(window.end.minute(), 50);
        assert_eq!(remaining_seconds(start, window.end), 8 * 60);
    }

    #[test]
    fn computes_t_minus_5_trigger_time() {
        let start = Local.with_ymd_and_hms(2025, 1, 1, 10, 0, 0).unwrap();
        let window = compute_session_window(start);

        assert_eq!(window.t_minus_5_at.minute(), 45);
        assert_eq!(remaining_seconds(window.t_minus_5_at, window.end), 5 * 60);
        assert!(should_emit_t_minus_5(window.t_minus_5_at, &window));
    }

    #[test]
    fn does_not_emit_t_minus_5_before_threshold() {
        let start = Local.with_ymd_and_hms(2025, 1, 1, 10, 0, 0).unwrap();
        let window = compute_session_window(start);
        let just_before = window.t_minus_5_at - chrono::Duration::seconds(1);

        assert!(!should_emit_t_minus_5(just_before, &window));
    }

    #[test]
    fn does_not_emit_t_minus_5_at_or_after_end() {
        let start = Local.with_ymd_and_hms(2025, 1, 1, 10, 0, 0).unwrap();
        let window = compute_session_window(start);

        assert!(!should_emit_t_minus_5(window.end, &window));
        assert!(!should_emit_t_minus_5(
            window.end + chrono::Duration::seconds(1),
            &window
        ));
    }
}
