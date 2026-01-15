use super::models::BoothySession;
use super::sanitizer::sanitize_session_name;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Emitter};

pub struct SessionManager {
    pub active_session: Arc<Mutex<Option<BoothySession>>>,
}

impl SessionManager {
    pub fn new() -> Self {
        Self {
            active_session: Arc::new(Mutex::new(None)),
        }
    }

    /// Get the sessions root directory: %USERPROFILE%\Pictures\dabi_shoot
    fn get_sessions_root() -> Result<PathBuf, String> {
        let user_profile = std::env::var("USERPROFILE")
            .map_err(|_| "USERPROFILE environment variable not found".to_string())?;

        let sessions_root = PathBuf::from(user_profile)
            .join("Pictures")
            .join("dabi_shoot");

        Ok(sessions_root)
    }

    fn create_or_open_session_in_root(
        session_name: String,
        sessions_root: &Path,
    ) -> Result<(BoothySession, bool), String> {
        // Sanitize session name
        let session_folder_name = sanitize_session_name(&session_name)?;

        // Ensure sessions root exists
        if !sessions_root.exists() {
            fs::create_dir_all(&sessions_root)
                .map_err(|e| format!("Failed to create sessions root directory: {}", e))?;
        }

        // Session base path
        let session_base_path = sessions_root.join(&session_folder_name);

        // Check if session folder already exists (open existing)
        let session_exists = session_base_path.exists();

        // Create session folder if it doesn't exist
        if !session_exists {
            fs::create_dir(&session_base_path)
                .map_err(|e| format!("Failed to create session directory: {}", e))?;
        }

        // Ensure Raw/ and Jpg/ subdirectories exist
        let raw_path = session_base_path.join("Raw");
        let jpg_path = session_base_path.join("Jpg");

        if !raw_path.exists() {
            fs::create_dir(&raw_path)
                .map_err(|e| format!("Failed to create Raw subdirectory: {}", e))?;
        }

        if !jpg_path.exists() {
            fs::create_dir(&jpg_path)
                .map_err(|e| format!("Failed to create Jpg subdirectory: {}", e))?;
        }

        // Create session model
        let session = BoothySession::new(
            session_name,
            session_folder_name,
            session_base_path,
        );

        Ok((session, session_exists))
    }

    /// Create or open a session by name
    ///
    /// Collision policy (MVP): If session folder already exists, open/activate it
    /// Creates Raw/ and Jpg/ subdirectories if missing
    pub fn create_or_open_session(
        &self,
        session_name: String,
        app_handle: &AppHandle,
    ) -> Result<BoothySession, String> {
        // Get sessions root
        let sessions_root = Self::get_sessions_root()?;

        let (session, session_exists) =
            Self::create_or_open_session_in_root(session_name, &sessions_root)?;

        // Set as active session
        {
            let mut active = self.active_session.lock().unwrap();
            *active = Some(session.clone());
        }

        // Update settings to constrain library to session Raw/ folder
        if let Ok(mut settings) = crate::file_management::load_settings(app_handle.clone()) {
            settings.boothy_last_session = Some(session.session_folder_name.clone());
            settings.last_root_path = Some(session.raw_path.to_string_lossy().to_string());
            let _ = crate::file_management::save_settings(settings, app_handle.clone());
        }

        // Emit session change event
        let _ = app_handle.emit("boothy-session-changed", &session);

        // Log session creation/opening (session names redacted for security)
        log::info!(
            "Session {} - library constrained to Raw/",
            if session_exists { "opened" } else { "created" }
        );

        Ok(session)
    }

    /// Get the current active session
    pub fn get_active_session(&self) -> Option<BoothySession> {
        self.active_session.lock().unwrap().clone()
    }

    /// Restore the last active session from settings (called on app startup)
    pub fn restore_last_session(
        &self,
        app_handle: &AppHandle,
    ) -> Result<Option<BoothySession>, String> {
        let settings = crate::file_management::load_settings(app_handle.clone())
            .map_err(|e| format!("Failed to load settings: {}", e))?;

        if let Some(last_session_name) = settings.boothy_last_session {
            log::info!("Restoring last session");
            match self.create_or_open_session(last_session_name, app_handle) {
                Ok(session) => Ok(Some(session)),
                Err(e) => {
                    log::warn!("Failed to restore last session: {}", e);
                    Ok(None)
                }
            }
        } else {
            Ok(None)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn create_or_open_session_creates_expected_paths() {
        let temp_dir = tempdir().unwrap();
        let (session, existed) = SessionManager::create_or_open_session_in_root(
            "Test Session".to_string(),
            temp_dir.path(),
        )
        .unwrap();

        assert!(!existed);
        assert!(session.base_path.exists());
        assert!(session.raw_path.exists());
        assert!(session.jpg_path.exists());
        assert_eq!(session.session_folder_name, "Test-Session");
    }

    #[test]
    fn create_or_open_session_reuses_existing_folder() {
        let temp_dir = tempdir().unwrap();
        let (first, existed_first) = SessionManager::create_or_open_session_in_root(
            "Repeat Session".to_string(),
            temp_dir.path(),
        )
        .unwrap();

        let (second, existed_second) = SessionManager::create_or_open_session_in_root(
            "Repeat Session".to_string(),
            temp_dir.path(),
        )
        .unwrap();

        assert!(!existed_first);
        assert!(existed_second);
        assert_eq!(first.base_path, second.base_path);
        assert_eq!(first.session_folder_name, second.session_folder_name);
    }
}

impl Default for SessionManager {
    fn default() -> Self {
        Self::new()
    }
}
