use super::stabilizer::wait_for_stable;
use crate::AppState;
use crate::camera::generate_correlation_id;
use notify::{Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use serde_json::json;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::mpsc::channel;
use std::sync::{Arc, Mutex, RwLock};
use std::thread;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};

/// File watcher for the active session Raw/ folder
pub struct FileWatcher {
    watcher: Arc<Mutex<Option<RecommendedWatcher>>>,
    watched_path: Arc<Mutex<Option<PathBuf>>>,
    /// Tracks files currently being processed to prevent duplicate handling
    processing_files: Arc<RwLock<HashSet<PathBuf>>>,
}

impl FileWatcher {
    pub fn new() -> Self {
        Self {
            watcher: Arc::new(Mutex::new(None)),
            watched_path: Arc::new(Mutex::new(None)),
            processing_files: Arc::new(RwLock::new(HashSet::new())),
        }
    }

    /// Start watching a directory for new image files
    pub fn start_watching(&self, path: PathBuf, app_handle: AppHandle) -> Result<(), String> {
        // Check if already watching the same path - skip if so
        {
            let current_path = self.watched_path.lock().unwrap();
            if let Some(ref p) = *current_path {
                if p == &path {
                    log::debug!("Already watching path, skipping: {:?}", path);
                    return Ok(());
                }
            }
        }

        // Stop any existing watcher
        self.stop_watching();

        log::info!("Starting file watcher for: {:?}", path);

        let (tx, rx) = channel();

        let mut watcher = RecommendedWatcher::new(
            move |res: Result<Event, notify::Error>| {
                if let Ok(event) = res {
                    let _ = tx.send(event);
                }
            },
            Config::default(),
        )
        .map_err(|e| format!("Failed to create watcher: {}", e))?;

        watcher
            .watch(&path, RecursiveMode::NonRecursive)
            .map_err(|e| format!("Failed to watch directory: {}", e))?;

        // Store watcher and path
        *self.watcher.lock().unwrap() = Some(watcher);
        *self.watched_path.lock().unwrap() = Some(path.clone());

        // Spawn thread to handle events
        let app_handle_clone = app_handle.clone();
        let processing_files = Arc::clone(&self.processing_files);
        thread::spawn(move || {
            for event in rx {
                if let Err(e) = handle_file_event(event, &app_handle_clone, &processing_files) {
                    log::error!("Error handling file event: {}", e);
                }
            }
        });

        Ok(())
    }

    /// Stop watching the current directory
    pub fn stop_watching(&self) {
        let mut watcher = self.watcher.lock().unwrap();
        if watcher.is_some() {
            log::info!("Stopping file watcher");
            *watcher = None;
            *self.watched_path.lock().unwrap() = None;
            // Clear the processing files set
            if let Ok(mut processing) = self.processing_files.write() {
                processing.clear();
            }
        }
    }

    /// Get the currently watched path
    pub fn get_watched_path(&self) -> Option<PathBuf> {
        self.watched_path.lock().unwrap().clone()
    }
}

impl Default for FileWatcher {
    fn default() -> Self {
        Self::new()
    }
}

fn emit_session_file_change(app_handle: &AppHandle, path: &Path, kind: &str) {
    let path_str = path.to_string_lossy().to_string();
    if let Err(e) = app_handle.emit(
        "boothy-session-files-changed",
        json!({
            "path": path_str,
            "kind": kind,
        }),
    ) {
        log::error!("Failed to emit boothy-session-files-changed event: {}", e);
    }
}

