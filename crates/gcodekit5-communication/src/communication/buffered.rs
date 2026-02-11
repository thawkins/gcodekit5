//! Buffered communication implementation with flow control and acknowledgment tracking
//!
//! Implements the GRBL streaming protocol with command queueing, buffer management,
//! flow control, and acknowledgment tracking for reliable communication with CNC controllers.
//!
//! # Features
//! - Command queue management
//! - Sender buffer tracking
//! - Flow control to prevent buffer overflow
//! - Command acknowledgment tracking
//! - Retry logic for failed commands
//! - Pause/resume capabilities

use crate::communication::Communicator;
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

/// Status of a command in the buffer
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandStatus {
    /// Command is queued and waiting to be sent
    Queued,
    /// Command has been sent to the device
    Sent,
    /// Command has been acknowledged by the device
    Acknowledged,
    /// Command execution completed
    Completed,
    /// Command failed
    Failed,
}

/// Represents a command in the buffer with its metadata
#[derive(Debug, Clone)]
pub struct BufferedCommand {
    /// The command string to send
    pub command: String,
    /// Current status of the command
    pub status: CommandStatus,
    /// Number of retry attempts
    pub retry_count: u32,
    /// Maximum number of retries allowed
    pub max_retries: u32,
    /// Response from the device
    pub response: Option<String>,
}

impl BufferedCommand {
    /// Create a new buffered command
    pub fn new(command: String, max_retries: u32) -> Self {
        Self {
            command,
            status: CommandStatus::Queued,
            retry_count: 0,
            max_retries,
            response: None,
        }
    }

    /// Check if the command can be retried
    pub fn can_retry(&self) -> bool {
        self.retry_count < self.max_retries
    }

    /// Mark command as sent and increment retry counter
    pub fn mark_sent(&mut self) {
        self.status = CommandStatus::Sent;
        self.retry_count += 1;
    }

    /// Mark command as acknowledged
    pub fn mark_acknowledged(&mut self) {
        self.status = CommandStatus::Acknowledged;
    }

    /// Mark command as completed
    pub fn mark_completed(&mut self) {
        self.status = CommandStatus::Completed;
    }

    /// Mark command as failed
    pub fn mark_failed(&mut self) {
        self.status = CommandStatus::Failed;
    }

    /// Set the response received from device
    pub fn set_response(&mut self, response: String) {
        self.response = Some(response);
    }
}

/// Configuration for buffered communication
#[derive(Debug, Clone)]
pub struct BufferedCommunicatorConfig {
    /// Maximum size of the controller's buffer in bytes
    pub buffer_size: usize,
    /// Maximum number of commands to queue
    pub queue_size: usize,
    /// Maximum retries per command
    pub max_retries: u32,
    /// Enable flow control
    pub flow_control: bool,
}

impl Default for BufferedCommunicatorConfig {
    fn default() -> Self {
        Self {
            buffer_size: 128,
            queue_size: 100,
            max_retries: 3,
            flow_control: true,
        }
    }
}

/// Wrapper around a communicator that adds buffering and flow control
pub struct BufferedCommunicatorWrapper {
    /// The underlying communicator
    communicator: Box<dyn Communicator>,
    /// Configuration for buffering
    config: BufferedCommunicatorConfig,
    /// Queue of commands to send
    command_queue: Arc<Mutex<VecDeque<BufferedCommand>>>,
    /// Currently sent commands awaiting acknowledgment
    active_commands: Arc<Mutex<Vec<BufferedCommand>>>,
    /// Current amount of data in controller buffer
    sent_buffer_size: usize,
    /// Whether sending is paused
    send_paused: bool,
}

impl BufferedCommunicatorWrapper {
    /// Create a new buffered communicator wrapper
    pub fn new(communicator: Box<dyn Communicator>, config: BufferedCommunicatorConfig) -> Self {
        Self {
            communicator,
            config,
            command_queue: Arc::new(Mutex::new(VecDeque::new())),
            active_commands: Arc::new(Mutex::new(Vec::new())),
            sent_buffer_size: 0,
            send_paused: false,
        }
    }

    /// Queue a command for sending
    pub fn queue_command(&self, command: String) -> gcodekit5_core::Result<()> {
        let mut queue = self.command_queue.lock().map_err(|e| {
            gcodekit5_core::Error::other(format!("Failed to lock command queue: {}", e))
        })?;

        if queue.len() >= self.config.queue_size {
            return Err(gcodekit5_core::Error::other("Command queue is full"));
        }

        queue.push_back(BufferedCommand::new(command, self.config.max_retries));
        Ok(())
    }

    /// Get the number of queued commands
    pub fn queued_commands_count(&self) -> gcodekit5_core::Result<usize> {
        let queue = self.command_queue.lock().map_err(|e| {
            gcodekit5_core::Error::other(format!("Failed to lock command queue: {}", e))
        })?;
        Ok(queue.len())
    }

    /// Get the number of active commands
    pub fn active_commands_count(&self) -> gcodekit5_core::Result<usize> {
        let active = self.active_commands.lock().map_err(|e| {
            gcodekit5_core::Error::other(format!("Failed to lock active commands: {}", e))
        })?;
        Ok(active.len())
    }

