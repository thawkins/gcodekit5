//! Real-time device status tracking
//!
//! Comprehensive device status tracking including position, machine state,
//! buffer status, and other real-time parameters.

use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

/// Machine state enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MachineStateType {
    /// Machine is idle and ready
    Idle,
    /// Machine is actively running G-code
    Run,
    /// Machine is on hold/paused
    Hold,
    /// Door interlock is open
    Door,
    /// Machine has an alarm condition
    Alarm,
    /// Machine is in check mode
    Check,
    /// Unknown or uninitialized state
    Unknown,
}

impl MachineStateType {
    /// Parse from GRBL status string
    pub fn from_grbl_state(state_str: &str) -> Self {
        // GRBL 1.1 appends sub-states, e.g. "Hold:0" or "Door:1"; match on the base state.
        let base = state_str.trim().split(':').next().unwrap_or_default();
        match base {
            "Idle" => Self::Idle,
            "Run" => Self::Run,
            "Hold" => Self::Hold,
            "Door" => Self::Door,
            "Alarm" => Self::Alarm,
            "Check" => Self::Check,
            _ => Self::Unknown,
        }
    }
}

impl std::fmt::Display for MachineStateType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Idle => write!(f, "Idle"),
            Self::Run => write!(f, "Running"),
            Self::Hold => write!(f, "Hold"),
            Self::Door => write!(f, "Door Open"),
            Self::Alarm => write!(f, "Alarm"),
            Self::Check => write!(f, "Check"),
            Self::Unknown => write!(f, "Unknown"),
        }
    }
}

/// Buffer status information
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct BufferStatus {
    /// RX buffer count (available slots)
    pub rx: u16,
    /// TX buffer count (pending commands)
    pub tx: u16,
}

impl BufferStatus {
    /// Parse from GRBL status buffer info: "Bf:14,1"
    pub fn parse(buffer_str: &str) -> Option<Self> {
        // Format: "Bf:RX,TX"
        let parts: Vec<&str> = buffer_str.split(',').collect();
        if parts.len() >= 2 {
            let rx = parts[0].trim().parse::<u16>().ok()?;
            let tx = parts[1].trim().parse::<u16>().ok()?;
            return Some(Self { rx, tx });
        }
        None
    }
}

/// Override percentages
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Overrides {
    /// Feed rate override percentage (100 = normal)
    pub feed_rate: u16,
    /// Spindle speed override percentage (100 = normal)
    pub spindle_speed: u16,
    /// Rapid traverse override percentage (100 = normal)
    pub rapid: u16,
}

impl Overrides {
    /// Parse from GRBL status: "Ov:120,80,100"
    pub fn parse(override_str: &str) -> Option<Self> {
        let parts: Vec<&str> = override_str.split(',').collect();
        if parts.len() >= 3 {
            // GRBL 1.1 order is "Ov:feed,rapid,spindle"
            let feed_rate = parts[0].trim().parse::<u16>().ok()?;
            let rapid = parts[1].trim().parse::<u16>().ok()?;
            let spindle_speed = parts[2].trim().parse::<u16>().ok()?;
            return Some(Self {
                feed_rate,
                spindle_speed,
                rapid,
            });
        }
        None
    }
}

/// Complete device status
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DeviceStatus {
    /// Machine state
    pub state: MachineStateType,
    /// Machine position (X, Y, Z)
    pub machine_pos: (f64, f64, f64),
    /// Work position (X, Y, Z)
    pub work_pos: (f64, f64, f64),
    /// Feed rate in mm/min
    pub feed_rate: Option<f64>,
    /// Spindle speed in RPM
    pub spindle_speed: Option<u16>,
    /// Buffer status
    pub buffer: Option<BufferStatus>,
    /// Override percentages
    pub overrides: Option<Overrides>,
    /// Timestamp of this status
    pub timestamp: u64,
}

impl DeviceStatus {
    /// Create a new device status
    pub fn new(state: MachineStateType) -> Self {
        Self {
            state,
            machine_pos: (0.0, 0.0, 0.0),
            work_pos: (0.0, 0.0, 0.0),
            feed_rate: None,
            spindle_speed: None,
            buffer: None,
            overrides: None,
            timestamp: current_unix_timestamp(),
        }
    }

    /// Parse from complete GRBL status response
    /// Format: <Idle|MPos:10.000,20.000,0.000|WPos:10.000,20.000,0.000|Bf:14,1|Ov:120,80,100|F:2000|S:5000>
    pub fn parse_grbl_status(response: &str) -> Option<Self> {
        let trimmed = response.trim_matches(|c| c == '<' || c == '>');
        let parts: Vec<&str> = trimmed.split('|').collect();

        if parts.is_empty() {
            return None;
        }

        // Parse state (first part)
        let state = MachineStateType::from_grbl_state(parts[0]);

        let mut status = Self::new(state);

        // Parse remaining fields
        // Match field prefixes exactly: a substring search finds "S:" inside "FS:".
        for part in parts[1..].iter().map(|p| p.trim()) {
            if let Some(coords_str) = part.strip_prefix("MPos:") {
                status.machine_pos = parse_coordinates(coords_str)?;
            } else if let Some(coords_str) = part.strip_prefix("WPos:") {
                status.work_pos = parse_coordinates(coords_str)?;
            } else if let Some(bf_str) = part.strip_prefix("Bf:") {
                status.buffer = BufferStatus::parse(bf_str);
            } else if let Some(ov_str) = part.strip_prefix("Ov:") {
                status.overrides = Overrides::parse(ov_str);
            } else if let Some(fs_str) = part.strip_prefix("FS:") {
                // GRBL 1.1 combined "FS:feed,spindle"
                let mut fs = fs_str.split(',');
                status.feed_rate = fs.next().and_then(|f| f.trim().parse::<f64>().ok());
                status.spindle_speed = fs.next().and_then(|s| s.trim().parse::<u16>().ok());
            } else if let Some(f_str) = part.strip_prefix("F:") {
                status.feed_rate = f_str.trim().parse::<f64>().ok();
            } else if let Some(s_str) = part.strip_prefix("S:") {
                status.spindle_speed = s_str.trim().parse::<u16>().ok();
            }
        }

        Some(status)
    }
}

/// Parse coordinate string (X,Y,Z format)
fn parse_coordinates(coords_str: &str) -> Option<(f64, f64, f64)> {
    let parts: Vec<&str> = coords_str.split(',').collect();
    if parts.len() >= 3 {
        let x = parts[0].trim().parse::<f64>().ok()?;
        let y = parts[1].trim().parse::<f64>().ok()?;
        let z = parts[2].trim().parse::<f64>().ok()?;
        return Some((x, y, z));
    }
    None
}

/// Get current Unix timestamp in seconds
fn current_unix_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
