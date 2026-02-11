//! TinyG Response Parser
//!
//! This module provides parsing of TinyG JSON responses and status reports.

use gcodekit5_core::Position;
use serde_json::Value;
use std::collections::HashMap;

/// TinyG response types
#[derive(Debug, Clone, PartialEq)]
pub enum TinyGResponseType {
    /// OK acknowledgment
    Ok,
    /// NACK (negative acknowledgment)
    Nack,
    /// Status report
    StatusReport,
    /// System status
    SystemStatus,
    /// Settings response
    Settings,
    /// Error response
    Error,
    /// Startup message
    Startup,
    /// Unknown response type
    Unknown,
}

/// Parsed TinyG response
#[derive(Debug, Clone)]
pub struct TinyGResponse {
    /// Response type
    pub response_type: TinyGResponseType,
    /// Line number (if applicable)
    pub line_number: Option<u32>,
    /// Raw JSON value
    pub value: Option<Value>,
    /// Error code (if error)
    pub error_code: Option<u16>,
    /// Error message (if error)
    pub error_message: Option<String>,
}

impl TinyGResponse {
    /// Check if response indicates success
    pub fn is_success(&self) -> bool {
        self.response_type == TinyGResponseType::Ok
    }

    /// Check if response indicates an error
    pub fn is_error(&self) -> bool {
        matches!(
            self.response_type,
            TinyGResponseType::Error | TinyGResponseType::Nack
        )
    }

    /// Check if response is a status report
    pub fn is_status_report(&self) -> bool {
        self.response_type == TinyGResponseType::StatusReport
    }
}

/// Parsed TinyG status report
#[derive(Debug, Clone)]
pub struct TinyGStatus {
    /// Machine state
    pub state: String,
    /// Line number
    pub line_number: Option<u32>,
    /// Machine position (X, Y, Z, A)
    pub machine_position: Position,
    /// Work position (X, Y, Z, A)
    pub work_position: Position,
    /// Feed rate in units per minute
    pub feed_rate: f64,
    /// Spindle speed in RPM
    pub spindle_speed: f64,
    /// Current units (0 = mm, 1 = inches)
    pub units: u8,
    /// Work coordinate system offset
    pub work_offset: Option<(f64, f64, f64)>,
    /// Additional fields from status report
    pub extra_fields: HashMap<String, Value>,
}

impl Default for TinyGStatus {
    fn default() -> Self {
        Self {
            state: "Idle".to_string(),
            line_number: None,
            machine_position: Position::default(),
            work_position: Position::default(),
            feed_rate: 0.0,
            spindle_speed: 0.0,
            units: 0,
            work_offset: None,
            extra_fields: HashMap::new(),
        }
    }
}

/// TinyG response parser
#[derive(Debug, Default)]
#[allow(dead_code)]
pub struct TinyGResponseParser {
    /// Partial response buffer (for multi-line responses)
    buffer: String,
}

impl TinyGResponseParser {
    /// Create a new parser
    pub fn new() -> Self {
        Self {
            buffer: String::new(),
        }
    }

    /// Parse a complete TinyG JSON response
    pub fn parse(&mut self, response: &str) -> Result<TinyGResponse, String> {
        let trimmed = response.trim();

        if trimmed.is_empty() {
            return Err("Empty response".to_string());
        }

        // Try to parse as JSON
        match serde_json::from_str::<Value>(trimmed) {
            Ok(json) => self.parse_json_response(&json),
            Err(_) => {
                // Handle non-JSON responses
                if trimmed.contains("Grbl") {
                    Ok(TinyGResponse {
                        response_type: TinyGResponseType::Startup,
                        line_number: None,
                        value: None,
                        error_code: None,
                        error_message: None,
                    })
                } else {
                    Err(format!("Failed to parse response: {}", trimmed))
                }
            }
        }
    }

