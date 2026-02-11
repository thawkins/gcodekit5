//! GRBL Response Parser
//!
//! This module parses GRBL protocol responses including status reports, error messages,
//! alarm messages, settings responses, and other GRBL-specific responses.

use gcodekit5_core::{CNCPoint, Units};
use serde::{Deserialize, Serialize};
use std::fmt;

/// GRBL response types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum GrblResponse {
    /// OK acknowledgment
    Ok,
    /// Error response with error code
    Error(u8),
    /// Alarm response with alarm code
    Alarm(u8),
    /// Status report
    Status(StatusReport),
    /// Setting response ($n=value)
    Setting { number: u8, value: String },
    /// Parser state (e.g., modal state)
    ParserState(String),
    /// Version information
    Version(String),
    /// Build information ($I)
    BuildInfo(String),
    /// Status reports mask ($10)
    StatusMask(u8),
    /// Startup message or other text
    Message(String),
}

impl fmt::Display for GrblResponse {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Ok => write!(f, "ok"),
            Self::Error(code) => write!(f, "{}", super::error_decoder::format_error(*code)),
            Self::Alarm(code) => write!(f, "{}", super::error_decoder::format_alarm(*code)),
            Self::Status(_) => write!(f, "status"),
            Self::Setting { number, value } => write!(f, "setting:${}={}", number, value),
            Self::ParserState(state) => write!(f, "parser_state:{}", state),
            Self::Version(version) => write!(f, "version:{}", version),
            Self::BuildInfo(info) => write!(f, "build_info:{}", info),
            Self::StatusMask(mask) => write!(f, "status_mask:{}", mask),
            Self::Message(msg) => write!(f, "message:{}", msg),
        }
    }
}

/// GRBL status report
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StatusReport {
    /// Machine state
    pub state: String,
    /// Machine position in work coordinates
    pub machine_pos: CNCPoint,
    /// Work position
    pub work_pos: CNCPoint,
    /// Buffer state (Buf:plan:exec)
    pub buffer_state: Option<BufferState>,
    /// Feed rate
    pub feed_rate: Option<f64>,
    /// Spindle speed (RPM)
    pub spindle_speed: Option<u32>,
    /// Work coordinate system offset
    pub work_coord_offset: Option<CNCPoint>,
}

/// Buffer state in status report
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct BufferState {
    /// Plan buffer blocks
    pub plan: u8,
    /// Execution buffer bytes
    pub exec: u8,
}

/// GRBL settings response
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SettingsResponse {
    /// Setting number
    pub number: u8,
    /// Setting value as string
    pub value: String,
    /// Parsed numeric value (if applicable)
    pub numeric_value: Option<f64>,
}

/// GRBL response parser
#[allow(dead_code)]
pub struct GrblResponseParser {
    settings_cache: Vec<SettingsResponse>,
}

impl GrblResponseParser {
    /// Create a new GRBL response parser
    pub fn new() -> Self {
        Self {
            settings_cache: Vec::new(),
        }
    }

    /// Parse a GRBL response line
    pub fn parse(&self, line: &str) -> Option<GrblResponse> {
        let line = line.trim();

        if line.is_empty() {
            return None;
        }

        // Check for OK
        if line == "ok" {
            return Some(GrblResponse::Ok);
        }

        // Check for error: prefix
        if let Some(stripped) = line.strip_prefix("error:") {
            if let Ok(code) = stripped.parse::<u8>() {
                return Some(GrblResponse::Error(code));
            }
        }

        // Check for alarm: prefix
        if let Some(stripped) = line.strip_prefix("alarm:") {
            if let Ok(code) = stripped.parse::<u8>() {
                return Some(GrblResponse::Alarm(code));
            }
        }

        // Check for status report (starts with < and ends with >)
        if line.starts_with('<') && line.ends_with('>') {
            return self.parse_status_report(&line[1..line.len() - 1]);
        }

        // Check for setting response ($n=value)
        if line.starts_with('$') && line.contains('=') {
            return self.parse_setting(line);
        }

        // Check for version (starts with "Grbl ")
        if line.starts_with("Grbl ") {
            return Some(GrblResponse::Version(line.to_string()));
        }

        // Check for build info (starts with "[")
        if line.starts_with('[') && line.ends_with(']') {
            return Some(GrblResponse::BuildInfo(line.to_string()));
        }

        // Everything else is a message
        Some(GrblResponse::Message(line.to_string()))
    }

