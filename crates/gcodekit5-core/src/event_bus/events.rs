//! Event type definitions for the event bus.
//!
//! This module defines all application events organized by category.
//! Events are designed to be cloneable and serializable for logging/replay.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::Duration;

use crate::data::{ControllerState, Position, Units};

/// Root event enum for all application events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AppEvent {
    /// Machine connection events
    Connection(ConnectionEvent),
    /// Machine state and status
    Machine(MachineEvent),
    /// G-code file operations
    File(FileEvent),
    /// Communication layer events
    Communication(CommunicationEvent),
    /// User interface events
    Ui(UiEvent),
    /// Settings and configuration
    Settings(SettingsEvent),
    /// Error and diagnostic events
    Error(ErrorEvent),
}

impl AppEvent {
    /// Get the category of this event
    pub fn category(&self) -> EventCategory {
        match self {
            AppEvent::Connection(_) => EventCategory::Connection,
            AppEvent::Machine(_) => EventCategory::Machine,
            AppEvent::File(_) => EventCategory::File,
            AppEvent::Communication(_) => EventCategory::Communication,
            AppEvent::Ui(_) => EventCategory::Ui,
            AppEvent::Settings(_) => EventCategory::Settings,
            AppEvent::Error(_) => EventCategory::Error,
        }
    }

    /// Get a short description of this event for logging
    pub fn description(&self) -> String {
        match self {
            AppEvent::Connection(e) => e.description(),
            AppEvent::Machine(e) => e.description(),
            AppEvent::File(e) => e.description(),
            AppEvent::Communication(e) => e.description(),
            AppEvent::Ui(e) => e.description(),
            AppEvent::Settings(e) => e.description(),
            AppEvent::Error(e) => e.description(),
        }
    }
}

/// Event category for filtering
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EventCategory {
    /// Machine connection events.
    Connection,
    /// Machine state and status events.
    Machine,
    /// G-code file operation events.
    File,
    /// Communication layer events.
    Communication,
    /// User interface events.
    Ui,
    /// Settings and configuration events.
    Settings,
    /// Error and diagnostic events.
    Error,
}

impl std::fmt::Display for EventCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EventCategory::Connection => write!(f, "Connection"),
            EventCategory::Machine => write!(f, "Machine"),
            EventCategory::File => write!(f, "File"),
            EventCategory::Communication => write!(f, "Communication"),
            EventCategory::Ui => write!(f, "Ui"),
            EventCategory::Settings => write!(f, "Settings"),
            EventCategory::Error => write!(f, "Error"),
        }
    }
}

/// Reason for disconnection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DisconnectReason {
    /// User requested disconnect
    UserRequested,
    /// Connection lost unexpectedly
    ConnectionLost,
    /// Device was unplugged
    DeviceRemoved,
    /// Timeout occurred
    Timeout,
    /// Error occurred
    Error(String),
}

/// Connection-related events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConnectionEvent {
    /// Starting connection attempt.
    Connecting {
        /// Serial port path being connected to.
        port: String,
    },
    /// Successfully connected.
    Connected {
        /// Serial port path that was connected.
        port: String,
        /// Firmware identification string from the device.
        firmware: String,
    },
    /// Disconnected from device.
    Disconnected {
        /// Serial port path that was disconnected.
        port: String,
        /// Reason for the disconnection.
        reason: DisconnectReason,
    },
    /// Connection attempt failed.
    ConnectionFailed {
        /// Serial port path that failed to connect.
        port: String,
        /// Error message describing the failure.
        error: String,
    },
    /// Connection state changed.
    StateChanged {
        /// Whether the device is currently connected.
        connected: bool,
    },
}

impl ConnectionEvent {
    fn description(&self) -> String {
        match self {
            ConnectionEvent::Connecting { port } => format!("Connecting to {}", port),
            ConnectionEvent::Connected { port, firmware } => {
                format!("Connected to {} ({})", port, firmware)
            }
            ConnectionEvent::Disconnected { port, reason } => {
                format!("Disconnected from {}: {:?}", port, reason)
            }
            ConnectionEvent::ConnectionFailed { port, error } => {
                format!("Connection failed to {}: {}", port, error)
            }
            ConnectionEvent::StateChanged { connected } => {
                format!(
                    "Connection state: {}",
                    if *connected {
                        "connected"
                    } else {
                        "disconnected"
                    }
                )
            }
        }
    }
}

/// Source of position data
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum PositionSource {
    /// From status report polling
    StatusReport,
    /// From real-time position query
    RealtimeQuery,
    /// From probe result
    Probe,
    /// Manual entry
    Manual,
}