    /// Parse a JSON response into a TinyGResponse
    fn parse_json_response(&self, json: &Value) -> Result<TinyGResponse, String> {
        let json_obj = json
            .as_object()
            .ok_or_else(|| "Response is not a JSON object".to_string())?;

        // Check for error
        if let Some(err_obj) = json_obj.get("er") {
            return self.parse_error_response(err_obj);
        }

        // Check for status report
        if json_obj.contains_key("sr") {
            return Ok(TinyGResponse {
                response_type: TinyGResponseType::StatusReport,
                line_number: json_obj.get("n").and_then(Value::as_u64).map(|n| n as u32),
                value: Some(json.clone()),
                error_code: None,
                error_message: None,
            });
        }

        // Check for system status
        if json_obj.contains_key("sys") {
            return Ok(TinyGResponse {
                response_type: TinyGResponseType::SystemStatus,
                line_number: None,
                value: Some(json.clone()),
                error_code: None,
                error_message: None,
            });
        }

        // Check for settings
        if json_obj.len() == 1 {
            if let Some(key) = json_obj.keys().next() {
                if key.starts_with('f') || key.starts_with('m') || key.starts_with('$') {
                    return Ok(TinyGResponse {
                        response_type: TinyGResponseType::Settings,
                        line_number: None,
                        value: Some(json.clone()),
                        error_code: None,
                        error_message: None,
                    });
                }
            }
        }

        // Check for OK
        if json_obj.get("ok").and_then(Value::as_bool).unwrap_or(false) {
            return Ok(TinyGResponse {
                response_type: TinyGResponseType::Ok,
                line_number: json_obj.get("n").and_then(Value::as_u64).map(|n| n as u32),
                value: Some(json.clone()),
                error_code: None,
                error_message: None,
            });
        }

        // Default to unknown
        Ok(TinyGResponse {
            response_type: TinyGResponseType::Unknown,
            line_number: None,
            value: Some(json.clone()),
            error_code: None,
            error_message: None,
        })
    }

    /// Parse an error response
    fn parse_error_response(&self, err_obj: &Value) -> Result<TinyGResponse, String> {
        let error_code = err_obj
            .get("code")
            .and_then(Value::as_u64)
            .map(|c| c as u16);

        let error_message = err_obj
            .get("msg")
            .and_then(Value::as_str)
            .map(|s| s.to_string());

        Ok(TinyGResponse {
            response_type: TinyGResponseType::Error,
            line_number: None,
            value: None,
            error_code,
            error_message,
        })
    }

    /// Parse a status report into TinyGStatus
    pub fn parse_status_report(&self, response: &TinyGResponse) -> Result<TinyGStatus, String> {
        let json = response
            .value
            .as_ref()
            .ok_or_else(|| "No JSON value in response".to_string())?;

        let sr = json
            .get("sr")
            .ok_or_else(|| "No status report in response".to_string())?;

        let mut status = TinyGStatus::default();

        // Parse state
        if let Some(state_val) = sr.get("stat") {
            if let Some(state_obj) = state_val.as_object() {
                if let Some(state_str) = state_obj.get("state").and_then(Value::as_str) {
                    status.state = state_str.to_string();
                }
            }
        }

        // Parse positions
        if let Some(pos_val) = sr.get("pos") {
            if let Some(pos_obj) = pos_val.as_object() {
                let x = pos_obj.get("x").and_then(Value::as_f64).unwrap_or(0.0) as f32;
                let y = pos_obj.get("y").and_then(Value::as_f64).unwrap_or(0.0) as f32;
                let z = pos_obj.get("z").and_then(Value::as_f64).unwrap_or(0.0) as f32;
                let a = pos_obj.get("a").and_then(Value::as_f64).unwrap_or(0.0) as f32;
                status.work_position = Position::with_a(x, y, z, a);
            }
        }

        if let Some(mpos_val) = sr.get("mpos") {
            if let Some(mpos_obj) = mpos_val.as_object() {
                let x = mpos_obj.get("x").and_then(Value::as_f64).unwrap_or(0.0) as f32;
                let y = mpos_obj.get("y").and_then(Value::as_f64).unwrap_or(0.0) as f32;
                let z = mpos_obj.get("z").and_then(Value::as_f64).unwrap_or(0.0) as f32;
                let a = mpos_obj.get("a").and_then(Value::as_f64).unwrap_or(0.0) as f32;
                status.machine_position = Position::with_a(x, y, z, a);
            }
        }

        // Parse feed rate and speed
        if let Some(feed) = sr.get("feed").and_then(Value::as_f64) {
            status.feed_rate = feed;
        }

        if let Some(speed) = sr.get("speed").and_then(Value::as_f64) {
            status.spindle_speed = speed;
        }

        // Parse units
        if let Some(unit_val) = sr.get("unit").and_then(Value::as_u64) {
            status.units = unit_val as u8;
        }

        // Parse line number
        if let Some(line) = sr.get("line").and_then(Value::as_u64) {
            status.line_number = Some(line as u32);
        }

        Ok(status)
    }
}
