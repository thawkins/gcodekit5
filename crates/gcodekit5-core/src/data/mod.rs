//! Data models for positions, status, and machine information
//!
//! This module provides:
//! - Position tracking with full 6-axis support (X, Y, Z, A, B, C)
//! - Partial position updates for selective axis changes
//! - Controller status representation
//! - Machine capabilities
//! - Command structures
//! - Unit management (MM, INCH)
//! - Materials database with cutting parameters
//! - Tools palette for CAM operations

pub mod gtc_import;
pub mod materials;
pub mod materials_mpi_static;
pub mod tools;

use serde::{Deserialize, Serialize};
use std::fmt;

/// Machine coordinate units (millimeters or inches)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Units {
    /// Millimeters (metric)
    MM,
    /// Inches (imperial)
    INCH,
    /// Unknown or uninitialized
    Unknown,
}

impl Units {
    /// Convert a value from one unit to another
    ///
    /// # Arguments
    /// * `value` - The value to convert
    /// * `from` - The unit of the input value
    /// * `to` - The target unit
    ///
    /// # Returns
    /// The converted value, or the original value if units are the same or unknown
    pub fn convert(value: f64, from: Units, to: Units) -> f64 {
        if from == to {
            return value;
        }

        match (from, to) {
            (Units::MM, Units::INCH) => value / 25.4,
            (Units::INCH, Units::MM) => value * 25.4,
            _ => value,
        }
    }
}

impl fmt::Display for Units {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Units::MM => write!(f, "mm"),
            Units::INCH => write!(f, "in"),
            Units::Unknown => write!(f, "unknown"),
        }
    }
}

/// Base CNC point structure representing a 6-axis coordinate
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct CNCPoint {
    /// X-axis position
    pub x: f64,
    /// Y-axis position
    pub y: f64,
    /// Z-axis position
    pub z: f64,
    /// A-axis (4th axis) position
    pub a: f64,
    /// B-axis (5th axis) position
    pub b: f64,
    /// C-axis (6th axis) position
    pub c: f64,
    /// Coordinate unit
    pub unit: Units,
}

impl CNCPoint {
    /// Create a new 6-axis CNC point with all axes at zero
    pub fn new(unit: Units) -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            z: 0.0,
            a: 0.0,
            b: 0.0,
            c: 0.0,
            unit,
        }
    }

    /// Create a CNC point with specified 6-axis coordinates
    pub fn with_axes(x: f64, y: f64, z: f64, a: f64, b: f64, c: f64, unit: Units) -> Self {
        debug_assert!(
            x.is_finite()
                && y.is_finite()
                && z.is_finite()
                && a.is_finite()
                && b.is_finite()
                && c.is_finite(),
            "CNCPoint axes must be finite: x={x}, y={y}, z={z}, a={a}, b={b}, c={c}"
        );
        Self {
            x,
            y,
            z,
            a,
            b,
            c,
            unit,
        }
    }

    /// Get all axes as a tuple
    pub fn get_axes(&self) -> (f64, f64, f64, f64, f64, f64) {
        (self.x, self.y, self.z, self.a, self.b, self.c)
    }

    /// Set all axes from a tuple
    pub fn set_axes(&mut self, x: f64, y: f64, z: f64, a: f64, b: f64, c: f64) {
        debug_assert!(
            x.is_finite()
                && y.is_finite()
                && z.is_finite()
                && a.is_finite()
                && b.is_finite()
                && c.is_finite(),
            "CNCPoint axes must be finite: x={x}, y={y}, z={z}, a={a}, b={b}, c={c}"
        );
        self.x = x;
        self.y = y;
        self.z = z;
        self.a = a;
        self.b = b;
        self.c = c;
    }

    /// Convert this point to a different unit
    pub fn convert_to(&self, target_unit: Units) -> Self {
        let scale = match (self.unit, target_unit) {
            (Units::MM, Units::INCH) => 1.0 / 25.4,
            (Units::INCH, Units::MM) => 25.4,
            _ => 1.0,
        };

        Self {
            x: self.x * scale,
            y: self.y * scale,
            z: self.z * scale,
            a: self.a * scale,
            b: self.b * scale,
            c: self.c * scale,
            unit: target_unit,
        }
    }
}