/// Axis identifier
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum Axis {
    /// X-axis (typically left-right).
    X,
    /// Y-axis (typically front-back).
    Y,
    /// Z-axis (typically up-down).
    Z,
    /// A-axis (rotational around X).
    A,
    /// B-axis (rotational around Y).
    B,
    /// C-axis (rotational around Z).
    C,
}

/// Direction for limits
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum Direction {
    /// Positive direction (toward max limit).
    Positive,
    /// Negative direction (toward min limit).
    Negative,
}

/// Spindle state
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum SpindleState {
    /// Spindle is stopped.
    Off,
    /// Spindle is running clockwise (M3).
    Clockwise,
    /// Spindle is running counter-clockwise (M4).
    CounterClockwise,
}

/// Machine state and status events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MachineEvent {
    /// Controller state changed.
    StateChanged {
        /// Previous controller state.
        old: ControllerState,
        /// New controller state.
        new: ControllerState,
    },
    /// Position updated.
    PositionUpdated {
        /// Current machine position.
        position: Position,
        /// Source of the position data.
        source: PositionSource,
    },
    /// Alarm triggered.
    AlarmTriggered {
        /// Alarm code number.
        code: u8,
        /// Human-readable alarm message.
        message: String,
    },
    /// Alarm cleared.
    AlarmCleared,
    /// Probe triggered.
    ProbeTriggered {
        /// Position where probe was triggered.
        position: Position,
    },
    /// Limit switch triggered.
    LimitTriggered {
        /// Axis that triggered the limit.
        axis: Axis,
        /// Direction of the limit (positive or negative).
        direction: Direction,
    },
    /// Spindle state changed.
    SpindleChanged {
        /// Current spindle speed in RPM.
        rpm: f32,
        /// Current spindle rotation state.
        state: SpindleState,
    },
    /// Coolant state changed.
    CoolantChanged {
        /// Whether mist coolant is active.
        mist: bool,
        /// Whether flood coolant is active.
        flood: bool,
    },
    /// Feed rate override changed.
    FeedOverrideChanged {
        /// Feed rate override percentage (100 = normal).
        percent: u8,
    },
    /// Rapid override changed.
    RapidOverrideChanged {
        /// Rapid override percentage (100 = normal).
        percent: u8,
    },
    /// Spindle override changed.
    SpindleOverrideChanged {
        /// Spindle speed override percentage (100 = normal).
        percent: u8,
    },
    /// Homing completed.
    HomingCompleted,
    /// Homing started.
    HomingStarted,
}

impl MachineEvent {
    fn description(&self) -> String {
        match self {
            MachineEvent::StateChanged { old, new } => {
                format!("State: {:?} -> {:?}", old, new)
            }
            MachineEvent::PositionUpdated { position, .. } => {
                format!(
                    "Position: X{:.3} Y{:.3} Z{:.3}",
                    position.x, position.y, position.z
                )
            }
            MachineEvent::AlarmTriggered { code, message } => {
                format!("Alarm {}: {}", code, message)
            }
            MachineEvent::AlarmCleared => "Alarm cleared".to_string(),
            MachineEvent::ProbeTriggered { position } => {
                format!(
                    "Probe at: X{:.3} Y{:.3} Z{:.3}",
                    position.x, position.y, position.z
                )
            }
            MachineEvent::LimitTriggered { axis, direction } => {
                format!("Limit: {:?} {:?}", axis, direction)
            }
            MachineEvent::SpindleChanged { rpm, state } => {
                format!("Spindle: {:?} @ {} RPM", state, rpm)
            }
            MachineEvent::CoolantChanged { mist, flood } => {
                format!("Coolant: mist={}, flood={}", mist, flood)
            }
            MachineEvent::FeedOverrideChanged { percent } => {
                format!("Feed override: {}%", percent)
            }
            MachineEvent::RapidOverrideChanged { percent } => {
                format!("Rapid override: {}%", percent)
            }
            MachineEvent::SpindleOverrideChanged { percent } => {
                format!("Spindle override: {}%", percent)
            }
            MachineEvent::HomingCompleted => "Homing completed".to_string(),
            MachineEvent::HomingStarted => "Homing started".to_string(),
        }
    }
}

/// File operation events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FileEvent {
    /// File opened.
    Opened {
        /// Path to the opened file.
        path: PathBuf,
        /// Total number of lines in the file.
        lines: usize,
    },
    /// File closed.
    Closed,
    /// File modified.
    Modified,
    /// File saved.
    Saved {
        /// Path where the file was saved.
        path: PathBuf,
    },
    /// Parse error in file.
    ParseError {
        /// Line number where the error occurred.
        line: usize,
        /// Error message describing the parse failure.
        error: String,
    },
    /// Stream started.
    StreamStarted {
        /// Total number of lines to stream.
        total_lines: usize,
    },
    /// Stream progress update.
    StreamProgress {
        /// Current line being streamed.
        current_line: usize,
        /// Total number of lines to stream.
        total_lines: usize,
    },
    /// Stream completed.
    StreamCompleted {
        /// Total duration of the stream operation.
        duration: Duration,
    },
    /// Stream paused.
    StreamPaused,
    /// Stream resumed.
    StreamResumed,
    /// Stream cancelled.
    StreamCancelled,
}

