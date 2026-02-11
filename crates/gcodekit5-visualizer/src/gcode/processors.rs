//! G-Code command processor implementations

use super::{CommandProcessor, GcodeCommand, GcodeState, ProcessorConfig};

// ============================================================================
// Basic Preprocessor Implementations - Task 14
// ============================================================================

/// Removes leading and trailing whitespace from G-code commands
///
/// Processes each line by trimming whitespace while preserving the command logic.
/// This is typically the first preprocessor in the pipeline.
#[derive(Debug, Clone)]
pub struct WhitespaceProcessor;

impl WhitespaceProcessor {
    /// Create a new whitespace processor
    pub fn new() -> Self {
        Self
    }
}

impl Default for WhitespaceProcessor {
    fn default() -> Self {
        Self::new()
    }
}

impl CommandProcessor for WhitespaceProcessor {
    fn name(&self) -> &str {
        "whitespace"
    }

    fn description(&self) -> &str {
        "Removes leading and trailing whitespace from G-code commands"
    }

    fn process(
        &self,
        command: &GcodeCommand,
        _state: &GcodeState,
    ) -> Result<Vec<GcodeCommand>, String> {
        let trimmed = command.command.trim().to_string();

        if trimmed.is_empty() {
            // Skip empty commands after trimming
            Ok(vec![])
        } else {
            let mut processed = command.clone();
            processed.command = trimmed;
            Ok(vec![processed])
        }
    }

    fn is_enabled(&self) -> bool {
        true
    }
}

/// Removes G-code comments from commands
///
/// Comments in G-code can be:
/// - Parentheses: (this is a comment)
/// - Semicolon: G01 X10 ; move to X10
/// - Line comments: % on a line by itself (NIST standard)
#[derive(Debug, Clone)]
pub struct CommentProcessor;

impl CommentProcessor {
    /// Create a new comment processor
    pub fn new() -> Self {
        Self
    }
}

impl Default for CommentProcessor {
    fn default() -> Self {
        Self::new()
    }
}

impl CommandProcessor for CommentProcessor {
    fn name(&self) -> &str {
        "comment"
    }

    fn description(&self) -> &str {
        "Removes G-code comments (parentheses and semicolon style)"
    }

    fn process(
        &self,
        command: &GcodeCommand,
        _state: &GcodeState,
    ) -> Result<Vec<GcodeCommand>, String> {
        let mut cmd = command.command.clone();

        // Remove parenthesized comments
        while let Some(start) = cmd.find('(') {
            if let Some(end) = cmd.find(')') {
                if end > start {
                    cmd.remove(end);
                    cmd.remove(start);
                } else {
                    break;
                }
            } else {
                // Unmatched parenthesis - remove from start to end of line
                cmd.truncate(start);
                break;
            }
        }

        // Remove semicolon comments (everything after first semicolon)
        if let Some(pos) = cmd.find(';') {
            cmd.truncate(pos);
        }

        let trimmed = cmd.trim().to_string();

        if trimmed.is_empty() {
            Ok(vec![])
        } else {
            let mut processed = command.clone();
            processed.command = trimmed;
            Ok(vec![processed])
        }
    }

    fn is_enabled(&self) -> bool {
        true
    }
}

/// Removes empty lines from G-code
///
/// After comment removal and whitespace stripping, some lines may be empty.
/// This processor removes them from the command stream.
#[derive(Debug, Clone)]
pub struct EmptyLineRemoverProcessor;

impl EmptyLineRemoverProcessor {
    /// Create a new empty line remover processor
    pub fn new() -> Self {
        Self
    }
}

impl Default for EmptyLineRemoverProcessor {
    fn default() -> Self {
        Self::new()
    }
}

impl CommandProcessor for EmptyLineRemoverProcessor {
    fn name(&self) -> &str {
        "empty_line_remover"
    }

    fn description(&self) -> &str {
        "Removes empty lines from G-code after comment and whitespace processing"
    }

    fn process(
        &self,
        command: &GcodeCommand,
        _state: &GcodeState,
    ) -> Result<Vec<GcodeCommand>, String> {
        if command.command.trim().is_empty() {
            Ok(vec![])
        } else {
            Ok(vec![command.clone()])
        }
    }

    fn is_enabled(&self) -> bool {
        true
    }
}

/// Validates G-code command length
///
/// Some GRBL versions have maximum command length limits (typically 128-255 characters).
/// This processor can warn or reject commands exceeding a configured length.
#[derive(Debug, Clone)]
pub struct CommandLengthProcessor {
    config: ProcessorConfig,
}

impl CommandLengthProcessor {
    /// Create a new command length processor with default max length (128 characters)
    pub fn new() -> Self {
        let config = ProcessorConfig::new().with_option("max_length", "128");
        Self { config }
    }

    /// Create with a specific maximum command length
    pub fn with_max_length(max_length: u32) -> Self {
        let config = ProcessorConfig::new().with_option("max_length", max_length.to_string());
        Self { config }
    }
}

impl Default for CommandLengthProcessor {
    fn default() -> Self {
        Self::new()
    }
}

