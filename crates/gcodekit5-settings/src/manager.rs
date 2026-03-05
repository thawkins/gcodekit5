//! Settings Manager
//!
//! Manages configuration persistence and provides default settings for different firmware types.

use crate::config::{Config, ConnectionType};
use gcodekit5_core::{Error, Result};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Settings manager for different firmware types
///
/// Provides default settings for each supported firmware and manages configuration persistence.
pub struct SettingsManager {
    config: Config,
    // Reserved for firmware-specific settings storage (GRBL, TinyG, etc.).
    #[allow(dead_code)]
    firmware_settings: HashMap<String, Box<dyn std::any::Any>>,
}

impl SettingsManager {
    /// Create new settings manager with default config
    pub fn new() -> Self {
        Self {
            config: Config::default(),
            firmware_settings: HashMap::new(),
        }
    }

    /// Create settings manager with loaded config
    pub fn with_config(config: Config) -> Self {
        Self {
            config,
            firmware_settings: HashMap::new(),
        }
    }

    /// Load config from file
    pub fn load_from_file(path: &Path) -> Result<Self> {
        let config = Config::load_from_file(path)?;
        Ok(Self::with_config(config))
    }

    /// Get current config
    pub fn config(&self) -> &Config {
        &self.config
    }

    /// Get mutable config
    pub fn config_mut(&mut self) -> &mut Config {
        &mut self.config
    }

    /// Save config to file
    pub fn save_to_file(&self, path: &Path) -> Result<()> {
        self.config.save_to_file(path)
    }

    /// Get default settings for GRBL firmware
    pub fn default_grbl_settings() -> Config {
        let mut config = Config::default();
        config.connection.baud_rate = 115200;
        config.connection.timeout_ms = 5000;
        config.machine.x_limit = 200.0;
        config.machine.y_limit = 200.0;
        config.machine.z_limit = 100.0;
        config
    }

    /// Get default settings for TinyG firmware
    pub fn default_tinyg_settings() -> Config {
        let mut config = Config::default();
        config.connection.baud_rate = 115200;
        config.connection.timeout_ms = 5000;
        config.machine.x_limit = 250.0;
        config.machine.y_limit = 250.0;
        config.machine.z_limit = 150.0;
        config
    }

    /// Get default settings for g2core firmware
    pub fn default_g2core_settings() -> Config {
        let mut config = Config::default();
        config.connection.connection_type = ConnectionType::Tcp;
        config.connection.timeout_ms = 10000;
        config.machine.x_limit = 300.0;
        config.machine.y_limit = 300.0;
        config.machine.z_limit = 200.0;
        config
    }

    /// Get default settings for Smoothieware firmware
    pub fn default_smoothieware_settings() -> Config {
        let mut config = Config::default();
        config.connection.baud_rate = 115200;
        config.connection.timeout_ms = 5000;
        config.machine.x_limit = 200.0;
        config.machine.y_limit = 200.0;
        config.machine.z_limit = 100.0;
        config
    }

    /// Get default settings for FluidNC firmware
    pub fn default_fluidnc_settings() -> Config {
        let mut config = Config::default();
        config.connection.connection_type = ConnectionType::WebSocket;
        config.connection.timeout_ms = 10000;
        config.machine.x_limit = 300.0;
        config.machine.y_limit = 300.0;
        config.machine.z_limit = 200.0;
        config
    }

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

impl Default for SettingsManager {
    fn default() -> Self {
        Self::new()
    }
}
