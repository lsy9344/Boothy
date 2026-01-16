use crate::formats::is_raw_file;
use serde::Deserialize;
use serde_json::json;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum BoothyExportChoice {
    OverwriteAll,
    ContinueFromBackground,
}

#[derive(Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct BoothySessionMetadata {
    pub schema_version: Option<u32>,
    pub photos: Vec<BoothySessionPhoto>,
}

#[derive(Default, Deserialize, Clone)]
#[serde(default, rename_all = "snake_case")]
pub struct BoothySessionPhoto {
    pub raw_filename: String,
    pub background_export_completed: Option<bool>,
    pub correlation_id: Option<String>,
}

#[derive(Debug, Clone)]
pub struct BoothyPhotoState {
    pub completed: bool,
    pub correlation_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExportProgressStatus {
    Idle,
    Exporting,
    Complete,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExportProgressState {
    pub status: ExportProgressStatus,
    pub completed: usize,
    pub total: usize,
    pub current_path: String,
}

impl ExportProgressState {
    pub fn new(total: usize) -> Self {
        Self {
            status: ExportProgressStatus::Idle,
            completed: 0,
            total,
            current_path: String::new(),
        }
    }

    pub fn advance(&mut self, completed: usize, current_path: impl Into<String>) {
        self.status = ExportProgressStatus::Exporting;
        self.completed = completed;
        self.current_path = current_path.into();
    }

    pub fn mark_complete(&mut self) {
        if self.status != ExportProgressStatus::Error {
            self.status = ExportProgressStatus::Complete;
        }
        self.completed = self.total;
        self.current_path.clear();
    }

    pub fn mark_error(&mut self) {
        self.status = ExportProgressStatus::Error;
    }

    pub fn to_payload(&self) -> serde_json::Value {
        json!({
            "completed": self.completed,
            "total": self.total,
            "current_path": self.current_path,
        })
    }
}

pub fn load_session_metadata(session_root: &Path) -> Option<BoothySessionMetadata> {
    let metadata_path = session_root.join("boothy.session.json");
    let contents = fs::read_to_string(metadata_path).ok()?;
    serde_json::from_str(&contents).ok()
}

pub fn build_photo_state_map(metadata: &BoothySessionMetadata) -> HashMap<String, BoothyPhotoState> {
    metadata
        .photos
        .iter()
        .filter(|photo| !photo.raw_filename.trim().is_empty())
        .map(|photo| {
            (
                photo.raw_filename.clone(),
                BoothyPhotoState {
                    completed: photo.background_export_completed.unwrap_or(false),
                    correlation_id: photo.correlation_id.clone(),
                },
            )
        })
        .collect()
}

pub fn collect_session_raw_files(raw_path: &Path) -> Result<Vec<PathBuf>, String> {
    let entries = fs::read_dir(raw_path)
        .map_err(|e| format!("Failed to read session Raw folder: {}", e))?;
    let mut files: Vec<PathBuf> = entries
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| is_raw_file(&path.to_string_lossy()))
        .collect();
    files.sort_by(|a, b| a.to_string_lossy().cmp(&b.to_string_lossy()));
    Ok(files)
}

pub fn filter_export_paths(
    paths: Vec<PathBuf>,
    photo_states: Option<&HashMap<String, BoothyPhotoState>>,
    choice: BoothyExportChoice,
) -> Vec<PathBuf> {
    match choice {
        BoothyExportChoice::OverwriteAll => paths,
        BoothyExportChoice::ContinueFromBackground => paths
            .into_iter()
            .filter(|path| {
                let filename = path.file_name().and_then(|name| name.to_str()).unwrap_or("");
                let completed = photo_states
                    .and_then(|states| states.get(filename))
                    .map(|state| state.completed)
                    .unwrap_or(false);
                !completed
            })
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filter_continue_exports_only_incomplete_files() {
        let paths = vec![
            PathBuf::from("A.CR3"),
            PathBuf::from("B.CR3"),
            PathBuf::from("C.CR3"),
        ];
        let metadata = BoothySessionMetadata {
            schema_version: None,
            photos: vec![
                BoothySessionPhoto {
                    raw_filename: "A.CR3".to_string(),
                    background_export_completed: Some(true),
                    correlation_id: None,
                },
                BoothySessionPhoto {
                    raw_filename: "B.CR3".to_string(),
                    background_export_completed: Some(false),
                    correlation_id: None,
                },
            ],
        };
        let states = build_photo_state_map(&metadata);

        let selected = filter_export_paths(paths, Some(&states), BoothyExportChoice::ContinueFromBackground);
        let selected_names: Vec<String> = selected
            .iter()
            .map(|path| path.to_string_lossy().to_string())
            .collect();

        assert_eq!(selected_names, vec!["B.CR3".to_string(), "C.CR3".to_string()]);
    }

    #[test]
    fn filter_overwrite_exports_all_files() {
        let paths = vec![PathBuf::from("A.CR3"), PathBuf::from("B.CR3")];
        let selected = filter_export_paths(paths.clone(), None, BoothyExportChoice::OverwriteAll);
        assert_eq!(selected, paths);
    }

    #[test]
    fn export_progress_state_transitions() {
        let mut state = ExportProgressState::new(3);
        assert_eq!(state.status, ExportProgressStatus::Idle);

        state.advance(1, "A.CR3");
        assert_eq!(state.status, ExportProgressStatus::Exporting);
        assert_eq!(state.completed, 1);
        assert_eq!(state.current_path, "A.CR3");

        state.mark_error();
        assert_eq!(state.status, ExportProgressStatus::Error);

        state.mark_complete();
        assert_eq!(state.status, ExportProgressStatus::Error);
    }
}
