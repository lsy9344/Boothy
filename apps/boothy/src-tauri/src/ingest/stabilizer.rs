use log::{debug, info, warn};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};
use tokio::time::sleep;

/// File stabilization configuration
#[derive(Clone, Debug)]
pub struct StabilizationConfig {
    /// Poll interval for checking file stability (milliseconds)
    pub poll_interval_ms: u64,

    /// Number of consecutive stable checks required
    pub stable_count_required: u32,

    /// Maximum wait time before giving up (milliseconds)
    pub max_wait_ms: u64,

    /// Minimum file age before considering it stable (milliseconds)
    pub min_age_ms: u64,
}

impl Default for StabilizationConfig {
    fn default() -> Self {
        Self {
            poll_interval_ms: 200,
            stable_count_required: 3,
            max_wait_ms: 10000,
            min_age_ms: 500,
        }
    }
}

/// File stabilization result
#[derive(Debug)]
pub enum StabilizationResult {
    /// File is stable and ready for import
    Stable { path: PathBuf, size: u64 },

    /// File stabilization timed out
    Timeout { path: PathBuf },

    /// File does not exist or was deleted
    NotFound { path: PathBuf },

    /// File is locked or inaccessible
    Locked { path: PathBuf },
}

/// Check if a file is stable and ready for import
///
/// Stability criteria:
/// 1. File exists and is accessible
/// 2. File size hasn't changed for N consecutive checks
/// 3. File can be opened (not locked)
/// 4. Minimum age threshold met
pub async fn wait_for_file_stability(
    path: PathBuf,
    config: StabilizationConfig,
    correlation_id: &str,
) -> StabilizationResult {
    info!(
        "[{}] Starting file stability check: {}",
        correlation_id,
        path.display()
    );

    let start_time = SystemTime::now();
    let mut stable_checks = 0;
    let mut last_size: Option<u64> = None;
    let mut locked_detected = false;

    loop {
        // Check timeout
        if let Ok(elapsed) = start_time.elapsed() {
            if elapsed.as_millis() > config.max_wait_ms as u128 {
                warn!(
                    "[{}] File stability timeout: {}",
                    correlation_id,
                    path.display()
                );
                return if locked_detected {
                    StabilizationResult::Locked { path }
                } else {
                    StabilizationResult::Timeout { path }
                };
            }
        }

        // Check if file exists
        if !path.exists() {
            warn!("[{}] File not found: {}", correlation_id, path.display());
            return StabilizationResult::NotFound { path };
        }

        // Get file metadata
        let metadata = match fs::metadata(&path) {
            Ok(m) => m,
            Err(e) => {
                debug!(
                    "[{}] Failed to get metadata (may be locked): {}",
                    correlation_id, e
                );
                sleep(Duration::from_millis(config.poll_interval_ms)).await;
                continue;
            }
        };

        let current_size = metadata.len();

        // Check minimum age
        if let Ok(modified) = metadata.modified() {
            if let Ok(age) = SystemTime::now().duration_since(modified) {
                if age.as_millis() < config.min_age_ms as u128 {
                    debug!(
                        "[{}] File too new, waiting... (age: {}ms)",
                        correlation_id,
                        age.as_millis()
                    );
                    sleep(Duration::from_millis(config.poll_interval_ms)).await;
                    continue;
                }
            }
        }

        // Check if size has changed
        if let Some(prev_size) = last_size {
            if current_size == prev_size {
                stable_checks += 1;
                debug!(
                    "[{}] Stable check {}/{} (size: {} bytes)",
                    correlation_id, stable_checks, config.stable_count_required, current_size
                );

                if stable_checks >= config.stable_count_required {
                    // Final verification: try to open the file
                    if can_open_file(&path) {
                        info!(
                            "[{}] File stable: {} ({} bytes)",
                            correlation_id,
                            path.display(),
                            current_size
                        );
                        return StabilizationResult::Stable {
                            path,
                            size: current_size,
                        };
                    } else {
                        warn!(
                            "[{}] File still locked, retrying: {}",
                            correlation_id,
                            path.display()
                        );
                        locked_detected = true;
                        stable_checks = 0;
                    }
                }
            } else {
                // Size changed, reset counter
                debug!(
                    "[{}] File size changed: {} -> {} bytes (resetting counter)",
                    correlation_id, prev_size, current_size
                );
                stable_checks = 0;
            }
        }

        last_size = Some(current_size);

        // Wait before next check
        sleep(Duration::from_millis(config.poll_interval_ms)).await;
    }
}

/// Check if a file can be opened (not locked)
fn can_open_file(path: &Path) -> bool {
    match fs::File::open(path) {
        Ok(_) => true,
        Err(_) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_stable_file() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.raw");

        // Create a file and let it stabilize
        let mut file = File::create(&file_path).unwrap();
        file.write_all(b"test data").unwrap();
        drop(file);

        // Wait a bit for file to age
        tokio::time::sleep(Duration::from_millis(600)).await;

        let config = StabilizationConfig {
            poll_interval_ms: 100,
            stable_count_required: 2,
            max_wait_ms: 5000,
            min_age_ms: 500,
        };

        let result = wait_for_file_stability(file_path, config, "test").await;

        match result {
            StabilizationResult::Stable { size, .. } => {
                assert_eq!(size, 9);
            }
            _ => panic!("Expected Stable result"),
        }
    }

    #[tokio::test]
    async fn test_nonexistent_file() {
        let file_path = PathBuf::from("nonexistent_file.raw");
        let config = StabilizationConfig::default();

        let result = wait_for_file_stability(file_path, config, "test").await;

        match result {
            StabilizationResult::NotFound { .. } => {}
            _ => panic!("Expected NotFound result"),
        }
    }
}
