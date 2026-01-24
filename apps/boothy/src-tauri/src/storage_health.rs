use crate::error;
use crate::file_management::{
    AppSettings, BOOTHY_STORAGE_CRITICAL_THRESHOLD_BYTES_DEFAULT,
    BOOTHY_STORAGE_POLL_INTERVAL_SECONDS_DEFAULT, BOOTHY_STORAGE_WARNING_THRESHOLD_BYTES_DEFAULT,
    load_settings_for_handle,
};
use crate::AppState;
use chrono::Utc;
use log::warn;
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Duration;
use tauri::async_runtime::JoinHandle;
use tauri::{AppHandle, Emitter, Manager, Runtime};

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum StorageHealthStatus {
    Healthy,
    Warning,
    Critical,
    Unknown,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StorageHealthPayload {
    pub status: StorageHealthStatus,
    pub free_bytes: u64,
    pub total_bytes: u64,
    pub warning_threshold_bytes: u64,
    pub critical_threshold_bytes: u64,
    pub sampled_at: String,
    pub diagnostic: Option<String>,
}

impl StorageHealthPayload {
    fn unknown_with_thresholds(
        warning_threshold_bytes: u64,
        critical_threshold_bytes: u64,
        diagnostic: Option<String>,
    ) -> Self {
        Self {
            status: StorageHealthStatus::Unknown,
            free_bytes: 0,
            total_bytes: 0,
            warning_threshold_bytes,
            critical_threshold_bytes,
            sampled_at: Utc::now().to_rfc3339(),
            diagnostic,
        }
    }
}

#[derive(Debug, Clone)]
struct DiskSpaceSample {
    free_bytes: u64,
    total_bytes: u64,
}

#[derive(Debug, Clone)]
struct StorageHealthState {
    latest_payload: StorageHealthPayload,
    session_path: Option<PathBuf>,
}

impl StorageHealthState {
    fn new() -> Self {
        let settings = StorageHealthSettings::default();
        Self {
            latest_payload: StorageHealthPayload::unknown_with_thresholds(
                settings.warning_threshold_bytes,
                settings.critical_threshold_bytes,
                None,
            ),
            session_path: None,
        }
    }
}

#[derive(Debug, Clone)]
struct StorageHealthSettings {
    enabled: bool,
    warning_threshold_bytes: u64,
    critical_threshold_bytes: u64,
    poll_interval_seconds: u64,
}

impl Default for StorageHealthSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            warning_threshold_bytes: BOOTHY_STORAGE_WARNING_THRESHOLD_BYTES_DEFAULT,
            critical_threshold_bytes: BOOTHY_STORAGE_CRITICAL_THRESHOLD_BYTES_DEFAULT,
            poll_interval_seconds: BOOTHY_STORAGE_POLL_INTERVAL_SECONDS_DEFAULT,
        }
    }
}

impl StorageHealthSettings {
    fn from_settings(settings: &AppSettings) -> Self {
        let mut warning_threshold_bytes = settings
            .boothy_storage_warning_threshold_bytes
            .filter(|value| *value > 0)
            .unwrap_or(BOOTHY_STORAGE_WARNING_THRESHOLD_BYTES_DEFAULT);
        let critical_threshold_bytes = settings
            .boothy_storage_critical_threshold_bytes
            .filter(|value| *value > 0)
            .unwrap_or(BOOTHY_STORAGE_CRITICAL_THRESHOLD_BYTES_DEFAULT);
        let poll_interval_seconds = settings
            .boothy_storage_poll_interval_seconds
            .filter(|value| *value > 0)
            .unwrap_or(BOOTHY_STORAGE_POLL_INTERVAL_SECONDS_DEFAULT);
        let enabled = settings.boothy_storage_health_enabled.unwrap_or(true);

        if warning_threshold_bytes < critical_threshold_bytes {
            warning_threshold_bytes = critical_threshold_bytes;
        }

        Self {
            enabled,
            warning_threshold_bytes,
            critical_threshold_bytes,
            poll_interval_seconds,
        }
    }
}

pub struct StorageHealthMonitor {
    handles: Arc<Mutex<Option<JoinHandle<()>>>>,
    generation: Arc<AtomicU64>,
    state: Arc<Mutex<StorageHealthState>>,
}

impl StorageHealthMonitor {
    pub fn new() -> Self {
        Self {
            handles: Arc::new(Mutex::new(None)),
            generation: Arc::new(AtomicU64::new(0)),
            state: Arc::new(Mutex::new(StorageHealthState::new())),
        }
    }

