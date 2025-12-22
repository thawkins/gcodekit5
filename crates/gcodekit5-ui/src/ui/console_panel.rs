//! Console/Output Panel - Task 73
//!
//! Display controller responses, command history, and message filtering

use std::collections::VecDeque;

/// Console message level
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageLevel {
    /// Debug message
    Debug,
    /// Info message
    Info,
    /// Warning message
    Warning,
    /// Error message
    Error,
    /// Success message
    Success,
}

impl std::fmt::Display for MessageLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Debug => write!(f, "DEBUG"),
            Self::Info => write!(f, "INFO"),
            Self::Warning => write!(f, "WARN"),
            Self::Error => write!(f, "ERR"),
            Self::Success => write!(f, "OK"),
        }
    }
}

/// Console message
#[derive(Debug, Clone)]
pub struct ConsoleMessage {
    /// Message level
    pub level: MessageLevel,
    /// Message text
    pub text: String,
    /// Timestamp (seconds since epoch)
    pub timestamp: u64,
    /// Is command echo
    pub is_command: bool,
}

impl ConsoleMessage {
    /// Create new message
    pub fn new(level: MessageLevel, text: impl Into<String>) -> Self {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        Self {
            level,
            text: text.into(),
            timestamp,
            is_command: false,
        }
    }

    /// Create command message
    pub fn command(text: impl Into<String>) -> Self {
        let mut msg = Self::new(MessageLevel::Info, text);
        msg.is_command = true;
        msg
    }

    /// Get formatted message
    pub fn formatted(&self) -> String {
        format!("[{}] {}", self.level, self.text)
    }

    /// Get formatted message with timestamp
    pub fn formatted_with_time(&self) -> String {
        let hours = (self.timestamp / 3600) % 24;
        let minutes = (self.timestamp / 60) % 60;
        let seconds = self.timestamp % 60;

        format!(
            "[{:02}:{:02}:{:02}] [{}] {}",
            hours, minutes, seconds, self.level, self.text
        )
    }
}

/// Message filter for console
#[derive(Debug, Clone)]
pub struct MessageFilter {
    /// Show debug messages
    pub debug: bool,
    /// Show info messages
    pub info: bool,
    /// Show warning messages
    pub warning: bool,
    /// Show error messages
    pub error: bool,
    /// Show success messages
    pub success: bool,
    /// Show command echos
    pub commands: bool,
    /// Text search filter (case-insensitive)
    pub text_filter: Option<String>,
}

impl MessageFilter {
    /// Create filter showing all messages
    pub fn show_all() -> Self {
        Self {
            debug: true,
            info: true,
            warning: true,
            error: true,
            success: true,
            commands: true,
            text_filter: None,
        }
    }

    /// Create filter showing only errors
    pub fn errors_only() -> Self {
        Self {
            debug: false,
            info: false,
            warning: true,
            error: true,
            success: false,
            commands: false,
            text_filter: None,
        }
    }

    /// Check if message passes filter
    pub fn matches(&self, msg: &ConsoleMessage) -> bool {
        let level_match = match msg.level {
            MessageLevel::Debug => self.debug,
            MessageLevel::Info => self.info,
            MessageLevel::Warning => self.warning,
            MessageLevel::Error => self.error,
            MessageLevel::Success => self.success,
        };

        if !level_match {
            return false;
        }

        if msg.is_command && !self.commands {
            return false;
        }

        if let Some(ref filter) = self.text_filter {
            let text_lower = msg.text.to_lowercase();
            let filter_lower = filter.to_lowercase();
            if !text_lower.contains(&filter_lower) {
                return false;
            }
        }

        true
    }
}

impl Default for MessageFilter {
    fn default() -> Self {
        Self::show_all()
    }
}

/// Command history entry
#[derive(Debug, Clone)]
pub struct HistoryEntry {
    /// Command text
    pub command: String,
    /// Timestamp
    pub timestamp: u64,
}

impl HistoryEntry {
    /// Create new history entry
    pub fn new(command: impl Into<String>) -> Self {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        Self {
            command: command.into(),
            timestamp,
        }
    }
}

/// Console/Output panel
#[derive(Debug)]
pub struct ConsolePanel {
    /// All messages
    pub messages: VecDeque<ConsoleMessage>,
    /// Command history
    pub history: VecDeque<HistoryEntry>,
    /// Current filter
    pub filter: MessageFilter,
    /// Maximum messages to keep
    pub max_messages: usize,
    /// Maximum history entries to keep
    pub max_history: usize,
    /// Auto-scroll enabled
    pub auto_scroll: bool,
    /// Current scroll position (from end)
    pub scroll_position: usize,
    /// Show timestamps
    pub show_timestamps: bool,
}

