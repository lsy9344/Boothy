use std::path::{Path, PathBuf};

const SESSIONS_DIRECTORY_NAME: &str = "sessions";
const MANIFEST_FILENAME: &str = "session.json";
const EVENTS_FILENAME: &str = "events.ndjson";
const EXPORT_STATUS_FILENAME: &str = "export-status.json";
const PROCESSED_DIRECTORY_NAME: &str = "processed";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionPaths {
    pub session_dir: PathBuf,
    pub manifest_path: PathBuf,
    pub events_path: PathBuf,
    pub export_status_path: PathBuf,
    pub processed_dir: PathBuf,
}

pub fn resolve_booth_session_root(app_local_data_dir: &Path) -> PathBuf {
    app_local_data_dir.join(SESSIONS_DIRECTORY_NAME)
}

pub fn resolve_session_paths<P: AsRef<Path>, S: AsRef<Path>>(
    session_root_base: P,
    relative_session_path: S,
) -> SessionPaths {
    let session_dir = session_root_base.as_ref().join(relative_session_path);

    SessionPaths {
        manifest_path: session_dir.join(MANIFEST_FILENAME),
        events_path: session_dir.join(EVENTS_FILENAME),
        export_status_path: session_dir.join(EXPORT_STATUS_FILENAME),
        processed_dir: session_dir.join(PROCESSED_DIRECTORY_NAME),
        session_dir,
    }
}