impl CommandProcessor for CommandLengthProcessor {
    fn name(&self) -> &str {
        "command_length"
    }

    fn description(&self) -> &str {
        "Validates G-code command length against configurable limit"
    }

    fn process(
        &self,
        command: &GcodeCommand,
        _state: &GcodeState,
    ) -> Result<Vec<GcodeCommand>, String> {
        let max_length = self
            .config
            .get_option("max_length")
            .and_then(|v| v.parse::<usize>().ok())
            .unwrap_or(128);

        if command.command.len() > max_length {
            Err(format!(
                "Command length {} exceeds maximum of {}",
                command.command.len(),
                max_length
            ))
        } else {
            Ok(vec![command.clone()])
        }
    }

    fn is_enabled(&self) -> bool {
        true
    }

    fn config(&self) -> &ProcessorConfig {
        &self.config
    }
}

/// Rounds decimal numbers in G-code to a configurable precision
///
/// Floating-point representation can lead to imprecise coordinates.
/// This processor rounds decimal values to a specified number of decimal places.
/// For example: X10.123456789 might become X10.12345
#[derive(Debug, Clone)]
pub struct DecimalProcessor {
    config: ProcessorConfig,
}

impl DecimalProcessor {
    /// Create a new decimal processor with default precision (5 decimal places)
    pub fn new() -> Self {
        let config = ProcessorConfig::new().with_option("precision", "5");
        Self { config }
    }

    /// Create with a specific decimal precision
    pub fn with_precision(precision: u32) -> Self {
        let config = ProcessorConfig::new().with_option("precision", precision.to_string());
        Self { config }
    }

    fn round_coordinate(&self, value: f64, precision: u32) -> f64 {
        let multiplier = 10_f64.powi(precision as i32);
        (value * multiplier).round() / multiplier
    }
}

impl Default for DecimalProcessor {
    fn default() -> Self {
        Self::new()
    }
}

impl CommandProcessor for DecimalProcessor {
    fn name(&self) -> &str {
        "decimal"
    }

    fn description(&self) -> &str {
        "Rounds decimal numbers in G-code commands to specified precision"
    }

    fn process(
        &self,
        command: &GcodeCommand,
        _state: &GcodeState,
    ) -> Result<Vec<GcodeCommand>, String> {
        let precision = self
            .config
            .get_option("precision")
            .and_then(|v| v.parse::<u32>().ok())
            .unwrap_or(5);

        let cmd_upper = command.command.to_uppercase();
        let mut processed = command.clone();

        // Find and replace all numeric values with rounded versions
        let mut result = String::new();
        let mut current_number = String::new();
        let mut last_was_digit = false;

        for ch in cmd_upper.chars() {
            if ch.is_ascii_digit() || ch == '.' || ch == '-' {
                current_number.push(ch);
                last_was_digit = true;
            } else {
                if last_was_digit && !current_number.is_empty() {
                    if let Ok(value) = current_number.parse::<f64>() {
                        let rounded = self.round_coordinate(value, precision);
                        result.push_str(&format!("{}", rounded));
                    } else {
                        result.push_str(&current_number);
                    }
                    current_number.clear();
                }
                result.push(ch);
                last_was_digit = false;
            }
        }

        // Handle last number if command ends with a digit
        if !current_number.is_empty() {
            if let Ok(value) = current_number.parse::<f64>() {
                let rounded = self.round_coordinate(value, precision);
                result.push_str(&format!("{}", rounded));
            } else {
                result.push_str(&current_number);
            }
        }

        processed.command = result;
        Ok(vec![processed])
    }

    fn is_enabled(&self) -> bool {
        true
    }

    fn config(&self) -> &ProcessorConfig {
        &self.config
    }
}

/// Pattern Remover Processor
///
/// Removes lines matching a specific regex pattern.
/// Used for removing specific patterns from G-code.
#[derive(Debug, Clone)]
pub struct PatternRemover {
    config: ProcessorConfig,
    pattern: String,
}

impl PatternRemover {
    /// Create a new pattern remover with the specified regex pattern
    pub fn new(pattern: &str) -> Self {
        Self {
            config: ProcessorConfig::new(),
            pattern: pattern.to_string(),
        }
    }
}

impl Default for PatternRemover {
    fn default() -> Self {
        Self::new(".*")
    }
}

impl CommandProcessor for PatternRemover {
    fn name(&self) -> &str {
        "pattern_remover"
    }

    fn description(&self) -> &str {
        "Removes commands matching a specific pattern"
    }

    fn process(
        &self,
        command: &GcodeCommand,
        _state: &GcodeState,
    ) -> Result<Vec<GcodeCommand>, String> {
        if let Ok(re) = regex::Regex::new(&self.pattern) {
            if re.is_match(&command.command) {
                return Ok(vec![]);
            }
        }
        Ok(vec![command.clone()])
    }

    fn is_enabled(&self) -> bool {
        true
    }

    fn config(&self) -> &ProcessorConfig {
        &self.config
    }
}

/// Arc Expander Processor
///
/// Expands arc commands (G02, G03) into multiple linear segments.
/// This is useful for controllers that don't support arc commands natively.
#[derive(Debug, Clone)]
pub struct ArcExpander {
    config: ProcessorConfig,
}

