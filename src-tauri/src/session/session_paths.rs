use std::path::{Path, PathBuf};

use crate::contracts::dto::{validate_session_id, HostErrorEnvelope};

#[derive(Debug, Clone)]
pub struct SessionPaths {
    pub sessions_root: PathBuf,
    pub session_root: PathBuf,
    pub temp_root: PathBuf,
    pub manifest_path: PathBuf,
    pub captures_originals_dir: PathBuf,
    pub renders_previews_dir: PathBuf,
    pub renders_finals_dir: PathBuf,
    pub handoff_dir: PathBuf,
    pub diagnostics_dir: PathBuf,
}

impl SessionPaths {
    pub fn try_new(base_dir: &Path, session_id: &str) -> Result<Self, HostErrorEnvelope> {
        validate_session_id(session_id)?;

        Ok(Self::new(base_dir, session_id))
    }

    pub fn new(base_dir: &Path, session_id: &str) -> Self {
        let sessions_root = base_dir.join("sessions");
        let session_root = sessions_root.join(session_id);
        let temp_root = sessions_root.join(format!(".creating-{session_id}"));

        Self {
            sessions_root,
            manifest_path: session_root.join("session.json"),
            captures_originals_dir: session_root.join("captures").join("originals"),
            renders_previews_dir: session_root.join("renders").join("previews"),
            renders_finals_dir: session_root.join("renders").join("finals"),
            handoff_dir: session_root.join("handoff"),
            diagnostics_dir: session_root.join("diagnostics"),
            session_root,
            temp_root,
        }
    }

    pub fn temp_manifest_path(&self) -> PathBuf {
        self.temp_root.join("session.json")
    }

    pub fn temp_captures_originals_dir(&self) -> PathBuf {
        self.temp_root.join("captures").join("originals")
    }

    pub fn temp_renders_previews_dir(&self) -> PathBuf {
        self.temp_root.join("renders").join("previews")
    }

    pub fn temp_renders_finals_dir(&self) -> PathBuf {
        self.temp_root.join("renders").join("finals")
    }

    pub fn temp_handoff_dir(&self) -> PathBuf {
        self.temp_root.join("handoff")
    }

    pub fn temp_diagnostics_dir(&self) -> PathBuf {
        self.temp_root.join("diagnostics")
    }
}
