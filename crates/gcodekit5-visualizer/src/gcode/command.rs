//! G-Code command types and lifecycle management

use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use uuid::Uuid;

/// Unique identifier for a G-Code command
pub type CommandId = String;

/// Command execution state
///
/// Represents the lifecycle state of a G-Code command from creation through completion.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CommandState {
    /// Command created but not yet sent
    Pending,
    /// Command sent to controller, awaiting response
    Sent,
    /// Controller acknowledged command with "ok"
    Ok,
    /// Command execution completed
    Done,
    /// Command generated an error response
    Error,
    /// Command was skipped (not sent)
    Skipped,
}

impl std::fmt::Display for CommandState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pending => write!(f, "Pending"),
            Self::Sent => write!(f, "Sent"),
            Self::Ok => write!(f, "Ok"),
            Self::Done => write!(f, "Done"),
            Self::Error => write!(f, "Error"),
            Self::Skipped => write!(f, "Skipped"),
        }
    }
}

/// Response from the controller for a command
///
/// Captures the controller's response to a sent command,
/// including any error messages or status information.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CommandResponse {
    /// Whether the command was accepted/acknowledged
    pub success: bool,
    /// Response message from controller (e.g., "ok", error message)
    pub message: String,
    /// Error code if applicable
    pub error_code: Option<u32>,
    /// Additional response data
    pub data: Option<String>,
}

/// Represents a parsed and tracked G-Code command
///
/// Comprehensive representation of a G-Code command including:
/// - Command text and metadata
/// - Execution state tracking
/// - ID generation for tracking
/// - Response handling
/// - Timestamp information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GcodeCommand {
    /// Unique identifier for this command
    pub id: CommandId,
    /// G-Code line (e.g., "G00 X10.5 Y20.3 Z0.0")
    pub line: String,
    /// Line number if present in file
    pub line_number: Option<u32>,
    /// Raw command text (parsed)
    pub command: String,
    /// Command execution state
    pub state: CommandState,
    /// Command numbering (0-based, sequential)
    pub sequence_number: u32,
    /// Response from controller
    pub response: Option<CommandResponse>,
    /// Timestamp when command was created (milliseconds)
    pub created_at: u64,
    /// Timestamp when command was sent (milliseconds)
    pub sent_at: Option<u64>,
    /// Timestamp when command completed (milliseconds)
    pub completed_at: Option<u64>,
}

impl GcodeCommand {
    /// Create a new G-Code command with auto-generated ID
    pub fn new(line: impl Into<String>) -> Self {
        let line = line.into();
        Self {
            id: CommandId::from(Uuid::new_v4().to_string()),
            command: line.clone(),
            line,
            line_number: None,
            state: CommandState::Pending,
            sequence_number: 0,
            response: None,
            created_at: Self::current_timestamp(),
            sent_at: None,
            completed_at: None,
        }
    }

    /// Create a new G-Code command with sequence number
    pub fn with_sequence(line: impl Into<String>, sequence: u32) -> Self {
        let mut cmd = Self::new(line);
        cmd.sequence_number = sequence;
        cmd
    }

    /// Create a new G-Code command with explicit ID
    pub fn with_id(line: impl Into<String>, id: CommandId) -> Self {
        let mut cmd = Self::new(line);
        cmd.id = id;
        cmd
    }

    /// Set the line number for this command
    pub fn set_line_number(&mut self, line_number: u32) -> &mut Self {
        self.line_number = Some(line_number);
        self
    }

    /// Mark this command as sent
    pub fn mark_sent(&mut self) -> &mut Self {
        debug_assert!(
            self.state == CommandState::Pending,
            "mark_sent called on command in {:?} state (expected Pending)",
            self.state
        );
        self.state = CommandState::Sent;
        self.sent_at = Some(Self::current_timestamp());
        self
    }

    /// Mark this command as successfully executed (received "ok")
    pub fn mark_ok(&mut self) -> &mut Self {
        debug_assert!(
            self.state == CommandState::Sent,
            "mark_ok called on command in {:?} state (expected Sent)",
            self.state
        );
        self.state = CommandState::Ok;
        if self.completed_at.is_none() {
            self.completed_at = Some(Self::current_timestamp());
        }
        self
    }

    /// Mark this command as completed
    pub fn mark_done(&mut self) -> &mut Self {
        debug_assert!(
            matches!(self.state, CommandState::Ok | CommandState::Sent),
            "mark_done called on command in {:?} state (expected Ok or Sent)",
            self.state
        );
        self.state = CommandState::Done;
        if self.completed_at.is_none() {
            self.completed_at = Some(Self::current_timestamp());
        }
        self
    }

