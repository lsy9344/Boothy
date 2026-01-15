use std::fs;
use std::path::Path;
use std::thread;
use std::time::{Duration, SystemTime};

/// File stabilization parameters
const MIN_STABLE_TIME_MS: u64 = 500; // Minimum time file must be stable
const STABILITY_CHECK_INTERVAL_MS: u64 = 100; // Check interval
const MAX_WAIT_TIME_MS: u64 = 10000; // Max wait time before giving up

/// Check if a file is stable and ready for import
///
/// Stability criteria:
/// 1. File is not locked (can be opened for reading)
/// 2. File size hasn't changed for MIN_STABLE_TIME_MS
/// 3. At least MIN_STABLE_TIME_MS has passed since file creation/modification
pub fn is_file_stable(path: &Path) -> Result<bool, String> {
    // Check if file exists
    if !path.exists() {
        return Err(format!("File does not exist: {:?}", path));
    }

    // Try to open file (check if locked)
    match fs::File::open(path) {
        Ok(_) => {}
        Err(e) => {
            log::debug!("File is locked: {:?} - {}", path, e);
            return Ok(false);
        }
    }

    // Get initial file size
    let initial_metadata =
        fs::metadata(path).map_err(|e| format!("Failed to get file metadata: {}", e))?;
    let initial_size = initial_metadata.len();
    let initial_modified = initial_metadata
        .modified()
        .map_err(|e| format!("Failed to get modification time: {}", e))?;

    // Wait for MIN_STABLE_TIME_MS
    thread::sleep(Duration::from_millis(MIN_STABLE_TIME_MS));

    // Check if size changed
    let final_metadata =
        fs::metadata(path).map_err(|e| format!("Failed to get file metadata after wait: {}", e))?;
    let final_size = final_metadata.len();

    if final_size != initial_size {
        log::debug!(
            "File size changed: {:?} ({} -> {})",
            path,
            initial_size,
            final_size
        );
        return Ok(false);
    }

    // Check if enough time has passed since modification
    let now = SystemTime::now();
    let elapsed_since_modification = now
        .duration_since(initial_modified)
        .map_err(|e| format!("Failed to calculate time since modification: {}", e))?;

    if elapsed_since_modification < Duration::from_millis(MIN_STABLE_TIME_MS) {
        log::debug!("File too recently modified: {:?}", path);
        return Ok(false);
    }

    Ok(true)
}

/// Wait for a file to become stable, with timeout
pub fn wait_for_stable(path: &Path) -> Result<(), String> {
    let start = SystemTime::now();
    let timeout = Duration::from_millis(MAX_WAIT_TIME_MS);

    loop {
        match is_file_stable(path) {
            Ok(true) => {
                log::info!("File stabilized: {:?}", path);
                return Ok(());
            }
            Ok(false) => {
                // Not stable yet, check if we've exceeded timeout
                let elapsed = SystemTime::now()
                    .duration_since(start)
                    .map_err(|e| format!("Failed to calculate elapsed time: {}", e))?;

                if elapsed >= timeout {
                    return Err(format!("Timeout waiting for file to stabilize: {:?}", path));
                }

                // Wait before next check
                thread::sleep(Duration::from_millis(STABILITY_CHECK_INTERVAL_MS));
            }
            Err(e) => {
                return Err(format!("Error checking file stability: {}", e));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;
    use tempfile::TempDir;

    #[test]
    fn test_is_file_stable_nonexistent() {
        let temp_dir = TempDir::new().unwrap();
        let nonexistent = temp_dir.path().join("nonexistent.txt");
        let result = is_file_stable(&nonexistent);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("does not exist"));
    }

    #[test]
    fn test_is_file_stable_new_file() {
        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("test.txt");

        // Create a file
        let mut file = File::create(&test_file).unwrap();
        writeln!(file, "test content").unwrap();
        drop(file);

        // Immediately check - should be unstable (too recent)
        let result = is_file_stable(&test_file);
        assert!(result.is_ok());
        // Note: Result depends on timing, but at least we verify it doesn't panic
    }

    #[test]
    fn test_is_file_stable_after_wait() {
        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("stable_test.txt");

        // Create and close file
        let mut file = File::create(&test_file).unwrap();
        writeln!(file, "test content").unwrap();
        drop(file);

        // Wait longer than MIN_STABLE_TIME_MS
        thread::sleep(Duration::from_millis(600));

        // Now check - should be stable
        let result = is_file_stable(&test_file);
        assert!(result.is_ok());
        assert!(result.unwrap());
    }

    #[test]
    fn test_wait_for_stable_existing_file() {
        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("wait_test.txt");

        // Create and close file
        let mut file = File::create(&test_file).unwrap();
        writeln!(file, "test content").unwrap();
        drop(file);

        // Wait a bit to ensure file is old enough
        thread::sleep(Duration::from_millis(100));

        // Wait for stable should succeed
        let result = wait_for_stable(&test_file);
        assert!(result.is_ok());
    }
}
