use super::models::BoothySession;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use tempfile::NamedTempFile;

pub const SESSION_METADATA_FILENAME: &str = "boothy.session.json";

fn default_schema_version() -> u32 {
    1
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct BoothySessionMetadata {
    #[serde(default = "default_schema_version")]
    pub schema_version: u32,
    #[serde(default)]
    pub photos: Vec<SessionPhotoExportState>,
}

impl Default for BoothySessionMetadata {
    fn default() -> Self {
        Self {
            schema_version: default_schema_version(),
            photos: Vec::new(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SessionPhotoExportState {
    pub raw_filename: String,
    #[serde(default)]
    pub background_export_completed: bool,
    #[serde(default)]
    pub background_export_timestamp: Option<DateTime<Utc>>,
    #[serde(default)]
    pub attempt_count: u32,
    #[serde(default)]
    pub last_attempt_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub last_error: Option<SessionExportError>,
}

impl SessionPhotoExportState {
    fn new(raw_filename: String) -> Self {
        Self {
            raw_filename,
            background_export_completed: false,
            background_export_timestamp: None,
            attempt_count: 0,
            last_attempt_at: None,
            last_error: None,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SessionExportError {
    pub code: String,
    pub message: String,
    #[serde(default)]
    pub context: serde_json::Value,
}

impl SessionExportError {
    pub fn new(
        code: impl Into<String>,
        message: impl Into<String>,
        context: serde_json::Value,
    ) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            context,
        }
    }
}

pub fn load_session_metadata(session: &BoothySession) -> Result<BoothySessionMetadata, String> {
    let path = metadata_path(session);
    if !path.exists() {
        return Ok(BoothySessionMetadata::default());
    }

    let content = fs::read_to_string(path).map_err(|e| e.to_string())?;
    serde_json::from_str(&content).map_err(|e| e.to_string())
}

pub fn save_session_metadata(
    session: &BoothySession,
    metadata: &BoothySessionMetadata,
) -> Result<(), String> {
    let path = metadata_path(session);
    let json = serde_json::to_string_pretty(metadata).map_err(|e| e.to_string())?;
    let parent = path
        .parent()
        .ok_or_else(|| "Session metadata path missing parent".to_string())?;

    fs::create_dir_all(parent).map_err(|e| e.to_string())?;

    let mut temp_file = NamedTempFile::new_in(parent).map_err(|e| e.to_string())?;
    temp_file
        .write_all(json.as_bytes())
        .map_err(|e| e.to_string())?;
    temp_file.flush().map_err(|e| e.to_string())?;

    if path.exists() {
        fs::remove_file(&path).map_err(|e| e.to_string())?;
    }

    temp_file.persist(&path).map_err(|e| e.to_string())?;
    Ok(())
}

pub fn record_background_export_attempt(
    session: &BoothySession,
    raw_path: &Path,
) -> Result<(), String> {
    update_photo_state(session, raw_path, |entry| {
        entry.attempt_count = entry.attempt_count.saturating_add(1);
        entry.last_attempt_at = Some(Utc::now());
        entry.last_error = None;
        entry.background_export_completed = false;
    })
}

pub fn mark_background_export_success(
    session: &BoothySession,
    raw_path: &Path,
) -> Result<(), String> {
    update_photo_state(session, raw_path, |entry| {
        entry.background_export_completed = true;
        entry.background_export_timestamp = Some(Utc::now());
        entry.last_error = None;
        if entry.attempt_count == 0 {
            entry.attempt_count = 1;
            entry.last_attempt_at = Some(Utc::now());
        }
    })
}

pub fn mark_background_export_failure(
    session: &BoothySession,
    raw_path: &Path,
    error: SessionExportError,
) -> Result<(), String> {
    update_photo_state(session, raw_path, |entry| {
        entry.background_export_completed = false;
        entry.last_error = Some(error);
        if entry.attempt_count == 0 {
            entry.attempt_count = 1;
            entry.last_attempt_at = Some(Utc::now());
        }
    })
}

pub fn is_background_export_completed(
    session: &BoothySession,
    raw_path: &Path,
) -> Result<bool, String> {
    let raw_filename = raw_filename(raw_path)?;
    let metadata = load_session_metadata(session)?;
    Ok(metadata
        .photos
        .iter()
        .find(|entry| entry.raw_filename == raw_filename)
        .map(|entry| entry.background_export_completed)
        .unwrap_or(false))
}

fn update_photo_state<F>(session: &BoothySession, raw_path: &Path, mutator: F) -> Result<(), String>
where
    F: FnOnce(&mut SessionPhotoExportState),
{
    let raw_filename = raw_filename(raw_path)?;
    let mut metadata = load_session_metadata(session)?;

    let entry = if let Some(existing) = metadata
        .photos
        .iter_mut()
        .find(|entry| entry.raw_filename == raw_filename)
    {
        existing
    } else {
        metadata
            .photos
            .push(SessionPhotoExportState::new(raw_filename.clone()));
        metadata
            .photos
            .last_mut()
            .ok_or_else(|| "Failed to append new session metadata entry".to_string())?
    };

    mutator(entry);
    save_session_metadata(session, &metadata)
}

fn raw_filename(raw_path: &Path) -> Result<String, String> {
    raw_path
        .file_name()
        .and_then(|name| name.to_str())
        .map(|name| name.to_string())
        .ok_or_else(|| "Raw file name missing".to_string())
}

fn metadata_path(session: &BoothySession) -> PathBuf {
    session.base_path.join(SESSION_METADATA_FILENAME)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session::BoothySession;
    use serde_json::json;
    use tempfile::TempDir;

    fn build_session(temp_dir: &TempDir) -> BoothySession {
        BoothySession::new(
            "Test Session".to_string(),
            "Test-Session".to_string(),
            temp_dir.path().to_path_buf(),
        )
    }

    #[test]
    fn metadata_round_trip_records_failure_details() {
        let temp_dir = TempDir::new().unwrap();
        let session = build_session(&temp_dir);
        let raw_path = session.base_path.join("Raw").join("IMG_0001.CR3");

        record_background_export_attempt(&session, &raw_path).unwrap();
        mark_background_export_failure(
            &session,
            &raw_path,
            SessionExportError::new(
                "EXPORT_FAILED",
                "Failed to export image.",
                json!({ "destination": "Jpg/IMG_0001.jpg" }),
            ),
        )
        .unwrap();

        let metadata = load_session_metadata(&session).unwrap();
        let entry = metadata
            .photos
            .iter()
            .find(|photo| photo.raw_filename == "IMG_0001.CR3")
            .expect("photo entry should exist");

        assert_eq!(entry.attempt_count, 1);
        assert!(entry.last_error.is_some());
        assert!(!entry.background_export_completed);
    }

    #[test]
    fn metadata_backwards_compat_defaults_missing_fields() {
        let temp_dir = TempDir::new().unwrap();
        let session = build_session(&temp_dir);
        let path = metadata_path(&session);

        let json = serde_json::json!({
            "schemaVersion": 1,
            "photos": [
                {
                    "rawFilename": "IMG_0002.CR3",
                    "backgroundExportCompleted": true,
                    "backgroundExportTimestamp": "2026-01-15T14:23:45Z"
                }
            ]
        });
        fs::write(&path, serde_json::to_string_pretty(&json).unwrap()).unwrap();

        let metadata = load_session_metadata(&session).unwrap();
        let entry = metadata
            .photos
            .iter()
            .find(|photo| photo.raw_filename == "IMG_0002.CR3")
            .expect("photo entry should exist");

        assert_eq!(entry.attempt_count, 0);
        assert!(entry.last_attempt_at.is_none());
        assert!(entry.last_error.is_none());
    }

    #[test]
    fn metadata_atomic_write_produces_valid_json() {
        let temp_dir = TempDir::new().unwrap();
        let session = build_session(&temp_dir);
        let raw_path = session.base_path.join("Raw").join("IMG_0003.CR3");

        record_background_export_attempt(&session, &raw_path).unwrap();
        mark_background_export_success(&session, &raw_path).unwrap();

        let contents = fs::read_to_string(metadata_path(&session)).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&contents).unwrap();

        assert_eq!(parsed["schemaVersion"], 1);
    }
}