    /// Mark this command with an error
    pub fn mark_error(&mut self, error_code: Option<u32>, message: String) -> &mut Self {
        debug_assert!(
            matches!(self.state, CommandState::Pending | CommandState::Sent),
            "mark_error called on command in {:?} state (expected Pending or Sent)",
            self.state
        );
        self.state = CommandState::Error;
        self.completed_at = Some(Self::current_timestamp());
        self.response = Some(CommandResponse {
            success: false,
            message,
            error_code,
            data: None,
        });
        self
    }

    /// Mark this command as skipped
    pub fn mark_skipped(&mut self) -> &mut Self {
        debug_assert!(
            self.state == CommandState::Pending,
            "mark_skipped called on command in {:?} state (expected Pending)",
            self.state
        );
        self.state = CommandState::Skipped;
        self.completed_at = Some(Self::current_timestamp());
        self
    }

    /// Set the response for this command
    pub fn set_response(&mut self, response: CommandResponse) -> &mut Self {
        self.response = Some(response);
        self
    }

    /// Check if command is in a terminal state
    pub fn is_terminal(&self) -> bool {
        matches!(
            self.state,
            CommandState::Done | CommandState::Error | CommandState::Skipped
        )
    }

    /// Check if command has been sent
    pub fn is_sent(&self) -> bool {
        self.sent_at.is_some()
    }

    /// Get duration from creation to completion (milliseconds)
    pub fn total_duration(&self) -> Option<u64> {
        self.completed_at
            .map(|completed| completed - self.created_at)
    }

    /// Get duration from sent to completion (milliseconds)
    pub fn execution_duration(&self) -> Option<u64> {
        self.sent_at
            .and_then(|sent| self.completed_at.map(|completed| completed - sent))
    }

    /// Get current timestamp in milliseconds
    fn current_timestamp() -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64
    }
}

impl Default for GcodeCommand {
    fn default() -> Self {
        Self::new("")
    }
}

impl std::fmt::Display for GcodeCommand {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}] {}: {}", self.id, self.state, self.command)
    }
}

/// Trait for listening to command lifecycle events
///
/// Implementations can react to various stages of command execution:
/// - Creation
/// - Sending
/// - Success/failure
/// - Completion
/// - Error conditions
pub trait CommandListener: Send + Sync {
    /// Called when a command is created
    fn on_command_created(&self, command: &GcodeCommand);

    /// Called when a command is sent to the controller
    fn on_command_sent(&self, command: &GcodeCommand);

    /// Called when a command receives an "ok" response
    fn on_command_ok(&self, command: &GcodeCommand);

    /// Called when a command completes execution
    fn on_command_completed(&self, command: &GcodeCommand);

    /// Called when a command encounters an error
    fn on_command_error(&self, command: &GcodeCommand, error: &CommandResponse);

    /// Called when a command is skipped
    fn on_command_skipped(&self, command: &GcodeCommand);

    /// Called when a command state changes
    fn on_command_state_changed(&self, command: &GcodeCommand, old_state: CommandState);
}

/// Default no-op command listener implementation
pub struct NoOpCommandListener;

impl CommandListener for NoOpCommandListener {
    fn on_command_created(&self, _command: &GcodeCommand) {}
    fn on_command_sent(&self, _command: &GcodeCommand) {}
    fn on_command_ok(&self, _command: &GcodeCommand) {}
    fn on_command_completed(&self, _command: &GcodeCommand) {}
    fn on_command_error(&self, _command: &GcodeCommand, _error: &CommandResponse) {}
    fn on_command_skipped(&self, _command: &GcodeCommand) {}
    fn on_command_state_changed(&self, _command: &GcodeCommand, _old_state: CommandState) {}
}

/// Arc-wrapped command listener for thread-safe sharing
pub type CommandListenerHandle = Arc<dyn CommandListener>;

/// Command numbering generator for sequential tracking
#[derive(Clone)]
pub struct CommandNumberGenerator {
    counter: Arc<AtomicU32>,
}

impl CommandNumberGenerator {
    /// Create a new command number generator
    pub fn new() -> Self {
        Self {
            counter: Arc::new(AtomicU32::new(0)),
        }
    }

    /// Get the next command number
    pub fn next(&self) -> u32 {
        self.counter.fetch_add(1, Ordering::SeqCst)
    }

    /// Get current command count without incrementing
    pub fn current(&self) -> u32 {
        self.counter.load(Ordering::SeqCst)
    }

    /// Reset the counter
    pub fn reset(&self) {
        self.counter.store(0, Ordering::SeqCst);
    }
}

impl Default for CommandNumberGenerator {
    fn default() -> Self {
        Self::new()
    }
}