impl Default for CNCPoint {
    fn default() -> Self {
        Self::new(Units::MM)
    }
}

impl fmt::Display for CNCPoint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "X:{:.3} Y:{:.3} Z:{:.3} A:{:.3} B:{:.3} C:{:.3} ({})",
            self.x, self.y, self.z, self.a, self.b, self.c, self.unit
        )
    }
}

/// Position in 3D space with optional fourth axis (simplified for backward compatibility)
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Position {
    /// X-axis position
    pub x: f32,
    /// Y-axis position
    pub y: f32,
    /// Z-axis position
    pub z: f32,
    /// Fourth axis (A/U) if present
    pub a: Option<f32>,
}

impl Position {
    /// Create a new position with X, Y, Z coordinates
    pub fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z, a: None }
    }

    /// Create a position with four axes including the A axis
    pub fn with_a(x: f32, y: f32, z: f32, a: f32) -> Self {
        Self {
            x,
            y,
            z,
            a: Some(a),
        }
    }

    /// Convert from a CNCPoint to Position (takes only X, Y, Z, A)
    pub fn from_cnc_point(point: &CNCPoint) -> Self {
        Self {
            x: point.x as f32,
            y: point.y as f32,
            z: point.z as f32,
            a: Some(point.a as f32),
        }
    }

    /// Convert this Position to a CNCPoint
    pub fn to_cnc_point(&self, unit: Units) -> CNCPoint {
        CNCPoint::with_axes(
            self.x as f64,
            self.y as f64,
            self.z as f64,
            self.a.unwrap_or(0.0) as f64,
            0.0,
            0.0,
            unit,
        )
    }

    /// Calculate distance to another position (XYZ only)
    pub fn distance_to(&self, other: &Position) -> f32 {
        let dx = self.x - other.x;
        let dy = self.y - other.y;
        let dz = self.z - other.z;
        (dx * dx + dy * dy + dz * dz).sqrt()
    }

    /// Get absolute value of all coordinates
    pub fn abs(&self) -> Self {
        Self {
            x: self.x.abs(),
            y: self.y.abs(),
            z: self.z.abs(),
            a: self.a.map(|v| v.abs()),
        }
    }

    /// Add another position (component-wise)
    pub fn add(&self, other: &Position) -> Self {
        Self {
            x: self.x + other.x,
            y: self.y + other.y,
            z: self.z + other.z,
            a: match (self.a, other.a) {
                (Some(a1), Some(a2)) => Some(a1 + a2),
                (Some(a), None) | (None, Some(a)) => Some(a),
                _ => None,
            },
        }
    }

    /// Subtract another position (component-wise)
    pub fn subtract(&self, other: &Position) -> Self {
        Self {
            x: self.x - other.x,
            y: self.y - other.y,
            z: self.z - other.z,
            a: match (self.a, other.a) {
                (Some(a1), Some(a2)) => Some(a1 - a2),
                (Some(a), None) | (None, Some(a)) => Some(a),
                _ => None,
            },
        }
    }
}

impl Default for Position {
    fn default() -> Self {
        Self::new(0.0, 0.0, 0.0)
    }
}

impl std::fmt::Display for Position {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.a {
            Some(a) => write!(
                f,
                "X:{:.2} Y:{:.2} Z:{:.2} A:{:.2}",
                self.x, self.y, self.z, a
            ),
            None => write!(f, "X:{:.2} Y:{:.2} Z:{:.2}", self.x, self.y, self.z),
        }
    }
}

/// Partial position for updating only specific axes
///
/// Used when only some axes need to be updated. Each axis is represented as an `Option`
/// where `None` means "don't change this axis" and `Some(value)` means "set to value".
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct PartialPosition {
    /// X-axis position (if Some, update this axis)
    pub x: Option<f32>,
    /// Y-axis position (if Some, update this axis)
    pub y: Option<f32>,
    /// Z-axis position (if Some, update this axis)
    pub z: Option<f32>,
    /// A-axis position (if Some, update this axis)
    pub a: Option<f32>,
    /// B-axis position (if Some, update this axis)
    pub b: Option<f32>,
    /// C-axis position (if Some, update this axis)
    pub c: Option<f32>,
}

