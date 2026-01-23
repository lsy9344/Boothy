use super::metadata::{
    SessionExportError, is_background_export_completed, mark_background_export_failure,
    mark_background_export_success, record_background_export_attempt,
};
use crate::camera::generate_correlation_id;
use crate::error;
use crate::file_management::{load_settings_for_handle, parse_virtual_path, read_file_mapped};
use crate::formats::is_raw_file;
use crate::image_loader::load_and_composite;
use crate::image_processing::{ImageMetadata, get_or_init_gpu_context};
use crate::{AppState, ExportSettings, export_photo};
use chrono::Utc;
use serde_json::json;
use std::collections::HashSet;
use std::fs;
use std::future::Future;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Manager, Runtime};
use tokio::sync::{Notify, mpsc};

type ProcessorFuture = Pin<Box<dyn Future<Output = Result<(), String>> + Send>>;
type ProcessorFn<R> = Arc<
    dyn Fn(BackgroundExportJob, AppHandle<R>, Arc<AtomicBool>) -> ProcessorFuture + Send + Sync,
>;

#[derive(Clone, Debug)]
pub struct BackgroundExportJob {
    pub path: PathBuf,
    pub correlation_id: String,
    pub received_at: chrono::DateTime<chrono::Utc>,
}

struct QueueState {
    pending_keys: Mutex<HashSet<String>>,
    inflight: Mutex<Option<BackgroundExportJob>>,
    paused: AtomicBool,
    cancel_requested: Arc<AtomicBool>,
    resume_notify: Notify,
    idle_notify: Notify,
}

pub struct BackgroundExportQueue {
    sender: mpsc::Sender<BackgroundExportJob>,
    receiver: Mutex<Option<mpsc::Receiver<BackgroundExportJob>>>,
    state: Arc<QueueState>,
    started: AtomicBool,
}

impl BackgroundExportQueue {
    pub fn new() -> Self {
        let (sender, receiver) = mpsc::channel(64);
        Self {
            sender,
            receiver: Mutex::new(Some(receiver)),
            state: Arc::new(QueueState {
                pending_keys: Mutex::new(HashSet::new()),
                inflight: Mutex::new(None),
                paused: AtomicBool::new(false),
                cancel_requested: Arc::new(AtomicBool::new(false)),
                resume_notify: Notify::new(),
                idle_notify: Notify::new(),
            }),
            started: AtomicBool::new(false),
        }
    }

    pub fn start<R: Runtime>(&self, app_handle: AppHandle<R>) {
        let processor = default_processor::<R>();
        self.start_with_processor(app_handle, processor);
    }

    pub fn start_with_processor<R: Runtime>(
        &self,
        app_handle: AppHandle<R>,
        processor: ProcessorFn<R>,
    ) {
        if self.started.swap(true, Ordering::SeqCst) {
            return;
        }

        let receiver = self.receiver.lock().unwrap().take();
        let Some(mut receiver) = receiver else {
            return;
        };

        let state = Arc::clone(&self.state);
        tauri::async_runtime::spawn(async move {
            while let Some(job) = receiver.recv().await {
                while state.paused.load(Ordering::SeqCst) {
                    state.resume_notify.notified().await;
                }

                {
                    let mut inflight = state.inflight.lock().unwrap();
                    *inflight = Some(job.clone());
                }

                let result = (processor)(
                    job.clone(),
                    app_handle.clone(),
                    Arc::clone(&state.cancel_requested),
                )
                .await;

                if let Err(err) = result {
                    log::error!("Background export job failed: {}", err);
                }

                state.cancel_requested.store(false, Ordering::SeqCst);

                {
                    let mut inflight = state.inflight.lock().unwrap();
                    *inflight = None;
                }

                state.idle_notify.notify_waiters();

                let key = Self::dedupe_key(&job.path);
                let mut pending = state.pending_keys.lock().unwrap();
                pending.remove(&key);
            }
        });
    }

    pub async fn enqueue<R: Runtime>(
        &self,
        app_handle: &AppHandle<R>,
        path: PathBuf,
        correlation_id: String,
    ) -> Result<(), String> {
        self.start(app_handle.clone());

        let key = Self::dedupe_key(&path);
        {
            let mut pending = self.state.pending_keys.lock().unwrap();
            if pending.contains(&key) {
                log::debug!("Background export already queued: {}", path.display());
                return Ok(());
            }
            pending.insert(key);
        }

        self.sender
            .send(BackgroundExportJob {
                path,
                correlation_id,
                received_at: Utc::now(),
            })
            .await
            .map_err(|e| e.to_string())
    }

    pub fn pause(&self) {
        self.state.paused.store(true, Ordering::SeqCst);
    }

