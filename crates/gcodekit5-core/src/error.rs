//! Error handling for GCodeKit4
//!
//! Provides comprehensive error types for all layers of the application:
//! - Controller errors (device/firmware related)
//! - G-Code errors (parsing/validation)
//! - Connection errors (communication)
//! - Firmware errors (firmware-specific)
//!
//! All error types use `thiserror` for ergonomic error handling.

use thiserror::Error;

/// Controller error type
///
/// Represents errors related to CNC controller operation,
/// including state machine violations, command failures, and device issues.
#[derive(Error, Debug, Clone)]
pub enum ControllerError {
    /// Controller is not connected
    #[error("Controller not connected")]
    NotConnected,

    /// Controller is already connected
    #[error("Controller already connected")]
    AlreadyConnected,

    /// Controller operation timed out
    #[error("Controller operation timed out after {timeout_ms}ms")]
    Timeout {
        /// The timeout duration in milliseconds.
        timeout_ms: u64,
    },

    /// Invalid state transition
    #[error("Invalid state transition from {current:?} to {requested:?}")]
    InvalidStateTransition {
        /// The current state name.
        current: String,
        /// The requested state name.
        requested: String,
    },

    /// Command was rejected by controller
    #[error("Command rejected: {reason}")]
    CommandRejected {
        /// The reason the command was rejected.
        reason: String,
    },

    /// Buffer overflow - too many commands queued
    #[error("Buffer overflow: {message}")]
    BufferOverflow {
        /// A message describing the buffer overflow.
        message: String,
    },

    /// Alarm condition detected
    #[error("Alarm: {code} - {message}")]
    Alarm {
        /// The alarm code.
        code: u32,
        /// The alarm message.
        message: String,
    },

    /// Machine hard limit triggered
    #[error("Hard limit triggered on {axis}")]
    HardLimit {
        /// The axis that triggered the hard limit.
        axis: String,
    },

    /// Machine soft limit exceeded
    #[error("Soft limit exceeded on {axis}")]
    SoftLimit {
        /// The axis that exceeded the soft limit.
        axis: String,
    },

    /// Probe operation failed
    #[error("Probe failed: {reason}")]
    ProbeFailed {
        /// The reason the probe operation failed.
        reason: String,
    },

    /// Homing cycle failed
    #[error("Homing failed: {reason}")]
    HomingFailed {
        /// The reason the homing cycle failed.
        reason: String,
    },

    /// Unknown controller state
    #[error("Unknown controller state: {state}")]
    UnknownState {
        /// The unknown state identifier.
        state: String,
    },

    /// Generic controller error
    #[error("Controller error: {message}")]
    Other {
        /// The error message.
        message: String,
    },
}

/// G-Code error type
///
/// Represents errors related to G-Code parsing, validation, and processing.
#[derive(Error, Debug, Clone)]
pub enum GcodeError {
    /// Invalid G-Code syntax
    #[error("Invalid syntax at line {line_number}: {reason}")]
    InvalidSyntax {
        /// The line number where the syntax error occurred.
        line_number: u32,
        /// The reason for the syntax error.
        reason: String,
    },

    /// Unknown G-Code command
    #[error("Unknown G-Code at line {line_number}: {code}")]
    UnknownCode {
        /// The line number where the unknown code was found.
        line_number: u32,
        /// The unknown G-Code command.
        code: String,
    },

    /// Invalid parameter value
    #[error("Invalid parameter '{param}' at line {line_number}: {reason}")]
    InvalidParameter {
        /// The line number where the invalid parameter was found.
        line_number: u32,
        /// The parameter name.
        param: String,
        /// The reason the parameter is invalid.
        reason: String,
    },

    /// Missing required parameter
    #[error("Missing required parameter '{param}' at line {line_number}")]
    MissingParameter {
        /// The line number where the parameter was missing.
        line_number: u32,
        /// The name of the missing parameter.
        param: String,
    },

