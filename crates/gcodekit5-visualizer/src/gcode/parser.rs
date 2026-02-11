//! G-Code parser and modal state tracking

use regex::Regex;
use serde::{Deserialize, Serialize};

use super::{CommandNumberGenerator, GcodeCommand};

/// G-Code parser with modal state tracking
pub struct GcodeParser {
    current_state: GcodeState,
    command_generator: CommandNumberGenerator,
}

/// Modal state for G-Code execution
///
/// Tracks the active modal groups during G-Code execution.
/// Modal groups are persistent states that affect all subsequent commands
/// until changed by another command in the same group.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ModalState {
    /// Motion mode (G00=rapid, G01=linear, G02=arc_cw, G03=arc_ccw)
    pub motion_mode: u8,
    /// Plane selection (G17=XY, G18=XZ, G19=YZ)
    pub plane: u8,
    /// Distance mode (G90=absolute, G91=incremental)
    pub distance_mode: u8,
    /// Feed rate mode (G93=inverse_time, G94=units_per_minute, G95=units_per_revolution)
    pub feed_rate_mode: u8,
}

impl Default for ModalState {
    fn default() -> Self {
        Self {
            motion_mode: 0,     // G00
            plane: 17,          // G17 (XY plane)
            distance_mode: 90,  // G90 (absolute)
            feed_rate_mode: 94, // G94 (units per minute)
        }
    }
}

/// Comprehensive G-Code execution state
///
/// Tracks all modal groups and execution state required for proper G-Code interpretation:
/// - Motion group (G00, G01, G02, G03)
/// - Plane selection group (G17, G18, G19)
/// - Distance mode group (G90, G91)
/// - Feed rate mode group (G93, G94, G95)
/// - Units group (G20, G21)
/// - Coordinate system group (G54-G59)
/// - Tool offset group (G43, G49)
/// - Cutter compensation group (G40, G41, G42)
/// - Spindle mode group (G03, G04, G05)
/// - Path control group (G61, G61.1, G64)
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct GcodeState {
    /// Motion mode - Group 1 (G00, G01, G02, G03)
    pub motion_mode: u8,

    /// Plane selection - Group 2 (G17=XY, G18=XZ, G19=YZ)
    pub plane_mode: u8,

    /// Distance mode - Group 3 (G90=absolute, G91=incremental)
    pub distance_mode: u8,

    /// Feed rate mode - Group 5 (G93=inverse_time, G94=units_per_minute, G95=units_per_revolution)
    pub feed_rate_mode: u8,

    /// Units mode - Group 6 (G20=inches, G21=millimeters)
    pub units_mode: u8,

    /// Coordinate system - Group 12 (G54-G59)
    pub coordinate_system: u8,

    /// Tool offset mode - Group 8 (G43=enable, G49=disable)
    pub tool_offset_mode: u8,

    /// Cutter compensation - Group 7 (G40=off, G41=left, G42=right)
    pub compensation_mode: u8,

    /// Spindle mode - Group 3 (G03=spindle_sync, G04=CSS, G05=SFM)
    pub spindle_mode: u8,

    /// Path control - Group 17 (G61=exact, G61.1=exact_stop, G64=blend)
    pub path_control_mode: u8,

    /// Current feed rate (F value)
    pub feed_rate: f64,

    /// Current spindle speed (S value)
    pub spindle_speed: f64,

    /// Tool number (T value)
    pub tool_number: u16,
}

impl Default for GcodeState {
    fn default() -> Self {
        Self {
            motion_mode: 0,        // G00 (rapid)
            plane_mode: 17,        // G17 (XY plane)
            distance_mode: 90,     // G90 (absolute)
            feed_rate_mode: 94,    // G94 (units per minute)
            units_mode: 21,        // G21 (millimeters)
            coordinate_system: 54, // G54 (first WCS)
            tool_offset_mode: 49,  // G49 (offset disabled)
            compensation_mode: 40, // G40 (cutter compensation off)
            spindle_mode: 0,       // No spindle mode
            path_control_mode: 64, // G64 (blend/continuous)
            feed_rate: 0.0,
            spindle_speed: 0.0,
            tool_number: 0,
        }
    }
}

impl GcodeState {
    /// Create a new G-Code state with default values
    pub fn new() -> Self {
        Self::default()
    }

    /// Set motion mode (G00, G01, G02, G03)
    pub fn set_motion_mode(&mut self, mode: u8) -> Result<(), String> {
        match mode {
            0..=3 => {
                self.motion_mode = mode;
                Ok(())
            }
            _ => Err(format!("Invalid motion mode: {}", mode)),
        }
    }

    /// Set plane mode (G17, G18, G19)
    pub fn set_plane_mode(&mut self, mode: u8) -> Result<(), String> {
        match mode {
            17..=19 => {
                self.plane_mode = mode;
                Ok(())
            }
            _ => Err(format!("Invalid plane mode: {}", mode)),
        }
    }

