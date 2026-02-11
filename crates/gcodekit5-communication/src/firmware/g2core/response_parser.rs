//! g2core Response Parser
//!
//! This module provides parsing of g2core JSON responses and status reports.
//! g2core supports extended status reports with 6-axis support and advanced features.

use gcodekit5_core::Position;
use serde_json::Value;
use std::collections::HashMap;

/// g2core response types
#[derive(Debug, Clone, PartialEq)]
pub enum G2CoreResponseType {
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

/// Parsed g2core response
#[derive(Debug, Clone)]
pub struct G2CoreResponse {
    /// Response type
    pub response_type: G2CoreResponseType,
    /// Line number (if applicable)
    pub line_number: Option<u32>,
    /// Raw JSON value
    pub value: Option<Value>,
    /// Error code (if error)
    pub error_code: Option<u16>,
    /// Error message (if error)
    pub error_message: Option<String>,
}

impl G2CoreResponse {
    /// Check if response indicates success
    pub fn is_success(&self) -> bool {
        self.response_type == G2CoreResponseType::Ok
    }

    /// Check if response indicates an error
    pub fn is_error(&self) -> bool {
        matches!(
            self.response_type,
            G2CoreResponseType::Error | G2CoreResponseType::Nack
        )
    }

    /// Check if response is a status report
    pub fn is_status_report(&self) -> bool {
        self.response_type == G2CoreResponseType::StatusReport
    }
}

/// Parsed g2core status report (supports 6 axes)
#[derive(Debug, Clone)]
pub struct G2CoreStatus {
    /// Machine state
    pub state: String,
    /// Line number
    pub line_number: Option<u32>,
    /// Machine position (X, Y, Z, A, B, C)
    pub machine_position: Position,
    /// Work position (X, Y, Z, A, B, C)
    pub work_position: Position,
    /// Feed rate in units per minute
    pub feed_rate: f64,
    /// Spindle speed in RPM
    pub spindle_speed: f64,
    /// Current units (0 = mm, 1 = inches)
    pub units: u8,
    /// Work coordinate system offset
    pub work_offset: Option<(f32, f32, f32)>,
    /// Rotational axes (A, B, C if applicable)
    pub rotational_axes: Option<(f32, f32, f32)>,
    /// Additional fields from status report
    pub extra_fields: HashMap<String, Value>,
}

impl Default for G2CoreStatus {
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
            rotational_axes: None,
            extra_fields: HashMap::new(),
        }
    }
}

/// g2core response parser
#[derive(Debug, Default)]
pub struct G2CoreResponseParser {
    /// Partial response buffer (for multi-line responses)
    buffer: String,
}

impl G2CoreResponseParser {
    /// Create a new parser
    pub fn new() -> Self {
        Self {
            buffer: String::new(),
        }
    }

    /// Parse a complete g2core JSON response
    pub fn parse(&mut self, response: &str) -> Result<G2CoreResponse, String> {
        let trimmed = response.trim();

        if trimmed.is_empty() {
            return Err("Empty response".to_string());
        }

        // Try to parse as JSON
        match serde_json::from_str::<Value>(trimmed) {
            Ok(json) => self.parse_json_response(&json),
            Err(_) => {
                // Handle non-JSON responses
                if trimmed.contains("g2core") {
                    Ok(G2CoreResponse {
                        response_type: G2CoreResponseType::Startup,
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

    /// Parse a JSON response into a G2CoreResponse
    fn parse_json_response(&self, json: &Value) -> Result<G2CoreResponse, String> {
        let json_obj = json
            .as_object()
            .ok_or_else(|| "Response is not a JSON object".to_string())?;

        // Check for error
        if let Some(err_obj) = json_obj.get("er") {
            return self.parse_error_response(err_obj);
        }

        // Check for status report
        if json_obj.contains_key("sr") {
            return Ok(G2CoreResponse {
                response_type: G2CoreResponseType::StatusReport,
                line_number: json_obj.get("n").and_then(Value::as_u64).map(|n| n as u32),
                value: Some(json.clone()),
                error_code: None,
                error_message: None,
            });
        }

        // Check for system status
        if json_obj.contains_key("sys") {
            return Ok(G2CoreResponse {
                response_type: G2CoreResponseType::SystemStatus,
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
                    return Ok(G2CoreResponse {
                        response_type: G2CoreResponseType::Settings,
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
            return Ok(G2CoreResponse {
                response_type: G2CoreResponseType::Ok,
                line_number: json_obj.get("n").and_then(Value::as_u64).map(|n| n as u32),
                value: Some(json.clone()),
                error_code: None,
                error_message: None,
            });
        }

        // Default to unknown
        Ok(G2CoreResponse {
            response_type: G2CoreResponseType::Unknown,
            line_number: None,
            value: Some(json.clone()),
            error_code: None,
            error_message: None,
        })
    }

    /// Parse an error response
    fn parse_error_response(&self, err_obj: &Value) -> Result<G2CoreResponse, String> {
        let error_code = err_obj
            .get("code")
            .and_then(Value::as_u64)
            .map(|c| c as u16);

        let error_message = err_obj
            .get("msg")
            .and_then(Value::as_str)
            .map(|s| s.to_string());

        Ok(G2CoreResponse {
            response_type: G2CoreResponseType::Error,
            line_number: None,
            value: None,
            error_code,
            error_message,
        })
    }

    /// Parse a status report into G2CoreStatus
    pub fn parse_status_report(&self, response: &G2CoreResponse) -> Result<G2CoreStatus, String> {
        let json = response
            .value
            .as_ref()
            .ok_or_else(|| "No JSON value in response".to_string())?;

        let sr = json
            .get("sr")
            .ok_or_else(|| "No status report in response".to_string())?;

        let mut status = G2CoreStatus::default();

        // Parse state
        if let Some(state_val) = sr.get("stat") {
            if let Some(state_obj) = state_val.as_object() {
                if let Some(state_str) = state_obj.get("state").and_then(Value::as_str) {
                    status.state = state_str.to_string();
                }
            }
        }

        // Parse positions with 6-axis support
        if let Some(pos_val) = sr.get("pos") {
            if let Some(pos_obj) = pos_val.as_object() {
                let x = pos_obj.get("x").and_then(Value::as_f64).unwrap_or(0.0) as f32;
                let y = pos_obj.get("y").and_then(Value::as_f64).unwrap_or(0.0) as f32;
                let z = pos_obj.get("z").and_then(Value::as_f64).unwrap_or(0.0) as f32;
                let a = pos_obj.get("a").and_then(Value::as_f64).unwrap_or(0.0) as f32;
                let b = pos_obj.get("b").and_then(Value::as_f64).unwrap_or(0.0) as f32;
                let c = pos_obj.get("c").and_then(Value::as_f64).unwrap_or(0.0) as f32;
                status.work_position = Position::with_a(x, y, z, a);
                status.rotational_axes = Some((a, b, c));
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