impl ConsolePanel {
    /// Create new console panel
    pub fn new() -> Self {
        Self {
            messages: VecDeque::new(),
            history: VecDeque::new(),
            filter: MessageFilter::default(),
            max_messages: 1000,
            max_history: 100,
            auto_scroll: true,
            scroll_position: 0,
            show_timestamps: true,
        }
    }

    /// Add message to console
    pub fn add_message(&mut self, level: MessageLevel, text: impl Into<String>) {
        let msg = ConsoleMessage::new(level, text);
        self.messages.push_back(msg);

        if self.messages.len() > self.max_messages {
            self.messages.pop_front();
        }

        if self.auto_scroll {
            self.scroll_position = 0;
        }
    }

    /// Add command message
    pub fn add_command(&mut self, command: impl Into<String>) {
        let msg = ConsoleMessage::command(command);
        self.messages.push_back(msg);

        if self.messages.len() > self.max_messages {
            self.messages.pop_front();
        }

        if self.auto_scroll {
            self.scroll_position = 0;
        }
    }

    /// Add to command history
    pub fn add_to_history(&mut self, command: impl Into<String>) {
        let entry = HistoryEntry::new(command);
        self.history.push_back(entry);

        if self.history.len() > self.max_history {
            self.history.pop_front();
        }
    }

    /// Get command history list
    pub fn get_history(&self) -> Vec<String> {
        self.history.iter().map(|e| e.command.clone()).collect()
    }

    /// Get filtered messages
    pub fn get_filtered_messages(&self) -> Vec<ConsoleMessage> {
        self.messages
            .iter()
            .filter(|m| self.filter.matches(m))
            .cloned()
            .collect()
    }

    /// Get displayed messages (respecting scroll)
    pub fn get_displayed_messages(&self, limit: usize) -> Vec<ConsoleMessage> {
        let filtered = self.get_filtered_messages();
        let total = filtered.len();

        if total <= limit {
            filtered
        } else {
            let start = total.saturating_sub(limit + self.scroll_position);
            let end = start + limit;
            filtered[start..end.min(total)].to_vec()
        }
    }

    /// Get displayed message strings
    pub fn get_displayed_strings(&self, limit: usize) -> Vec<String> {
        self.get_displayed_messages(limit)
            .iter()
            .rev()
            .map(|m| {
                if self.show_timestamps {
                    m.formatted_with_time()
                } else {
                    m.formatted()
                }
            })
            .collect()
    }

    /// Clear all messages
    pub fn clear(&mut self) {
        self.messages.clear();
        self.scroll_position = 0;
    }

    /// Clear history
    pub fn clear_history(&mut self) {
        self.history.clear();
    }

    /// Set filter
    pub fn set_filter(&mut self, filter: MessageFilter) {
        self.filter = filter;
        self.scroll_position = 0;
    }

    /// Toggle debug messages
    pub fn toggle_debug(&mut self) {
        self.filter.debug = !self.filter.debug;
    }

    /// Toggle info messages
    pub fn toggle_info(&mut self) {
        self.filter.info = !self.filter.info;
    }

    /// Toggle warning messages
    pub fn toggle_warning(&mut self) {
        self.filter.warning = !self.filter.warning;
    }

    /// Toggle error messages
    pub fn toggle_error(&mut self) {
        self.filter.error = !self.filter.error;
    }

    /// Toggle command echos
    pub fn toggle_commands(&mut self) {
        self.filter.commands = !self.filter.commands;
    }

    /// Set text filter
    pub fn set_text_filter(&mut self, text: Option<String>) {
        self.filter.text_filter = text;
    }

    /// Scroll up
    pub fn scroll_up(&mut self) {
        self.scroll_position = self.scroll_position.saturating_add(1);
    }

    /// Scroll down
    pub fn scroll_down(&mut self) {
        self.scroll_position = self.scroll_position.saturating_sub(1);
    }

    /// Get message count (total)
    pub fn message_count(&self) -> usize {
        self.messages.len()
    }

    /// Get filtered message count
    pub fn filtered_count(&self) -> usize {
        self.get_filtered_messages().len()
    }

    /// Get history count
    pub fn history_count(&self) -> usize {
        self.history.len()
    }
}

impl Default for ConsolePanel {
    fn default() -> Self {
        Self::new()
    }
}
