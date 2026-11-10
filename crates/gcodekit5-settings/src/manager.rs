//! Settings Manager
//!
//! Locations of the configuration files.

use gcodekit5_core::{Error, Result};
use std::path::PathBuf;

/// Locations of the application's configuration files.
pub struct SettingsManager;

impl SettingsManager {
    /// Get platform-specific config directory
    pub fn config_directory() -> Result<PathBuf> {
        #[cfg(target_os = "windows")]
        {
            let appdata = std::env::var("APPDATA")
                .map_err(|_| Error::other("APPDATA environment variable not set".to_string()))?;
            Ok(PathBuf::from(appdata).join("gcodekit5"))
        }

        #[cfg(target_os = "macos")]
        {
            let home = std::env::var("HOME")
                .map_err(|_| Error::other("HOME environment variable not set".to_string()))?;
            Ok(PathBuf::from(home).join("Library/Application Support/gcodekit5"))
        }

        #[cfg(target_os = "linux")]
        {
            let home = std::env::var("HOME")
                .map_err(|_| Error::other("HOME environment variable not set".to_string()))?;
            Ok(PathBuf::from(home).join(".config/gcodekit5"))
        }

        #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
        {
            Err(Error::other("Unsupported platform".to_string()))
        }
    }

    /// Get config file path for platform
    pub fn config_file_path() -> Result<PathBuf> {
        let dir = Self::config_directory()?;
        Ok(dir.join("config.json"))
    }

    /// Ensure config directory exists
    pub fn ensure_config_dir() -> Result<PathBuf> {
        let dir = Self::config_directory()?;
        std::fs::create_dir_all(&dir)
            .map_err(|e| Error::other(format!("Failed to create config directory: {}", e)))?;
        Ok(dir)
    }
}
