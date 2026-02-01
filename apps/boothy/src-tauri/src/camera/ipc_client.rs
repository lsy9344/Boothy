use super::ipc_models::*;
use crate::error::{self, BoothyError};
use log::{debug, error, info, warn};
use serde::Serialize;
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::mpsc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};
use std::time::{Duration as StdDuration, Instant};
use tokio::sync::oneshot;

const PIPE_NAME: &str = "\\\\.\\pipe\\boothy_camera_sidecar";
const IPC_TIMEOUT_MS: u64 = 5000;
const STATUS_MONITOR_INTERVAL: StdDuration = StdDuration::from_secs(5);
const STATUS_MONITOR_TIMEOUT: StdDuration = StdDuration::from_secs(4);
const STATUS_MONITOR_INITIAL_DELAY: StdDuration = StdDuration::from_secs(2);
const STATUS_MONITOR_ERROR_BACKOFF_MAX: StdDuration = StdDuration::from_secs(30);
const IPC_WRITE_TIMEOUT: StdDuration = StdDuration::from_millis(1500);

#[derive(Debug)]
struct PipeWriteRequest {
    bytes: Vec<u8>,
    ack: oneshot::Sender<Result<(), String>>,
}

/// Camera IPC Client State
/// Manages sidecar process lifecycle and Named Pipe communication
#[derive(Clone)]
pub struct CameraIpcClient {
    /// Sidecar process handle
    sidecar_process: Arc<Mutex<Option<Child>>>,

    /// Whether the sidecar is connected
    connected: Arc<Mutex<bool>>,

    /// IPC diagnostics state
    diagnostics: Arc<Mutex<CameraDiagnosticsInternal>>,

    /// App handle for emitting events
    app_handle: AppHandle,

    /// Read handle for IPC messages (event listener uses a clone of this)
    rx_pipe: Arc<Mutex<Option<std::fs::File>>>,

    /// Write channel (single writer thread owns the actual pipe write handle)
    tx_writer: Arc<Mutex<Option<mpsc::Sender<PipeWriteRequest>>>>,

    /// Prevent concurrent start attempts
    starting: Arc<AtomicBool>,

    /// Pending request map for request/response correlation
    pending_requests: Arc<Mutex<HashMap<String, oneshot::Sender<Result<serde_json::Value, IpcError>>>>>,

    /// Background camera.getStatus poller state (to keep UI lamp updated even if the frontend reloads/stalls)
    status_monitor_started: Arc<AtomicBool>,
}

#[derive(Clone, Debug, Default)]
struct CameraDiagnosticsInternal {
    ipc_state: IpcConnectionState,
    last_error: Option<String>,
    last_request_id: Option<String>,
    last_correlation_id: Option<String>,
    no_camera_streak: u32,
    no_camera_since: Option<Instant>,
    last_camera_detected_at: Option<Instant>,
    last_forced_restart_at: Option<Instant>,
    sidecar_connected_at: Option<Instant>,
}

#[derive(Clone, Debug, Default)]
enum IpcConnectionState {
    Connected,
    #[default]
    Disconnected,
    Reconnecting,
}