    pub fn start_for_session<R: Runtime>(&self, app_handle: AppHandle<R>, session_path: PathBuf) {
        self.stop();

        let generation_value = self.generation.fetch_add(1, Ordering::SeqCst) + 1;
        let session_path = canonicalize_path(session_path);
        {
            let mut state = lock_or_recover(&self.state, "storage_health_state");
            state.session_path = Some(session_path.clone());
        }

        let settings = load_storage_health_settings(&app_handle);
        let payload = build_payload(&session_path, &settings);
        let previous_status = update_latest_payload(&self.state, payload.clone());
        handle_storage_health_transition(&app_handle, previous_status, payload.status);
        emit_storage_health(&app_handle, payload);

        let handle = spawn_monitor_loop(
            app_handle,
            session_path,
            Arc::clone(&self.generation),
            generation_value,
            Arc::clone(&self.state),
        );
        *lock_or_recover(&self.handles, "storage_health_handles") = Some(handle);
    }

    pub fn stop(&self) {
        if let Some(handle) = lock_or_recover(&self.handles, "storage_health_handles").take() {
            handle.abort();
        }
    }

    pub fn latest_payload(&self) -> StorageHealthPayload {
        lock_or_recover(&self.state, "storage_health_state")
            .latest_payload
            .clone()
    }

    pub fn is_critical(&self) -> bool {
        matches!(self.latest_payload().status, StorageHealthStatus::Critical)
    }

    pub fn guard_critical(&self) -> Result<(), error::BoothyError> {
        if self.is_critical() {
            Err(error::storage::critical_lockout())
        } else {
            Ok(())
        }
    }
}

impl Default for StorageHealthMonitor {
    fn default() -> Self {
        Self::new()
    }
}

fn canonicalize_path(path: PathBuf) -> PathBuf {
    std::fs::canonicalize(&path).unwrap_or(path)
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

fn spawn_monitor_loop<R: Runtime>(
    app_handle: AppHandle<R>,
    session_path: PathBuf,
    generation: Arc<AtomicU64>,
    generation_value: u64,
    state: Arc<Mutex<StorageHealthState>>,
) -> JoinHandle<()> {
    tauri::async_runtime::spawn(async move {
        loop {
            if generation.load(Ordering::SeqCst) != generation_value {
                break;
            }

            let settings = load_storage_health_settings(&app_handle);
            let payload = build_payload(&session_path, &settings);
            let previous_status = update_latest_payload(&state, payload.clone());
            handle_storage_health_transition(&app_handle, previous_status, payload.status);
            emit_storage_health(&app_handle, payload);

            let interval = Duration::from_secs(settings.poll_interval_seconds.max(1));
            tokio::time::sleep(interval).await;
        }
    })
}

fn load_storage_health_settings<R: Runtime>(app_handle: &AppHandle<R>) -> StorageHealthSettings {
    load_settings_for_handle(app_handle)
        .map(|settings| StorageHealthSettings::from_settings(&settings))
        .unwrap_or_default()
}

fn build_payload(session_path: &Path, settings: &StorageHealthSettings) -> StorageHealthPayload {
    let sampled_at = Utc::now().to_rfc3339();
    if !settings.enabled {
        return StorageHealthPayload::unknown_with_thresholds(
            settings.warning_threshold_bytes,
            settings.critical_threshold_bytes,
            Some("storage health disabled".to_string()),
        );
    }

    match sample_disk_space(session_path) {
        Ok(sample) => {
            let status = status_from_sample(Some(&sample), settings);
            StorageHealthPayload {
                status,
                free_bytes: sample.free_bytes,
                total_bytes: sample.total_bytes,
                warning_threshold_bytes: settings.warning_threshold_bytes,
                critical_threshold_bytes: settings.critical_threshold_bytes,
                sampled_at,
                diagnostic: None,
            }
        }
        Err(err) => StorageHealthPayload::unknown_with_thresholds(
            settings.warning_threshold_bytes,
            settings.critical_threshold_bytes,
            Some(err),
        ),
    }
}

fn update_latest_payload(
    state: &Mutex<StorageHealthState>,
    payload: StorageHealthPayload,
) -> StorageHealthStatus {
    let mut guard = lock_or_recover(state, "storage_health_state");
    let previous_status = guard.latest_payload.status;
    guard.latest_payload = payload;
    previous_status
}

fn handle_storage_health_transition<R: Runtime>(
    app_handle: &AppHandle<R>,
    previous_status: StorageHealthStatus,
    new_status: StorageHealthStatus,
) {
    if previous_status == new_status {
        return;
    }

    let state = app_handle.state::<AppState>();
    if new_status == StorageHealthStatus::Critical {
        let queue = Arc::clone(&state.background_export_queue);
        queue.set_storage_lockout(true);
        tauri::async_runtime::spawn(async move {
            queue.pause_and_cancel().await;
        });
    } else if previous_status == StorageHealthStatus::Critical {
        state.background_export_queue.set_storage_lockout(false);
        if state.export_task_handle.lock().unwrap().is_none() {
            state.background_export_queue.resume();
        }
    }
}

fn emit_storage_health<R: Runtime>(app_handle: &AppHandle<R>, payload: StorageHealthPayload) {
    if !has_active_webview(app_handle) {
        return;
    }
    if let Err(err) = app_handle.emit("boothy-storage-health", payload) {
        warn!("Failed to emit boothy-storage-health: {}", err);
    }
}

fn has_active_webview<R: Runtime>(app_handle: &AppHandle<R>) -> bool {
    !app_handle.webview_windows().is_empty()
}

fn status_from_sample(sample: Option<&DiskSpaceSample>, settings: &StorageHealthSettings) -> StorageHealthStatus {
    match sample {
        Some(sample) => classify_storage_status(
            sample.free_bytes,
            settings.warning_threshold_bytes,
            settings.critical_threshold_bytes,
        ),
        None => StorageHealthStatus::Unknown,
    }
}

fn classify_storage_status(
    free_bytes: u64,
    warning_threshold_bytes: u64,
    critical_threshold_bytes: u64,
) -> StorageHealthStatus {
    if free_bytes <= critical_threshold_bytes {
        StorageHealthStatus::Critical
    } else if free_bytes <= warning_threshold_bytes {
        StorageHealthStatus::Warning
    } else {
        StorageHealthStatus::Healthy
    }
}

#[cfg(target_os = "windows")]
fn sample_disk_space(path: &Path) -> Result<DiskSpaceSample, String> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Storage::FileSystem::GetDiskFreeSpaceExW;

    let path = std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    let wide: Vec<u16> = path.as_os_str().encode_wide().chain(std::iter::once(0)).collect();
    let mut free_bytes: u64 = 0;
    let mut total_bytes: u64 = 0;
    let mut total_free_bytes: u64 = 0;

    let success = unsafe {
        GetDiskFreeSpaceExW(
            wide.as_ptr(),
            &mut free_bytes,
            &mut total_bytes,
            &mut total_free_bytes,
        )
    };
    if success == 0 {
        return Err(std::io::Error::last_os_error().to_string());
    }
    if total_bytes == 0 {
        return Err("Total bytes returned zero".to_string());
    }

    Ok(DiskSpaceSample {
        free_bytes,
        total_bytes,
    })
}