    /// Parse a status report
    fn parse_status_report(&self, status_line: &str) -> Option<GrblResponse> {
        let mut parts = status_line.split('|');

        // First part is always state
        let state = parts.next()?.trim().to_string();

        let mut machine_pos = CNCPoint::new(Units::MM);
        let mut work_pos = CNCPoint::new(Units::MM);
        let mut buffer_state = None;
        let mut feed_rate = None;
        let mut spindle_speed = None;
        let mut work_coord_offset = None;

        for part in parts {
            let part = part.trim();

            if let Some(pos_str) = part.strip_prefix("MPos:") {
                machine_pos = self.parse_position(pos_str, Units::MM)?;
            } else if let Some(pos_str) = part.strip_prefix("WPos:") {
                work_pos = self.parse_position(pos_str, Units::MM)?;
            } else if let Some(buf_str) = part.strip_prefix("Buf:") {
                buffer_state = self.parse_buffer_state(buf_str);
            } else if let Some(rate_str) = part.strip_prefix("F:") {
                feed_rate = rate_str.parse::<f64>().ok();
            } else if let Some(speed_str) = part.strip_prefix("S:") {
                spindle_speed = speed_str.parse::<u32>().ok();
            } else if let Some(offset_str) = part.strip_prefix("WCO:") {
                work_coord_offset = self.parse_position(offset_str, Units::MM);
            }
        }

        Some(GrblResponse::Status(StatusReport {
            state,
            machine_pos,
            work_pos,
            buffer_state,
            feed_rate,
            spindle_speed,
            work_coord_offset,
        }))
    }

    /// Parse position coordinates
    fn parse_position(&self, pos_str: &str, unit: Units) -> Option<CNCPoint> {
        let coords: Vec<f64> = pos_str
            .split(',')
            .filter_map(|s| s.trim().parse::<f64>().ok())
            .collect();

        if coords.is_empty() {
            return None;
        }

        let x = coords.first().copied().unwrap_or(0.0);
        let y = coords.get(1).copied().unwrap_or(0.0);
        let z = coords.get(2).copied().unwrap_or(0.0);
        let a = coords.get(3).copied().unwrap_or(0.0);
        let b = coords.get(4).copied().unwrap_or(0.0);
        let c = coords.get(5).copied().unwrap_or(0.0);

        Some(CNCPoint::with_axes(x, y, z, a, b, c, unit))
    }

    /// Parse buffer state
    fn parse_buffer_state(&self, buf_str: &str) -> Option<BufferState> {
        let parts: Vec<&str> = buf_str.split(':').collect();

        if parts.len() < 2 {
            return None;
        }

        let plan = parts[0].trim().parse::<u8>().ok()?;
        let exec = parts[1].trim().parse::<u8>().ok()?;

        Some(BufferState { plan, exec })
    }

    /// Parse a setting response
    fn parse_setting(&self, line: &str) -> Option<GrblResponse> {
        let line = &line[1..]; // Skip '$'
        let parts: Vec<&str> = line.split('=').collect();

        if parts.len() != 2 {
            return None;
        }

        let number = parts[0].trim().parse::<u8>().ok()?;
        let value = parts[1].trim().to_string();
        let _numeric_value = value.parse::<f64>().ok();

        Some(GrblResponse::Setting { number, value })
    }

    /// Get error description
    pub fn error_description(code: u8) -> &'static str {
        match code {
            1 => "Expected command letter",
            2 => "Bad number format",
            3 => "Invalid statement",
            4 => "Negative value",
            5 => "Setting disabled",
            20 => "Unsupported or invalid g-code command",
            21 => "Modal group violation",
            22 => "Undefined feed rate",
            23 => "Failed to execute startup block",
            24 => "EEPROM read failed",
            _ => "Unknown error",
        }
    }

    /// Get alarm description
    pub fn alarm_description(code: u8) -> &'static str {
        match code {
            1 => "Hard limit triggered",
            2 => "Soft limit exceeded",
            3 => "Abort during cycle",
            4 => "Probe fail",
            5 => "Probe not triggered",
            6 => "Homing fail",
            7 => "Homing fail pulloff",
            8 => "Spindle control failure",
            9 => "Cooling mist control failure",
            _ => "Unknown alarm",
        }
    }
}

impl Default for GrblResponseParser {
    fn default() -> Self {
        Self::new()
    }
}