    /// Check if there is room in the controller buffer for a command
    fn has_room_in_buffer(&self, command_size: usize) -> bool {
        if !self.config.flow_control {
            return true;
        }

        let used_space = self.sent_buffer_size + command_size + 1; // +1 for newline
        used_space <= self.config.buffer_size
    }

    /// Stream commands from the queue to the communicator
    pub fn stream_commands(&mut self) -> gcodekit5_core::Result<()> {
        if self.send_paused {
            return Ok(());
        }

        loop {
            let mut queue = self.command_queue.lock().map_err(|e| {
                gcodekit5_core::Error::other(format!("Failed to lock command queue: {}", e))
            })?;

            if queue.is_empty() {
                break;
            }

            if let Some(mut command) = queue.pop_front() {
                let command_size = command.command.len();

                if !self.has_room_in_buffer(command_size) {
                    // Put it back and stop streaming
                    queue.push_front(command);
                    break;
                }

                drop(queue); // Release lock before sending

                self.send_buffered_command(&mut command)?;

                let mut active = self.active_commands.lock().map_err(|e| {
                    gcodekit5_core::Error::other(format!("Failed to lock active commands: {}", e))
                })?;
                active.push(command);
            } else {
                break;
            }
        }

        Ok(())
    }

    /// Send a command and track it in the buffer
    fn send_buffered_command(
        &mut self,
        command: &mut BufferedCommand,
    ) -> gcodekit5_core::Result<()> {
        self.communicator
            .send_command(&command.command)
            .map_err(|e| {
                tracing::error!("Failed to send command: {}", e);
                e
            })?;

        self.sent_buffer_size += command.command.len() + 1; // +1 for newline
        command.mark_sent();

        Ok(())
    }

    /// Handle acknowledgment from the device
    pub fn handle_acknowledgment(&mut self) -> gcodekit5_core::Result<()> {
        let mut active = self.active_commands.lock().map_err(|e| {
            gcodekit5_core::Error::other(format!("Failed to lock active commands: {}", e))
        })?;

        if let Some(command) = active.first_mut() {
            let command_size = command.command.len() + 1;
            command.mark_acknowledged();
            command.mark_completed();

            self.sent_buffer_size = self.sent_buffer_size.saturating_sub(command_size);
            active.remove(0);
        }

        Ok(())
    }

    /// Handle error response from the device
    pub fn handle_error(&mut self, error_msg: String) -> gcodekit5_core::Result<()> {
        let mut active = self.active_commands.lock().map_err(|e| {
            gcodekit5_core::Error::other(format!("Failed to lock active commands: {}", e))
        })?;

        if let Some(command) = active.first_mut() {
            command.mark_failed();
            command.set_response(error_msg);

            if command.can_retry() {
                tracing::warn!(
                    "Command failed, retrying ({}/{})",
                    command.retry_count,
                    command.max_retries
                );

                // Move failed command back to queue for retry
                let retry_command = active.remove(0);
                let mut queue = self.command_queue.lock().map_err(|e| {
                    gcodekit5_core::Error::other(format!("Failed to lock command queue: {}", e))
                })?;
                queue.push_front(retry_command);

                if let Some(front) = queue.front() {
                    let command_size = front.command.len() + 1;
                    self.sent_buffer_size = self.sent_buffer_size.saturating_sub(command_size);
                }
            } else {
                tracing::error!("Command failed after {} retries", command.max_retries);
                let command_size = command.command.len() + 1;
                self.sent_buffer_size = self.sent_buffer_size.saturating_sub(command_size);
                active.remove(0);
            }
        }

        Ok(())
    }

    /// Pause sending commands
    pub fn pause(&mut self) {
        self.send_paused = true;
    }

    /// Resume sending commands
    pub fn resume(&mut self) -> gcodekit5_core::Result<()> {
        self.send_paused = false;
        self.stream_commands()
    }

    /// Check if sending is paused
    pub fn is_paused(&self) -> bool {
        self.send_paused
    }

    /// Clear all queued commands
    pub fn clear_queue(&mut self) -> gcodekit5_core::Result<()> {
        let mut queue = self.command_queue.lock().map_err(|e| {
            gcodekit5_core::Error::other(format!("Failed to lock command queue: {}", e))
        })?;
        queue.clear();

        let mut active = self.active_commands.lock().map_err(|e| {
            gcodekit5_core::Error::other(format!("Failed to lock active commands: {}", e))
        })?;
        active.clear();

        self.sent_buffer_size = 0;
        self.send_paused = false;

        Ok(())
    }

    /// Get the current buffer usage as a percentage
    pub fn buffer_usage_percent(&self) -> u32 {
        if self.config.buffer_size == 0 {
            return 0;
        }

        ((self.sent_buffer_size as f64 / self.config.buffer_size as f64) * 100.0) as u32
    }

    /// Get a reference to the underlying communicator
    pub fn communicator(&self) -> &dyn Communicator {
        self.communicator.as_ref()
    }

    /// Get a mutable reference to the underlying communicator
    pub fn communicator_mut(&mut self) -> &mut dyn Communicator {
        self.communicator.as_mut()
    }
}