    /// Set distance mode (G90, G91)
    pub fn set_distance_mode(&mut self, mode: u8) -> Result<(), String> {
        match mode {
            90 | 91 => {
                self.distance_mode = mode;
                Ok(())
            }
            _ => Err(format!("Invalid distance mode: {}", mode)),
        }
    }

    /// Set feed rate mode (G93, G94, G95)
    pub fn set_feed_rate_mode(&mut self, mode: u8) -> Result<(), String> {
        match mode {
            93..=95 => {
                self.feed_rate_mode = mode;
                Ok(())
            }
            _ => Err(format!("Invalid feed rate mode: {}", mode)),
        }
    }

    /// Set units mode (G20 for inches, G21 for mm)
    pub fn set_units_mode(&mut self, mode: u8) -> Result<(), String> {
        match mode {
            20 | 21 => {
                self.units_mode = mode;
                Ok(())
            }
            _ => Err(format!("Invalid units mode: {}", mode)),
        }
    }

    /// Set coordinate system (G54-G59)
    pub fn set_coordinate_system(&mut self, system: u8) -> Result<(), String> {
        match system {
            54..=59 => {
                self.coordinate_system = system;
                Ok(())
            }
            _ => Err(format!("Invalid coordinate system: {}", system)),
        }
    }

    /// Set tool offset mode (G43 enable, G49 disable)
    pub fn set_tool_offset_mode(&mut self, mode: u8) -> Result<(), String> {
        match mode {
            43 | 49 => {
                self.tool_offset_mode = mode;
                Ok(())
            }
            _ => Err(format!("Invalid tool offset mode: {}", mode)),
        }
    }

    /// Set cutter compensation mode (G40 off, G41 left, G42 right)
    pub fn set_compensation_mode(&mut self, mode: u8) -> Result<(), String> {
        match mode {
            40..=42 => {
                self.compensation_mode = mode;
                Ok(())
            }
            _ => Err(format!("Invalid compensation mode: {}", mode)),
        }
    }

    /// Set feed rate value
    pub fn set_feed_rate(&mut self, rate: f64) -> Result<(), String> {
        if rate < 0.0 {
            return Err("Feed rate cannot be negative".to_string());
        }
        self.feed_rate = rate;
        Ok(())
    }

    /// Set spindle speed value
    pub fn set_spindle_speed(&mut self, speed: f64) -> Result<(), String> {
        if speed < 0.0 {
            return Err("Spindle speed cannot be negative".to_string());
        }
        self.spindle_speed = speed;
        Ok(())
    }

    /// Set tool number
    pub fn set_tool_number(&mut self, tool: u16) {
        self.tool_number = tool;
    }

    /// Check if state is valid
    pub fn validate(&self) -> Result<(), String> {
        if !matches!(self.motion_mode, 0..=3) {
            return Err(format!("Invalid motion mode: {}", self.motion_mode));
        }
        if !matches!(self.plane_mode, 17..=19) {
            return Err(format!("Invalid plane mode: {}", self.plane_mode));
        }
        if !matches!(self.distance_mode, 90 | 91) {
            return Err(format!("Invalid distance mode: {}", self.distance_mode));
        }
        if !matches!(self.feed_rate_mode, 93..=95) {
            return Err(format!("Invalid feed rate mode: {}", self.feed_rate_mode));
        }
        if !matches!(self.units_mode, 20 | 21) {
            return Err(format!("Invalid units mode: {}", self.units_mode));
        }
        if !matches!(self.coordinate_system, 54..=59) {
            return Err(format!(
                "Invalid coordinate system: {}",
                self.coordinate_system
            ));
        }
        Ok(())
    }

    /// Get a human-readable description of the current motion mode
    pub fn motion_mode_description(&self) -> &'static str {
        match self.motion_mode {
            0 => "Rapid positioning (G00)",
            1 => "Linear interpolation (G01)",
            2 => "Clockwise arc (G02)",
            3 => "Counter-clockwise arc (G03)",
            _ => "Unknown motion mode",
        }
    }

    /// Get a human-readable description of the current plane
    pub fn plane_description(&self) -> &'static str {
        match self.plane_mode {
            17 => "XY plane (G17)",
            18 => "XZ plane (G18)",
            19 => "YZ plane (G19)",
            _ => "Unknown plane",
        }
    }

    /// Get a human-readable description of distance mode
    pub fn distance_mode_description(&self) -> &'static str {
        match self.distance_mode {
            90 => "Absolute positioning (G90)",
            91 => "Incremental positioning (G91)",
            _ => "Unknown distance mode",
        }
    }

    /// Get a human-readable description of units
    pub fn units_description(&self) -> &'static str {
        match self.units_mode {
            20 => "Inches (G20)",
            21 => "Millimeters (G21)",
            _ => "Unknown units",
        }
    }
}

impl GcodeParser {
    /// Create a new G-Code parser
    pub fn new() -> Self {
        Self {
            current_state: GcodeState::default(),
            command_generator: CommandNumberGenerator::new(),
        }
    }
}

impl Default for GcodeParser {
    fn default() -> Self {
        Self::new()
    }
}

