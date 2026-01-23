use fs2;
use std::path::Path;

#[derive(Debug, Clone)]
pub struct DriveStats {
    pub free_bytes: u64,
    pub total_bytes: u64,
}

pub fn sample_drive_stats(path: &Path) -> Result<DriveStats, String> {
    let free_bytes = fs2::available_space(path).map_err(|err| err.to_string())?;
    let total_bytes = fs2::total_space(path).map_err(|err| err.to_string())?;
    if total_bytes == 0 {
        return Err("Total space returned zero".to_string());
    }

    Ok(DriveStats {
        free_bytes,
        total_bytes,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn sample_drive_stats_returns_values() {
        let dir = tempdir().unwrap();
        let stats = sample_drive_stats(dir.path()).unwrap();
        assert!(stats.total_bytes > 0);
        assert!(stats.free_bytes <= stats.total_bytes);
    }
}
