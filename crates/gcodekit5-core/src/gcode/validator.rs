//! # G-Code Validator
//!
//! Validates G-code for correctness, including 6-axis support.
//! Checks for syntax errors, axis limits, and machine compatibility.

use crate::Units;

/// Validation result for a single G-code line
#[derive(Debug, Clone, PartialEq)]
pub enum ValidationResult {
    /// Line is valid
    Valid,
    /// Warning (non-fatal issue)
    Warning(String),
    /// Error (fatal issue)
    Error(String),
}

/// Configuration for G-code validation
#[derive(Debug, Clone)]
pub struct GcodeValidatorConfig {
    /// Number of axes supported (3, 4, 5, or 6)
    pub axis_count: u8,
    /// Maximum feed rate in mm/min
    pub max_feed_rate: f64,
    /// Maximum spindle speed in RPM
    pub max_spindle_speed: f64,
    /// Minimum/maximum positions for each axis
    pub limits: AxisLimits,
    /// Units for the validation (MM or INCH)
    pub units: Units,
}

/// Axis limits for validation
#[derive(Debug, Clone)]
pub struct AxisLimits {
    /// X axis limits (min, max)
    pub x: (f64, f64),
    /// Y axis limits (min, max)
    pub y: (f64, f64),
    /// Z axis limits (min, max)
    pub z: (f64, f64),
    /// A axis limits (min, max) in degrees
    pub a: (f64, f64),
    /// B axis limits (min, max) in degrees
    pub b: (f64, f64),
    /// C axis limits (min, max) in degrees
    pub c: (f64, f64),
}

impl Default for AxisLimits {
    fn default() -> Self {
        Self {
            x: (0.0, 200.0),
            y: (0.0, 200.0),
            z: (0.0, 100.0),
            a: (0.0, 360.0),
            b: (0.0, 360.0),
            c: (0.0, 360.0),
        }
    }
}

impl Default for GcodeValidatorConfig {
    fn default() -> Self {
        Self {
            axis_count: 3,
            max_feed_rate: 10000.0,
            max_spindle_speed: 12000.0,
            limits: AxisLimits::default(),
            units: Units::MM,
        }
    }
}

/// G-Code validator for 6-axis support
#[derive(Debug, Clone)]
pub struct GcodeValidator {
    config: GcodeValidatorConfig,
    /// Errors found during validation
    errors: Vec<(usize, String)>,
    /// Warnings found during validation
    warnings: Vec<(usize, String)>,
}

impl GcodeValidator {
    /// Create a new validator with default configuration
    pub fn new() -> Self {
        Self {
            config: GcodeValidatorConfig::default(),
            errors: Vec::new(),
            warnings: Vec::new(),
        }
    }

    /// Create a validator with custom configuration
    pub fn with_config(config: GcodeValidatorConfig) -> Self {
        Self {
            config,
            errors: Vec::new(),
            warnings: Vec::new(),
        }
    }

    /// Validate a complete G-code program
    pub fn validate(&mut self, gcode: &str) -> ValidationReport {
        self.errors.clear();
        self.warnings.clear();

        for (line_num, line) in gcode.lines().enumerate() {
            let line_num = line_num + 1; // 1-based line numbers
            let result = self.validate_line(line.trim());

            match result {
                ValidationResult::Error(msg) => self.errors.push((line_num, msg)),
                ValidationResult::Warning(msg) => self.warnings.push((line_num, msg)),
                ValidationResult::Valid => {}
            }
        }

        ValidationReport {
            is_valid: self.errors.is_empty(),
            errors: self.errors.clone(),
            warnings: self.warnings.clone(),
        }
    }