impl GcodeParser {
    /// Parse a G-Code line into a command with sequence number
    pub fn parse(&mut self, line: &str) -> Result<GcodeCommand, String> {
        // Remove comments
        let cleaned = self.remove_comments(line);

        if cleaned.trim().is_empty() {
            return Err("Empty command".to_string());
        }

        let sequence = self.command_generator.next();
        let command = GcodeCommand::with_sequence(cleaned, sequence);

        // Update modal state
        self.update_modal_state(&command)?;

        Ok(command)
    }

    /// Remove comments from a G-Code line
    fn remove_comments(&self, line: &str) -> String {
        static COMMENT_REGEX: std::sync::OnceLock<Regex> = std::sync::OnceLock::new();
        let regex =
            COMMENT_REGEX.get_or_init(|| Regex::new(r"[;(].*").expect("invalid regex pattern"));
        regex.replace(line, "").to_string()
    }

    /// Get current modal state (for backward compatibility)
    pub fn get_modal_state(&self) -> ModalState {
        ModalState {
            motion_mode: self.current_state.motion_mode,
            plane: self.current_state.plane_mode,
            distance_mode: self.current_state.distance_mode,
            feed_rate_mode: self.current_state.feed_rate_mode,
        }
    }

    /// Get current GcodeState
    pub fn get_state(&self) -> GcodeState {
        self.current_state
    }

    /// Set current GcodeState
    pub fn set_state(&mut self, state: GcodeState) {
        self.current_state = state;
    }

    /// Update modal state based on parsed command
    fn update_modal_state(&mut self, command: &GcodeCommand) -> Result<(), String> {
        let cmd_upper = command.command.to_uppercase();

        // Parse G-codes
        if cmd_upper.contains("G00") {
            self.current_state.set_motion_mode(0)?;
        } else if cmd_upper.contains("G01") {
            self.current_state.set_motion_mode(1)?;
        } else if cmd_upper.contains("G02") {
            self.current_state.set_motion_mode(2)?;
        } else if cmd_upper.contains("G03") {
            self.current_state.set_motion_mode(3)?;
        }

        // Plane selection
        if cmd_upper.contains("G17") {
            self.current_state.set_plane_mode(17)?;
        } else if cmd_upper.contains("G18") {
            self.current_state.set_plane_mode(18)?;
        } else if cmd_upper.contains("G19") {
            self.current_state.set_plane_mode(19)?;
        }

        // Distance mode
        if cmd_upper.contains("G90") {
            self.current_state.set_distance_mode(90)?;
        } else if cmd_upper.contains("G91") {
            self.current_state.set_distance_mode(91)?;
        }

        // Feed rate mode
        if cmd_upper.contains("G93") {
            self.current_state.set_feed_rate_mode(93)?;
        } else if cmd_upper.contains("G94") {
            self.current_state.set_feed_rate_mode(94)?;
        } else if cmd_upper.contains("G95") {
            self.current_state.set_feed_rate_mode(95)?;
        }

        // Units
        if cmd_upper.contains("G20") {
            self.current_state.set_units_mode(20)?;
        } else if cmd_upper.contains("G21") {
            self.current_state.set_units_mode(21)?;
        }

        // Coordinate system (G54-G59)
        for cs in 54..=59 {
            if cmd_upper.contains(&format!("G{}", cs)) {
                self.current_state.set_coordinate_system(cs as u8)?;
                break;
            }
        }

        // Tool offset
        if cmd_upper.contains("G43") {
            self.current_state.set_tool_offset_mode(43)?;
        } else if cmd_upper.contains("G49") {
            self.current_state.set_tool_offset_mode(49)?;
        }

        // Cutter compensation
        if cmd_upper.contains("G40") {
            self.current_state.set_compensation_mode(40)?;
        } else if cmd_upper.contains("G41") {
            self.current_state.set_compensation_mode(41)?;
        } else if cmd_upper.contains("G42") {
            self.current_state.set_compensation_mode(42)?;
        }

        // Parse F value (feed rate)
        if let Some(f_pos) = cmd_upper.find('F') {
            let remaining = &command.command[f_pos + 1..];
            if let Some(f_value) = remaining.split_whitespace().next() {
                if let Ok(rate) = f_value.parse::<f64>() {
                    self.current_state.set_feed_rate(rate)?;
                }
            }
        }

        // Parse S value (spindle speed)
        if let Some(s_pos) = cmd_upper.find('S') {
            let remaining = &command.command[s_pos + 1..];
            if let Some(s_value) = remaining.split_whitespace().next() {
                if let Ok(speed) = s_value.parse::<f64>() {
                    self.current_state.set_spindle_speed(speed)?;
                }
            }
        }

        // Parse T value (tool number)
        if let Some(t_pos) = cmd_upper.find('T') {
            let remaining = &command.command[t_pos + 1..];
            if let Some(t_value) = remaining.split_whitespace().next() {
                if let Ok(tool) = t_value.parse::<u16>() {
                    self.current_state.set_tool_number(tool);
                }
            }
        }

        Ok(())
    }

    /// Get command number generator
    pub fn command_generator(&self) -> &CommandNumberGenerator {
        &self.command_generator
    }
}
