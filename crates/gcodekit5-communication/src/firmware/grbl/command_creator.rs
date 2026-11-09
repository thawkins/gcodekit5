//! GRBL Command Creator
//!
//! This module provides functions to create GRBL-specific commands including
//! real-time commands, system commands, jog commands, and probe commands.

use gcodekit5_core::CNCPoint;

/// GRBL real-time command types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RealTimeCommand {
    /// Query status (?)
    QueryStatus,
    /// Feed hold (!)
    FeedHold,
    /// Cycle start (~)
    CycleStart,
    /// Soft reset (Ctrl+X)
    SoftReset,
}

impl RealTimeCommand {
    /// Get the command byte
    pub fn to_byte(self) -> u8 {
        match self {
            Self::QueryStatus => b'?',
            Self::FeedHold => b'!',
            Self::CycleStart => b'~',
            Self::SoftReset => 0x18,
        }
    }

    /// Get the command description
    pub fn description(&self) -> &'static str {
        match self {
            Self::QueryStatus => "Query Status",
            Self::FeedHold => "Feed Hold",
            Self::CycleStart => "Cycle Start",
            Self::SoftReset => "Soft Reset",
        }
    }
}

/// GRBL system command types
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SystemCommand {
    /// Home all axes ($H)
    HomeAll,
    /// Kill alarm lock ($X)
    KillAlarmLock,
    /// Check mode ($C)
    CheckMode,
    /// Query parser state ($G)
    QueryParserState,
    /// Query build info ($I)
    QueryBuildInfo,
    /// Reset EEPROM ($RST=$)
    ResetEeprom,
    /// Reset settings and data ($RST=*)
    ResetAll,
    /// Sleep ($SLP)
    Sleep,
}

impl SystemCommand {
    /// Get the command string
    pub fn command(&self) -> String {
        match self {
            Self::HomeAll => "$H".to_string(),
            Self::KillAlarmLock => "$X".to_string(),
            Self::CheckMode => "$C".to_string(),
            Self::QueryParserState => "$G".to_string(),
            Self::QueryBuildInfo => "$I".to_string(),
            Self::ResetEeprom => "$RST=$".to_string(),
            Self::ResetAll => "$RST=*".to_string(),
            Self::Sleep => "$SLP".to_string(),
        }
    }

    /// Get the command description
    pub fn description(&self) -> &'static str {
        match self {
            Self::HomeAll => "Home All Axes",
            Self::KillAlarmLock => "Kill Alarm Lock",
            Self::CheckMode => "Check Mode",
            Self::QueryParserState => "Query Parser State",
            Self::QueryBuildInfo => "Query Build Info",
            Self::ResetEeprom => "Reset EEPROM",
            Self::ResetAll => "Reset All Settings",
            Self::Sleep => "Sleep",
        }
    }
}

/// GRBL jog mode (G0 without block delete)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JogMode {
    /// XY plane
    XY,
    /// XZ plane
    XZ,
    /// YZ plane
    YZ,
}

/// GRBL jog command
#[derive(Debug, Clone, PartialEq)]
pub struct JogCommand {
    /// Jog plane
    pub plane: JogMode,
    /// Target position
    pub target: CNCPoint,
    /// Feed rate (units/minute)
    pub feed_rate: f64,
}

impl JogCommand {
    /// Create a new jog command
    pub fn new(plane: JogMode, target: CNCPoint, feed_rate: f64) -> Self {
        Self {
            plane,
            target,
            feed_rate,
        }
    }

    /// Format jog command as GRBL G-code
    pub fn to_gcode(&self) -> String {
        let _axis_code = match self.plane {
            JogMode::XY => "G0 X",
            JogMode::XZ => "G0 X",
            JogMode::YZ => "G0 Y",
        };

        match self.plane {
            JogMode::XY => {
                format!(
                    "$J=G91 X{:.3} Y{:.3} F{:.0}\n",
                    self.target.x, self.target.y, self.feed_rate
                )
            }
            JogMode::XZ => {
                format!(
                    "$J=G91 X{:.3} Z{:.3} F{:.0}\n",
                    self.target.x, self.target.z, self.feed_rate
                )
            }
            JogMode::YZ => {
                format!(
                    "$J=G91 Y{:.3} Z{:.3} F{:.0}\n",
                    self.target.y, self.target.z, self.feed_rate
                )
            }
        }
    }
}

/// GRBL probe command
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ProbeType {
    /// Probe to contact (G38.2)
    Touching,
    /// Probe to contact, fail if not touched (G38.3)
    TouchingRequired,
    /// Probe away from contact (G38.4)
    Backing,
    /// Probe away from contact, fail if touched (G38.5)
    BackingRequired,
}

impl ProbeType {
    /// Get G-code command
    pub fn gcode_command(&self) -> &'static str {
        match self {
            Self::Touching => "G38.2",
            Self::TouchingRequired => "G38.3",
            Self::Backing => "G38.4",
            Self::BackingRequired => "G38.5",
        }
    }

    /// Get description
    pub fn description(&self) -> &'static str {
        match self {
            Self::Touching => "Probe to Contact",
            Self::TouchingRequired => "Probe to Contact (Required)",
            Self::Backing => "Probe Away",
            Self::BackingRequired => "Probe Away (Required)",
        }
    }
}

