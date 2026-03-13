use std::{
    fs::OpenOptions,
    io::Write,
    path::{Component, Path, PathBuf},
};

use chrono::{SecondsFormat, Utc};
use serde_json::json;

use crate::{
    contracts::{
        dto::{
            DeleteSessionPhotoRequest, DeleteSessionPhotoResponse, SessionGalleryItem,
            SessionGalleryRequest, SessionGallerySnapshot,
        },
        error_envelope::HostErrorEnvelope,
        schema_version::CONTRACT_SCHEMA_VERSION,
    },
    session::{
        session_manifest::{ManifestCaptureRecord, SessionManifest},
        session_repository::SessionRepository,
    },
};

#[derive(Debug, Clone)]
pub struct ThumbnailGuard {
    repository: SessionRepository,
}

#[derive(Debug)]
struct CaptureAssetPaths {
    original_path: PathBuf,
    processed_path: PathBuf,
}

#[derive(Debug)]
struct CaptureAssetBackup {
    path: PathBuf,
    bytes: Vec<u8>,
}

impl ThumbnailGuard {
    pub fn new(repository: SessionRepository) -> Self {
        Self { repository }
    }

    pub fn load_session_gallery(
        &self,
        request: SessionGalleryRequest,
    ) -> Result<SessionGallerySnapshot, HostErrorEnvelope> {
        let manifest = self.repository.load_manifest(&request.manifest_path)?;
        self.validate_session_binding(&manifest, &request.session_id)?;

        self.build_gallery_snapshot(&manifest, None)
    }

    pub fn delete_session_capture(
        &self,
        request: DeleteSessionPhotoRequest,
    ) -> Result<DeleteSessionPhotoResponse, HostErrorEnvelope> {
        let mut manifest = self.repository.load_manifest(&request.manifest_path)?;
        let previous_manifest = manifest.clone();
        self.validate_session_binding(&manifest, &request.session_id)?;

        let capture_index = manifest
            .captures
            .iter()
            .position(|capture| capture.capture_id == request.capture_id)
            .ok_or_else(|| {
                HostErrorEnvelope::session_capture_not_found(format!(
                    "capture not found for session: {}",
                    request.capture_id
                ))
            })?;

        let capture = manifest.captures[capture_index].clone();
        let capture_paths = self.resolve_capture_paths(&manifest, &capture)?;
        let capture_backups = self.read_capture_backups(&capture.capture_id, &capture_paths)?;

        self.remove_capture_files(&capture_backups)?;

        manifest.captures.remove(capture_index);
        manifest.capture_revision = manifest.capture_revision.saturating_add(1);
        manifest.latest_capture_id = manifest
            .captures
            .last()
            .map(|remaining_capture| remaining_capture.capture_id.clone());

        if let Err(error) = self.repository.save_manifest(&request.manifest_path, &manifest) {
            self.restore_capture_files(&capture_backups)
                .map_err(|restore_error| {
                    HostErrorEnvelope::session_manifest_invalid(format!(
                        "{}; capture restore failed: {}",
                        error.message, restore_error.message
                    ))
                })?;

            return Err(error);
        }

        if let Err(error) = self.append_delete_audit_event(&manifest, &request.capture_id) {
            let restore_manifest_error = self
                .repository
                .save_manifest(&request.manifest_path, &previous_manifest)
                .err()
                .map(|restore_error| restore_error.message);
            let restore_capture_error = self
                .restore_capture_files(&capture_backups)
                .err()
                .map(|restore_error| restore_error.message);

            let mut message = error.message;

            if let Some(restore_manifest_error) = restore_manifest_error {
                message = format!("{message}; manifest restore failed: {restore_manifest_error}");
            }

            if let Some(restore_capture_error) = restore_capture_error {
                message = format!("{message}; capture restore failed: {restore_capture_error}");
            }

            return Err(HostErrorEnvelope::session_manifest_invalid(message));
        }

        let selected_capture_id = if manifest.captures.is_empty() {
            None
        } else {
            let next_index = if capture_index < manifest.captures.len() {
                capture_index
            } else {
                capture_index.saturating_sub(1)
            };

            manifest
                .captures
                .get(next_index)
                .or_else(|| manifest.captures.last())
                .map(|remaining_capture| remaining_capture.capture_id.clone())
        };
        let gallery = self.build_gallery_snapshot(&manifest, selected_capture_id)?;

        Ok(DeleteSessionPhotoResponse {
            schema_version: CONTRACT_SCHEMA_VERSION.into(),
            deleted_capture_id: request.capture_id,
            confirmation_message: "사진이 삭제되었습니다.".into(),
            gallery,
        })
    }

    fn build_gallery_snapshot(
        &self,
        manifest: &SessionManifest,
        selected_capture_id: Option<String>,
    ) -> Result<SessionGallerySnapshot, HostErrorEnvelope> {
        let mut items = Vec::with_capacity(manifest.captures.len());

        for (display_order, capture) in manifest.captures.iter().enumerate() {
            let capture_path = self.resolve_capture_paths(manifest, capture)?.processed_path;

            if !capture_path.exists() {
                continue;
            }

            let wire_path = to_wire_path(&capture_path);
            items.push(SessionGalleryItem {
                capture_id: capture.capture_id.clone(),
                session_id: manifest.session_id.clone(),
                captured_at: capture.captured_at.clone(),
                display_order,
                is_latest: manifest.latest_capture_id.as_deref() == Some(capture.capture_id.as_str()),
                preview_path: wire_path.clone(),
                thumbnail_path: wire_path,
                label: format!("Session photo {}", display_order + 1),
            });
        }

        let latest_capture_id = manifest
            .latest_capture_id
            .clone()
            .filter(|capture_id| items.iter().any(|item| item.capture_id == *capture_id));
        let selected_capture_id = selected_capture_id
            .filter(|capture_id| items.iter().any(|item| item.capture_id == *capture_id))
            .or_else(|| latest_capture_id.clone())
            .or_else(|| items.first().map(|item| item.capture_id.clone()));

        Ok(SessionGallerySnapshot {
            schema_version: CONTRACT_SCHEMA_VERSION.into(),
            session_id: manifest.session_id.clone(),
            session_name: manifest.session_name.clone(),
            shoot_ends_at: Some(manifest.timing.actual_shoot_end_at.clone()),
            active_preset_name: manifest.active_preset_name.clone(),
            latest_capture_id,
            selected_capture_id,
            items,
        })
    }

