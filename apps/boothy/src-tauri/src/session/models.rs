use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Folder-backed session model with paths to Raw and Jpg subdirectories
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BoothySession {
    pub session_name: String,
    pub session_folder_name: String,
    pub base_path: PathBuf,
    pub raw_path: PathBuf,
    pub jpg_path: PathBuf,
    pub created_at: DateTime<Utc>,
    pub last_accessed: DateTime<Utc>,
}

impl BoothySession {
    pub fn new(session_name: String, session_folder_name: String, base_path: PathBuf) -> Self {
        let raw_path = base_path.join("Raw");
        let jpg_path = base_path.join("Jpg");
        let now = Utc::now();

        Self {
            session_name,
            session_folder_name,
            base_path,
            raw_path,
            jpg_path,
            created_at: now,
            last_accessed: now,
        }
    }

    pub fn update_last_accessed(&mut self) {
        self.last_accessed = Utc::now();
    }
}