impl PartialPosition {
    /// Create a new empty partial position (all axes None)
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a partial position with only X axis set
    pub fn x_only(x: f32) -> Self {
        Self {
            x: Some(x),
            ..Default::default()
        }
    }

    /// Create a partial position with only Y axis set
    pub fn y_only(y: f32) -> Self {
        Self {
            y: Some(y),
            ..Default::default()
        }
    }

    /// Create a partial position with only Z axis set
    pub fn z_only(z: f32) -> Self {
        Self {
            z: Some(z),
            ..Default::default()
        }
    }

    /// Create a partial position with XY axes set
    pub fn xy(x: f32, y: f32) -> Self {
        Self {
            x: Some(x),
            y: Some(y),
            ..Default::default()
        }
    }

    /// Create a partial position with XYZ axes set
    pub fn xyz(x: f32, y: f32, z: f32) -> Self {
        Self {
            x: Some(x),
            y: Some(y),
            z: Some(z),
            ..Default::default()
        }
    }

    /// Apply this partial position to an existing position, updating only specified axes
    pub fn apply_to(&self, pos: &Position) -> Position {
        Position {
            x: self.x.unwrap_or(pos.x),
            y: self.y.unwrap_or(pos.y),
            z: self.z.unwrap_or(pos.z),
            a: self.a.or(pos.a),
        }
    }

    /// Apply this partial position to a CNC point, updating only specified axes
    pub fn apply_to_cnc_point(&self, point: &CNCPoint) -> CNCPoint {
        CNCPoint {
            x: self.x.map(|v| v as f64).unwrap_or(point.x),
            y: self.y.map(|v| v as f64).unwrap_or(point.y),
            z: self.z.map(|v| v as f64).unwrap_or(point.z),
            a: self.a.map(|v| v as f64).unwrap_or(point.a),
            b: self.b.map(|v| v as f64).unwrap_or(point.b),
            c: self.c.map(|v| v as f64).unwrap_or(point.c),
            unit: point.unit,
        }
    }

    /// Count how many axes are set in this partial position
    pub fn axis_count(&self) -> usize {
        [self.x, self.y, self.z, self.a, self.b, self.c]
            .iter()
            .filter(|opt| opt.is_some())
            .count()
    }

    /// Check if this partial position is empty (no axes set)
    pub fn is_empty(&self) -> bool {
        self.x.is_none()
            && self.y.is_none()
            && self.z.is_none()
            && self.a.is_none()
            && self.b.is_none()
            && self.c.is_none()
    }
}

/// Machine/Controller state machine states
///
/// Represents the operational state of the CNC controller.
/// This enum tracks the full lifecycle of controller operation from
/// initial connection through execution and error states.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ControllerState {
    /// Not connected to any controller
    Disconnected,
    /// In the process of connecting to controller
    Connecting,
    /// Connected and idle, ready for commands
    Idle,
    /// Executing a G-code program
    Run,
    /// Program paused, awaiting resume
    Hold,
    /// Manual jog/movement mode
    Jog,
    /// Machine alarm state (requires manual intervention)
    Alarm,
    /// Check mode (dry-run without machine movement)
    Check,
    /// Safety door interlock triggered
    Door,
    /// Homing/homing cycle in progress
    Home,
    /// Low-power sleep/idle state
    Sleep,
}

impl ControllerState {
    /// Check if this state indicates the controller is connected
    pub fn is_connected(&self) -> bool {
        !matches!(
            self,
            ControllerState::Disconnected | ControllerState::Connecting
        )
    }

    /// Check if this state indicates the controller is ready for commands
    pub fn is_ready(&self) -> bool {
        matches!(
            self,
            ControllerState::Idle | ControllerState::Jog | ControllerState::Sleep
        )
    }

    /// Check if this state indicates an error condition
    pub fn is_error(&self) -> bool {
        matches!(self, ControllerState::Alarm)
    }

    /// Check if this state indicates active motion
    pub fn is_moving(&self) -> bool {
        matches!(
            self,
            ControllerState::Run | ControllerState::Jog | ControllerState::Home
        )
    }

