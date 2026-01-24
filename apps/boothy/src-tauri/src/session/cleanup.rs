use chrono::{DateTime, Utc};
use serde::Serialize;
use std::fs;
use std::path::{Component, Path, PathBuf};
use walkdir::WalkDir;

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CleanupSessionEntry {
    pub name: String,
    pub path: String,
    pub last_modified: Option<String>,
    pub size_bytes: Option<u64>,
    pub is_active: bool,
    pub diagnostic: Option<String>,
}

#[derive(Debug, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct CleanupDeleteSummary {
    pub deleted: Vec<String>,
    pub skipped_active: Vec<String>,
    pub skipped_invalid: Vec<String>,
    pub failed: Vec<CleanupDeleteFailure>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CleanupDeleteFailure {
    pub name: String,
    pub diagnostic: String,
}

pub fn list_sessions(
    sessions_root: &Path,
    active_session_name: Option<&str>,
) -> Result<Vec<CleanupSessionEntry>, String> {
    if !sessions_root.exists() {
        return Ok(Vec::new());
    }

    let entries = fs::read_dir(sessions_root).map_err(|err| err.to_string())?;
    let mut results = Vec::new();

    for entry in entries {
        let entry = match entry {
            Ok(entry) => entry,
            Err(err) => {
                results.push(CleanupSessionEntry {
                    name: "Unknown".to_string(),
                    path: sessions_root.to_string_lossy().to_string(),
                    last_modified: None,
                    size_bytes: None,
                    is_active: false,
                    diagnostic: Some(format!("Failed to read session entry: {}", err)),
                });
                continue;
            }
        };

        let path = entry.path();
        if !path.is_dir() {
            continue;
        }

        let name = entry.file_name().to_string_lossy().to_string();
        let is_active = active_session_name
            .map(|active| active == name)
            .unwrap_or(false);

        let (last_modified, last_modified_err) = read_last_modified(&path);
        let (size_bytes, size_err) = calculate_dir_size(&path);

        let diagnostic = match (last_modified_err, size_err) {
            (Some(left), Some(right)) => Some(format!("{}; {}", left, right)),
            (Some(left), None) => Some(left),
            (None, Some(right)) => Some(right),
            (None, None) => None,
        };

        results.push(CleanupSessionEntry {
            name,
            path: path.to_string_lossy().to_string(),
            last_modified,
            size_bytes,
            is_active,
            diagnostic,
        });
    }

    Ok(results)
}

pub fn delete_sessions(
    sessions_root: &Path,
    active_session_name: Option<&str>,
    session_names: &[String],
) -> Result<CleanupDeleteSummary, String> {
    let canonical_root = fs::canonicalize(sessions_root).map_err(|err| err.to_string())?;
    let mut summary = CleanupDeleteSummary::default();

    for session_name in session_names {
        if active_session_name
            .map(|active| active == session_name)
            .unwrap_or(false)
        {
            summary.skipped_active.push(session_name.clone());
            continue;
        }

        let target_path = match resolve_session_dir(&canonical_root, session_name) {
            Ok(path) => path,
            Err(_) => {
                summary.skipped_invalid.push(session_name.clone());
                continue;
            }
        };

        match delete_dir(&target_path) {
            Ok(()) => summary.deleted.push(session_name.clone()),
            Err(err) => summary.failed.push(CleanupDeleteFailure {
                name: session_name.clone(),
                diagnostic: err,
            }),
        }
    }

    Ok(summary)
}

fn resolve_session_dir(canonical_root: &Path, session_name: &str) -> Result<PathBuf, String> {
    validate_session_name(session_name)?;

    let candidate = canonical_root.join(session_name);
    if !candidate.exists() {
        return Err("Session folder not found".to_string());
    }

    let canonical_candidate = fs::canonicalize(&candidate).map_err(|err| err.to_string())?;
    if canonical_candidate.parent() != Some(canonical_root) {
        return Err("Session path is outside the sessions root".to_string());
    }

    Ok(canonical_candidate)
}

fn validate_session_name(session_name: &str) -> Result<(), String> {
    let trimmed = session_name.trim();
    if trimmed.is_empty() {
        return Err("Session name is empty".to_string());
    }

    let path = Path::new(trimmed);
    if path.is_absolute() {
        return Err("Absolute paths are not allowed".to_string());
    }

    let mut components = path.components();
    match components.next() {
        Some(Component::Normal(_)) => {}
        _ => return Err("Invalid session name".to_string()),
    }

    if components.next().is_some() {
        return Err("Nested paths are not allowed".to_string());
    }

    Ok(())
}

fn read_last_modified(path: &Path) -> (Option<String>, Option<String>) {
    let metadata = match fs::metadata(path) {
        Ok(meta) => meta,
        Err(err) => return (None, Some(format!("Failed to read metadata: {}", err))),
    };
    let modified = match metadata.modified() {
        Ok(time) => time,
        Err(err) => return (None, Some(format!("Failed to read modified time: {}", err))),
    };
    let chrono_time: DateTime<Utc> = modified.into();
    (Some(chrono_time.to_rfc3339()), None)
}

fn calculate_dir_size(path: &Path) -> (Option<u64>, Option<String>) {
    let mut total = 0u64;
    let mut error: Option<String> = None;

    for entry in WalkDir::new(path).into_iter() {
        match entry {
            Ok(entry) => {
                if entry.file_type().is_file() {
                    match entry.metadata() {
                        Ok(metadata) => {
                            total = total.saturating_add(metadata.len());
                        }
                        Err(err) => {
                            if error.is_none() {
                                error = Some(format!(
                                    "Failed to read file metadata for {}: {}",
                                    entry.path().display(),
                                    err
                                ));
                            }
                        }
                    }
                }
            }
            Err(err) => {
                if error.is_none() {
                    error = Some(format!("Failed to read directory: {}", err));
                }
            }
        }
    }

    if error.is_some() {
        (Some(total), error)
    } else {
        (Some(total), None)
    }
}

fn delete_dir(path: &Path) -> Result<(), String> {
    if !path.exists() {
        return Ok(());
    }
    fs::remove_dir_all(path).map_err(|err| err.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn validate_session_name_allows_single_component() {
        assert!(validate_session_name("Session-1").is_ok());
    }

    #[test]
    fn validate_session_name_rejects_nested_paths() {
        assert!(validate_session_name("Session-1/Sub").is_err());
        assert!(validate_session_name("Session-1\\Sub").is_err());
    }

    #[test]
    fn validate_session_name_rejects_parent_dir() {
        assert!(validate_session_name("../Session").is_err());
        assert!(validate_session_name("..").is_err());
    }

    #[test]
    fn resolve_session_dir_rejects_outside_root() {
        let temp_dir = tempdir().unwrap();
        let canonical_root = fs::canonicalize(temp_dir.path()).unwrap();
        assert!(resolve_session_dir(&canonical_root, "..").is_err());
    }

    #[test]
    fn delete_sessions_skips_active() {
        let temp_dir = tempdir().unwrap();
        let active = temp_dir.path().join("Active");
        let old = temp_dir.path().join("Old");
        fs::create_dir_all(&active).unwrap();
        fs::create_dir_all(&old).unwrap();

        let summary = delete_sessions(
            temp_dir.path(),
            Some("Active"),
            &[String::from("Active"), String::from("Old")],
        )
        .unwrap();

        assert!(summary.deleted.contains(&"Old".to_string()));
        assert!(summary.skipped_active.contains(&"Active".to_string()));
        assert!(active.exists());
        assert!(!old.exists());
    }
}