    /// Validate a single G-code line
    fn validate_line(&mut self, line: &str) -> ValidationResult {
        // Skip empty lines and comments
        if line.is_empty() || line.starts_with(';') || line.starts_with('(') {
            return ValidationResult::Valid;
        }

        // Remove inline comments
        let line = match line.find(';') {
            Some(idx) => line[..idx].trim(),
            None => line,
        };

        if line.is_empty() {
            return ValidationResult::Valid;
        }

        let upper = line.to_uppercase();

        // Check for unsupported axes based on axis_count
        if let Some(err) = self.check_axis_support(&upper) {
            return ValidationResult::Error(err);
        }

        // Check axis limits
        if let Some(err) = self.check_axis_limits(&upper) {
            return ValidationResult::Error(err);
        }

        // Check feed rate limits
        if let Some(err) = self.check_feed_rate(&upper) {
            return ValidationResult::Error(err);
        }

        // Check spindle speed limits
        if let Some(err) = self.check_spindle_speed(&upper) {
            return ValidationResult::Error(err);
        }

        ValidationResult::Valid
    }

    /// Check if the line uses unsupported axes
    fn check_axis_support(&self, line: &str) -> Option<String> {
        let max_axis = match self.config.axis_count {
            3 => "Z",
            4 => "A",
            5 => "B",
            6 => "C",
            _ => "Z",
        };

        // Check for unsupported axis letters
        for word in line.split_whitespace() {
            if word.len() < 2 {
                continue;
            }

            let first_char = word.chars().next().unwrap();
            let value = &word[1..];

            // Skip if not a coordinate word
            if !['X', 'Y', 'Z', 'A', 'B', 'C'].contains(&first_char) {
                continue;
            }

            // Check if this axis is supported
            match (max_axis, first_char) {
                ("Z", 'A' | 'B' | 'C') => {
                    return Some(format!(
                        "Axis {} not supported on {}-axis machine",
                        first_char, self.config.axis_count
                    ));
                }
                ("A", 'B' | 'C') => {
                    return Some(format!(
                        "Axis {} not supported on {}-axis machine",
                        first_char, self.config.axis_count
                    ));
                }
                ("B", 'C') => {
                    return Some(format!(
                        "Axis {} not supported on {}-axis machine",
                        first_char, self.config.axis_count
                    ));
                }
                _ => {}
            }

            // Validate coordinate value
            if let Ok(val) = value.parse::<f64>() {
                if !val.is_finite() {
                    return Some(format!(
                        "Invalid {} coordinate value: {}",
                        first_char, value
                    ));
                }
            }
        }

        None
    }

    /// Check axis limits
    fn check_axis_limits(&self, line: &str) -> Option<String> {
        for word in line.split_whitespace() {
            if word.len() < 2 {
                continue;
            }

            let first_char = word.chars().next().unwrap();
            let value = &word[1..];

            // Get limits for this axis
            let (min, max) = match first_char {
                'X' => self.config.limits.x,
                'Y' => self.config.limits.y,
                'Z' => self.config.limits.z,
                'A' => self.config.limits.a,
                'B' => self.config.limits.b,
                'C' => self.config.limits.c,
                _ => continue,
            };

            if let Ok(val) = value.parse::<f64>() {
                // Convert to mm if needed
                let val = if self.config.units == Units::INCH {
                    val * 25.4
                } else {
                    val
                };

                if val < min || val > max {
                    return Some(format!(
                        "{} coordinate {} is outside limits [{:.3}, {:.3}]",
                        first_char, val, min, max
                    ));
                }
            }
        }

        None
    }

    /// Check feed rate limits
    fn check_feed_rate(&self, line: &str) -> Option<String> {
        for word in line.split_whitespace() {
            if word.starts_with('F') || word.starts_with('f') {
                let value = &word[1..];
                if let Ok(feed) = value.parse::<f64>() {
                    if feed > self.config.max_feed_rate {
                        return Some(format!(
                            "Feed rate {} exceeds maximum {}",
                            feed, self.config.max_feed_rate
                        ));
                    }
                }
            }
        }
        None
    }

    /// Check spindle speed limits
    fn check_spindle_speed(&self, line: &str) -> Option<String> {
        for word in line.split_whitespace() {
            if word.starts_with('S') || word.starts_with('s') {
                let value = &word[1..];
                if let Ok(speed) = value.parse::<f64>() {
                    if speed > self.config.max_spindle_speed {
                        return Some(format!(
                            "Spindle speed {} exceeds maximum {}",
                            speed, self.config.max_spindle_speed
                        ));
                    }
                }
            }
        }
        None
    }