    fn validate_session_binding(
        &self,
        manifest: &SessionManifest,
        session_id: &str,
    ) -> Result<(), HostErrorEnvelope> {
        if manifest.session_id != session_id {
            return Err(HostErrorEnvelope::session_capture_wrong_session(format!(
                "manifest session does not match request: {session_id}"
            )));
        }

        Ok(())
    }

    fn resolve_capture_paths(
        &self,
        manifest: &SessionManifest,
        capture: &ManifestCaptureRecord,
    ) -> Result<CaptureAssetPaths, HostErrorEnvelope> {
        let session_root = normalize_path(Path::new(&manifest.session_dir));
        let processed_root = normalize_path(Path::new(&manifest.processed_dir));
        let original_path = normalize_path(session_root.join(&capture.original_file_name));
        let processed_path = normalize_path(processed_root.join(&capture.processed_file_name));

        if !original_path.starts_with(&session_root) {
            return Err(HostErrorEnvelope::session_capture_out_of_root(format!(
                "capture resolved outside active session root: {}",
                capture.original_file_name
            )));
        }

        if !processed_path.starts_with(&session_root) || !processed_path.starts_with(&processed_root) {
            return Err(HostErrorEnvelope::session_capture_out_of_root(format!(
                "capture resolved outside active session root: {}",
                capture.processed_file_name
            )));
        }

        Ok(CaptureAssetPaths {
            original_path,
            processed_path,
        })
    }

    fn read_capture_backups(
        &self,
        capture_id: &str,
        capture_paths: &CaptureAssetPaths,
    ) -> Result<Vec<CaptureAssetBackup>, HostErrorEnvelope> {
        [capture_paths.original_path.clone(), capture_paths.processed_path.clone()]
            .into_iter()
            .map(|path| {
                if !path.exists() {
                    return Err(HostErrorEnvelope::session_capture_not_found(format!(
                        "capture file not found: {capture_id}"
                    )));
                }

                let bytes = std::fs::read(&path)
                    .map_err(|error| HostErrorEnvelope::session_manifest_invalid(error.to_string()))?;

                Ok(CaptureAssetBackup { path, bytes })
            })
            .collect()
    }

    fn remove_capture_files(
        &self,
        capture_backups: &[CaptureAssetBackup],
    ) -> Result<(), HostErrorEnvelope> {
        for capture_backup in capture_backups {
            std::fs::remove_file(&capture_backup.path)
                .map_err(|error| HostErrorEnvelope::session_manifest_invalid(error.to_string()))?;
        }

        Ok(())
    }

    fn restore_capture_files(
        &self,
        capture_backups: &[CaptureAssetBackup],
    ) -> Result<(), HostErrorEnvelope> {
        for capture_backup in capture_backups {
            if let Some(parent) = capture_backup.path.parent() {
                std::fs::create_dir_all(parent)
                    .map_err(|error| HostErrorEnvelope::session_manifest_invalid(error.to_string()))?;
            }

            std::fs::write(&capture_backup.path, &capture_backup.bytes)
                .map_err(|error| HostErrorEnvelope::session_manifest_invalid(error.to_string()))?;
        }

        Ok(())
    }

    fn append_delete_audit_event(
        &self,
        manifest: &SessionManifest,
        capture_id: &str,
    ) -> Result<(), HostErrorEnvelope> {
        let session_root = normalize_path(Path::new(&manifest.session_dir));
        let events_path = normalize_path(Path::new(&manifest.events_path));
        let expected_events_path = normalize_path(session_root.join("events.ndjson"));

        if !events_path.starts_with(&session_root) {
            return Err(HostErrorEnvelope::session_capture_out_of_root(format!(
                "events file resolved outside active session root: {}",
                manifest.events_path
            )));
        }

        if events_path != expected_events_path {
            return Err(HostErrorEnvelope::session_manifest_invalid(format!(
                "events path must target the session-local events.ndjson artifact: {}",
                manifest.events_path
            )));
        }

        let mut events_file = OpenOptions::new()
            .append(true)
            .open(&events_path)
            .map_err(|error| HostErrorEnvelope::session_manifest_invalid(error.to_string()))?;
        let payload = json!({
            "eventType": "photo_deleted",
            "occurredAt": Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true),
            "sessionId": manifest.session_id,
            "captureId": capture_id,
        });

        writeln!(events_file, "{payload}")
            .map_err(|error| HostErrorEnvelope::session_manifest_invalid(error.to_string()))
    }
}

fn normalize_path(path: impl AsRef<Path>) -> PathBuf {
    let mut normalized = PathBuf::new();

    for component in path.as_ref().components() {
        match component {
            Component::ParentDir => {
                normalized.pop();
            }
            Component::CurDir => {}
            other => normalized.push(other.as_os_str()),
        }
    }

    normalized
}

fn to_wire_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}
