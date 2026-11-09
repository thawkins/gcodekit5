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

/// A single G-code word: an uppercase address letter and its numeric value.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GcodeWord {
    /// Address letter (always uppercase ASCII)
    pub letter: char,
    /// Numeric value, or `None` if the letter was not followed by a plain number
    /// (e.g. an expression like `X[#1+2]` or stray text)
    pub value: Option<f64>,
}

/// Split a comment-free G-code line into words without requiring spaces between
/// them, so `G1X10Y-2.5F300` and `G1 X10 Y-2.5 F300` tokenise identically.
///
/// Whitespace inside a word (`X 10`) is accepted as GRBL does. Characters that are
/// not ASCII letters or part of a number are ignored.
///
/// # Arguments
/// * `line` - A G-code line with comments already removed.
///
/// # Returns
/// The words in order of appearance.
pub fn tokenize_words(line: &str) -> Vec<GcodeWord> {
    let chars: Vec<char> = line.chars().collect();
    let mut words = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if !c.is_ascii_alphabetic() {
            i += 1;
            continue;
        }
        let letter = c.to_ascii_uppercase();
        i += 1;
        while i < chars.len() && chars[i].is_whitespace() {
            i += 1;
        }
        let mut number = String::new();
        if i < chars.len() && (chars[i] == '+' || chars[i] == '-') {
            number.push(chars[i]);
            i += 1;
        }
        while i < chars.len() && (chars[i].is_ascii_digit() || chars[i] == '.') {
            number.push(chars[i]);
            i += 1;
        }
        let value = number.parse::<f64>().ok();
        words.push(GcodeWord { letter, value });
    }
    words
}

/// G-code validator for 6-axis support.
///
/// Tracks the modal distance mode (G90/G91) and units (G20/G21) across the lines
/// of a program so limits are only checked against absolute targets.
#[derive(Debug, Clone)]
pub struct GcodeValidator {
    config: GcodeValidatorConfig,
    /// Errors found during validation
    errors: Vec<(usize, String)>,
    /// Warnings found during validation
    warnings: Vec<(usize, String)>,
    /// Modal distance mode: `true` after G91 (incremental), `false` after G90
    relative: bool,
    /// Modal units in effect for the program being validated
    units: Units,
}

/// Axis letters in order; index matches axis count requirement - 1 for A/B/C.
const AXIS_LETTERS: [char; 6] = ['X', 'Y', 'Z', 'A', 'B', 'C'];

impl GcodeValidator {
    /// Create a new validator with default configuration
    pub fn new() -> Self {
        Self::with_config(GcodeValidatorConfig::default())
    }

    /// Create a validator with custom configuration
    pub fn with_config(config: GcodeValidatorConfig) -> Self {
        let units = config.units;
        Self {
            config,
            errors: Vec::new(),
            warnings: Vec::new(),
            relative: false,
            units,
        }
    }