    /// Get the configuration
    pub fn config(&self) -> &GcodeValidatorConfig {
        &self.config
    }

    /// Update configuration
    pub fn set_config(&mut self, config: GcodeValidatorConfig) {
        self.config = config;
    }
}

impl Default for GcodeValidator {
    fn default() -> Self {
        Self::new()
    }
}

/// Validation report for a complete G-code program
#[derive(Debug, Clone, PartialEq)]
pub struct ValidationReport {
    /// Whether the program is valid (no errors)
    pub is_valid: bool,
    /// List of errors (line number, message)
    pub errors: Vec<(usize, String)>,
    /// List of warnings (line number, message)
    pub warnings: Vec<(usize, String)>,
}

impl ValidationReport {
    /// Check if the report has any errors
    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }

    /// Check if the report has any warnings
    pub fn has_warnings(&self) -> bool {
        !self.warnings.is_empty()
    }

    /// Get error count
    pub fn error_count(&self) -> usize {
        self.errors.len()
    }

    /// Get warning count
    pub fn warning_count(&self) -> usize {
        self.warnings.len()
    }
}

impl std::fmt::Display for ValidationReport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.is_valid {
            writeln!(f, "✓ G-code is valid")?;
        } else {
            writeln!(f, "✗ G-code has {} error(s)", self.errors.len())?;
        }

        for (line, msg) in &self.errors {
            writeln!(f, "  Error at line {}: {}", line, msg)?;
        }

        for (line, msg) in &self.warnings {
            writeln!(f, "  Warning at line {}: {}", line, msg)?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_empty_line() {
        let validator = GcodeValidator::new();
        assert_eq!(validator.validate_line(""), ValidationResult::Valid);
        assert_eq!(
            validator.validate_line("; comment"),
            ValidationResult::Valid
        );
    }

    #[test]
    fn test_validate_basic_move() {
        let mut validator = GcodeValidator::new();
        let report = validator.validate("G0 X10 Y20 Z5");
        assert!(report.is_valid);
    }

    #[test]
    fn test_validate_6axis_move() {
        let mut config = GcodeValidatorConfig::default();
        config.axis_count = 6;
        let mut validator = GcodeValidator::with_config(config);

        let report = validator.validate("G1 X10 Y20 Z5 A45 B30 C90 F1000");
        assert!(report.is_valid);
    }

    #[test]
    fn test_validate_unsupported_axis() {
        let mut config = GcodeValidatorConfig::default();
        config.axis_count = 3; // Only XYZ
        let mut validator = GcodeValidator::with_config(config);

        let report = validator.validate("G1 X10 Y20 Z5 A45");
        assert!(!report.is_valid);
        assert_eq!(report.error_count(), 1);
    }

    #[test]
    fn test_validate_axis_limits() {
        let mut config = GcodeValidatorConfig::default();
        config.limits.x = (0.0, 100.0);
        let mut validator = GcodeValidator::with_config(config);

        let report = validator.validate("G0 X150");
        assert!(!report.is_valid);
    }

    #[test]
    fn test_validate_rotary_limits() {
        let mut config = GcodeValidatorConfig::default();
        config.axis_count = 4;
        config.limits.a = (0.0, 360.0);
        let mut validator = GcodeValidator::with_config(config);

        // Valid within limits
        let report = validator.validate("G0 A180");
        assert!(report.is_valid);

        // Outside limits
        let report = validator.validate("G0 A400");
        assert!(!report.is_valid);
    }

    #[test]
    fn test_validate_feed_rate() {
        let mut config = GcodeValidatorConfig::default();
        config.max_feed_rate = 5000.0;
        let mut validator = GcodeValidator::with_config(config);

        let report = validator.validate("G1 X10 F6000");
        assert!(!report.is_valid);
    }
}