    /// Check if a transition from this state to `target` is valid.
    ///
    /// Returns `true` for valid transitions according to the CNC state machine:
    /// - Disconnected can only go to Connecting
    /// - Connecting can go to Idle, Alarm, or back to Disconnected
    /// - Alarm requires explicit reset to Idle (or disconnect)
    /// - Any connected state can go to Disconnected (connection loss)
    pub fn can_transition_to(&self, target: ControllerState) -> bool {
        use ControllerState::*;
        if *self == target {
            return true;
        }
        match (self, target) {
            // Connection lifecycle
            (Disconnected, Connecting) => true,
            (Connecting, Idle | Alarm | Disconnected) => true,
            // Any connected state can disconnect
            (_, Disconnected) => true,
            // Cannot transition from Disconnected/Connecting to active states directly
            (Disconnected | Connecting, _) => false,
            // Alarm can only go to Idle (reset) or Disconnected
            (Alarm, Idle) => true,
            (Alarm, _) => false,
            // Idle can go to any active state
            (Idle, _) => true,
            // Run can hold, alarm, complete to idle, or door
            (Run, Hold | Alarm | Idle | Door | Check) => true,
            // Hold can resume to run, go idle, or alarm
            (Hold, Run | Idle | Alarm) => true,
            // Home completes to idle or alarm
            (Home, Idle | Alarm) => true,
            // Jog completes to idle or alarm
            (Jog, Idle | Alarm) => true,
            // Door can go back to hold or idle when cleared
            (Door, Hold | Idle | Alarm) => true,
            // Check can return to idle
            (Check, Idle | Alarm) => true,
            // Sleep wakes to idle
            (Sleep, Idle | Alarm) => true,
            // All other transitions are invalid
            _ => false,
        }
    }
}

impl fmt::Display for ControllerState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Disconnected => write!(f, "Disconnected"),
            Self::Connecting => write!(f, "Connecting"),
            Self::Idle => write!(f, "Idle"),
            Self::Run => write!(f, "Run"),
            Self::Hold => write!(f, "Hold"),
            Self::Jog => write!(f, "Jog"),
            Self::Alarm => write!(f, "Alarm"),
            Self::Check => write!(f, "Check"),
            Self::Door => write!(f, "Door"),
            Self::Home => write!(f, "Home"),
            Self::Sleep => write!(f, "Sleep"),
        }
    }
}

/// State of the communication layer
///
/// Tracks the connection state between the application and the physical controller.
/// Separate from ControllerState as it represents the communication layer specifically.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CommunicatorState {
    /// Connection not established or closed
    Disconnected,
    /// Attempting to establish connection
    Connecting,
    /// Connection active and ready for communication
    Connected,
    /// Connection exists but communication is stalled (no response)
    Stalled,
    /// Connection in error state
    Error,
}

impl CommunicatorState {
    /// Check if a transition from this state to `target` is valid.
    ///
    /// Returns `true` for valid transitions:
    /// - Disconnected → Connecting
    /// - Connecting → Connected, Error, Disconnected
    /// - Connected → Stalled, Error, Disconnected
    /// - Stalled → Connected, Error, Disconnected
    /// - Error → Disconnected, Connecting
    pub fn can_transition_to(&self, target: CommunicatorState) -> bool {
        use CommunicatorState::*;
        if *self == target {
            return true;
        }
        matches!(
            (self, target),
            (Disconnected, Connecting)
                | (Connecting, Connected | Error | Disconnected)
                | (Connected, Stalled | Error | Disconnected)
                | (Stalled, Connected | Error | Disconnected)
                | (Error, Disconnected | Connecting)
        )
    }
}

impl fmt::Display for CommunicatorState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Disconnected => write!(f, "Disconnected"),
            Self::Connecting => write!(f, "Connecting"),
            Self::Connected => write!(f, "Connected"),
            Self::Stalled => write!(f, "Stalled"),
            Self::Error => write!(f, "Error"),
        }
    }
}

/// Current status indicator of the controller
///
/// Simple enum representing immediate operational status.
/// Used for UI display and basic state tracking.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ControllerStatus {
    /// Idle and ready for commands
    Idle,
    /// Processing a command
    Run,
    /// Paused during execution
    Hold,
    /// Alarm condition
    Alarm,
    /// Error state
    Error,
}

