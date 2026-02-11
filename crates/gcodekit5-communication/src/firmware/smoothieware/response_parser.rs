//! Smoothieware response parser
//!
//! Parses responses from Smoothieware firmware including status reports,
//! errors, and standard responses.

/// Parsed Smoothieware response
#[derive(Debug, Clone, PartialEq)]
pub enum SmoothiewareResponse {
    /// Command was successful (ok)
    Ok,
    /// Error response with message
    Error(String),
    /// Position feedback
    Position { x: f64, y: f64, z: f64 },
    /// Firmware version
    Version(String),
    /// Temperature feedback
    Temperature(f32),
    /// Raw line from controller
    Raw(String),
}

/// Parser for Smoothieware protocol responses
#[derive(Debug, Clone)]
pub struct SmoothiewareResponseParser {
    /// Buffer for incomplete responses
    buffer: String,
}

impl SmoothiewareResponseParser {
    /// Create a new response parser
    pub fn new() -> Self {
        Self {
            buffer: String::new(),
        }
    }

    /// Parse a single line from Smoothieware
    pub fn parse_line(&mut self, line: &str) -> Option<SmoothiewareResponse> {
        let line = line.trim();

        if line.is_empty() {
            return None;
        }

        // Check for acknowledgment
        if line == "ok" || line == "OK" {
            return Some(SmoothiewareResponse::Ok);
        }

        // Check for error
        if line.starts_with("Error:") {
            let error_msg = line
                .strip_prefix("Error:")
                .unwrap_or(line)
                .trim()
                .to_string();
            return Some(SmoothiewareResponse::Error(error_msg));
        }

        // Check for version
        if line.starts_with("Smoothieware") {
            return Some(SmoothiewareResponse::Version(line.to_string()));
        }

        // Check for position feedback (X:... Y:... Z:...)
        if line.contains("X:") && line.contains("Y:") && line.contains("Z:") {
            if let Some(parsed) = self.parse_position(line) {
                return Some(parsed);
            }
        }

        // Check for temperature
        if line.contains("Temp:") || line.contains("temp:") {
            if let Some(temp) = self.parse_temperature(line) {
                return Some(SmoothiewareResponse::Temperature(temp));
            }
        }

        // Return as raw response
        Some(SmoothiewareResponse::Raw(line.to_string()))
    }

    /// Parse position from response line
    fn parse_position(&self, line: &str) -> Option<SmoothiewareResponse> {
        let mut x = 0.0;
        let mut y = 0.0;
        let mut z = 0.0;
        let mut found_all = false;

        for part in line.split_whitespace() {
            if let Some(stripped) = part.strip_prefix("X:") {
                if let Ok(val) = stripped.parse::<f64>() {
                    x = val;
                }
            } else if let Some(stripped) = part.strip_prefix("Y:") {
                if let Ok(val) = stripped.parse::<f64>() {
                    y = val;
                }
            } else if let Some(stripped) = part.strip_prefix("Z:") {
                if let Ok(val) = stripped.parse::<f64>() {
                    z = val;
                    found_all = true;
                }
            }
        }

        if found_all {
            Some(SmoothiewareResponse::Position { x, y, z })
        } else {
            None
        }
    }

    /// Parse temperature from response
    fn parse_temperature(&self, line: &str) -> Option<f32> {
        for part in line.split_whitespace() {
            if part.starts_with("Temp:") || part.starts_with("temp:") {
                let temp_str = part.split(':').nth(1)?;
                if let Ok(temp) = temp_str.parse::<f32>() {
                    return Some(temp);
                }
            }
        }
        None
    }

    /// Clear the parser buffer
    pub fn reset(&mut self) {
        self.buffer.clear();
    }
}

impl Default for SmoothiewareResponseParser {
    fn default() -> Self {
        Self::new()
    }
}
