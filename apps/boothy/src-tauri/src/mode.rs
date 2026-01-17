use argon2::{
    Argon2,
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString, rand_core::OsRng},
};
use log::{error, info, warn};
use serde::{Deserialize, Serialize};
use std::sync::MutexGuard;
use std::sync::{Arc, Mutex};

fn lock_or_recover<'a, T>(mutex: &'a Mutex<T>, label: &str) -> MutexGuard<'a, T> {
    match mutex.lock() {
        Ok(guard) => guard,
        Err(poisoned) => {
            error!("Mutex poisoned for {}. Recovering inner state.", label);
            poisoned.into_inner()
        }
    }
}

/// Boothy operating mode (customer vs admin)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BoothyMode {
    /// Customer mode: minimal booth UI only
    /// - Preset selection
    /// - Capture trigger
    /// - Thumbnail selection
    /// - Export image button
    /// - Delete
    /// Advanced controls are HIDDEN (not disabled)
    Customer,

    /// Admin mode: full feature set visible
    /// - All customer features
    /// - Full camera controls (digiCamControl-equivalent)
    /// - RapidRAW advanced panels (including rotate)
    Admin,
}

impl Default for BoothyMode {
    fn default() -> Self {
        BoothyMode::Customer
    }
}

/// Mode manager state (single source of truth)
pub struct ModeManager {
    /// Current operating mode
    current_mode: Arc<Mutex<BoothyMode>>,

    /// Admin password hash (Argon2 with salt)
    admin_password_hash: Arc<Mutex<Option<String>>>,
}

impl ModeManager {
    /// Create a new mode manager (starts in Customer mode)
    pub fn new() -> Self {
        Self {
            current_mode: Arc::new(Mutex::new(BoothyMode::Customer)),
            admin_password_hash: Arc::new(Mutex::new(None)),
        }
    }

    /// Get the current mode
    pub fn get_mode(&self) -> BoothyMode {
        *lock_or_recover(&self.current_mode, "current_mode")
    }

    /// Check if currently in admin mode
    pub fn is_admin(&self) -> bool {
        self.get_mode() == BoothyMode::Admin
    }

    /// Set admin password (stores salted hash only)
    pub fn set_admin_password(&self, password: &str) -> Result<(), String> {
        let salt = SaltString::generate(&mut OsRng);
        let argon2 = Argon2::default();

        let password_hash = argon2
            .hash_password(password.as_bytes(), &salt)
            .map_err(|e| format!("Failed to hash password: {}", e))?
            .to_string();

        *lock_or_recover(&self.admin_password_hash, "admin_password_hash") = Some(password_hash);
        info!("Admin password set successfully");
        Ok(())
    }

    /// Verify admin password and switch to admin mode if correct
    pub fn authenticate(&self, password: &str) -> Result<bool, String> {
        let stored_hash_opt: Option<String> = {
            let guard = lock_or_recover(&self.admin_password_hash, "admin_password_hash");
            (*guard).clone()
        };

        if stored_hash_opt.is_none() {
            warn!("Authentication attempted but no admin password is set");
            return Ok(false);
        }

        let stored_hash = stored_hash_opt.unwrap();

        let parsed_hash = PasswordHash::new(&stored_hash)
            .map_err(|e| format!("Failed to parse stored hash: {}", e))?;

        let argon2 = Argon2::default();
        let is_valid = argon2
            .verify_password(password.as_bytes(), &parsed_hash)
            .is_ok();

        if is_valid {
            *lock_or_recover(&self.current_mode, "current_mode") = BoothyMode::Admin;
            info!("Admin authentication successful");
        } else {
            warn!("Admin authentication failed");
        }

        Ok(is_valid)
    }

    /// Switch to customer mode (always succeeds)
    pub fn switch_to_customer_mode(&self) {
        *lock_or_recover(&self.current_mode, "current_mode") = BoothyMode::Customer;
        info!("Switched to Customer mode");
    }

    /// Check if admin password is set
    pub fn has_admin_password(&self) -> bool {
        lock_or_recover(&self.admin_password_hash, "admin_password_hash").is_some()
    }

    /// Get the admin password hash (for persistence to settings)
    pub fn get_password_hash(&self) -> Option<String> {
        let guard = lock_or_recover(&self.admin_password_hash, "admin_password_hash");
        (*guard).clone()
    }

    /// Load admin password hash from settings (called on startup)
    pub fn load_password_hash(&self, hash: String) {
        *lock_or_recover(&self.admin_password_hash, "admin_password_hash") = Some(hash);
        info!("Admin password hash loaded from settings");
    }
}

impl Default for ModeManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Mode state response for UI
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModeState {
    pub mode: BoothyMode,
    pub has_admin_password: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_mode_is_customer() {
        let manager = ModeManager::new();
        assert_eq!(manager.get_mode(), BoothyMode::Customer);
        assert!(!manager.is_admin());
    }

    #[test]
    fn test_password_authentication() {
        let manager = ModeManager::new();
        manager.set_admin_password("test123").unwrap();

        // Correct password
        assert!(manager.authenticate("test123").unwrap());
        assert!(manager.is_admin());

        // Switch back to customer
        manager.switch_to_customer_mode();
        assert!(!manager.is_admin());

        // Wrong password
        assert!(!manager.authenticate("wrong").unwrap());
        assert!(!manager.is_admin());
    }

    #[test]
    fn test_password_hashing() {
        let manager = ModeManager::new();
        manager.set_admin_password("mypassword").unwrap();

        // Password hash should not be stored in plaintext
        let hash = manager.admin_password_hash.lock().unwrap();
        assert!(hash.is_some());
        assert!(!hash.as_ref().unwrap().contains("mypassword"));
        assert!(hash.as_ref().unwrap().starts_with("$argon2"));
    }
}