impl FileEvent {
    fn description(&self) -> String {
        match self {
            FileEvent::Opened { path, lines } => {
                format!("Opened: {} ({} lines)", path.display(), lines)
            }
            FileEvent::Closed => "File closed".to_string(),
            FileEvent::Modified => "File modified".to_string(),
            FileEvent::Saved { path } => format!("Saved: {}", path.display()),
            FileEvent::ParseError { line, error } => {
                format!("Parse error at line {}: {}", line, error)
            }
            FileEvent::StreamStarted { total_lines } => {
                format!("Stream started: {} lines", total_lines)
            }
            FileEvent::StreamProgress {
                current_line,
                total_lines,
            } => format!("Progress: {}/{}", current_line, total_lines),
            FileEvent::StreamCompleted { duration } => {
                format!("Stream completed in {:?}", duration)
            }
            FileEvent::StreamPaused => "Stream paused".to_string(),
            FileEvent::StreamResumed => "Stream resumed".to_string(),
            FileEvent::StreamCancelled => "Stream cancelled".to_string(),
        }
    }
}

/// Communication layer events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CommunicationEvent {
    /// Data sent to device.
    DataSent {
        /// The data that was transmitted.
        data: String,
    },
    /// Data received from device.
    DataReceived {
        /// The data that was received.
        data: String,
    },
    /// Timeout occurred.
    Timeout {
        /// Description of the operation that timed out.
        operation: String,
    },
    /// Buffer status update.
    BufferStatus {
        /// Number of available buffer slots.
        available: usize,
        /// Total buffer capacity.
        total: usize,
    },
}

impl CommunicationEvent {
    fn description(&self) -> String {
        match self {
            CommunicationEvent::DataSent { data } => {
                let truncated = if data.len() > 50 {
                    format!("{}...", &data[..50])
                } else {
                    data.clone()
                };
                format!("TX: {}", truncated.trim())
            }
            CommunicationEvent::DataReceived { data } => {
                let truncated = if data.len() > 50 {
                    format!("{}...", &data[..50])
                } else {
                    data.clone()
                };
                format!("RX: {}", truncated.trim())
            }
            CommunicationEvent::Timeout { operation } => {
                format!("Timeout: {}", operation)
            }
            CommunicationEvent::BufferStatus { available, total } => {
                format!("Buffer: {}/{}", available, total)
            }
        }
    }
}

/// View types for UI
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum ViewType {
    /// Machine control view.
    Control,
    /// Design workspace view.
    Designer,
    /// G-code editor view.
    GCodeEditor,
    /// 3D visualization view.
    Visualizer,
    /// Application settings view.
    Settings,
    /// Serial console view.
    Console,
}

/// Theme setting
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum Theme {
    /// Light color theme.
    Light,
    /// Dark color theme.
    Dark,
    /// Follow system preference.
    System,
}

/// UI-related events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum UiEvent {
    /// View changed.
    ViewChanged {
        /// The new active view type.
        view: ViewType,
    },
    /// Theme changed.
    ThemeChanged {
        /// The new theme setting.
        theme: Theme,
    },
    /// Units changed.
    UnitsChanged {
        /// The new measurement units.
        units: Units,
    },
    /// Zoom level changed.
    ZoomChanged {
        /// Zoom level as a multiplier (1.0 = 100%).
        level: f32,
    },
    /// Selection changed.
    SelectionChanged {
        /// Number of items currently selected.
        count: usize,
    },
    /// Action triggered.
    ActionTriggered {
        /// Name of the action that was triggered.
        action: String,
    },
    /// Dialog opened.
    DialogOpened {
        /// Name of the dialog that was opened.
        dialog: String,
    },
    /// Dialog closed.
    DialogClosed {
        /// Name of the dialog that was closed.
        dialog: String,
    },
}

impl UiEvent {
    fn description(&self) -> String {
        match self {
            UiEvent::ViewChanged { view } => format!("View: {:?}", view),
            UiEvent::ThemeChanged { theme } => format!("Theme: {:?}", theme),
            UiEvent::UnitsChanged { units } => format!("Units: {:?}", units),
            UiEvent::ZoomChanged { level } => format!("Zoom: {:.0}%", level * 100.0),
            UiEvent::SelectionChanged { count } => format!("Selected: {} items", count),
            UiEvent::ActionTriggered { action } => format!("Action: {}", action),
            UiEvent::DialogOpened { dialog } => format!("Dialog opened: {}", dialog),
            UiEvent::DialogClosed { dialog } => format!("Dialog closed: {}", dialog),
        }
    }
}

