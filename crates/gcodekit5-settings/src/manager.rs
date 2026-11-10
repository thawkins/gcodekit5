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
        // The same directory as the device, tool and material files:
        // %APPDATA% on Windows, ~/Library/Application Support on macOS and
        // $XDG_CONFIG_HOME (default ~/.config) on Linux.
        dirs::config_dir()
            .map(|dir| dir.join("gcodekit5"))
            .ok_or_else(|| Error::other("No configuration directory for this platform"))
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