/// GRBL Probe command
#[derive(Debug, Clone, PartialEq)]
pub struct ProbeCommand {
    /// Probe type
    pub probe_type: ProbeType,
    /// Target position
    pub target: CNCPoint,
    /// Feed rate (units/minute)
    pub feed_rate: f64,
}

impl ProbeCommand {
    /// Create a new probe command
    pub fn new(probe_type: ProbeType, target: CNCPoint, feed_rate: f64) -> Self {
        Self {
            probe_type,
            target,
            feed_rate,
        }
    }

    /// Format probe command as GRBL G-code
    pub fn to_gcode(&self) -> String {
        format!(
            "{} X{:.3} Y{:.3} Z{:.3} F{:.0}\n",
            self.probe_type.gcode_command(),
            self.target.x,
            self.target.y,
            self.target.z,
            self.feed_rate
        )
    }
}

/// GRBL Command Creator
pub struct CommandCreator;

impl CommandCreator {
    /// Create a real-time command
    pub fn real_time_command(cmd: RealTimeCommand) -> Vec<u8> {
        vec![cmd.to_byte()]
    }

    /// Create a system command
    pub fn system_command(cmd: SystemCommand) -> String {
        format!("{}\n", cmd.command())
    }

    /// Create a soft reset command
    pub fn soft_reset() -> Vec<u8> {
        vec![0x18]
    }

    /// Create a query status command
    pub fn query_status() -> Vec<u8> {
        vec![b'?']
    }

    /// Create a feed hold command
    pub fn feed_hold() -> Vec<u8> {
        vec![b'!']
    }

    /// Create a cycle start/resume command
    pub fn cycle_start() -> Vec<u8> {
        vec![b'~']
    }

    /// Create a home all axes command
    pub fn home_all() -> String {
        "$H\n".to_string()
    }

    /// Create a kill alarm lock command
    pub fn kill_alarm_lock() -> String {
        "$X\n".to_string()
    }

    /// Create a jog command
    pub fn jog(plane: JogMode, target: CNCPoint, feed_rate: f64) -> String {
        let jog = JogCommand::new(plane, target, feed_rate);
        jog.to_gcode()
    }

    /// Create an incremental jog command (relative motion)
    pub fn jog_incremental(axis: &str, distance: f64, feed_rate: f64) -> String {
        // GRBL rejects motion modes (G0/G1) inside $J= (error:16); the sign comes
        // from `distance` itself so negative jogs are not rendered as "X+-5.000".
        format!("$J=G91 {}{:.3} F{:.0}\n", axis, distance, feed_rate)
    }

    /// Create a zero work offset command (G10)
    pub fn set_work_offset(axes: &[&str]) -> String {
        // G10 requires an L word; L20 sets the offset so the current position
        // becomes the given value in the active WCS (P0).
        let mut cmd = "G10 L20 P0".to_string();
        for axis in axes {
            cmd.push(' ');
            cmd.push_str(axis);
            cmd.push('0');
        }
        cmd.push('\n');
        cmd
    }

    /// Create a probe command
    pub fn probe(probe_type: ProbeType, target: CNCPoint, feed_rate: f64) -> String {
        let probe = ProbeCommand::new(probe_type, target, feed_rate);
        probe.to_gcode()
    }

    /// Create a spindle on command
    pub fn spindle_on(speed: u32) -> String {
        format!("M3 S{}\n", speed)
    }

    /// Create a spindle off command
    pub fn spindle_off() -> String {
        "M5\n".to_string()
    }

    /// Create a coolant on command
    pub fn coolant_on() -> String {
        "M8\n".to_string()
    }

    /// Create a coolant off command
    pub fn coolant_off() -> String {
        "M9\n".to_string()
    }

    /// Create a program pause command
    pub fn program_pause() -> String {
        "M0\n".to_string()
    }

    /// Create a program end command
    pub fn program_end() -> String {
        "M2\n".to_string()
    }

    /// Create a tool change command
    pub fn tool_change(tool_number: u8) -> String {
        format!("T{} M6\n", tool_number)
    }

    /// Create a dwell command (pause for X seconds)
    pub fn dwell(seconds: f64) -> String {
        format!("G4 P{:.1}\n", seconds)
    }

    /// Create a rapid move command
    pub fn rapid_move(x: Option<f64>, y: Option<f64>, z: Option<f64>) -> String {
        let mut cmd = "G0".to_string();
        if let Some(x_val) = x {
            cmd.push_str(&format!(" X{:.3}", x_val));
        }
        if let Some(y_val) = y {
            cmd.push_str(&format!(" Y{:.3}", y_val));
        }
        if let Some(z_val) = z {
            cmd.push_str(&format!(" Z{:.3}", z_val));
        }
        cmd.push('\n');
        cmd
    }

    /// Create a linear move command
    pub fn linear_move(x: Option<f64>, y: Option<f64>, z: Option<f64>, feed_rate: f64) -> String {
        let mut cmd = "G1".to_string();
        if let Some(x_val) = x {
            cmd.push_str(&format!(" X{:.3}", x_val));
        }
        if let Some(y_val) = y {
            cmd.push_str(&format!(" Y{:.3}", y_val));
        }
        if let Some(z_val) = z {
            cmd.push_str(&format!(" Z{:.3}", z_val));
        }
        cmd.push_str(&format!(" F{:.0}\n", feed_rate));
        cmd
    }
}
