use chrono::{DateTime, Utc};
use log::info;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

/// Current preset snapshot (source for subsequent imports)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PresetSnapshot {
    pub preset_id: String,
    pub preset_name: Option<String>,
    pub adjustments: Value,
    pub selected_at: DateTime<Utc>,
}

/// Boothy per-image metadata stored in .rrdata
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BoothyImageMetadata {
    pub preset_id: String,
    pub preset_name: Option<String>,
    pub applied_at: DateTime<Utc>,
}

/// Preset assignment service
/// Manages the current preset and applies it to newly imported photos
pub struct PresetManager {
    /// Current preset snapshot (used for new imports)
    current_preset: Arc<Mutex<Option<PresetSnapshot>>>,
}

impl PresetManager {
    pub fn new() -> Self {
        Self {
            current_preset: Arc::new(Mutex::new(None)),
        }
    }

    /// Set the current preset snapshot
    /// This preset will be applied to all subsequently imported photos
    pub fn set_current_preset(
        &self,
        preset_id: String,
        preset_name: Option<String>,
        adjustments: Value,
        correlation_id: &str,
    ) {
        let snapshot = PresetSnapshot {
            preset_id: preset_id.clone(),
            preset_name: preset_name.clone(),
            adjustments,
            selected_at: Utc::now(),
        };

        info!(
            "[{}] Setting current preset: {} ({})",
            correlation_id,
            preset_name.as_deref().unwrap_or("unnamed"),
            preset_id
        );

        let mut current = self.current_preset.lock().unwrap();
        *current = Some(snapshot);
    }

    /// Get the current preset snapshot
    pub fn get_current_preset(&self) -> Option<PresetSnapshot> {
        self.current_preset.lock().unwrap().clone()
    }

    /// Clear the current preset (no preset will be applied to new imports)
    pub fn clear_current_preset(&self, correlation_id: &str) {
        info!("[{}] Clearing current preset", correlation_id);
        let mut current = self.current_preset.lock().unwrap();
        *current = None;
    }

    /// Apply the current preset to a newly imported photo
    /// Updates the .rrdata file with preset metadata and adjustments snapshot
    pub fn apply_preset_on_import(
        &self,
        image_path: &Path,
        correlation_id: &str,
    ) -> Result<(), String> {
        // Get current preset
        let preset = match self.get_current_preset() {
            Some(p) => p,
            None => {
                info!(
                    "[{}] No preset selected, skipping preset application for: {}",
                    correlation_id,
                    image_path.display()
                );
                return Ok(());
            }
        };

        info!(
            "[{}] Applying preset '{}' to: {}",
            correlation_id,
            preset.preset_name.as_deref().unwrap_or("unnamed"),
            image_path.display()
        );

        // Get .rrdata file path
        let rrdata_path = Self::get_rrdata_path(image_path);

        // Load existing .rrdata or create new
        let mut rrdata = if rrdata_path.exists() {
            Self::load_rrdata(&rrdata_path)?
        } else {
            serde_json::json!({
                "version": "1.0",
                "adjustments": {}
            })
        };

        // Ensure adjustments object exists
        if !rrdata.get("adjustments").is_some() {
            rrdata["adjustments"] = serde_json::json!({});
        }

        // Store Boothy metadata under adjustments.boothy (to avoid collision with RapidRAW keys)
        let boothy_metadata = BoothyImageMetadata {
            preset_id: preset.preset_id.clone(),
            preset_name: preset.preset_name.clone(),
            applied_at: Utc::now(),
        };

        rrdata["adjustments"]["boothy"] = serde_json::to_value(boothy_metadata)
            .map_err(|e| format!("Failed to serialize Boothy metadata: {}", e))?;

        // Merge preset adjustments into top-level adjustments (snapshot)
        // This allows RapidRAW to see the actual adjustment values
        if let Some(preset_adjustments) = preset.adjustments.as_object() {
            if let Some(adjustments) = rrdata["adjustments"].as_object_mut() {
                for (key, value) in preset_adjustments {
                    // Skip internal keys
                    if key == "boothy" {
                        continue;
                    }
                    // Append preset values (non-destructive)
                    adjustments.insert(key.clone(), value.clone());
                }
            }
        }

        // Write .rrdata back
        Self::save_rrdata(&rrdata_path, &rrdata)?;

        info!(
            "[{}] Preset applied successfully: {}",
            correlation_id,
            image_path.display()
        );

        Ok(())
    }

    /// Get the .rrdata file path for an image
    fn get_rrdata_path(image_path: &Path) -> PathBuf {
        let mut rrdata_path = image_path.to_path_buf();
        let filename = image_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown");
        rrdata_path.set_file_name(format!("{}.rrdata", filename));
        rrdata_path
    }

    /// Load .rrdata file
    fn load_rrdata(path: &Path) -> Result<Value, String> {
        let contents =
            fs::read_to_string(path).map_err(|e| format!("Failed to read .rrdata: {}", e))?;

        serde_json::from_str(&contents).map_err(|e| format!("Failed to parse .rrdata JSON: {}", e))
    }

    /// Save .rrdata file
    fn save_rrdata(path: &Path, data: &Value) -> Result<(), String> {
        let json = serde_json::to_string_pretty(data)
            .map_err(|e| format!("Failed to serialize .rrdata: {}", e))?;

        fs::write(path, json).map_err(|e| format!("Failed to write .rrdata: {}", e))?;

        Ok(())
    }
}

impl Default for PresetManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;
    use tempfile::TempDir;

    #[test]
    fn test_set_and_get_preset() {
        let manager = PresetManager::new();

        assert!(manager.get_current_preset().is_none());

        let adjustments = serde_json::json!({
            "exposure": 0.5,
            "contrast": 1.2
        });

        manager.set_current_preset(
            "preset-123".to_string(),
            Some("My Preset".to_string()),
            adjustments.clone(),
            "test",
        );

        let preset = manager.get_current_preset().unwrap();
        assert_eq!(preset.preset_id, "preset-123");
        assert_eq!(preset.preset_name, Some("My Preset".to_string()));
        assert_eq!(preset.adjustments, adjustments);
    }

    #[test]
    fn test_apply_preset_on_import() {
        let temp_dir = TempDir::new().unwrap();
        let image_path = temp_dir.path().join("test.CR3");

        // Create a dummy image file
        let mut file = File::create(&image_path).unwrap();
        file.write_all(b"dummy image data").unwrap();
        drop(file);

        let manager = PresetManager::new();

        // Set preset
        let adjustments = serde_json::json!({
            "exposure": 0.5,
            "contrast": 1.2
        });

        manager.set_current_preset(
            "preset-123".to_string(),
            Some("Test Preset".to_string()),
            adjustments,
            "test",
        );

        // Apply preset
        manager.apply_preset_on_import(&image_path, "test").unwrap();

        // Verify .rrdata was created
        let rrdata_path = temp_dir.path().join("test.CR3.rrdata");
        assert!(rrdata_path.exists());

        // Load and verify content
        let rrdata = PresetManager::load_rrdata(&rrdata_path).unwrap();

        assert_eq!(rrdata["adjustments"]["exposure"], serde_json::json!(0.5));
        assert_eq!(rrdata["adjustments"]["contrast"], serde_json::json!(1.2));

        let boothy_metadata: BoothyImageMetadata =
            serde_json::from_value(rrdata["adjustments"]["boothy"].clone()).unwrap();
        assert_eq!(boothy_metadata.preset_id, "preset-123");
        assert_eq!(boothy_metadata.preset_name, Some("Test Preset".to_string()));
    }
}