#[cfg(not(target_os = "windows"))]
fn sample_disk_space(_path: &Path) -> Result<DiskSpaceSample, String> {
    Err("Storage health sampling is only supported on Windows.".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_payload(status: StorageHealthStatus) -> StorageHealthPayload {
        StorageHealthPayload {
            status,
            free_bytes: 0,
            total_bytes: 0,
            warning_threshold_bytes: 10,
            critical_threshold_bytes: 5,
            sampled_at: Utc::now().to_rfc3339(),
            diagnostic: None,
        }
    }

    #[test]
    fn classify_storage_status_thresholds() {
        let warning = 10;
        let critical = 5;

        assert_eq!(
            classify_storage_status(11, warning, critical),
            StorageHealthStatus::Healthy
        );
        assert_eq!(
            classify_storage_status(10, warning, critical),
            StorageHealthStatus::Warning
        );
        assert_eq!(
            classify_storage_status(5, warning, critical),
            StorageHealthStatus::Critical
        );
    }

    #[test]
    fn status_from_sample_handles_unknown() {
        let settings = StorageHealthSettings {
            enabled: true,
            warning_threshold_bytes: 10,
            critical_threshold_bytes: 5,
            poll_interval_seconds: 10,
        };

        assert_eq!(
            status_from_sample(None, &settings),
            StorageHealthStatus::Unknown
        );
    }

    #[test]
    fn guard_critical_blocks_when_critical() {
        let monitor = StorageHealthMonitor::new();
        update_latest_payload(&monitor.state, make_payload(StorageHealthStatus::Critical));

        let err = monitor
            .guard_critical()
            .expect_err("critical status should block guard");

        assert_eq!(err.code, error::storage::STORAGE_CRITICAL);
        assert_eq!(err.message, error::storage::STORAGE_CRITICAL_MESSAGE);
    }

    #[test]
    fn guard_critical_allows_non_critical() {
        let monitor = StorageHealthMonitor::new();

        update_latest_payload(&monitor.state, make_payload(StorageHealthStatus::Healthy));
        assert!(monitor.guard_critical().is_ok());

        update_latest_payload(&monitor.state, make_payload(StorageHealthStatus::Unknown));
        assert!(monitor.guard_critical().is_ok());
    }
}
