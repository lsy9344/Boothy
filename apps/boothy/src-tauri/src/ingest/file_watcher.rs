use super::stabilizer::{StabilizationConfig, StabilizationResult, wait_for_file_stability};
use crate::AppState;
use log::{error, info, warn};
use std::path::PathBuf;
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Manager, Runtime};
use tokio::sync::Mutex;

/// File arrival watcher
/// Listens for `boothy-photo-transferred` events and triggers stability checks
pub struct FileArrivalWatcherInner<R: Runtime> {
    app_handle: AppHandle<R>,
    stabilization_config: StabilizationConfig,
    pending_imports: Arc<Mutex<Vec<PathBuf>>>,
}

pub type FileArrivalWatcher = FileArrivalWatcherInner<tauri::Wry>;

impl<R: Runtime> Clone for FileArrivalWatcherInner<R> {
    fn clone(&self) -> Self {
        Self {
            app_handle: self.app_handle.clone(),
            stabilization_config: self.stabilization_config.clone(),
            pending_imports: Arc::clone(&self.pending_imports),
        }
    }
}

impl<R: Runtime> FileArrivalWatcherInner<R> {
    pub fn new(app_handle: AppHandle<R>) -> Self {
        Self {
            app_handle,
            stabilization_config: StabilizationConfig::default(),
            pending_imports: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Handle a photo transferred event from the camera sidecar
    /// This initiates the file stability check and import process
    pub async fn handle_photo_transferred(
        &self,
        path: PathBuf,
        correlation_id: String,
    ) -> Result<(), String> {
        info!(
            "[{}] Photo transferred notification received: {}",
            correlation_id,
            path.display()
        );

        // Add to pending imports
        {
            let mut pending = self.pending_imports.lock().await;
            pending.push(path.clone());
        }

        // Spawn stability check in background
        let app_handle = self.app_handle.clone();
        let config = self.stabilization_config.clone();
        let pending_imports = Arc::clone(&self.pending_imports);

        tokio::spawn(async move {
            // Wait for file to stabilize
            let result = wait_for_file_stability(path.clone(), config, &correlation_id).await;

            match result {
                StabilizationResult::Stable { path, size } => {
                    info!(
                        "[{}] File stable, triggering import: {} ({} bytes)",
                        correlation_id,
                        path.display(),
                        size
                    );

                    let state = app_handle.state::<AppState>();
                    if let Err(err) = state
                        .preset_manager
                        .apply_preset_on_import(&path, &correlation_id)
                    {
                        error!(
                            "[{}] Failed to apply preset on import: {}",
                            correlation_id, err
                        );
                        let _ = app_handle.emit(
                            "boothy-import-error",
                            serde_json::json!({
                                "path": path,
                                "error": err,
                                "correlationId": correlation_id,
                            }),
                        );
                    }

                    // Remove from pending
                    {
                        let mut pending = pending_imports.lock().await;
                        pending.retain(|p| p != &path);
                    }

                    // Emit event to trigger library refresh and import
                    let _ = app_handle.emit(
                        "boothy-new-photo",
                        serde_json::json!({
                            "path": path,
                            "size": size,
                            "correlationId": correlation_id,
                        }),
                    );
                }

                StabilizationResult::Timeout { path } => {
                    error!(
                        "[{}] File stabilization timeout: {}",
                        correlation_id,
                        path.display()
                    );

                    // Remove from pending
                    {
                        let mut pending = pending_imports.lock().await;
                        pending.retain(|p| p != &path);
                    }

                    // Emit error event
                    let _ = app_handle.emit(
                        "boothy-import-error",
                        serde_json::json!({
                            "path": path,
                            "error": "File stabilization timeout",
                            "correlationId": correlation_id,
                        }),
                    );
                }

                StabilizationResult::NotFound { path } => {
                    warn!(
                        "[{}] File not found (may have been deleted): {}",
                        correlation_id,
                        path.display()
                    );

                    // Remove from pending
                    {
                        let mut pending = pending_imports.lock().await;
                        pending.retain(|p| p != &path);
                    }
                }

                StabilizationResult::Locked { path } => {
                    error!(
                        "[{}] File still locked after stabilization: {}",
                        correlation_id,
                        path.display()
                    );

                    // Remove from pending
                    {
                        let mut pending = pending_imports.lock().await;
                        pending.retain(|p| p != &path);
                    }

                    let _ = app_handle.emit(
                        "boothy-import-error",
                        serde_json::json!({
                            "path": path,
                            "error": "File is locked",
                            "correlationId": correlation_id,
                        }),
                    );
                }
            }
        });

        Ok(())
    }

    /// Get list of pending imports (for diagnostics)
    pub async fn get_pending_imports(&self) -> Vec<PathBuf> {
        self.pending_imports.lock().await.clone()
    }

    /// Configure stabilization parameters
    pub fn set_stabilization_config(&mut self, config: StabilizationConfig) {
        self.stabilization_config = config;
    }
}

/// Initialize the file arrival watcher
/// This sets up event listeners for photo transferred events
pub fn init_file_watcher(app_handle: AppHandle) -> Arc<FileArrivalWatcher> {
    let watcher = Arc::new(FileArrivalWatcherInner::new(app_handle.clone()));

    info!("File arrival watcher initialized");
    watcher
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{mode, session, watcher, AppState};
    use std::collections::HashMap;
    use std::sync::atomic::AtomicBool;
    use std::sync::{Arc, Mutex};
    use std::time::Duration;
    use tauri::{Listener, Manager};
    use tempfile::TempDir;

    fn build_test_app() -> tauri::App<tauri::test::MockRuntime> {
        tauri::test::mock_builder()
            .manage(AppState {
                original_image: Mutex::new(None),
                cached_preview: Mutex::new(None),
                gpu_context: Mutex::new(None),
                gpu_image_cache: Mutex::new(None),
                gpu_processor: Mutex::new(None),
                export_task_handle: Mutex::new(None),
                panorama_result: Arc::new(Mutex::new(None)),
                denoise_result: Arc::new(Mutex::new(None)),
                lut_cache: Mutex::new(HashMap::new()),
                initial_file_path: Mutex::new(None),
                thumbnail_cancellation_token: Arc::new(AtomicBool::new(false)),
                preview_worker_tx: Mutex::new(None),
                mask_cache: Mutex::new(HashMap::new()),
                session_manager: session::SessionManager::new(),
                mode_manager: mode::ModeManager::new(),
                file_watcher: watcher::FileWatcher::new(),
                camera_client: Mutex::new(None),
                file_arrival_watcher: Mutex::new(None),
                preset_manager: crate::preset::preset_manager::PresetManager::new(),
            })
            .build(tauri::test::mock_context(tauri::test::noop_assets()))
            .unwrap()
    }

    #[tokio::test]
    async fn handles_photo_transfer_and_applies_preset() {
        let app = build_test_app();
        let app_handle = app.handle().clone();
        let state = app_handle.state::<AppState>();

        state.preset_manager.set_current_preset(
            "preset-1".to_string(),
            Some("Warm".to_string()),
            serde_json::json!({ "exposure": 0.2 }),
            "corr-1",
        );

        let mut watcher = FileArrivalWatcherInner::new(app_handle.clone());
        watcher.set_stabilization_config(StabilizationConfig {
            poll_interval_ms: 10,
            stable_count_required: 1,
            max_wait_ms: 2000,
            min_age_ms: 0,
        });

        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        app.listen_any("boothy-new-photo", move |event: tauri::Event| {
            let _ = tx.send(event.payload().to_string());
        });

        let temp_dir = TempDir::new().unwrap();
        let image_path = temp_dir.path().join("shot.CR3");
        std::fs::write(&image_path, b"rawdata").unwrap();

        watcher
            .handle_photo_transferred(image_path.clone(), "corr-1".to_string())
            .await
            .unwrap();

        let payload_str = tokio::time::timeout(Duration::from_secs(2), rx.recv())
            .await
            .expect("Expected boothy-new-photo event")
            .expect("Expected boothy-new-photo event");
        let payload: serde_json::Value = serde_json::from_str(&payload_str).unwrap();

        assert_eq!(
            payload["path"].as_str().expect("payload.path should be string"),
            image_path.to_string_lossy().as_ref()
        );
        assert_eq!(
            payload["correlationId"]
                .as_str()
                .expect("payload.correlationId should be string"),
            "corr-1"
        );

        let rrdata_path = image_path.with_file_name("shot.CR3.rrdata");
        for _ in 0..50 {
            if rrdata_path.exists() {
                break;
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }

        assert!(rrdata_path.exists());

        let rrdata_contents = std::fs::read_to_string(&rrdata_path).unwrap();
        let rrdata_json: serde_json::Value = serde_json::from_str(&rrdata_contents).unwrap();

        assert_eq!(rrdata_json["adjustments"]["exposure"], serde_json::json!(0.2));
        assert_eq!(rrdata_json["adjustments"]["boothy"]["preset_id"], "preset-1");
        assert_eq!(rrdata_json["adjustments"]["boothy"]["preset_name"], "Warm");
    }
}
