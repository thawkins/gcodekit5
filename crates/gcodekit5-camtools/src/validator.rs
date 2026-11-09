//! G-Code Validator - Task 65
//!
//! Validates G-code syntax, ranges, and consistency.
//!
//! Lines are tokenized with [`crate::comment_processor::words`], so comments are ignored and
//! only whole words are checked.
//!
//! GRBL system commands (lines starting with `$`) are classified with
//! [`crate::comment_processor::system_command`]: known commands (`$H`, `$X`, `$$`, `$#`,
//! `$RST=*`, `$100=80`, ...) are accepted, unknown ones are reported, and the G-code payload
//! of a `$J=` jog is checked with the normal word checks (it must also carry a feed rate, as
//! GRBL requires).

use crate::comment_processor::{system_command, words, SystemCommand, Word};

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
        let messages = match system_command(line) {
            None => self.check_words(&words(line)),
            Some(SystemCommand::Known(_)) => Vec::new(),
            Some(SystemCommand::Jog(payload)) => {
                let jog_words = words(&payload);
                let mut messages = self.check_words(&jog_words);
                if jog_words.is_empty() {
                    messages.push("Jog command '$J=' has no G-code payload".to_string());
                } else if !jog_words.iter().any(|w| w.letter == 'F') {
                    messages.push("Jog command '$J=' requires a feed rate (F)".to_string());
                }
                messages
            }
            Some(SystemCommand::Unknown(command)) => {
                vec![format!("Unknown GRBL system command '{}'", command)]
            }
        };

        if messages.is_empty() {
            Ok(())
        } else {
            Err(messages
                .into_iter()
                .map(|message| ValidationError {
                    line: line_num,
                    message,
                })
                .collect())
        }
    }

    /// Check G-code words for unparsable numbers, out-of-range coordinates and bad feeds.
    fn check_words(&self, line_words: &[Word]) -> Vec<String> {
        let mut messages = Vec::new();
        for word in line_words {
            if word.value.is_nan() {
                messages.push(format!("Invalid number for word '{}'", word.letter));
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
                    messages.push(format!(
                        "{} {} out of range [{}, {}]",
                        word.letter, word.value, min, max
                    ));
                }
            }
            if word.letter == 'F' && word.value <= 0.0 {
                messages.push(format!("Feed rate must be positive, got {}", word.value));
            }
        }
        messages
    }
}

impl Default for GCodeValidator {
    fn default() -> Self {
        Self::new(ValidatorConfig::default())
    }
}
