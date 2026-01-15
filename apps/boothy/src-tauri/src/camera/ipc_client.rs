use super::ipc_models::*;
use log::{debug, error, info, warn};
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};

const PIPE_NAME: &str = "\\\\.\\pipe\\boothy_camera_sidecar";
const IPC_TIMEOUT_MS: u64 = 5000;

/// Camera IPC Client State
/// Manages sidecar process lifecycle and Named Pipe communication
#[derive(Clone)]
pub struct CameraIpcClient {
    /// Sidecar process handle
    sidecar_process: Arc<Mutex<Option<Child>>>,

    /// Whether the sidecar is connected
    connected: Arc<Mutex<bool>>,

    /// App handle for emitting events
    app_handle: AppHandle,

    /// Channel for sending IPC messages
    tx_pipe: Arc<Mutex<Option<std::fs::File>>>,

    /// Prevent concurrent start attempts
    starting: Arc<AtomicBool>,
}

impl CameraIpcClient {
    /// Create a new Camera IPC Client
    pub fn new(app_handle: AppHandle) -> Self {
        Self {
            sidecar_process: Arc::new(Mutex::new(None)),
            connected: Arc::new(Mutex::new(false)),
            app_handle,
            tx_pipe: Arc::new(Mutex::new(None)),
            starting: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Check if the sidecar is connected
    pub fn is_connected(&self) -> bool {
        self.connected.lock().map(|g| *g).unwrap_or(false)
    }

    /// Start the camera sidecar process and establish IPC connection
    pub async fn start_sidecar(&self) -> Result<(), String> {
        let correlation_id = generate_correlation_id();
        info!("[{}] Starting camera sidecar...", correlation_id);

        if self.starting.swap(true, Ordering::SeqCst) {
            warn!("[{}] Sidecar start already in progress", correlation_id);
            return Ok(());
        }

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
            .connect_to_pipe_with_retries(2, Duration::from_millis(100))
            .await
            .is_ok()
        {
            self.start_event_listener();
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
            if self.connect_to_pipe_with_retries(3, Duration::from_millis(200)).await.is_ok() {
                self.start_event_listener();
                info!("[{}] Connected to existing sidecar", correlation_id);
                return Ok(());
            }

            warn!(
                "[{}] Existing sidecar unresponsive, restarting",
                correlation_id
            );
            self.stop_sidecar();
        }

        // Get sidecar executable path
        let sidecar_path = self.get_sidecar_path()?;
        info!(
            "[{}] Sidecar path: {}",
            correlation_id,
            sidecar_path.display()
        );

        // Start sidecar process
        let mut child = Command::new(&sidecar_path)
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
            return Err(err);
        }

        // Start event listener
        self.start_event_listener();

        info!("[{}] Sidecar started and connected", correlation_id);
        Ok(())
    }

    /// Stop the camera sidecar process
    pub fn stop_sidecar(&self) {
        let correlation_id = generate_correlation_id();
        info!("[{}] Stopping camera sidecar...", correlation_id);

        self.send_shutdown_signal(&correlation_id);

        // Mark as disconnected
        if let Ok(mut guard) = self.connected.lock() {
            *guard = false;
        }

        // Close pipe
        if let Ok(mut pipe_guard) = self.tx_pipe.lock() {
            *pipe_guard = None;
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
                warn!("[{}] Failed to serialize shutdown request: {}", correlation_id, e);
                return;
            }
        };

        let mut pipe_guard = match self.tx_pipe.lock() {
            Ok(guard) => guard,
            Err(_) => return,
        };

        if let Some(pipe) = pipe_guard.as_mut() {
            if pipe.write_all(json.as_bytes()).is_err()
                || pipe.write_all(b"\n").is_err()
                || pipe.flush().is_err()
            {
                warn!("[{}] Failed to send shutdown request", correlation_id);
            }
        }
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

            if let Some(path) = find_dev_sidecar("Debug") {
                return Ok(path);
            }
            if let Some(path) = find_dev_sidecar("Release") {
                return Ok(path);
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

        // Production: bundled with app (TODO: configure Tauri bundler)
        let sidecar_path = self
            .app_handle
            .path()
            .resource_dir()
            .map_err(|e| format!("Failed to get resource dir: {}", e))?
            .join("sidecar")
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
        self.connect_to_pipe_with_retries(10, Duration::from_millis(200))
            .await
    }

    async fn connect_to_pipe_with_retries(
        &self,
        max_retries: usize,
        retry_delay: Duration,
    ) -> Result<(), String> {
        use std::fs::OpenOptions;
        use std::os::windows::fs::OpenOptionsExt;

        let correlation_id = generate_correlation_id();
        info!(
            "[{}] Connecting to Named Pipe: {}",
            correlation_id, PIPE_NAME
        );

        // Retry connection with timeout
        let mut last_error = String::new();

        for i in 0..max_retries {
            match OpenOptions::new()
                .read(true)
                .write(true)
                .custom_flags(0x40000000) // FILE_FLAG_OVERLAPPED for async
                .open(PIPE_NAME)
            {
                Ok(pipe) => {
                    info!("[{}] Connected to Named Pipe", correlation_id);

                    // Store pipe for sending
                    if let Ok(mut pipe_guard) = self.tx_pipe.lock() {
                        *pipe_guard = Some(pipe);
                    }

                    // Mark as connected
                    if let Ok(mut guard) = self.connected.lock() {
                        *guard = true;
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

        Err(format!(
            "Failed to connect to Named Pipe after {} retries: {}",
            max_retries, last_error
        ))
    }

    /// Start listening for events from the sidecar
    fn start_event_listener(&self) {
        let app_handle = self.app_handle.clone();
        let connected = Arc::clone(&self.connected);
        let pipe = {
            let guard = self.tx_pipe.lock().unwrap();
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
                    Ok(line_str) => {
                        match serde_json::from_str::<IpcMessage>(&line_str) {
                            Ok(message) => {
                                debug!(
                                    "[{}] Received IPC message: {}",
                                    message.correlation_id, message.method
                                );
                                handle_sidecar_event(&app_handle, message);
                            }
                            Err(e) => {
                                warn!(
                                    "[{}] Failed to parse IPC message: {}",
                                    correlation_id, e
                                );
                            }
                        }
                    }
                    Err(e) => {
                        warn!("[{}] Pipe read error: {}", correlation_id, e);
                        break;
                    }
                }
            }

            let mut guard = connected.lock().unwrap();
            if *guard {
                *guard = false;
                let _ = app_handle.emit("boothy-camera-error", "Camera connection lost.");
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
        if !self.is_connected() {
            return Err("Sidecar not connected".to_string());
        }

        let request_id = generate_request_id();
        let message =
            IpcMessage::new_request(method, correlation_id.clone(), request_id.clone(), payload);

        // Serialize message
        let json = serde_json::to_string(&message)
            .map_err(|e| format!("Failed to serialize request: {}", e))?;

        // Send via Named Pipe
        {
            let mut pipe_guard = self.tx_pipe.lock().unwrap();
            if let Some(pipe) = pipe_guard.as_mut() {
                pipe.write_all(json.as_bytes())
                    .map_err(|e| format!("Failed to write to pipe: {}", e))?;
                pipe.write_all(b"\n")
                    .map_err(|e| format!("Failed to write newline: {}", e))?;
                pipe.flush()
                    .map_err(|e| format!("Failed to flush pipe: {}", e))?;
            } else {
                return Err("Pipe not available".to_string());
            }
        }

        debug!("[{}] Sent request: {}", correlation_id, message.method);

        // TODO: Wait for response (requires response channel implementation)
        // For MVP, we'll use a simplified approach
        tokio::time::sleep(Duration::from_millis(100)).await;
        Ok(serde_json::json!({}))
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
}

fn find_dev_sidecar(configuration: &str) -> Option<PathBuf> {
    let mut dir = std::env::current_exe().ok()?;
    dir.pop();

    for _ in 0..8 {
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

/// Handle events received from the sidecar
fn handle_sidecar_event<R: tauri::Runtime>(app_handle: &AppHandle<R>, message: IpcMessage) {
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

                    let _ = app_handle.emit(
                        "boothy-camera-error",
                        serde_json::json!({
                            "code": format!("{:?}", error_payload.error.code),
                            "message": error_payload.error.customer_safe_message(),
                            "correlationId": message.correlation_id,
                        }),
                    );
                }
            }
        }

        _ => {
            warn!(
                "[{}] Unknown sidecar event: {}",
                message.correlation_id, message.method
            );
        }
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

        handle_sidecar_event(&app_handle, message);

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

        handle_sidecar_event(&app_handle, message);

        let payload_str = rx
            .recv_timeout(Duration::from_secs(1))
            .expect("Expected boothy-camera-error event");
        let value: serde_json::Value = serde_json::from_str(&payload_str).unwrap();

        assert_eq!(value["code"], "CameraNotConnected");
        assert_eq!(
            value["message"],
            "Camera is not connected. Please check the camera connection and try again."
        );
        assert_eq!(value["correlationId"], "corr-err");
    }
}