    /// Coordinate out of machine limits
    #[error("Coordinate {coordinate} out of limits at line {line_number}: {bounds}")]
    CoordinateOutOfBounds {
        /// The line number where the out-of-bounds coordinate was found.
        line_number: u32,
        /// The coordinate value that is out of bounds.
        coordinate: String,
        /// The valid bounds for the coordinate.
        bounds: String,
    },

    /// Invalid modal state
    #[error("Invalid modal state: {reason}")]
    InvalidModalState {
        /// The reason for the invalid modal state.
        reason: String,
    },

    /// Tool not found
    #[error("Tool {tool_number} not found")]
    ToolNotFound {
        /// The tool number that was not found.
        tool_number: u32,
    },

    /// Probe not present when required
    #[error("Probe required but not available")]
    ProbeNotAvailable,

    /// Spindle error
    #[error("Spindle error: {reason}")]
    SpindleError {
        /// The reason for the spindle error.
        reason: String,
    },

    /// Coolant system error
    #[error("Coolant error: {reason}")]
    CoolantError {
        /// The reason for the coolant error.
        reason: String,
    },

    /// File parsing error
    #[error("File error: {reason}")]
    FileError {
        /// The reason for the file error.
        reason: String,
    },

    /// Generic G-Code error
    #[error("G-Code error: {message}")]
    Other {
        /// The error message.
        message: String,
    },
}

/// Connection error type
///
/// Represents errors related to communication with CNC controllers,
/// including serial port, TCP, and WebSocket connection issues.
#[derive(Error, Debug, Clone)]
pub enum ConnectionError {
    /// Port not found
    #[error("Port not found: {port}")]
    PortNotFound {
        /// The name of the port that was not found.
        port: String,
    },

    /// Port is already in use
    #[error("Port already in use: {port}")]
    PortInUse {
        /// The name of the port that is in use.
        port: String,
    },

    /// Failed to open port
    #[error("Failed to open port {port}: {reason}")]
    FailedToOpen {
        /// The name of the port that failed to open.
        port: String,
        /// The reason the port failed to open.
        reason: String,
    },

    /// Connection timeout
    #[error("Connection timeout after {timeout_ms}ms")]
    ConnectionTimeout {
        /// The timeout duration in milliseconds.
        timeout_ms: u64,
    },

    /// Connection lost
    #[error("Connection lost: {reason}")]
    ConnectionLost {
        /// The reason the connection was lost.
        reason: String,
    },

    /// Invalid hostname/IP
    #[error("Invalid hostname: {hostname}")]
    InvalidHostname {
        /// The invalid hostname or IP address.
        hostname: String,
    },

    /// Failed to resolve hostname
    #[error("Failed to resolve hostname {hostname}")]
    HostnameResolution {
        /// The hostname that failed to resolve.
        hostname: String,
    },

    /// TCP connection error
    #[error("TCP connection error: {reason}")]
    TcpError {
        /// The reason for the TCP error.
        reason: String,
    },

    /// WebSocket error
    #[error("WebSocket error: {reason}")]
    WebSocketError {
        /// The reason for the WebSocket error.
        reason: String,
    },

    /// Serial port error
    #[error("Serial port error: {reason}")]
    SerialError {
        /// The reason for the serial port error.
        reason: String,
    },

    /// Baud rate not supported
    #[error("Baud rate {baud} not supported")]
    UnsupportedBaudRate {
        /// The unsupported baud rate.
        baud: u32,
    },

    /// I/O error
    #[error("I/O error: {reason}")]
    IoError {
        /// The reason for the I/O error.
        reason: String,
    },

    /// Invalid connection parameters
    #[error("Invalid connection parameters: {reason}")]
    InvalidParameters {
        /// The reason the parameters are invalid.
        reason: String,
    },

    /// Generic connection error
    #[error("Connection error: {message}")]
    Other {
        /// The error message.
        message: String,
    },
}