impl IpcConnectionState {
    fn as_str(&self) -> &'static str {
        match self {
            IpcConnectionState::Connected => "connected",
            IpcConnectionState::Disconnected => "disconnected",
            IpcConnectionState::Reconnecting => "reconnecting",
        }
    }
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CameraDiagnosticsSnapshot {
    pub ipc_state: String,
    pub last_error: Option<String>,
    pub protocol_version: String,
    pub request_id: Option<String>,
    pub correlation_id: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CameraAutoRestartReason {
    LostAfterDetected,
    ProlongedNoCamera,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CameraAutoRestartDecision {
    pub should_restart: bool,
    pub reason: Option<CameraAutoRestartReason>,
}

fn note_camera_status_internal(
    diag: &mut CameraDiagnosticsInternal,
    status: &CameraStatusResponse,
    now: Instant,
) -> CameraAutoRestartDecision {
    if matches!(diag.ipc_state, IpcConnectionState::Disconnected) {
        return CameraAutoRestartDecision {
            should_restart: false,
            reason: None,
        };
    }

    if status.connected && status.camera_detected {
        diag.no_camera_streak = 0;
        diag.no_camera_since = None;
        diag.last_camera_detected_at = Some(now);
        return CameraAutoRestartDecision {
            should_restart: false,
            reason: None,
        };
    }

    // If the camera is simply disconnected/off, restarting the sidecar is not useful and causes
    // customer lamp flapping. Only consider auto-restart when the camera is connected but the SDK
    // can't detect it (a "stuck" state).
    if !status.connected {
        diag.no_camera_streak = 0;
        diag.no_camera_since = None;
        return CameraAutoRestartDecision {
            should_restart: false,
            reason: None,
        };
    }

    // Give the sidecar a short grace period after connecting/restarting.
    // During boot, getStatus can legitimately return "not detected" briefly.
    let startup_grace = StdDuration::from_secs(10);
    if let Some(connected_at) = diag.sidecar_connected_at {
        if now.duration_since(connected_at) < startup_grace {
            diag.no_camera_streak = 0;
            diag.no_camera_since = None;
            return CameraAutoRestartDecision {
                should_restart: false,
                reason: None,
            };
        }
    }

    diag.no_camera_streak = diag.no_camera_streak.saturating_add(1);
    if diag.no_camera_since.is_none() {
        diag.no_camera_since = Some(now);
    }

    let throttle = StdDuration::from_secs(30);
    if let Some(last_restart) = diag.last_forced_restart_at {
        if now.duration_since(last_restart) < throttle {
            return CameraAutoRestartDecision {
                should_restart: false,
                reason: None,
            };
        }
    }

    let has_detected_before = diag.last_camera_detected_at.is_some();
    let no_camera_duration = diag
        .no_camera_since
        .map(|since| now.duration_since(since))
        .unwrap_or_else(|| StdDuration::from_secs(0));

    // Use a time-based gate to avoid false positives and periodic restarts.
    let decision = if has_detected_before
        && diag.no_camera_streak >= 4
        && no_camera_duration >= StdDuration::from_secs(20)
    {
        CameraAutoRestartDecision {
            should_restart: true,
            reason: Some(CameraAutoRestartReason::LostAfterDetected),
        }
    } else if !has_detected_before
        && diag.no_camera_streak >= 8
        && no_camera_duration >= StdDuration::from_secs(45)
    {
        CameraAutoRestartDecision {
            should_restart: true,
            reason: Some(CameraAutoRestartReason::ProlongedNoCamera),
        }
    } else {
        CameraAutoRestartDecision {
            should_restart: false,
            reason: None,
        }
    };

    if decision.should_restart {
        diag.last_forced_restart_at = Some(now);
    }

    decision
}

impl CameraIpcClient {
    /// Create a new Camera IPC Client
    pub fn new(app_handle: AppHandle) -> Self {
        Self {
            sidecar_process: Arc::new(Mutex::new(None)),
            connected: Arc::new(Mutex::new(false)),
            diagnostics: Arc::new(Mutex::new(CameraDiagnosticsInternal::default())),
            app_handle,
            rx_pipe: Arc::new(Mutex::new(None)),
            tx_writer: Arc::new(Mutex::new(None)),
            starting: Arc::new(AtomicBool::new(false)),
            pending_requests: Arc::new(Mutex::new(HashMap::new())),
            status_monitor_started: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Check if the sidecar is connected
    pub fn is_connected(&self) -> bool {
        self.connected.lock().map(|g| *g).unwrap_or(false)
    }

    pub fn note_camera_status(&self, status: &CameraStatusResponse) -> CameraAutoRestartDecision {
        let now = Instant::now();
        let mut diag = match self.diagnostics.lock() {
            Ok(guard) => guard,
            Err(_) => {
                return CameraAutoRestartDecision {
                    should_restart: false,
                    reason: None,
                };
            }
        };

        note_camera_status_internal(&mut diag, status, now)
    }

    /// Start the camera sidecar process and establish IPC connection
    pub async fn start_sidecar(&self) -> Result<(), String> {
        let correlation_id = generate_correlation_id();
        info!("[{}] Starting camera sidecar...", correlation_id);

        // If we're already connected, do not downgrade diagnostics to "reconnecting".
        // This can happen after an F5/WebView reload where the frontend re-calls session
        // setup, but the Rust backend + sidecar are still connected.
        if self.is_connected() {
            info!(
                "[{}] Sidecar already connected; start_sidecar is a no-op",
                correlation_id
            );
            self.set_ipc_state(IpcConnectionState::Connected);
            if let Ok(mut diag) = self.diagnostics.lock() {
                diag.last_error = None;
            }
            return Ok(());
        }

        // If another start is already in progress (e.g. React dev StrictMode double-mount or rapid
        // refreshes), wait briefly so callers don't get stuck seeing ipcState=reconnecting.
        if self
            .starting
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .is_err()
        {
            warn!("[{}] Sidecar start already in progress", correlation_id);
            let deadline = tokio::time::Instant::now() + Duration::from_secs(3);
            while tokio::time::Instant::now() < deadline {
                if self.is_connected() {
                    self.set_ipc_state(IpcConnectionState::Connected);
                    if let Ok(mut diag) = self.diagnostics.lock() {
                        diag.last_error = None;
                    }
                    return Ok(());
                }
                if !self.starting.load(Ordering::SeqCst) {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(50)).await;
            }

            if self.is_connected() {
                self.set_ipc_state(IpcConnectionState::Connected);
                if let Ok(mut diag) = self.diagnostics.lock() {
                    diag.last_error = None;
                }
                return Ok(());
            }

            // Try to become the starter if the other attempt has finished.
            if self
                .starting
                .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
                .is_err()
            {
                return Ok(());
            }
        }

        // We're the active starter and not connected yet: now mark as reconnecting.
        self.set_ipc_state(IpcConnectionState::Reconnecting);

        struct StartGuard {
            flag: Arc<AtomicBool>,
        }

        impl Drop for StartGuard {
            fn drop(&mut self) {
                self.flag.store(false, Ordering::SeqCst);
            }
        }

        let _start_guard = StartGuard {
            flag: Arc::clone(&self.starting),
        };

        // Check if already running
        if self.is_connected() {
            warn!("[{}] Sidecar already running", correlation_id);
            return Ok(());
        }

        if self
            .connect_to_pipe_with_retries(2, Duration::from_millis(100), false)
            .await
            .is_ok()
        {
            self.start_event_listener();
            self.start_status_monitor();
            info!("[{}] Connected to existing sidecar pipe", correlation_id);
            return Ok(());
        }

        // If a process is already running, try to connect before spawning another
        let process_running = {
            let mut process_guard = self.sidecar_process.lock().unwrap();
            if let Some(child) = process_guard.as_mut() {
                match child.try_wait() {
                    Ok(None) => true,
                    Ok(Some(status)) => {
                        warn!(
                            "[{}] Previous sidecar process exited: {}",
                            correlation_id, status
                        );
                        *process_guard = None;
                        false
                    }
                    Err(e) => {
                        warn!(
                            "[{}] Failed to check sidecar process status: {}",
                            correlation_id, e
                        );
                        false
                    }
                }
            } else {
                false
            }
        };

        if process_running {
            if self
                .connect_to_pipe_with_retries(3, Duration::from_millis(200), false)
                .await
                .is_ok()
            {
                self.start_event_listener();
                self.start_status_monitor();
                info!("[{}] Connected to existing sidecar", correlation_id);
                return Ok(());
            }

            warn!(
                "[{}] Existing sidecar unresponsive, restarting",
                correlation_id
            );
            self.stop_sidecar_for_restart();
        }

        // Get sidecar executable path
        let sidecar_path = self.get_sidecar_path()?;
        info!(
            "[{}] Sidecar path: {}",
            correlation_id,
            sidecar_path.display()
        );

        // Start sidecar process
        let mut command = Command::new(&sidecar_path);
        if let Some(mode) = resolve_sidecar_mode() {
            command.arg("--mode").arg(mode);
        }

        let mut child = command
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| format!("Failed to start sidecar: {}", e))?;

        let pid = child.id();
        info!(
            "[{}] Sidecar process started (PID: {})",
            correlation_id, pid
        );

        // Capture stdout/stderr for logging
        if let Some(stdout) = child.stdout.take() {
            std::thread::spawn(move || {
                let reader = BufReader::new(stdout);
                for line in reader.lines() {
                    if let Ok(line) = line {
                        info!("[Sidecar] {}", line);
                    }
                }
            });
        }

        if let Some(stderr) = child.stderr.take() {
            std::thread::spawn(move || {
                let reader = BufReader::new(stderr);
                for line in reader.lines() {
                    if let Ok(line) = line {
                        warn!("[Sidecar] {}", line);
                    }
                }
            });
        }

        // Store process handle
        {
            let mut process_guard = self.sidecar_process.lock().unwrap();
            *process_guard = Some(child);
        }

        // Wait for sidecar to start Named Pipe server
        tokio::time::sleep(Duration::from_millis(500)).await;

        // Connect to Named Pipe
        if let Err(err) = self.connect_to_pipe().await {
            self.stop_sidecar();
            self.record_last_error(err.clone());
            return Err(err);
        }

        // Start event listener
        self.start_event_listener();
        self.start_status_monitor();

        info!("[{}] Sidecar started and connected", correlation_id);
        Ok(())
    }

    /// Stop the camera sidecar process
    pub fn stop_sidecar(&self) {
        self.stop_sidecar_internal(IpcConnectionState::Disconnected, "stopSidecar");
    }

    pub fn stop_sidecar_for_restart(&self) {
        self.stop_sidecar_internal(IpcConnectionState::Reconnecting, "stopSidecar");
    }

    fn stop_sidecar_internal(&self, desired_state: IpcConnectionState, hint_source: &'static str) {
        let correlation_id = generate_correlation_id();
        info!("[{}] Stopping camera sidecar...", correlation_id);

        self.send_shutdown_signal(&correlation_id);

        // Mark as disconnected
        if let Ok(mut guard) = self.connected.lock() {
            *guard = false;
        }
        let ipc_state_str = desired_state.as_str();
        self.set_ipc_state(desired_state);
        if let Ok(mut diag) = self.diagnostics.lock() {
            diag.no_camera_streak = 0;
            diag.no_camera_since = None;
            diag.last_camera_detected_at = None;
            diag.sidecar_connected_at = None;
        }

        // Ensure the UI refreshes even when this stop was triggered from a background poll / timeout path
        // where we intentionally suppress boothy-camera-error events.
        let _ = self.app_handle.emit(
            "boothy-camera-status-hint",
            serde_json::json!({
                "source": hint_source,
                "correlationId": correlation_id,
                "ipcState": ipc_state_str,
            }),
        );

        // Close pipe
        if let Ok(mut guard) = self.tx_writer.lock() {
            *guard = None;
        }
        if let Ok(mut guard) = self.rx_pipe.lock() {
            *guard = None;
        }

        // Kill process
        if let Ok(mut process_guard) = self.sidecar_process.lock() {
            if let Some(mut child) = process_guard.take() {
                let _ = child.kill();
                let _ = child.wait();
                info!("[{}] Sidecar process terminated", correlation_id);
            }
        }
    }

    fn send_shutdown_signal(&self, correlation_id: &str) {
        if !self.is_connected() {
            return;
        }

        let request_id = generate_request_id();
        let message = IpcMessage::new_request(
            "system.shutdown".to_string(),
            correlation_id.to_string(),
            request_id,
            serde_json::json!({}),
        );

        let json = match serde_json::to_string(&message) {
            Ok(json) => json,
            Err(e) => {
                warn!(
                    "[{}] Failed to serialize shutdown request: {}",
                    correlation_id, e
                );
                return;
            }
        };

        let sender = self
            .tx_writer
            .lock()
            .ok()
            .and_then(|guard| guard.as_ref().cloned());
        let Some(sender) = sender else {
            return;
        };

        let mut bytes = json.into_bytes();
        bytes.push(b'\n');
        let (ack, _rx) = oneshot::channel::<Result<(), String>>();
        let _ = sender.send(PipeWriteRequest { bytes, ack });
    }

    /// Get the path to the sidecar executable
    fn get_sidecar_path(&self) -> Result<PathBuf, String> {
        if cfg!(debug_assertions) {
            if let Ok(override_path) = std::env::var("BOOTHY_SIDECAR_PATH") {
                let override_path = PathBuf::from(override_path);
                if override_path.exists() {
                    return Ok(override_path);
                }
            }

            // Prefer the self-contained publish output when available.
            // This makes it easy to iterate on the sidecar without fighting file locks on the
            // repo-bundled `resources/camera-sidecar/Boothy.CameraSidecar.exe` while Boothy is running.
            if let Some(path) = find_dev_sidecar("Release") {
                return Ok(path);
            }
            if let Some(path) = find_dev_sidecar("Debug") {
                return Ok(path);
            }

            // Fallback: repo-bundled sidecar (typically copied during packaging).
            let repo_resource_sidecar = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("resources")
                .join("camera-sidecar")
                .join("Boothy.CameraSidecar.exe");
            if repo_resource_sidecar.exists() {
                return Ok(repo_resource_sidecar);
            }

            let fallback = PathBuf::from(
                "../../apps/camera-sidecar/bin/Debug/net8.0/Boothy.CameraSidecar.exe",
            );
            if fallback.exists() {
                return Ok(fallback);
            }

            return Err(
                "Sidecar executable not found. Tried BOOTHY_SIDECAR_PATH and repo-relative paths."
                    .to_string(),
            );
        }

        // Production: bundled with app
        let sidecar_path = self
            .app_handle
            .path()
            .resource_dir()
            .map_err(|e| format!("Failed to get resource dir: {}", e))?
            .join("camera-sidecar")
            .join("Boothy.CameraSidecar.exe");

        if !sidecar_path.exists() {
            return Err(format!(
                "Sidecar executable not found at: {}",
                sidecar_path.display()
            ));
        }

        Ok(sidecar_path)
    }

    /// Connect to the Named Pipe
    async fn connect_to_pipe(&self) -> Result<(), String> {
        self.connect_to_pipe_with_retries(10, Duration::from_millis(200), true)
            .await
    }

    async fn connect_to_pipe_with_retries(
        &self,
        max_retries: usize,
        retry_delay: Duration,
        record_error: bool,
    ) -> Result<(), String> {
        use std::fs::OpenOptions;

        let correlation_id = generate_correlation_id();
        if record_error {
            info!(
                "[{}] Connecting to Named Pipe: {}",
                correlation_id, PIPE_NAME
            );
        } else {
            debug!(
                "[{}] Probing for existing Named Pipe: {}",
                correlation_id, PIPE_NAME
            );
        }

        // Retry connection with timeout
        let mut last_error = String::new();

        for i in 0..max_retries {
            match OpenOptions::new().read(true).write(true).open(PIPE_NAME) {
                Ok(pipe) => {
                    info!("[{}] Connected to Named Pipe", correlation_id);

                    let rx_pipe = pipe.try_clone().map_err(|e| {
                        format!("Failed to clone Named Pipe handle for reader: {}", e)
                    })?;

                    let (sender, receiver) = mpsc::channel::<PipeWriteRequest>();
                    let mut tx_pipe = pipe;
                    tokio::task::spawn_blocking(move || {
                        for request in receiver {
                            let result = tx_pipe
                                .write_all(&request.bytes)
                                .map_err(|e| format!("Failed to write to pipe: {}", e));
                            let should_stop = result.is_err();
                            let _ = request.ack.send(result);
                            // If writing fails, stop accepting further writes on this handle.
                            if should_stop {
                                break;
                            }
                        }
                    });

                    if let Ok(mut guard) = self.rx_pipe.lock() {
                        *guard = Some(rx_pipe);
                    }
                    if let Ok(mut guard) = self.tx_writer.lock() {
                        *guard = Some(sender);
                    }

                    // Mark as connected
                    if let Ok(mut guard) = self.connected.lock() {
                        *guard = true;
                    }
                    self.set_ipc_state(IpcConnectionState::Connected);

                    // Clear transient connection errors (e.g. initial "pipe not found" probes).
                    if let Ok(mut diag) = self.diagnostics.lock() {
                        diag.last_error = None;
                        diag.sidecar_connected_at = Some(Instant::now());
                        diag.no_camera_streak = 0;
                        diag.no_camera_since = None;
                    }

                    // Emit connection event
                    let _ = self.app_handle.emit("boothy-camera-connected", ());

                    return Ok(());
                }
                Err(e) => {
                    last_error = format!("{}", e);
                    if i < max_retries - 1 {
                        tokio::time::sleep(retry_delay).await;
                    }
                }
            }
        }

        let message = format!(
            "Failed to connect to Named Pipe after {} retries: {}",
            max_retries, last_error
        );
        if record_error {
            self.set_ipc_state(IpcConnectionState::Disconnected);
            self.record_last_error(message.clone());
        }
        Err(message)
    }

    /// Start listening for events from the sidecar
    fn start_event_listener(&self) {
        let app_handle = self.app_handle.clone();
        let connected = Arc::clone(&self.connected);
        let pending_requests = Arc::clone(&self.pending_requests);
        let diagnostics = Arc::clone(&self.diagnostics);
        let pipe = {
            let guard = self.rx_pipe.lock().unwrap();
            guard.as_ref().and_then(|pipe| pipe.try_clone().ok())
        };

        let Some(pipe) = pipe else {
            warn!("Failed to start event listener: pipe not connected");
            return;
        };

        tokio::task::spawn_blocking(move || {
            let correlation_id = generate_correlation_id();
            info!("[{}] Starting sidecar event listener...", correlation_id);
            let reader = BufReader::new(pipe);
            for line in reader.lines() {
                if !*connected.lock().unwrap() {
                    info!("[{}] Event listener stopped (disconnected)", correlation_id);
                    return;
                }

                match line {
                    Ok(line_str) => match serde_json::from_str::<IpcMessage>(&line_str) {
                        Ok(message) => {
                            debug!(
                                "[{}] Received IPC message: {}",
                                message.correlation_id, message.method
                            );
                            handle_incoming_message(
                                &app_handle,
                                message,
                                &pending_requests,
                                &diagnostics,
                            );
                        }
                        Err(e) => {
                            warn!("[{}] Failed to parse IPC message: {}", correlation_id, e);
                        }
                    },
                    Err(e) => {
                        warn!("[{}] Pipe read error: {}", correlation_id, e);
                        break;
                    }
                }
            }

            {
                let mut pending = pending_requests.lock().unwrap();
                for (_, sender) in pending.drain() {
                    let _ = sender.send(Err(IpcError {
                        code: IpcErrorCode::Disconnect,
                        message: "Sidecar disconnected".to_string(),
                        context: None,
                    }));
                }
            }

            let mut guard = connected.lock().unwrap();
            if *guard {
                *guard = false;
                set_diagnostics_state(&diagnostics, IpcConnectionState::Disconnected);
                let error = error::ipc::disconnect();
                if let Ok(mut diag) = diagnostics.lock() {
                    diag.last_error = Some(error.message.clone());
                }
                emit_camera_error(&app_handle, error, &correlation_id);
            }
        });
    }

    fn start_status_monitor(&self) {
        if self.status_monitor_started.swap(true, Ordering::SeqCst) {
            return;
        }

        let start_correlation_id = generate_correlation_id();
        info!(
            "[{}] Starting camera status monitor (poll=5s)",
            start_correlation_id
        );

        let client = self.clone();
        let app_handle = self.app_handle.clone();

        tauri::async_runtime::spawn(async move {
            let mut last_observed: Option<(bool, bool)> = None; // (connected, camera_detected)
            let mut last_poll_had_error = false;
            let mut error_backoff = STATUS_MONITOR_INTERVAL;

            // Avoid immediately racing the frontend's initial getStatus during dev reload/StrictMode.
            tokio::time::sleep(Duration::from_secs(STATUS_MONITOR_INITIAL_DELAY.as_secs())).await;

            loop {
                if !client.is_connected() {
                    client.status_monitor_started.store(false, Ordering::SeqCst);
                    let correlation_id = generate_correlation_id();
                    info!("[{}] Camera status monitor stopped (disconnected)", correlation_id);
                    return;
                }

                let correlation_id = generate_correlation_id();
                let result = client
                    .send_request_with_options(
                        "camera.getStatus".to_string(),
                        serde_json::json!({}),
                        correlation_id.clone(),
                        Duration::from_secs(STATUS_MONITOR_TIMEOUT.as_secs()),
                        false,
                    )
                    .await;

                if let Ok(payload) = result {
                    last_poll_had_error = false;
                    error_backoff = STATUS_MONITOR_INTERVAL;
                    if let Ok(status) =
                        serde_json::from_value::<CameraStatusResponse>(payload.clone())
                    {
                        let current = (status.connected, status.camera_detected);

                        // Keep the auto-restart heuristics warm even when the frontend isn't polling.
                        let _ = client.note_camera_status(&status);

                        if last_observed.map_or(true, |prev| prev != current) {
                            last_observed = Some(current);

                            // Frontend listeners already treat this as a "refresh now" hint.
                            // Payload is currently ignored by the UI, but include minimal context for future debugging.
                            let _ = app_handle.emit(
                                "boothy-camera-status-hint",
                                serde_json::json!({
                                    "source": "backendPoll",
                                    "correlationId": correlation_id,
                                    "connected": status.connected,
                                    "cameraDetected": status.camera_detected,
                                }),
                            );
                        }
                    } else {
                        debug!(
                            "[{}] Status monitor: invalid camera.getStatus payload: {}",
                            correlation_id, payload
                        );
                    }
                } else if !last_poll_had_error {
                    // Avoid spamming the UI: emit at most once until we observe a successful poll again.
                    last_poll_had_error = true;
                    error_backoff =
                        std::cmp::min(error_backoff.saturating_mul(2u32), STATUS_MONITOR_ERROR_BACKOFF_MAX);
                    let _ = app_handle.emit(
                        "boothy-camera-status-hint",
                        serde_json::json!({
                            "source": "backendPollError",
                            "correlationId": correlation_id,
                        }),
                    );
                }

                tokio::time::sleep(Duration::from_secs(error_backoff.as_secs())).await;
            }
        });
    }

    /// Send a request to the sidecar and wait for response
    pub async fn send_request(
        &self,
        method: String,
        payload: serde_json::Value,
        correlation_id: String,
    ) -> Result<serde_json::Value, String> {
        self.send_request_with_options(
            method,
            payload,
            correlation_id,
            Duration::from_millis(IPC_TIMEOUT_MS),
            true,
        )
        .await
    }

    pub async fn send_request_with_timeout(
        &self,
        method: String,
        payload: serde_json::Value,
        correlation_id: String,
        timeout: Duration,
    ) -> Result<serde_json::Value, String> {
        self.send_request_with_options(method, payload, correlation_id, timeout, true)
            .await
    }

    pub async fn send_request_with_options(
        &self,
        method: String,
        payload: serde_json::Value,
        correlation_id: String,
        timeout: Duration,
        emit_errors: bool,
    ) -> Result<serde_json::Value, String> {
        if !self.is_connected() {
            self.record_last_error("Sidecar not connected".to_string());
            return Err("Sidecar not connected".to_string());
        }

        let request_id = generate_request_id();
        self.record_last_request(&request_id, &correlation_id);
        let message =
            IpcMessage::new_request(method, correlation_id.clone(), request_id.clone(), payload);

        // Serialize message
        let json = serde_json::to_string(&message)
            .map_err(|e| format!("Failed to serialize request: {}", e))?;

        let (tx, rx) = oneshot::channel();
        {
            let mut pending = self.pending_requests.lock().unwrap();
            pending.insert(request_id.clone(), tx);
        }

        debug!("[{}] Preparing to send request: {}", correlation_id, message.method);

        let sender = self
            .tx_writer
            .lock()
            .ok()
            .and_then(|guard| guard.as_ref().cloned());
        let Some(sender) = sender else {
            self.pending_requests.lock().unwrap().remove(&request_id);
            self.record_last_error("Pipe not available".to_string());
            return Err("Pipe not available".to_string());
        };

        let mut bytes = json.into_bytes();
        bytes.push(b'\n');

        let (ack, ack_rx) = oneshot::channel::<Result<(), String>>();
        if sender.send(PipeWriteRequest { bytes, ack }).is_err() {
            self.pending_requests.lock().unwrap().remove(&request_id);
            let message = "Pipe writer is not available".to_string();
            self.record_last_error(message.clone());
            return Err(message);
        }

        match tokio::time::timeout(IPC_WRITE_TIMEOUT, ack_rx).await {
            Ok(Ok(Ok(()))) => {}
            Ok(Ok(Err(e))) => {
                self.pending_requests.lock().unwrap().remove(&request_id);
                self.record_last_error(e.clone());
                return Err(e);
            }
            Ok(Err(_)) => {
                self.pending_requests.lock().unwrap().remove(&request_id);
                let message = "Pipe writer ack channel closed".to_string();
                self.record_last_error(message.clone());
                return Err(message);
            }
            Err(_) => {
                self.pending_requests.lock().unwrap().remove(&request_id);
                let message = format!("IPC pipe write timeout during {}", message.method);
                self.record_last_error(message.clone());
                if let Ok(mut guard) = self.connected.lock() {
                    *guard = false;
                }
                self.set_ipc_state(IpcConnectionState::Disconnected);
                if let Ok(mut guard) = self.tx_writer.lock() {
                    *guard = None;
                }
                if let Ok(mut guard) = self.rx_pipe.lock() {
                    *guard = None;
                }
                warn!("[{}] {} - restarting sidecar", correlation_id, message);
                let client = self.clone();
                tokio::task::spawn_blocking(move || {
                    client.stop_sidecar_for_restart();
                });
                let client = self.clone();
                tauri::async_runtime::spawn(async move {
                    tokio::time::sleep(Duration::from_millis(400)).await;
                    let _ = client.start_sidecar().await;
                });
                return Err(message);
            }
        }

        debug!("[{}] Sent request: {}", correlation_id, message.method);

        match tokio::time::timeout(timeout, rx).await {
            Ok(Ok(Ok(payload))) => {
                if let Ok(mut diag) = self.diagnostics.lock() {
                    diag.last_error = None;
                }
                Ok(payload)
            }
            Ok(Ok(Err(ipc_error))) => {
                self.record_last_error(ipc_error.diagnostic_message());
                let boothy_error: BoothyError = ipc_error.into();
                if emit_errors {
                    emit_camera_error(&self.app_handle, boothy_error.clone(), &correlation_id);
                }
                Err(boothy_error.message)
            }
            Ok(Err(_)) => {
                self.record_last_error("IPC response channel closed".to_string());
                // Treat this as an unresponsive sidecar and force a restart on next request.
                let client = self.clone();
                tokio::task::spawn_blocking(move || {
                    client.stop_sidecar_for_restart();
                });
                let client = self.clone();
                tauri::async_runtime::spawn(async move {
                    tokio::time::sleep(Duration::from_millis(400)).await;
                    let _ = client.start_sidecar().await;
                });
                Err("IPC response channel closed".to_string())
            }
            Err(_) => {
                self.pending_requests.lock().unwrap().remove(&request_id);
                let timeout_error = error::ipc::timeout(&message.method);
                self.record_last_error(timeout_error.message.clone());
                if emit_errors {
                    emit_camera_error(&self.app_handle, timeout_error.clone(), &correlation_id);
                }
                // If the sidecar doesn't respond within the deadline, it is often stuck in an EDSDK call.
                // Force a restart so UI polling can recover without a full app restart.
                warn!(
                    "[{}] IPC timeout during {} - restarting sidecar",
                    correlation_id, message.method
                );
                let client = self.clone();
                tokio::task::spawn_blocking(move || {
                    client.stop_sidecar_for_restart();
                });
                let client = self.clone();
                tauri::async_runtime::spawn(async move {
                    tokio::time::sleep(Duration::from_millis(400)).await;
                    let _ = client.start_sidecar().await;
                });
                Err(timeout_error.message)
            }
        }
    }

    /// Set session destination (convenience method)
    pub async fn set_session_destination(
        &self,
        destination_path: PathBuf,
        session_name: String,
        correlation_id: String,
    ) -> Result<(), String> {
        let payload = serde_json::to_value(SetSessionDestinationRequest {
            destination_path,
            session_name,
        })
        .map_err(|e| format!("Serialization error: {}", e))?;

        self.send_request(
            "camera.setSessionDestination".to_string(),
            payload,
            correlation_id,
        )
        .await?;

        Ok(())
    }

    pub fn diagnostics_snapshot(&self) -> CameraDiagnosticsSnapshot {
        let diagnostics = self.diagnostics.lock().unwrap().clone();
        CameraDiagnosticsSnapshot {
            ipc_state: diagnostics.ipc_state.as_str().to_string(),
            last_error: diagnostics.last_error,
            protocol_version: IPC_PROTOCOL_VERSION.to_string(),
            request_id: diagnostics.last_request_id,
            correlation_id: diagnostics.last_correlation_id,
        }
    }

    fn record_last_request(&self, request_id: &str, correlation_id: &str) {
        if let Ok(mut diag) = self.diagnostics.lock() {
            diag.last_request_id = Some(request_id.to_string());
            diag.last_correlation_id = Some(correlation_id.to_string());
        }
    }

    fn record_last_error(&self, error: String) {
        if let Ok(mut diag) = self.diagnostics.lock() {
            diag.last_error = Some(error);
        }
    }

    fn set_ipc_state(&self, state: IpcConnectionState) {
        set_diagnostics_state(&self.diagnostics, state);
    }
}

fn find_dev_sidecar(configuration: &str) -> Option<PathBuf> {
    let mut dir = std::env::current_exe().ok()?;
    dir.pop();

    for _ in 0..8 {
        // Prefer a self-contained x86 publish output when present. This avoids requiring a separate
        // x86 .NET runtime on the machine while still allowing Canon EDSDK (x86) interop.
        let candidate = dir
            .join("apps")
            .join("camera-sidecar")
            .join("bin")
            .join(configuration)
            .join("net8.0")
            .join("win-x86")
            .join("publish")
            .join("Boothy.CameraSidecar.exe");
        if candidate.exists() {
            return Some(candidate);
        }

        let candidate = dir
            .join("apps")
            .join("camera-sidecar")
            .join("bin")
            .join(configuration)
            .join("net8.0")
            .join("Boothy.CameraSidecar.exe");
        if candidate.exists() {
            return Some(candidate);
        }
        if !dir.pop() {
            break;
        }
    }

    None
}

fn resolve_sidecar_mode() -> Option<String> {
    let mode = std::env::var("BOOTHY_CAMERA_MODE").ok()?;
    let normalized = mode.trim().to_lowercase();
    if normalized == "mock" || normalized == "real" {
        Some(normalized)
    } else {
        None
    }
}

fn emit_camera_error<R: tauri::Runtime>(
    app_handle: &AppHandle<R>,
    error: BoothyError,
    correlation_id: &str,
) {
    let _ = app_handle.emit("boothy-camera-error", error.to_ui_payload(correlation_id));
}

/// Handle events received from the sidecar
fn handle_sidecar_event<R: tauri::Runtime>(
    app_handle: &AppHandle<R>,
    message: IpcMessage,
    diagnostics: &Arc<Mutex<CameraDiagnosticsInternal>>,
) {
    if message.message_type != IpcMessageType::Event {
        return;
    }

    match message.method.as_str() {
        "event.camera.photoTransferred" => {
            // Parse payload
            if let Some(payload) = message.payload {
                match serde_json::from_value::<PhotoTransferredPayload>(payload) {
                    Ok(photo) => {
                        info!(
                            "[{}] Photo transferred: {} ({} bytes)",
                            message.correlation_id, photo.original_filename, photo.file_size
                        );

                        // Emit to UI (will trigger file stability check + ingest)
                        let _ = app_handle.emit(
                            "boothy-photo-transferred",
                            serde_json::json!({
                                "path": photo.path,
                                "filename": photo.original_filename,
                                "fileSize": photo.file_size,
                                "transferredAt": photo.transferred_at,
                                "correlationId": message.correlation_id,
                            }),
                        );
                    }
                    Err(e) => {
                        warn!(
                            "[{}] Failed to parse photoTransferred payload: {}",
                            message.correlation_id, e
                        );
                    }
                }
            }
        }

        "event.camera.captureStarted" => {
            info!("[{}] Capture started", message.correlation_id);
            let _ = app_handle.emit("boothy-capture-started", ());
        }

        "event.camera.error" => {
            if let Some(payload) = message.payload {
                if let Ok(error_payload) = serde_json::from_value::<CameraErrorPayload>(payload) {
                    error!(
                        "[{}] Camera error: {:?} - {}",
                        message.correlation_id,
                        error_payload.error.code,
                        error_payload.error.message
                    );

                    if let Ok(mut diag) = diagnostics.lock() {
                        diag.last_error = Some(error_payload.error.diagnostic_message());
                    }

                    let boothy_error: BoothyError = error_payload.error.into();
                    emit_camera_error(app_handle, boothy_error, &message.correlation_id);
                }
            }
        }
        "event.camera.statusHint" => {
            // Hint from sidecar that camera availability likely changed (power-cycle/hotplug).
            // The UI can use this to refresh camera.getStatus immediately instead of waiting
            // for polling intervals.
            debug!(
                "[{}] Camera status hint received",
                message.correlation_id
            );
            let _ = app_handle.emit("boothy-camera-status-hint", message.payload);
        }
        "event.camera.statusChanged" => {
            // Snapshot from sidecar representing the latest observed camera state.
            // This is the source of truth for the UI lamp (push-first).
            debug!("[{}] Camera status snapshot received", message.correlation_id);
            let _ = app_handle.emit("boothy-camera-status", message.payload);
        }

        _ => {
            warn!(
                "[{}] Unknown sidecar event: {}",
                message.correlation_id, message.method
            );
        }
    }
}

fn handle_incoming_message<R: tauri::Runtime>(
    app_handle: &AppHandle<R>,
    message: IpcMessage,
    pending_requests: &Arc<Mutex<HashMap<String, oneshot::Sender<Result<serde_json::Value, IpcError>>>>>,
    diagnostics: &Arc<Mutex<CameraDiagnosticsInternal>>,
) {
    match message.message_type {
        IpcMessageType::Event => {
            handle_sidecar_event(app_handle, message, diagnostics);
        }
        IpcMessageType::Response => {
            if let Some(request_id) = &message.request_id {
                let sender = pending_requests.lock().unwrap().remove(request_id);
                if let Some(sender) = sender {
                    let payload = message.payload.unwrap_or_else(|| serde_json::json!({}));
                    let _ = sender.send(Ok(payload));
                } else {
                    warn!(
                        "[{}] No pending request for response {}",
                        message.correlation_id, request_id
                    );
                }
            } else {
                warn!(
                    "[{}] Response missing requestId for method {}",
                    message.correlation_id, message.method
                );
            }
        }
        IpcMessageType::Error => {
            if let Some(request_id) = &message.request_id {
                let sender = pending_requests.lock().unwrap().remove(request_id);
                if let Some(sender) = sender {
                    let error = message.error.unwrap_or(IpcError {
                        code: IpcErrorCode::Unknown,
                        message: "Unknown IPC error".to_string(),
                        context: None,
                    });
                    if let Ok(mut diag) = diagnostics.lock() {
                        diag.last_error = Some(error.diagnostic_message());
                    }
                    let _ = sender.send(Err(error));
                } else {
                    warn!(
                        "[{}] No pending request for error {}",
                        message.correlation_id, request_id
                    );
                }
            } else if let Some(error) = message.error {
                if let Ok(mut diag) = diagnostics.lock() {
                    diag.last_error = Some(error.diagnostic_message());
                }
                let boothy_error: BoothyError = error.into();
                emit_camera_error(app_handle, boothy_error, &message.correlation_id);
            }
        }
        IpcMessageType::Request => {
            warn!(
                "[{}] Unexpected request received from sidecar: {}",
                message.correlation_id, message.method
            );
        }
    }
}

fn set_diagnostics_state(
    diagnostics: &Arc<Mutex<CameraDiagnosticsInternal>>,
    state: IpcConnectionState,
) {
    if let Ok(mut diag) = diagnostics.lock() {
        diag.ipc_state = state;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use std::path::PathBuf;
    use std::sync::mpsc;
    use std::time::Duration;
    use tauri::Listener;

    #[test]
    fn emits_photo_transferred_event_for_ui() {
        let app = tauri::test::mock_app();
        let app_handle = app.handle().clone();
        let (tx, rx) = mpsc::channel();
        let diagnostics = Arc::new(Mutex::new(CameraDiagnosticsInternal::default()));

        app.listen_any("boothy-photo-transferred", move |event: tauri::Event| {
            let _ = tx.send(event.payload().to_string());
        });

        let payload = PhotoTransferredPayload {
            path: PathBuf::from("C:\\shots\\IMG_0001.CR3"),
            transferred_at: Utc::now(),
            original_filename: "IMG_0001.CR3".to_string(),
            file_size: 2048,
        };

        let message = IpcMessage::new_event(
            "event.camera.photoTransferred".to_string(),
            "corr-123".to_string(),
            serde_json::to_value(payload).unwrap(),
        );

        handle_sidecar_event(&app_handle, message, &diagnostics);

        let payload_str = rx
            .recv_timeout(Duration::from_secs(1))
            .expect("Expected boothy-photo-transferred event");
        let value: serde_json::Value = serde_json::from_str(&payload_str).unwrap();

        assert_eq!(value["path"], "C:\\shots\\IMG_0001.CR3");
        assert_eq!(value["filename"], "IMG_0001.CR3");
        assert_eq!(value["fileSize"], 2048);
        assert_eq!(value["correlationId"], "corr-123");
    }

    #[test]
    fn emits_customer_safe_camera_error() {
        let app = tauri::test::mock_app();
        let app_handle = app.handle().clone();
        let (tx, rx) = mpsc::channel();
        let diagnostics = Arc::new(Mutex::new(CameraDiagnosticsInternal::default()));

        app.listen_any("boothy-camera-error", move |event: tauri::Event| {
            let _ = tx.send(event.payload().to_string());
        });

        let payload = CameraErrorPayload {
            error: IpcError {
                code: IpcErrorCode::CameraNotConnected,
                message: "Camera missing".to_string(),
                context: None,
            },
        };

        let message = IpcMessage::new_event(
            "event.camera.error".to_string(),
            "corr-err".to_string(),
            serde_json::to_value(payload).unwrap(),
        );

        handle_sidecar_event(&app_handle, message, &diagnostics);

        let payload_str = rx
            .recv_timeout(Duration::from_secs(1))
            .expect("Expected boothy-camera-error event");
        let value: serde_json::Value = serde_json::from_str(&payload_str).unwrap();

        assert_eq!(value["code"], "CAMERA_NOT_CONNECTED");
        assert_eq!(
            value["message"],
            "Camera is not connected. Please check the camera connection and try again."
        );
        assert_eq!(value["diagnostic"], "[CameraNotConnected] Camera missing");
        assert_eq!(value["correlationId"], "corr-err");
    }

    #[test]
    fn routes_response_to_pending_request() {
        let app = tauri::test::mock_app();
        let app_handle = app.handle().clone();
        let pending_requests: Arc<
            Mutex<HashMap<String, oneshot::Sender<Result<serde_json::Value, IpcError>>>>,
        > = Arc::new(Mutex::new(HashMap::new()));
        let diagnostics = Arc::new(Mutex::new(CameraDiagnosticsInternal::default()));

        let (tx, rx) = oneshot::channel();
        pending_requests
            .lock()
            .unwrap()
            .insert("req-123".to_string(), tx);

        let message = IpcMessage::new_response(
            "camera.getStatus".to_string(),
            "corr-123".to_string(),
            "req-123".to_string(),
            serde_json::json!({ "connected": true }),
        );

        handle_incoming_message(&app_handle, message, &pending_requests, &diagnostics);

        let received = tokio::runtime::Runtime::new().unwrap().block_on(rx).unwrap().unwrap();
        assert_eq!(received["connected"], true);
    }

    #[test]
    fn records_diagnostics_on_error_response() {
        let app = tauri::test::mock_app();
        let app_handle = app.handle().clone();
        let pending_requests: Arc<
            Mutex<HashMap<String, oneshot::Sender<Result<serde_json::Value, IpcError>>>>,
        > = Arc::new(Mutex::new(HashMap::new()));
        let diagnostics = Arc::new(Mutex::new(CameraDiagnosticsInternal::default()));

        let (tx, rx) = oneshot::channel();
        pending_requests
            .lock()
            .unwrap()
            .insert("req-err".to_string(), tx);

        let message = IpcMessage::new_error(
            "camera.getStatus".to_string(),
            "corr-err".to_string(),
            Some("req-err".to_string()),
            IpcError {
                code: IpcErrorCode::CameraNotConnected,
                message: "Camera missing".to_string(),
                context: None,
            },
        );

        handle_incoming_message(&app_handle, message, &pending_requests, &diagnostics);
        let _ = tokio::runtime::Runtime::new().unwrap().block_on(rx);

        let snapshot = diagnostics.lock().unwrap();
        assert!(snapshot
            .last_error
            .as_deref()
            .unwrap_or_default()
            .contains("CameraNotConnected"));
    }

    #[test]
    fn note_camera_status_restarts_after_lost_detected() {
        let mut diag = CameraDiagnosticsInternal::default();
        diag.ipc_state = IpcConnectionState::Connected;
        let base = Instant::now();

        let detected = CameraStatusResponse {
            connected: true,
            camera_detected: true,
            session_destination: None,
            camera_model: Some("EOS".to_string()),
        };
        assert_eq!(
            note_camera_status_internal(&mut diag, &detected, base),
            CameraAutoRestartDecision {
                should_restart: false,
                reason: None
            }
        );

        let not_detected = CameraStatusResponse {
            connected: true,
            camera_detected: false,
            session_destination: None,
            camera_model: None,
        };
        assert_eq!(
            note_camera_status_internal(&mut diag, &not_detected, base + Duration::from_secs(5)),
            CameraAutoRestartDecision {
                should_restart: false,
                reason: None
            }
        );
        assert_eq!(
            note_camera_status_internal(&mut diag, &not_detected, base + Duration::from_secs(10)),
            CameraAutoRestartDecision {
                should_restart: false,
                reason: None
            }
        );
        assert_eq!(
            note_camera_status_internal(&mut diag, &not_detected, base + Duration::from_secs(15)),
            CameraAutoRestartDecision {
                should_restart: false,
                reason: None
            }
        );
        assert_eq!(
            note_camera_status_internal(&mut diag, &not_detected, base + Duration::from_secs(25)),
            CameraAutoRestartDecision {
                should_restart: true,
                reason: Some(CameraAutoRestartReason::LostAfterDetected)
            }
        );

        // Immediate follow-up is throttled.
        assert_eq!(
            note_camera_status_internal(&mut diag, &not_detected, base + Duration::from_secs(27)),
            CameraAutoRestartDecision {
                should_restart: false,
                reason: None
            }
        );
    }

    #[test]
    fn note_camera_status_restarts_after_prolonged_no_camera() {
        let mut diag = CameraDiagnosticsInternal::default();
        diag.ipc_state = IpcConnectionState::Connected;
        let base = Instant::now();

        let not_detected = CameraStatusResponse {
            connected: true,
            camera_detected: false,
            session_destination: None,
            camera_model: None,
        };

        for i in 0..7 {
            assert_eq!(
                note_camera_status_internal(&mut diag, &not_detected, base + Duration::from_secs(5 * i)),
                CameraAutoRestartDecision {
                    should_restart: false,
                    reason: None
                }
            );
        }

        assert_eq!(
            note_camera_status_internal(&mut diag, &not_detected, base + Duration::from_secs(45)),
            CameraAutoRestartDecision {
                should_restart: true,
                reason: Some(CameraAutoRestartReason::ProlongedNoCamera)
            }
        );
    }
}