    pub fn resume(&self) {
        self.state.paused.store(false, Ordering::SeqCst);
        self.state.resume_notify.notify_waiters();
    }

    pub fn request_cancel(&self) {
        self.state.cancel_requested.store(true, Ordering::SeqCst);
    }

    pub async fn wait_for_idle(&self) {
        loop {
            let inflight_none = {
                let inflight = self.state.inflight.lock().unwrap();
                inflight.is_none()
            };
            let pending_empty = {
                let pending = self.state.pending_keys.lock().unwrap();
                pending.is_empty()
            };
            let paused = self.state.paused.load(Ordering::SeqCst);
            if inflight_none && (paused || pending_empty) {
                return;
            }
            self.state.idle_notify.notified().await;
        }
    }

    pub async fn pause_and_cancel(&self) {
        self.pause();
        self.request_cancel();
        self.wait_for_idle().await;
    }

    fn dedupe_key(path: &Path) -> String {
        path.to_string_lossy().to_string()
    }
}

fn default_processor<R: Runtime>() -> ProcessorFn<R> {
    Arc::new(|job, app_handle, cancel_flag| {
        Box::pin(process_background_export(job, app_handle, cancel_flag))
    })
}

async fn process_background_export<R: Runtime>(
    job: BackgroundExportJob,
    app_handle: AppHandle<R>,
    cancel_flag: Arc<AtomicBool>,
) -> Result<(), String> {
    let state = app_handle.state::<AppState>();
    let session = match state.session_manager.get_active_session() {
        Some(session) => session,
        None => {
            log::warn!("Background export skipped: no active session.");
            return Ok(());
        }
    };

    let correlation_id = if job.correlation_id.is_empty() {
        generate_correlation_id()
    } else {
        job.correlation_id.clone()
    };

    if !job.path.starts_with(&session.raw_path) {
        log::warn!(
            "[{}] Background export skipped: path outside active session: {}",
            correlation_id,
            job.path.display()
        );
        return Ok(());
    }

    if is_background_export_completed(&session, &job.path).unwrap_or(false) {
        log::info!(
            "[{}] Background export already completed for {}",
            correlation_id,
            job.path.display()
        );
        return Ok(());
    }

    if cancel_flag.load(Ordering::SeqCst) {
        return Ok(());
    }

    if let Err(err) = record_background_export_attempt(&session, &job.path) {
        log::warn!(
            "[{}] Failed to record export attempt: {}",
            correlation_id,
            err
        );
    }

    log::info!(
        "[{}] Background export start at {} for {}",
        correlation_id,
        Utc::now().to_rfc3339(),
        job.path.display()
    );

    let raw_path = job.path.clone();
    let raw_path_for_processing = raw_path.clone();
    let output_path = session
        .jpg_path
        .join(raw_path.file_name().unwrap_or_default())
        .with_extension("jpg");

    let export_settings = ExportSettings {
        jpeg_quality: 90,
        resize: None,
        keep_metadata: true,
        strip_gps: true,
        filename_template: None,
        watermark: None,
    };

    let app_handle_clone = app_handle.clone();
    let correlation_id_clone = correlation_id.clone();
    let cancel_for_blocking = Arc::clone(&cancel_flag);
    let output_path_clone = output_path.clone();

    let processing_result = tokio::task::spawn_blocking(move || {
        if cancel_for_blocking.load(Ordering::SeqCst) {
            return Err("BACKGROUND_EXPORT_CANCELLED".to_string());
        }

        let state = app_handle_clone.state::<AppState>();
        let context = get_or_init_gpu_context(&state)?;
        let settings = load_settings_for_handle(&app_handle_clone).unwrap_or_default();
        let highlight_compression = settings.raw_highlight_compression.unwrap_or(2.5);

        let raw_path_str = raw_path_for_processing.to_string_lossy().to_string();
        let (source_path, sidecar_path) = parse_virtual_path(&raw_path_str);
        let source_path_str = source_path.to_string_lossy().to_string();

        let metadata: ImageMetadata = if sidecar_path.exists() {
            let file_content = fs::read_to_string(sidecar_path)
                .map_err(|e| format!("Failed to read sidecar: {}", e))?;
            serde_json::from_str(&file_content).unwrap_or_default()
        } else {
            ImageMetadata::default()
        };
        let js_adjustments = metadata.adjustments;
        let is_raw = is_raw_file(&source_path_str);

        let base_image = match read_file_mapped(Path::new(&source_path_str)) {
            Ok(mmap) => load_and_composite(
                &mmap,
                &source_path_str,
                &js_adjustments,
                false,
                highlight_compression,
            )
            .map_err(|e| format!("Failed to load image from mmap: {}", e))?,
            Err(e) => {
                log::warn!(
                    "Failed to memory-map file '{}': {}. Falling back to standard read.",
                    source_path_str,
                    e
                );
                let bytes = fs::read(&source_path_str).map_err(|io_err| {
                    format!("Fallback read failed for {}: {}", source_path_str, io_err)
                })?;
                load_and_composite(
                    &bytes,
                    &source_path_str,
                    &js_adjustments,
                    false,
                    highlight_compression,
                )
                .map_err(|e| format!("Failed to load image from bytes: {}", e))?
            }
        };

        export_photo(
            &source_path_str,
            &output_path_clone,
            &base_image,
            &js_adjustments,
            &export_settings,
            &context,
            &state,
            is_raw,
            Some(cancel_for_blocking.as_ref()),
        )
    })
    .await
    .map_err(|e| e.to_string())?;

    match processing_result {
        Ok(()) => {
            if let Err(err) = mark_background_export_success(&session, &raw_path) {
                log::warn!(
                    "[{}] Failed to record export success for {}: {}",
                    correlation_id,
                    raw_path.display(),
                    err
                );
            }
            log::info!(
                "[{}] Background export complete at {} for {}",
                correlation_id,
                Utc::now().to_rfc3339(),
                raw_path.display()
            );
            Ok(())
        }
        Err(err) => {
            let session_error = if err == "BACKGROUND_EXPORT_CANCELLED" {
                SessionExportError::new(
                    "BACKGROUND_EXPORT_CANCELLED",
                    "Background export cancelled.",
                    json!({ "correlationId": correlation_id_clone }),
                )
            } else {
                SessionExportError::new(
                    error::export::EXPORT_FAILED,
                    error::export::failed(output_path.to_string_lossy().as_ref(), &err).message,
                    json!({
                        "destination": output_path.to_string_lossy(),
                        "detail": err,
                        "correlationId": correlation_id_clone,
                    }),
                )
            };

            if let Err(meta_err) =
                mark_background_export_failure(&session, &raw_path, session_error)
            {
                log::warn!(
                    "[{}] Failed to record export failure for {}: {}",
                    correlation_id,
                    raw_path.display(),
                    meta_err
                );
            }

            log::error!(
                "[{}] Background export failed at {} for {}",
                correlation_id,
                Utc::now().to_rfc3339(),
                raw_path.display()
            );
            Err(err)
        }
    }
}

