use chrono::Local;
use fern::Dispatch;
use log::{LevelFilter, info};
use std::fs;
use std::path::PathBuf;

/// Initialize offline-first logging for field diagnostics
/// Logs are written to %APPDATA%/Boothy/logs/ for offline troubleshooting
pub fn init_logging() -> Result<(), Box<dyn std::error::Error>> {
    let log_dir = get_log_directory()?;
    fs::create_dir_all(&log_dir)?;

    let log_file_path = log_dir.join(format!("boothy-{}.log", Local::now().format("%Y%m%d")));

    let file_config = Dispatch::new()
        .format(|out, message, record| {
            out.finish(format_args!(
                "[{}][{}][{}] {}",
                Local::now().format("%Y-%m-%d %H:%M:%S%.3f"),
                record.level(),
                record.target(),
                message
            ))
        })
        .level(LevelFilter::Info)
        .level_for("boothy", LevelFilter::Debug)
        .chain(fern::log_file(log_file_path)?);

    let stdout_config = Dispatch::new()
        .format(|out, message, record| {
            out.finish(format_args!(
                "[{}][{}] {}",
                Local::now().format("%H:%M:%S"),
                record.level(),
                message
            ))
        })
        .level(LevelFilter::Info)
        .chain(std::io::stdout());

    Dispatch::new()
        .chain(file_config)
        .chain(stdout_config)
        .apply()?;

    info!("Logging initialized. Log directory: {:?}", log_dir);
    Ok(())
}

/// Get the log directory path (offline-first: AppData)
pub fn get_log_directory() -> Result<PathBuf, Box<dyn std::error::Error>> {
    let appdata = std::env::var("APPDATA")
        .or_else(|_| std::env::var("HOME").map(|h| format!("{}/AppData/Roaming", h)))
        .map_err(|_| "Could not determine AppData directory")?;

    Ok(PathBuf::from(appdata).join("Boothy").join("logs"))
}

/// Correlation ID utilities for end-to-end tracing
pub mod correlation {
    use uuid::Uuid;

    /// Generate a new correlation ID
    pub fn generate() -> String {
        format!("corr-{}", Uuid::new_v4())
    }

    /// Log with correlation ID context
    /// Example: log_with_correlation("abc-123", "Capture started")
    pub fn log_info(correlation_id: &str, message: &str) {
        log::info!("[{}] {}", correlation_id, message);
    }

    pub fn log_debug(correlation_id: &str, message: &str) {
        log::debug!("[{}] {}", correlation_id, message);
    }

    pub fn log_error(correlation_id: &str, message: &str) {
        log::error!("[{}] {}", correlation_id, message);
    }

    pub fn log_warn(correlation_id: &str, message: &str) {
        log::warn!("[{}] {}", correlation_id, message);
    }
}

/// Diagnostic log entry for structured logging
#[derive(Debug, Clone, serde::Serialize)]
pub struct DiagnosticEvent {
    pub timestamp: String,
    pub correlation_id: String,
    pub event_type: String,
    pub component: String,
    pub message: String,
    pub context: serde_json::Value,
}

impl DiagnosticEvent {
    pub fn new(
        correlation_id: String,
        event_type: String,
        component: String,
        message: String,
    ) -> Self {
        Self {
            timestamp: Local::now().to_rfc3339(),
            correlation_id,
            event_type,
            component,
            message,
            context: serde_json::json!({}),
        }
    }

    pub fn with_context(mut self, key: &str, value: serde_json::Value) -> Self {
        if let Some(obj) = self.context.as_object_mut() {
            obj.insert(key.to_string(), value);
        }
        self
    }

    pub fn log(&self) {
        let json = serde_json::to_string(self).unwrap_or_else(|_| {
            format!(
                "[{}] {} - {}",
                self.correlation_id, self.event_type, self.message
            )
        });
        info!("{}", json);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_correlation_id_generation() {
        let id1 = correlation::generate();
        let id2 = correlation::generate();
        assert_ne!(id1, id2);
        assert!(id1.starts_with("corr-"));
    }

    #[test]
    fn test_diagnostic_event_serialization() {
        let event = DiagnosticEvent::new(
            "corr-123".to_string(),
            "capture.started".to_string(),
            "camera".to_string(),
            "Capture initiated".to_string(),
        )
        .with_context("sessionName", serde_json::json!("test-session"));

        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("corr-123"));
        assert!(json.contains("test-session"));
    }
}
