//! Error types for the settings crate.
//!
//! This module provides structured error types for configuration management,
//! settings persistence, and validation.

use std::io;
use thiserror::Error;

/// Errors that can occur during settings operations.
#[derive(Error, Debug)]
pub enum SettingsError {
    /// The configuration file could not be loaded.
    #[error("Failed to load settings: {0}")]
    LoadError(String),

    /// The configuration file could not be saved.
    #[error("Failed to save settings: {0}")]
    SaveError(String),

    /// A configuration value is invalid.
    #[error("Invalid setting '{key}': {reason}")]
    InvalidSetting { key: String, reason: String },

    /// The configuration directory could not be found or created.
    #[error("Config directory error: {0}")]
    ConfigDirectory(String),

    /// I/O error during file operations.
    #[error("I/O error: {0}")]
    IoError(#[from] io::Error),

    /// JSON serialization/deserialization error.
    #[error("JSON error: {0}")]
    JsonError(#[from] serde_json::Error),

    /// TOML serialization/deserialization error.
    #[error("TOML error: {0}")]
    TomlError(#[from] toml::de::Error),

    /// A configuration validation error occurred.
    #[error("Config error: {0}")]
    Config(#[from] ConfigError),

    /// A persistence error occurred.
    #[error("Persistence error: {0}")]
    Persistence(#[from] PersistenceError),
}

/// Errors related to configuration validation.
#[derive(Error, Debug)]
pub enum ConfigError {
    /// A required configuration key is missing.
    #[error("Missing configuration key: {0}")]
    MissingKey(String),

    /// The configuration file format is not supported.
    #[error("Unsupported config format: {0}")]
    UnsupportedFormat(String),

    /// A configuration value is out of valid range.
    #[error("Value out of range for '{key}': {value}")]
    ValueOutOfRange { key: String, value: String },

    /// The configuration file is corrupted or malformed.
    #[error("Corrupted configuration: {0}")]
    Corrupted(String),

    /// Platform is not supported for config directory resolution.
    #[error("Unsupported platform: {0}")]
    UnsupportedPlatform(String),
}

/// Errors related to settings persistence operations.
#[derive(Error, Debug)]
pub enum PersistenceError {
    /// The settings dialog data is invalid.
    #[error("Invalid dialog data: {0}")]
    InvalidDialogData(String),

    /// A setting category is unknown.
    #[error("Unknown settings category: {0}")]
    UnknownCategory(String),

    /// Failed to validate persisted settings.
    #[error("Settings validation failed: {0}")]
    ValidationFailed(String),

    /// I/O error during persistence.
    #[error("I/O error: {0}")]
    IoError(#[from] io::Error),

    /// JSON error during persistence.
    #[error("JSON error: {0}")]
    JsonError(#[from] serde_json::Error),
}

/// Result type alias for settings operations.
pub type SettingsResult<T> = Result<T, SettingsError>;

/// Result type alias for configuration operations.
pub type ConfigResult<T> = Result<T, ConfigError>;

/// Result type alias for persistence operations.
pub type PersistenceResult<T> = Result<T, PersistenceError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_settings_error_display() {
        let err = SettingsError::LoadError("file not found".to_string());
        assert_eq!(err.to_string(), "Failed to load settings: file not found");

        let err = SettingsError::InvalidSetting {
            key: "baud_rate".to_string(),
            reason: "must be positive".to_string(),
        };
        assert_eq!(
            err.to_string(),
            "Invalid setting 'baud_rate': must be positive"
        );

        let err = SettingsError::ConfigDirectory("permission denied".to_string());
        assert_eq!(err.to_string(), "Config directory error: permission denied");
    }

    #[test]
    fn test_config_error_display() {
        let err = ConfigError::MissingKey("machine.max_x".to_string());
        assert_eq!(err.to_string(), "Missing configuration key: machine.max_x");

        let err = ConfigError::UnsupportedFormat("yaml".to_string());
        assert_eq!(err.to_string(), "Unsupported config format: yaml");

        let err = ConfigError::UnsupportedPlatform("wasm".to_string());
        assert_eq!(err.to_string(), "Unsupported platform: wasm");
    }

    #[test]
    fn test_persistence_error_display() {
        let err = PersistenceError::UnknownCategory("networking".to_string());
        assert_eq!(err.to_string(), "Unknown settings category: networking");

        let err = PersistenceError::ValidationFailed("max_x < min_x".to_string());
        assert_eq!(err.to_string(), "Settings validation failed: max_x < min_x");
    }

    #[test]
    fn test_error_conversion() {
        let config_err = ConfigError::MissingKey("theme".to_string());
        let settings_err: SettingsError = config_err.into();
        assert!(matches!(settings_err, SettingsError::Config(_)));

        let persist_err = PersistenceError::UnknownCategory("test".to_string());
        let settings_err: SettingsError = persist_err.into();
        assert!(matches!(settings_err, SettingsError::Persistence(_)));

        let io_err = io::Error::new(io::ErrorKind::NotFound, "file not found");
        let settings_err: SettingsError = io_err.into();
        assert!(matches!(settings_err, SettingsError::IoError(_)));
    }
}