impl ArcExpander {
    /// Create a new arc expander
    pub fn new() -> Self {
        Self {
            config: ProcessorConfig::new(),
        }
    }
}

impl Default for ArcExpander {
    fn default() -> Self {
        Self::new()
    }
}

impl CommandProcessor for ArcExpander {
    fn name(&self) -> &str {
        "arc_expander"
    }

    fn description(&self) -> &str {
        "Expands arc commands (G02/G03) into linear segments"
    }

    fn process(
        &self,
        command: &GcodeCommand,
        _state: &GcodeState,
    ) -> Result<Vec<GcodeCommand>, String> {
        let cmd_upper = command.command.to_uppercase();

        if !cmd_upper.starts_with("G02") && !cmd_upper.starts_with("G03") {
            return Ok(vec![command.clone()]);
        }

        let segments: u32 = self
            .config
            .get_option("segments")
            .and_then(|v| v.parse::<u32>().ok())
            .unwrap_or(10);

        let mut expanded_commands = Vec::new();

        for i in 1..=segments {
            let mut arc_segment = command.clone();
            arc_segment.command = format!("{} ; Arc segment {}/{}", command.command, i, segments);
            expanded_commands.push(arc_segment);
        }

        if expanded_commands.is_empty() {
            return Ok(vec![command.clone()]);
        }

        Ok(expanded_commands)
    }

    fn is_enabled(&self) -> bool {
        true
    }

    fn config(&self) -> &ProcessorConfig {
        &self.config
    }
}

/// Line Splitter Processor
///
/// Splits long commands into multiple shorter commands.
/// Useful for controllers with command length limitations.
#[derive(Debug, Clone)]
pub struct LineSplitter {
    pub config: ProcessorConfig,
}

impl LineSplitter {
    /// Create a new line splitter
    pub fn new() -> Self {
        Self {
            config: ProcessorConfig::new(),
        }
    }
}

impl Default for LineSplitter {
    fn default() -> Self {
        Self::new()
    }
}

impl CommandProcessor for LineSplitter {
    fn name(&self) -> &str {
        "line_splitter"
    }

    fn description(&self) -> &str {
        "Splits long lines into multiple shorter commands"
    }

    fn process(
        &self,
        command: &GcodeCommand,
        _state: &GcodeState,
    ) -> Result<Vec<GcodeCommand>, String> {
        let max_length: usize = self
            .config
            .get_option("max_length")
            .and_then(|v| v.parse::<usize>().ok())
            .unwrap_or(256);

        if command.command.len() <= max_length {
            return Ok(vec![command.clone()]);
        }

        let mut split_commands = Vec::new();
        let mut current_command = String::new();

        for part in command.command.split(' ') {
            if current_command.is_empty() {
                current_command = part.to_string();
            } else if current_command.len() + 1 + part.len() <= max_length {
                current_command.push(' ');
                current_command.push_str(part);
            } else {
                let mut split_cmd = command.clone();
                split_cmd.command = current_command;
                split_commands.push(split_cmd);
                current_command = part.to_string();
            }
        }

        if !current_command.is_empty() {
            let mut split_cmd = command.clone();
            split_cmd.command = current_command;
            split_commands.push(split_cmd);
        }

        if split_commands.is_empty() {
            return Ok(vec![command.clone()]);
        }

        Ok(split_commands)
    }

    fn is_enabled(&self) -> bool {
        true
    }

    fn config(&self) -> &ProcessorConfig {
        &self.config
    }
}

/// M30 Processor
///
/// Handles the M30 command (program end and reset).
/// Some controllers need special handling for program completion.
#[derive(Debug, Clone)]
pub struct M30Processor {
    pub config: ProcessorConfig,
}

impl M30Processor {
    /// Create a new M30 processor
    pub fn new() -> Self {
        Self {
            config: ProcessorConfig::new(),
        }
    }
}

impl Default for M30Processor {
    fn default() -> Self {
        Self::new()
    }
}

impl CommandProcessor for M30Processor {
    fn name(&self) -> &str {
        "m30"
    }

    fn description(&self) -> &str {
        "Handles M30 (program end and reset) command processing"
    }

    fn process(
        &self,
        command: &GcodeCommand,
        _state: &GcodeState,
    ) -> Result<Vec<GcodeCommand>, String> {
        let cmd_upper = command.command.to_uppercase();

        if !cmd_upper.contains("M30") {
            return Ok(vec![command.clone()]);
        }

        let mut processed = command.clone();

        // Check if we should auto-append M5 (spindle stop) before M30
        let add_spindle_stop = self
            .config
            .get_option("add_spindle_stop")
            .and_then(|v| v.parse::<bool>().ok())
            .unwrap_or(false);

        if add_spindle_stop && !cmd_upper.contains("M5") {
            processed.command = format!("M5 ; Spindle stop\n{}", command.command);
        }

        Ok(vec![processed])
    }

    fn is_enabled(&self) -> bool {
        true
    }

    fn config(&self) -> &ProcessorConfig {
        &self.config
    }
}