    /// Validate a complete G-code program.
    ///
    /// Modal state starts as absolute (G90) in the configured units and is updated
    /// by G90/G91/G20/G21 words as the program is read.
    pub fn validate(&mut self, gcode: &str) -> ValidationReport {
        self.errors.clear();
        self.warnings.clear();
        self.relative = false;
        self.units = self.config.units;

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

    /// Remove `;` and `( ... )` comments from a line.
    fn strip_comments(line: &str) -> String {
        let line = match line.find(';') {
            Some(idx) => &line[..idx],
            None => line,
        };
        let mut stripped = String::with_capacity(line.len());
        let mut depth = 0usize;
        for c in line.chars() {
            match c {
                '(' => {
                    if depth == 0 {
                        stripped.push(' '); // keep words on either side separate
                    }
                    depth += 1;
                }
                ')' if depth > 0 => depth -= 1,
                _ if depth == 0 => stripped.push(c),
                _ => {}
            }
        }
        stripped
    }

    /// Validate a single G-code line, updating modal state.
    fn validate_line(&mut self, line: &str) -> ValidationResult {
        let stripped = Self::strip_comments(line);
        let mut body = stripped.trim();
        if body.is_empty() || body.starts_with('%') {
            return ValidationResult::Valid;
        }

        // GRBL jog: `$J=<block>`. Its G90/G91/G20/G21 apply to the jog only.
        let mut is_jog = false;
        if body.starts_with('$') {
            match body.get(..3) {
                Some(prefix) if prefix.eq_ignore_ascii_case("$J=") => {
                    is_jog = true;
                    body = &body[3..];
                }
                // Other system commands ($H, $X, $$, $110=500 ...) are not motion.
                _ => return ValidationResult::Valid,
            }
        }

        let words = tokenize_words(body);

        // Modal/non-modal G-codes on this line apply to the whole line.
        let mut relative = self.relative;
        let mut units = self.units;
        let mut non_positional = false;
        for w in words.iter().filter(|w| w.letter == 'G') {
            let Some(g) = w.value else { continue };
            // Compare in tenths so G91.1 (arc IJK mode) is not mistaken for G91.
            match (g * 10.0).round() as i64 {
                900 => relative = false,
                910 => relative = true,
                200 => units = Units::INCH,
                210 => units = Units::MM,
                // G10 (set offsets/tool table) and G92 family carry axis words that
                // are offsets, not target positions.
                100 | 920 | 921 | 922 | 923 => non_positional = true,
                _ => {}
            }
        }
        if !is_jog {
            self.relative = relative;
            self.units = units;
        }

        if let Some(err) = self.check_axis_support(&words) {
            return ValidationResult::Error(err);
        }
        if !non_positional {
            if let Some(err) = self.check_axis_limits(&words, relative, units) {
                return ValidationResult::Error(err);
            }
        }
        if let Some(err) = self.check_feed_rate(&words, units) {
            return ValidationResult::Error(err);
        }
        if let Some(err) = self.check_spindle_speed(&words) {
            return ValidationResult::Error(err);
        }

        ValidationResult::Valid
    }

    /// Check if the line uses unsupported axes or non-finite coordinates
    fn check_axis_support(&self, words: &[GcodeWord]) -> Option<String> {
        let axis_count = self.config.axis_count.clamp(3, 6) as usize;
        for w in words {
            let Some(idx) = AXIS_LETTERS.iter().position(|&a| a == w.letter) else {
                continue;
            };
            let Some(val) = w.value else { continue };
            if idx >= axis_count {
                return Some(format!(
                    "Axis {} not supported on {}-axis machine",
                    w.letter, self.config.axis_count
                ));
            }
            if !val.is_finite() {
                return Some(format!("Invalid {} coordinate value: {}", w.letter, val));
            }
        }
        None
    }

    /// Limits for an axis letter, if it is an axis.
    fn limits_for(&self, letter: char) -> Option<(f64, f64)> {
        let l = &self.config.limits;
        match letter {
            'X' => Some(l.x),
            'Y' => Some(l.y),
            'Z' => Some(l.z),
            'A' => Some(l.a),
            'B' => Some(l.b),
            'C' => Some(l.c),
            _ => None,
        }
    }

    /// Check axis limits.
    ///
    /// In absolute mode each target is checked against the axis range. In relative
    /// mode the start position is unknown, so only increments larger than the
    /// whole axis travel (which must leave the envelope) are rejected.
    fn check_axis_limits(
        &self,
        words: &[GcodeWord],
        relative: bool,
        units: Units,
    ) -> Option<String> {
        for w in words {
            let Some((min, max)) = self.limits_for(w.letter) else {
                continue;
            };
            let Some(raw) = w.value else { continue };
            // Rotary axes are in degrees and never unit-converted.
            let is_linear = matches!(w.letter, 'X'..='Z');
            let val = if is_linear && units == Units::INCH {
                raw * 25.4
            } else {
                raw
            };

            if relative {
                let span = max - min;
                if val.abs() > span {
                    return Some(format!(
                        "{} incremental move {:.3} exceeds axis travel {:.3}",
                        w.letter, val, span
                    ));
                }
            } else if val < min || val > max {
                return Some(format!(
                    "{} coordinate {} is outside limits [{:.3}, {:.3}]",
                    w.letter, val, min, max
                ));
            }
        }
        None
    }

    /// Check feed rate limits (feed is converted to mm/min in inch mode)
    fn check_feed_rate(&self, words: &[GcodeWord], units: Units) -> Option<String> {
        for w in words.iter().filter(|w| w.letter == 'F') {
            let Some(feed) = w.value else { continue };
            let feed_mm = if units == Units::INCH {
                feed * 25.4
            } else {
                feed
            };
            if feed_mm > self.config.max_feed_rate {
                return Some(format!(
                    "Feed rate {} exceeds maximum {}",
                    feed, self.config.max_feed_rate
                ));
            }
        }
        None
    }

    /// Check spindle speed limits
    fn check_spindle_speed(&self, words: &[GcodeWord]) -> Option<String> {
        for w in words.iter().filter(|w| w.letter == 'S') {
            let Some(speed) = w.value else { continue };
            if speed > self.config.max_spindle_speed {
                return Some(format!(
                    "Spindle speed {} exceeds maximum {}",
                    speed, self.config.max_spindle_speed
                ));
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
        let mut validator = GcodeValidator::new();
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
        let config = GcodeValidatorConfig {
            axis_count: 6,
            ..Default::default()
        };
        let mut validator = GcodeValidator::with_config(config);

        let report = validator.validate("G1 X10 Y20 Z5 A45 B30 C90 F1000");
        assert!(report.is_valid);
    }

    #[test]
    fn test_validate_unsupported_axis() {
        let config = GcodeValidatorConfig {
            axis_count: 3, // Only XYZ
            ..Default::default()
        };
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
        let mut config = GcodeValidatorConfig {
            axis_count: 4,
            ..Default::default()
        };
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
        let config = GcodeValidatorConfig {
            max_feed_rate: 5000.0,
            ..Default::default()
        };
        let mut validator = GcodeValidator::with_config(config);

        let report = validator.validate("G1 X10 F6000");
        assert!(!report.is_valid);
    }
}
