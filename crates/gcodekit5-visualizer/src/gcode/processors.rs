//! G-Code command processor implementations

use super::words::strip_comments;
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
        // Remove parenthesized comments (including their text) and `;` comments
        let cmd = strip_comments(&command.command);

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

    /// Upper-case a comment-free code segment and round every number in it,
    /// appending the result to `out`.
    fn round_code_segment(&self, code: &str, precision: u32, out: &mut String) {
        let mut current_number = String::new();
        let flush = |number: &mut String, out: &mut String| {
            if number.is_empty() {
                return;
            }
            match number.parse::<f64>() {
                Ok(value) => out.push_str(&format!("{}", self.round_coordinate(value, precision))),
                Err(_) => out.push_str(number),
            }
            number.clear();
        };

        for ch in code.chars() {
            if ch.is_ascii_digit() || ch == '.' || ch == '-' {
                current_number.push(ch);
            } else {
                flush(&mut current_number, out);
                out.extend(ch.to_uppercase());
            }
        }
        flush(&mut current_number, out);
    }
}

/// Split a G-code line into code and comment segments.
///
/// Returns `(is_comment, text)` pairs that concatenate back to `line`.
/// Parenthesised comments run to the closing `)` (or the end of the line when
/// unclosed); a `;` outside parentheses starts a comment that runs to the end.
fn split_code_and_comments(line: &str) -> Vec<(bool, &str)> {
    let mut segments = Vec::new();
    let mut start = 0;
    let mut in_paren = false;
    for (idx, ch) in line.char_indices() {
        if in_paren {
            if ch == ')' {
                segments.push((true, &line[start..idx + 1]));
                start = idx + 1;
                in_paren = false;
            }
            continue;
        }
        match ch {
            '(' => {
                if idx > start {
                    segments.push((false, &line[start..idx]));
                }
                start = idx;
                in_paren = true;
            }
            ';' => {
                if idx > start {
                    segments.push((false, &line[start..idx]));
                }
                segments.push((true, &line[idx..]));
                return segments;
            }
            _ => {}
        }
    }
    if start < line.len() {
        segments.push((in_paren, &line[start..]));
    }
    segments
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

        let mut processed = command.clone();
        let mut result = String::with_capacity(command.command.len());
        for (is_comment, segment) in split_code_and_comments(&command.command) {
            if is_comment {
                // Comments are copied verbatim (no case change, no rounding).
                result.push_str(segment);
            } else {
                self.round_code_segment(segment, precision, &mut result);
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
    /// Compiled once at construction; `None` when the pattern is invalid, in
    /// which case every command passes through unchanged.
    regex: Option<regex::Regex>,
}

impl PatternRemover {
    /// Create a new pattern remover with the specified regex pattern
    ///
    /// # Arguments
    /// * `pattern` - Regular expression; matching commands are removed.
    pub fn new(pattern: &str) -> Self {
        let regex = match regex::Regex::new(pattern) {
            Ok(re) => Some(re),
            Err(e) => {
                tracing::warn!(pattern, error = %e, "PatternRemover: invalid regex, disabled");
                None
            }
        };
        Self {
            config: ProcessorConfig::new(),
            pattern: pattern.to_string(),
            regex,
        }
    }

    /// The pattern this remover was created with.
    pub fn pattern(&self) -> &str {
        &self.pattern
    }

    /// Whether the pattern compiled successfully.
    pub fn is_valid(&self) -> bool {
        self.regex.is_some()
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
        if let Some(re) = &self.regex {
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
/// Placeholder for expanding arc commands (G02, G03) into linear segments.
/// Expansion needs the machine position at the start of each arc, which the
/// processor pipeline does not provide, so arcs are currently passed through
/// unchanged for the controller to interpolate natively.
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
        "Passes arc commands (G02/G03) through unchanged; arc expansion is not supported"
    }

    fn process(
        &self,
        command: &GcodeCommand,
        _state: &GcodeState,
    ) -> Result<Vec<GcodeCommand>, String> {
        // Expanding an arc requires the machine position at the start of the arc,
        // which is not part of `GcodeState`. Emitting copies of the arc (the
        // previous behaviour) re-executes it several times, which in G90 turns
        // every copy after the first into a full circle. Pass the command through
        // unchanged instead so the controller interpolates it natively.
        tracing::debug!(
            "ArcExpander: passing '{}' through unchanged (start position unknown)",
            command.command
        );
        Ok(vec![command.clone()])
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
