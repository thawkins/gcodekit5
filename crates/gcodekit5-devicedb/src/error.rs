//! Error types for the device database crate.
//!
//! This module provides structured error types for device profile management,
//! persistence, and validation.

use std::io;
use thiserror::Error;

/// Errors that can occur during device management operations.
#[derive(Error, Debug)]
pub enum DeviceError {
    /// The requested profile was not found.
    #[error("Profile not found: {0}")]
    ProfileNotFound(String),

    /// A profile with this ID already exists.
    #[error("Profile already exists: {0}")]
    ProfileAlreadyExists(String),

    /// The profile data is invalid.
    #[error("Invalid profile: {0}")]
    InvalidProfile(String),

    /// Failed to load profiles from storage.
    #[error("Failed to load profiles: {0}")]
    LoadError(String),

    /// Failed to save profiles to storage.
    #[error("Failed to save profiles: {0}")]
    SaveError(String),

    /// I/O error during file operations.
    #[error("I/O error: {0}")]
    IoError(#[from] io::Error),

    /// JSON serialization/deserialization error.
    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),

    /// A profile validation error occurred.
    #[error("Validation error: {0}")]
    Validation(#[from] ProfileError),
}

/// Errors related to device profile validation.
#[derive(Error, Debug)]
pub enum ProfileError {
    /// A required field is missing or empty.
    #[error("Missing required field: {0}")]
    MissingField(String),

    /// The device type is unknown or unsupported.
    #[error("Unknown device type: {0}")]
    UnknownDeviceType(String),

    /// The controller type is unknown or unsupported.
    #[error("Unknown controller type: {0}")]
    UnknownControllerType(String),

    /// An axis limit value is invalid.
    #[error("Invalid axis limit for {axis}: {reason}")]
    InvalidAxisLimit { axis: String, reason: String },

    /// The baud rate is not supported.
    #[error("Unsupported baud rate: {0}")]
    UnsupportedBaudRate(u32),

    /// A numeric value is out of valid range.
    #[error("Value out of range for '{field}': {value}")]
    ValueOutOfRange { field: String, value: String },
}

/// Result type alias for device management operations.
pub type DeviceResult<T> = Result<T, DeviceError>;

/// Result type alias for profile validation operations.
pub type ProfileResult<T> = Result<T, ProfileError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_device_error_display() {
        let err = DeviceError::ProfileNotFound("machine-001".to_string());
        assert_eq!(err.to_string(), "Profile not found: machine-001");

        let err = DeviceError::ProfileAlreadyExists("cnc-router".to_string());
        assert_eq!(err.to_string(), "Profile already exists: cnc-router");

        let err = DeviceError::LoadError("corrupted JSON".to_string());
        assert_eq!(err.to_string(), "Failed to load profiles: corrupted JSON");
    }

    #[test]
    fn test_profile_error_display() {
        let err = ProfileError::UnknownDeviceType("quantum_mill".to_string());
        assert_eq!(err.to_string(), "Unknown device type: quantum_mill");

        let err = ProfileError::InvalidAxisLimit {
            axis: "X".to_string(),
            reason: "max less than min".to_string(),
        };
        assert_eq!(
            err.to_string(),
            "Invalid axis limit for X: max less than min"
        );

        let err = ProfileError::UnsupportedBaudRate(999);
        assert_eq!(err.to_string(), "Unsupported baud rate: 999");
    }

    #[test]
    fn test_error_conversion() {
        let profile_err = ProfileError::MissingField("name".to_string());
        let device_err: DeviceError = profile_err.into();
        assert!(matches!(device_err, DeviceError::Validation(_)));

        let io_err = io::Error::new(io::ErrorKind::NotFound, "file not found");
        let device_err: DeviceError = io_err.into();
        assert!(matches!(device_err, DeviceError::IoError(_)));
    }
}