/// Enqueue existing raw files that have preset snapshots (.rrdata) but haven't been background-exported yet.
/// Called at session open to catch up on files that arrived while the app was closed or were
/// manually copied into the Raw folder.
pub async fn enqueue_existing_raw_files_for_export<R: Runtime>(
    session: &super::models::BoothySession,
    queue: &BackgroundExportQueue,
    app_handle: &AppHandle<R>,
) {
    let raw_path = &session.raw_path;

    // List all files in Raw folder
    let entries = match fs::read_dir(raw_path) {
        Ok(entries) => entries,
        Err(err) => {
            log::warn!(
                "Failed to list files in Raw folder {:?}: {}",
                raw_path,
                err
            );
            return;
        }
    };

    let mut enqueued_count = 0;
    let mut skipped_count = 0;

    for entry in entries.flatten() {
        let path = entry.path();

        // Skip directories
        if path.is_dir() {
            continue;
        }

        // Only process RAW files
        let path_str = path.to_string_lossy().to_string();
        if !is_raw_file(&path_str) {
            continue;
        }

        // Check if .rrdata sidecar exists (preset was applied)
        let rrdata_path = PathBuf::from(format!("{}.rrdata", path_str));
        if !rrdata_path.exists() {
            skipped_count += 1;
            continue;
        }

        // Check if already background-exported
        if super::metadata::is_background_export_completed(session, &path).unwrap_or(false) {
            skipped_count += 1;
            continue;
        }

        // Enqueue for background export
        let correlation_id = generate_correlation_id();
        if let Err(err) = queue.enqueue(app_handle, path.clone(), correlation_id.clone()).await {
            log::warn!(
                "[{}] Failed to enqueue existing raw file for background export: {} - {}",
                correlation_id,
                path.display(),
                err
            );
        } else {
            enqueued_count += 1;
        }
    }

    if enqueued_count > 0 || skipped_count > 0 {
        log::info!(
            "Session open: enqueued {} existing raw files for background export, skipped {} (already exported or no preset)",
            enqueued_count,
            skipped_count
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{mode, session, storage_health, watcher};
    use std::collections::HashMap;
    use std::sync::atomic::AtomicBool as StdAtomicBool;
    use std::sync::{Arc, Mutex};
    use std::time::Duration;
    use tokio::time::sleep;

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
                thumbnail_cancellation_token: Arc::new(StdAtomicBool::new(false)),
                preview_worker_tx: Mutex::new(None),
                mask_cache: Mutex::new(HashMap::new()),
                session_manager: session::SessionManager::new(),
                session_timer: session::SessionTimer::new(),
                storage_health_monitor: storage_health::StorageHealthMonitor::new(),
                mode_manager: mode::ModeManager::new(),
                file_watcher: watcher::FileWatcher::new(),
                camera_client: Mutex::new(None),
                file_arrival_watcher: Mutex::new(None),
                preset_manager: crate::preset::preset_manager::PresetManager::new(),
                background_export_queue: Arc::new(BackgroundExportQueue::new()),
            })
            .build(tauri::test::mock_context(tauri::test::noop_assets()))
            .unwrap()
    }

    #[tokio::test]
    async fn queue_processes_one_job_at_a_time() {
        let app = build_test_app();
        let app_handle = app.handle().clone();
        let current = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let max = Arc::new(std::sync::atomic::AtomicUsize::new(0));

        let processor = {
            let current = Arc::clone(&current);
            let max = Arc::clone(&max);
            Arc::new(
                move |_job: BackgroundExportJob,
                      _app_handle: AppHandle<tauri::test::MockRuntime>,
                      _cancel_flag: Arc<AtomicBool>|
                      -> ProcessorFuture {
                    let current = Arc::clone(&current);
                    let max = Arc::clone(&max);
                    Box::pin(async move {
                        let in_flight = current.fetch_add(1, Ordering::SeqCst) + 1;
                        max.fetch_max(in_flight, Ordering::SeqCst);
                        sleep(Duration::from_millis(50)).await;
                        current.fetch_sub(1, Ordering::SeqCst);
                        Ok(())
                    })
                },
            )
        };

        let queue = BackgroundExportQueue::new();
        queue.start_with_processor(app_handle.clone(), processor);

        queue
            .enqueue(
                &app_handle,
                PathBuf::from("C:/tmp/shot1.CR3"),
                "corr-1".to_string(),
            )
            .await
            .unwrap();
        queue
            .enqueue(
                &app_handle,
                PathBuf::from("C:/tmp/shot2.CR3"),
                "corr-2".to_string(),
            )
            .await
            .unwrap();

        queue.wait_for_idle().await;
        assert_eq!(max.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn queue_dedupes_same_path() {
        let app = build_test_app();
        let app_handle = app.handle().clone();
        let processed = Arc::new(std::sync::atomic::AtomicUsize::new(0));

        let processor = {
            let processed = Arc::clone(&processed);
            Arc::new(
                move |_job: BackgroundExportJob,
                      _app_handle: AppHandle<tauri::test::MockRuntime>,
                      _cancel_flag: Arc<AtomicBool>|
                      -> ProcessorFuture {
                    let processed = Arc::clone(&processed);
                    Box::pin(async move {
                        processed.fetch_add(1, Ordering::SeqCst);
                        Ok(())
                    })
                },
            )
        };

        let queue = BackgroundExportQueue::new();
        queue.start_with_processor(app_handle.clone(), processor);

        let path = PathBuf::from("C:/tmp/shot1.CR3");
        queue
            .enqueue(&app_handle, path.clone(), "corr-1".to_string())
            .await
            .unwrap();
        queue
            .enqueue(&app_handle, path, "corr-1".to_string())
            .await
            .unwrap();

        queue.wait_for_idle().await;
        assert_eq!(processed.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn queue_cancel_signal_reaches_processor() {
        let app = build_test_app();
        let app_handle = app.handle().clone();
        let cancel_observed = Arc::new(StdAtomicBool::new(false));

        let processor = {
            let cancel_observed = Arc::clone(&cancel_observed);
            Arc::new(
                move |_job: BackgroundExportJob,
                      _app_handle: AppHandle<tauri::test::MockRuntime>,
                      cancel_flag: Arc<AtomicBool>|
                      -> ProcessorFuture {
                    let cancel_observed = Arc::clone(&cancel_observed);
                    Box::pin(async move {
                        while !cancel_flag.load(Ordering::SeqCst) {
                            sleep(Duration::from_millis(10)).await;
                        }
                        cancel_observed.store(true, Ordering::SeqCst);
                        Ok(())
                    })
                },
            )
        };

        let queue = BackgroundExportQueue::new();
        queue.start_with_processor(app_handle.clone(), processor);

        queue
            .enqueue(
                &app_handle,
                PathBuf::from("C:/tmp/shot3.CR3"),
                "corr-3".to_string(),
            )
            .await
            .unwrap();

        queue.request_cancel();
        queue.wait_for_idle().await;

        assert!(cancel_observed.load(Ordering::SeqCst));
    }
}
