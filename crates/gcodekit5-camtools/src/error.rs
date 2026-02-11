//! Error types for the CAM tools crate.
//!
//! This module provides structured error types for CAM tool operations,
//! parameter validation, and file format processing.

use std::io;
use thiserror::Error;

/// Errors that can occur during CAM tool operations.
#[derive(Error, Debug)]
pub enum CamToolError {
    /// Invalid parameters were provided to a CAM tool.
    #[error("Invalid parameters: {0}")]
    InvalidParameters(String),

    /// The requested file format is not supported.
    #[error("Unsupported file format: {0}")]
    UnsupportedFormat(String),

    /// File could not be loaded.
    #[error("Failed to load file: {0}")]
    LoadError(String),

    /// G-code generation failed.
    #[error("G-code generation failed: {0}")]
    GenerationFailed(String),

    /// A geometry operation failed during toolpath creation.
    #[error("Geometry error: {0}")]
    GeometryError(String),

    /// Image processing failed.
    #[error("Image processing error: {0}")]
    ImageError(String),

    /// I/O error during file operations.
    #[error("I/O error: {0}")]
    IoError(#[from] io::Error),

    /// JSON serialization/deserialization error.
    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),

    /// A parameter validation error occurred.
    #[error("Parameter error: {0}")]
    Parameter(#[from] ParameterError),

    /// A file format error occurred.
    #[error("File format error: {0}")]
    FileFormat(#[from] FileFormatError),
}

/// Errors related to CAM tool parameter validation.
#[derive(Error, Debug)]
pub enum ParameterError {
    /// A required parameter is missing.
    #[error("Missing required parameter: {0}")]
    Missing(String),

    /// A parameter value is out of the valid range.
    #[error("Parameter '{name}' out of range: {value} (valid: {min}..{max})")]
    OutOfRange {
        name: String,
        value: f64,
        min: f64,
        max: f64,
    },

    /// A parameter value is invalid.
    #[error("Invalid value for '{name}': {reason}")]
    InvalidValue { name: String, reason: String },

    /// Parameters are mutually incompatible.
    #[error("Incompatible parameters: {0}")]
    Incompatible(String),

    /// Dimensions are invalid (zero or negative).
    #[error("Invalid dimensions: {0}")]
    InvalidDimensions(String),
}

/// Errors related to file format parsing and conversion.
#[derive(Error, Debug)]
pub enum FileFormatError {
    /// The SVG file could not be parsed.
    #[error("SVG parse error: {0}")]
    SvgParseError(String),

    /// The DXF file could not be parsed.
    #[error("DXF parse error: {0}")]
    DxfParseError(String),

    /// The Gerber file could not be parsed.
    #[error("Gerber parse error: {0}")]
    GerberParseError(String),

    /// The file is empty or contains no usable data.
    #[error("Empty file: {0}")]
    EmptyFile(String),

    /// The file extension is not recognized.
    #[error("Unknown file extension: {0}")]
    UnknownExtension(String),

    /// I/O error during file reading.
    #[error("I/O error: {0}")]
    IoError(#[from] io::Error),
}

/// Result type alias for CAM tool operations.
pub type CamToolResult<T> = Result<T, CamToolError>;

/// Result type alias for parameter validation.
pub type ParameterResult<T> = Result<T, ParameterError>;

/// Result type alias for file format operations.
pub type FileFormatResult<T> = Result<T, FileFormatError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cam_tool_error_display() {
        let err = CamToolError::InvalidParameters("depth must be positive".to_string());
        assert_eq!(
            err.to_string(),
            "Invalid parameters: depth must be positive"
        );

        let err = CamToolError::UnsupportedFormat("bmp".to_string());
        assert_eq!(err.to_string(), "Unsupported file format: bmp");

        let err = CamToolError::GenerationFailed("empty toolpath".to_string());
        assert_eq!(err.to_string(), "G-code generation failed: empty toolpath");
    }

    #[test]
    fn test_parameter_error_display() {
        let err = ParameterError::OutOfRange {
            name: "depth".to_string(),
            value: -5.0,
            min: 0.0,
            max: 100.0,
        };
        assert_eq!(
            err.to_string(),
            "Parameter 'depth' out of range: -5 (valid: 0..100)"
        );

        let err = ParameterError::Missing("tool_diameter".to_string());
        assert_eq!(err.to_string(), "Missing required parameter: tool_diameter");
    }

    #[test]
    fn test_file_format_error_display() {
        let err = FileFormatError::SvgParseError("invalid path data".to_string());
        assert_eq!(err.to_string(), "SVG parse error: invalid path data");

        let err = FileFormatError::UnknownExtension(".xyz".to_string());
        assert_eq!(err.to_string(), "Unknown file extension: .xyz");
    }

    #[test]
    fn test_error_conversion() {
        let param_err = ParameterError::Missing("width".to_string());
        let cam_err: CamToolError = param_err.into();
        assert!(matches!(cam_err, CamToolError::Parameter(_)));

        let fmt_err = FileFormatError::EmptyFile("test.svg".to_string());
        let cam_err: CamToolError = fmt_err.into();
        assert!(matches!(cam_err, CamToolError::FileFormat(_)));
    }

    #[test]
    fn test_io_error_conversion() {
        let io_err = io::Error::new(io::ErrorKind::NotFound, "file not found");
        let cam_err: CamToolError = io_err.into();
        assert!(matches!(cam_err, CamToolError::IoError(_)));

        let io_err = io::Error::new(io::ErrorKind::PermissionDenied, "access denied");
        let fmt_err: FileFormatError = io_err.into();
        assert!(matches!(fmt_err, FileFormatError::IoError(_)));
    }
}