/// Handle a file system event
fn handle_file_event(
    event: Event,
    app_handle: &AppHandle,
    processing_files: &Arc<RwLock<HashSet<PathBuf>>>,
) -> Result<(), String> {
    let Event { kind, paths, .. } = event;

    // On Windows, large files trigger Create before content is written, then Modify when done.
    // We need to handle both Create and all Modify events to ensure we catch the file.
    let (is_create, is_modify, is_remove, should_process) = match kind {
        EventKind::Create(_) => (true, false, false, true),
        // Handle ALL Modify events - Windows may send various ModifyKind variants
        EventKind::Modify(_) => (false, true, false, true),
        EventKind::Remove(_) => (false, false, true, false),
        _ => (false, false, false, false),
    };

    if !(is_create || is_modify || is_remove) {
        return Ok(());
    }

    for path in paths {
        if !is_image_file(&path) {
            continue;
        }

        if is_remove {
            // Remove from processing set if it was being tracked
            if let Ok(mut processing) = processing_files.write() {
                processing.remove(&path);
            }
            emit_session_file_change(app_handle, &path, "removed");
            continue;
        }

        if !should_process {
            continue;
        }

        // Check if this file is already being processed (prevent duplicates)
        {
            let processing = processing_files.read().map_err(|e| e.to_string())?;
            if processing.contains(&path) {
                log::debug!("File already being processed, skipping: {:?}", path);
                continue;
            }
        }

        // Small delay to allow Windows to finish creating the file entry
        // This helps with the race condition where Create event fires before file is fully accessible
        thread::sleep(Duration::from_millis(50));

        // Check if file exists and is a regular file
        if !path.is_file() {
            log::debug!(
                "Path is not a file yet, will catch on Modify event: {:?}",
                path
            );
            continue;
        }

        // Mark as being processed
        {
            let mut processing = processing_files.write().map_err(|e| e.to_string())?;
            if !processing.insert(path.clone()) {
                // Another thread already added it while we were waiting
                log::debug!("File was added by another thread, skipping: {:?}", path);
                continue;
            }
        }

        log::info!("New file detected: {:?}", path);
        process_new_file(
            path,
            app_handle,
            Arc::clone(processing_files),
            if is_create { "created" } else { "modified" },
        )?;
    }

    Ok(())
}

/// Process a newly detected file
fn process_new_file(
    path: PathBuf,
    app_handle: &AppHandle,
    processing_files: Arc<RwLock<HashSet<PathBuf>>>,
    emit_kind: &'static str,
) -> Result<(), String> {
    // Spawn a thread to wait for file stabilization
    let app_handle = app_handle.clone();
    let path_for_cleanup = path.clone();
    thread::spawn(move || {
        log::info!("Waiting for file to stabilize: {:?}", path);

        match wait_for_stable(&path) {
            Ok(_) => {
                log::info!("File stable: {:?}", path);

                let correlation_id = generate_correlation_id();
                let state = app_handle.state::<AppState>();
                if let Err(err) = state
                    .preset_manager
                    .apply_preset_on_import(&path, &correlation_id)
                {
                    log::error!(
                        "[{}] Failed to apply preset on import: {}",
                        correlation_id,
                        err
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

                emit_session_file_change(&app_handle, &path, emit_kind);

                // Note: We intentionally do not emit boothy-new-photo here.
                // We emit a single boothy-session-files-changed event after stabilization
                // to avoid hammering the frontend while the file is still being written.
            }
            Err(e) => {
                log::warn!("File did not stabilize: {:?} - {}", path, e);
            }
        }

        // Remove from processing set when done (success or failure)
        if let Ok(mut processing) = processing_files.write() {
            processing.remove(&path_for_cleanup);
        }
    });

    Ok(())
}

/// Check if a path is an image file based on extension
fn is_image_file(path: &Path) -> bool {
    if let Some(ext) = path.extension() {
        let ext_lower = ext.to_string_lossy().to_lowercase();
        matches!(
            ext_lower.as_str(),
            "jpg"
                | "jpeg"
                | "png"
                | "gif"
                | "bmp"
                | "tiff"
                | "tif"
                | "dng"
                | "cr2"
                | "cr3"
                | "nef"
                | "arw"
                | "raw"
                | "raf"
                | "orf"
                | "rw2"
        )
    } else {
        false
    }
}
