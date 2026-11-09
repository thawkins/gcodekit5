//! G-Code Validator - Task 65
//!
//! Validates G-code syntax, ranges, and consistency.
//!
//! Lines are tokenized with [`crate::comment_processor::words`], so comments are ignored and
//! only whole words are checked.

use crate::comment_processor::words;

/// Validation error
#[derive(Debug, Clone)]
pub struct ValidationError {
    /// Line number
    pub line: usize,
    /// Error message
    pub message: String,
}

/// Validator configuration
#[derive(Debug, Clone)]
pub struct ValidatorConfig {
    /// Max X
    pub max_x: f64,
    /// Min X
    pub min_x: f64,
    /// Max Y
    pub max_y: f64,
    /// Min Y
    pub min_y: f64,
    /// Max Z
    pub max_z: f64,
    /// Min Z
    pub min_z: f64,
}

impl Default for ValidatorConfig {
    fn default() -> Self {
        Self {
            max_x: gcodekit5_core::constants::WORLD_EXTENT_MM,
            min_x: -gcodekit5_core::constants::WORLD_EXTENT_MM,
            max_y: gcodekit5_core::constants::WORLD_EXTENT_MM,
            min_y: -gcodekit5_core::constants::WORLD_EXTENT_MM,
            max_z: 500.0,
            min_z: -500.0,
        }
    }
}

/// Validates G-code
#[derive(Debug)]
pub struct GCodeValidator {
    config: ValidatorConfig,
}

impl GCodeValidator {
    /// Create validator
    pub fn new(config: ValidatorConfig) -> Self {
        Self { config }
    }

    /// Validate program
    pub fn validate(&self, lines: &[String]) -> Result<(), Vec<ValidationError>> {
        let mut errors = Vec::new();

        for (line_num, line) in lines.iter().enumerate() {
            if let Err(e) = self.validate_line(line, line_num) {
                errors.extend(e);
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    fn validate_line(&self, line: &str, line_num: usize) -> Result<(), Vec<ValidationError>> {
        let mut errors = Vec::new();
        let mut push = |message: String| {
            errors.push(ValidationError {
                line: line_num,
                message,
            })
        };

        // Comments are removed first so letters inside them are not read as words.
        for word in words(line) {
            if word.value.is_nan() {
                push(format!("Invalid number for word '{}'", word.letter));
                continue;
            }
            let range = match word.letter {
                'X' => Some((self.config.min_x, self.config.max_x)),
                'Y' => Some((self.config.min_y, self.config.max_y)),
                'Z' => Some((self.config.min_z, self.config.max_z)),
                _ => None,
            };
            if let Some((min, max)) = range {
                if word.value < min || word.value > max {
                    push(format!(
                        "{} {} out of range [{}, {}]",
                        word.letter, word.value, min, max
                    ));
                }
            }
            if word.letter == 'F' && word.value <= 0.0 {
                push(format!("Feed rate must be positive, got {}", word.value));
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

impl Default for GCodeValidator {
    fn default() -> Self {
        Self::new(ValidatorConfig::default())
    }
}