/// Setting value types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SettingValue {
    /// Boolean setting value.
    Bool(bool),
    /// Integer setting value.
    Int(i64),
    /// Floating-point setting value.
    Float(f64),
    /// String setting value.
    String(String),
}

/// Settings-related events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SettingsEvent {
    /// Settings loaded.
    Loaded,
    /// Settings saved.
    Saved,
    /// Setting changed.
    Changed {
        /// Setting key that was changed.
        key: String,
        /// New value of the setting.
        value: SettingValue,
    },
    /// Profile changed.
    ProfileChanged {
        /// Name of the newly active profile.
        profile: String,
    },
}

impl SettingsEvent {
    fn description(&self) -> String {
        match self {
            SettingsEvent::Loaded => "Settings loaded".to_string(),
            SettingsEvent::Saved => "Settings saved".to_string(),
            SettingsEvent::Changed { key, value } => {
                format!("Setting: {} = {:?}", key, value)
            }
            SettingsEvent::ProfileChanged { profile } => {
                format!("Profile: {}", profile)
            }
        }
    }
}

/// Error severity levels
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ErrorSeverity {
    /// Non-critical warning that does not block operation.
    Warning,
    /// Error that may be recoverable.
    Error,
    /// Critical error requiring immediate attention.
    Critical,
}

/// Error and diagnostic events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ErrorEvent {
    /// Warning (non-blocking).
    Warning {
        /// Warning code identifier.
        code: String,
        /// Human-readable warning message.
        message: String,
    },
    /// Error (may be recoverable).
    Error {
        /// Error code identifier.
        code: String,
        /// Human-readable error message.
        message: String,
        /// Whether recovery is possible without user intervention.
        recoverable: bool,
    },
    /// Critical error (requires attention).
    Critical {
        /// Critical error code identifier.
        code: String,
        /// Human-readable critical error message.
        message: String,
    },
}

impl ErrorEvent {
    fn description(&self) -> String {
        match self {
            ErrorEvent::Warning { code, message } => {
                format!("Warning [{}]: {}", code, message)
            }
            ErrorEvent::Error { code, message, .. } => {
                format!("Error [{}]: {}", code, message)
            }
            ErrorEvent::Critical { code, message } => {
                format!("Critical [{}]: {}", code, message)
            }
        }
    }

    /// Get the severity of this error event
    pub fn severity(&self) -> ErrorSeverity {
        match self {
            ErrorEvent::Warning { .. } => ErrorSeverity::Warning,
            ErrorEvent::Error { .. } => ErrorSeverity::Error,
            ErrorEvent::Critical { .. } => ErrorSeverity::Critical,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_category() {
        let event = AppEvent::Connection(ConnectionEvent::Connected {
            port: "/dev/ttyUSB0".to_string(),
            firmware: "GRBL".to_string(),
        });
        assert_eq!(event.category(), EventCategory::Connection);

        let event = AppEvent::Machine(MachineEvent::AlarmCleared);
        assert_eq!(event.category(), EventCategory::Machine);
    }

    #[test]
    fn test_event_description() {
        let event = AppEvent::Connection(ConnectionEvent::Connected {
            port: "/dev/ttyUSB0".to_string(),
            firmware: "GRBL 1.1h".to_string(),
        });
        assert!(event.description().contains("Connected"));
        assert!(event.description().contains("GRBL"));
    }

    #[test]
    fn test_event_serialization() {
        let event = AppEvent::Machine(MachineEvent::FeedOverrideChanged { percent: 120 });
        let json = serde_json::to_string(&event).expect("Should serialize");
        let parsed: AppEvent = serde_json::from_str(&json).expect("Should deserialize");

        if let AppEvent::Machine(MachineEvent::FeedOverrideChanged { percent }) = parsed {
            assert_eq!(percent, 120);
        } else {
            panic!("Wrong event type after deserialization");
        }
    }

    #[test]
    fn test_error_severity() {
        let warning = ErrorEvent::Warning {
            code: "W001".to_string(),
            message: "Test".to_string(),
        };
        assert_eq!(warning.severity(), ErrorSeverity::Warning);

        let error = ErrorEvent::Error {
            code: "E001".to_string(),
            message: "Test".to_string(),
            recoverable: true,
        };
        assert_eq!(error.severity(), ErrorSeverity::Error);

        let critical = ErrorEvent::Critical {
            code: "C001".to_string(),
            message: "Test".to_string(),
        };
        assert_eq!(critical.severity(), ErrorSeverity::Critical);
    }
}
