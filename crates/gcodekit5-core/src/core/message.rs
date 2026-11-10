//! Message service for logging and displaying messages
//!
//! Provides:
//! - Message types with severity levels
//! - Message dispatcher for publishing messages
//! - Console output formatting
//! - Message filtering by level

/// Message severity level.
///
/// Ordered from least to most severe. Used to filter messages — setting a
/// minimum level of `Warning` suppresses `Verbose` and `Info` messages.
///
/// # Example
/// ```
/// use gcodekit5_core::core::message::MessageLevel;
///
/// assert!(MessageLevel::Error > MessageLevel::Warning);
/// assert!(MessageLevel::Warning > MessageLevel::Info);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum MessageLevel {
    /// Verbose/Debug level
    Verbose = 0,
    /// Information level
    Info = 1,
    /// Warning level
    Warning = 2,
    /// Error level
    Error = 3,
}

impl std::fmt::Display for MessageLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Verbose => write!(f, "VERB"),
            Self::Info => write!(f, "INFO"),
            Self::Warning => write!(f, "WARN"),
            Self::Error => write!(f, "ERR!"),
        }
    }
}

/// Message from controller or system.
///
/// Timestamped, leveled message with source identification. Used for the
/// console log, status display, and debugging output.
///
/// # Example
/// ```
/// use gcodekit5_core::core::message::Message;
///
/// let msg = Message::info("GRBL", "Homing cycle complete");
/// println!("{}", msg.format_console());
/// // Output: [HH:MM:SS.mmm] INFO | GRBL: Homing cycle complete
/// ```
#[derive(Debug, Clone)]
pub struct Message {
    /// Timestamp of message
    pub timestamp: String,
    /// Message level/severity
    pub level: MessageLevel,
    /// Message source (controller, system, etc.)
    pub source: String,
    /// Message text
    pub text: String,
}

impl Message {
    /// Create a new message
    pub fn new(level: MessageLevel, source: impl Into<String>, text: impl Into<String>) -> Self {
        let timestamp = chrono::Local::now().format("%H:%M:%S%.3f").to_string();
        Self {
            timestamp,
            level,
            source: source.into(),
            text: text.into(),
        }
    }

    /// Create an info message
    pub fn info(source: impl Into<String>, text: impl Into<String>) -> Self {
        Self::new(MessageLevel::Info, source, text)
    }

    /// Create a warning message
    pub fn warning(source: impl Into<String>, text: impl Into<String>) -> Self {
        Self::new(MessageLevel::Warning, source, text)
    }

    /// Create an error message
    pub fn error(source: impl Into<String>, text: impl Into<String>) -> Self {
        Self::new(MessageLevel::Error, source, text)
    }

    /// Create a verbose message
    pub fn verbose(source: impl Into<String>, text: impl Into<String>) -> Self {
        Self::new(MessageLevel::Verbose, source, text)
    }

    /// Format message for console output
    pub fn format_console(&self) -> String {
        format!(
            "[{}] {} | {}: {}",
            self.timestamp, self.level, self.source, self.text
        )
    }
}

impl std::fmt::Display for Message {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.format_console())
    }
}