impl std::fmt::Display for ControllerStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Idle => write!(f, "Idle"),
            Self::Run => write!(f, "Run"),
            Self::Hold => write!(f, "Hold"),
            Self::Alarm => write!(f, "Alarm"),
            Self::Error => write!(f, "Error"),
        }
    }
}

/// Complete machine state snapshot
///
/// Comprehensive representation of the machine's current operational state,
/// including positions, status, feed/spindle information, and work coordinate system.
/// Supports builder pattern for flexible construction.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MachineStatusSnapshot {
    /// Current position (machine coordinates)
    pub position: Position,
    /// Current position (work coordinates)
    pub work_position: Position,
    /// Current controller state machine state
    pub controller_state: ControllerState,
    /// Current controller operational status
    pub status: ControllerStatus,
    /// Current spindle speed (RPM)
    pub spindle_speed: f64,
    /// Current feed rate (units per minute)
    pub feed_rate: f64,
    /// Active work coordinate system (1-6 for G54-G59)
    pub active_wcs: u8,
    /// Buffer state: (used, total)
    pub buffer_state: (u16, u16),
    /// Work coordinate offsets (if available)
    pub work_offset: Option<Position>,
    /// Timestamp of this snapshot (optional)
    pub timestamp: Option<u64>,
}

impl MachineStatusSnapshot {
    /// Create a new machine status snapshot with default values
    pub fn new() -> Self {
        Self {
            position: Position::default(),
            work_position: Position::default(),
            controller_state: ControllerState::Disconnected,
            status: ControllerStatus::Idle,
            spindle_speed: 0.0,
            feed_rate: 0.0,
            active_wcs: 1,
            buffer_state: (0, 128),
            work_offset: None,
            timestamp: None,
        }
    }

    /// Builder method to set machine position
    pub fn with_position(mut self, position: Position) -> Self {
        self.position = position;
        self
    }

    /// Builder method to set work position
    pub fn with_work_position(mut self, work_position: Position) -> Self {
        self.work_position = work_position;
        self
    }

    /// Builder method to set controller state
    pub fn with_controller_state(mut self, state: ControllerState) -> Self {
        self.controller_state = state;
        self
    }

    /// Builder method to set status
    pub fn with_status(mut self, status: ControllerStatus) -> Self {
        self.status = status;
        self
    }

    /// Builder method to set spindle speed
    pub fn with_spindle_speed(mut self, speed: f64) -> Self {
        self.spindle_speed = speed;
        self
    }

    /// Builder method to set feed rate
    pub fn with_feed_rate(mut self, rate: f64) -> Self {
        self.feed_rate = rate;
        self
    }

    /// Builder method to set active WCS
    pub fn with_active_wcs(mut self, wcs: u8) -> Self {
        self.active_wcs = wcs.clamp(1, 6);
        self
    }

    /// Builder method to set buffer state
    pub fn with_buffer_state(mut self, used: u16, total: u16) -> Self {
        self.buffer_state = (used, total);
        self
    }

    /// Builder method to set work offset
    pub fn with_work_offset(mut self, offset: Position) -> Self {
        self.work_offset = Some(offset);
        self
    }

    /// Builder method to set timestamp
    pub fn with_timestamp(mut self, timestamp: u64) -> Self {
        self.timestamp = Some(timestamp);
        self
    }

    /// Get buffer usage as a percentage (0-100)
    pub fn buffer_percentage(&self) -> f64 {
        if self.buffer_state.1 == 0 {
            0.0
        } else {
            (self.buffer_state.0 as f64 / self.buffer_state.1 as f64) * 100.0
        }
    }

    /// Check if buffer is nearly full (>80%)
    pub fn is_buffer_full(&self) -> bool {
        self.buffer_percentage() > 80.0
    }

    /// Get available buffer space
    pub fn available_buffer(&self) -> u16 {
        self.buffer_state.1.saturating_sub(self.buffer_state.0)
    }
}

impl Default for MachineStatusSnapshot {
    fn default() -> Self {
        Self::new()
    }
}

/// Legacy alias for backward compatibility
/// Use `MachineStatusSnapshot` for new code
pub type MachineStatus = MachineStatusSnapshot;