/// Firmware error type
///
/// Represents errors specific to firmware implementations and protocols.
#[derive(Error, Debug, Clone)]
pub enum FirmwareError {
    /// Unknown firmware type
    #[error("Unknown firmware type: {firmware_type}")]
    UnknownFirmware {
        /// The unknown firmware type identifier.
        firmware_type: String,
    },

    /// Firmware version not supported
    #[error("Firmware version {version} not supported")]
    UnsupportedVersion {
        /// The unsupported firmware version.
        version: String,
    },

    /// Protocol mismatch
    #[error("Protocol mismatch: expected {expected}, got {actual}")]
    ProtocolMismatch {
        /// The expected protocol version.
        expected: String,
        /// The actual protocol version received.
        actual: String,
    },

    /// Unsupported feature
    #[error("Feature not supported by {firmware}: {feature}")]
    UnsupportedFeature {
        /// The firmware that does not support the feature.
        firmware: String,
        /// The unsupported feature name.
        feature: String,
    },

    /// Settings not available
    #[error("Setting {setting} not available")]
    SettingNotAvailable {
        /// The setting that is not available.
        setting: String,
    },

    /// Invalid setting value
    #[error("Invalid setting value for {setting}: {reason}")]
    InvalidSettingValue {
        /// The setting with the invalid value.
        setting: String,
        /// The reason the value is invalid.
        reason: String,
    },

    /// Capability not available
    #[error("Capability not available: {capability}")]
    CapabilityNotAvailable {
        /// The capability that is not available.
        capability: String,
    },

    /// Response parsing error
    #[error("Failed to parse firmware response: {reason}")]
    ResponseParseError {
        /// The reason the response parsing failed.
        reason: String,
    },

    /// Command not supported by firmware
    #[error("Command not supported by {firmware}")]
    CommandNotSupported {
        /// The firmware that does not support the command.
        firmware: String,
    },

    /// Configuration error
    #[error("Firmware configuration error: {reason}")]
    ConfigurationError {
        /// The reason for the configuration error.
        reason: String,
    },

    /// Generic firmware error
    #[error("Firmware error: {message}")]
    Other {
        /// The error message.
        message: String,
    },
}

/// Main error type for GCodeKit4
///
/// A unified error type that can represent any error from all layers.
/// This is the primary error type used in public APIs.
#[derive(Error, Debug)]
pub enum Error {
    /// Controller error
    #[error(transparent)]
    Controller(#[from] ControllerError),

    /// G-Code error
    #[error(transparent)]
    Gcode(#[from] GcodeError),

    /// Connection error
    #[error(transparent)]
    Connection(#[from] ConnectionError),

    /// Firmware error
    #[error(transparent)]
    Firmware(#[from] FirmwareError),

    /// Standard I/O error
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Generic error
    #[error("{0}")]
    Other(String),
}

impl Error {
    /// Create an error from a string message
    pub fn other(msg: impl Into<String>) -> Self {
        Error::Other(msg.into())
    }

    /// Check if this is a timeout error
    pub fn is_timeout(&self) -> bool {
        matches!(
            self,
            Error::Controller(ControllerError::Timeout { .. })
                | Error::Connection(ConnectionError::ConnectionTimeout { .. })
        )
    }

    /// Check if this is a connection error
    pub fn is_connection_error(&self) -> bool {
        matches!(self, Error::Connection(_))
    }

    /// Check if this is a G-Code error
    pub fn is_gcode_error(&self) -> bool {
        matches!(self, Error::Gcode(_))
    }

    /// Check if this is a controller error
    pub fn is_controller_error(&self) -> bool {
        matches!(self, Error::Controller(_))
    }

    /// Check if this is a firmware error
    pub fn is_firmware_error(&self) -> bool {
        matches!(self, Error::Firmware(_))
    }
}

/// Result type using Error
pub type Result<T> = std::result::Result<T, Error>;

// Conversions between error types are automatic via `from` implementations
