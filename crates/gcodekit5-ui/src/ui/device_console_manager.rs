//! Device Console Manager - Handles message routing and command execution
//!
//! This module manages the device console, routing messages from the backend,
//! handling command input, and maintaining console state. Inspired by UGS
//! ConsolePanel and CommandPanel architecture.

use crate::device_status;
use crate::ui::console_panel::{ConsolePanel, MessageLevel};
use gcodekit5_communication::CommunicatorListener;
use gcodekit5_core::{
    thread_safe, thread_safe_vec, DataCallback, ThreadSafe, ThreadSafeVec,
};
use std::sync::Arc;

/// Message type for device communication
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceMessageType {
    /// Standard output message
    Output,
    /// Error from device
    Error,
    /// Debug/verbose message
    Verbose,
    /// Success/OK response
    Success,
    /// Command echo
    Command,
}

/// Device console event
#[derive(Debug, Clone)]
pub enum ConsoleEvent {
    /// New message received
    MessageReceived {
        /// Message type
        msg_type: DeviceMessageType,
        /// Message content
        content: String,
    },
    /// Console cleared
    Cleared,
    /// Settings changed
    SettingsChanged,
}

/// Device Console Manager
#[allow(clippy::type_complexity)]
pub struct DeviceConsoleManager {
    /// Console panel
    console: ThreadSafe<ConsolePanel>,
    /// Whether verbose output is enabled
    verbose_enabled: ThreadSafe<bool>,
    /// Whether auto-scroll is enabled
    auto_scroll_enabled: ThreadSafe<bool>,
    /// Event callbacks (with interior mutability)
    on_event: ThreadSafeVec<DataCallback<ConsoleEvent>>,
    /// Modo silencioso - no mostrar mensajes en consola
    silent_mode: ThreadSafe<bool>,
}

impl DeviceConsoleManager {
    /// Create new device console manager
    pub fn new() -> Self {
        Self {
            console: thread_safe(ConsolePanel::new()),
            verbose_enabled: thread_safe(false),
            auto_scroll_enabled: thread_safe(true),
            on_event: thread_safe_vec(),
            silent_mode: thread_safe(false),
        }
    }

    /// Add message to console
    pub fn add_message(&self, msg_type: DeviceMessageType, content: impl Into<String>) {
        // Si estamos en modo silencioso, no mostrar nada
        if *self.silent_mode.lock() {
            return;
        }

        let content = content.into();

        let level = match msg_type {
            DeviceMessageType::Output => MessageLevel::Info,
            DeviceMessageType::Error => MessageLevel::Error,
            DeviceMessageType::Verbose => MessageLevel::Debug,
            DeviceMessageType::Success => MessageLevel::Success,
            DeviceMessageType::Command => MessageLevel::Info,
        };

        // Check if verbose filtering applies - recover from poisoned lock
        let verbose_check = self.verbose_enabled.lock();
        if msg_type == DeviceMessageType::Verbose && !*verbose_check {
            return;
        }

        {
            let mut console = self.console.lock();
            if msg_type == DeviceMessageType::Command {
                console.add_command(&content);
            } else {
                console.add_message(level, &content);
            }
        }

        self.emit_event(ConsoleEvent::MessageReceived { msg_type, content });
    }

    /// Add command to history
    pub fn add_command_to_history(&self, command: impl Into<String>) {
        let command = command.into();
        let mut console = self.console.lock();
        console.add_to_history(&command);
    }

    /// Get console output as model
    pub fn get_model(&self) -> Vec<String> {
        let console = self.console.lock();

        console.get_displayed_strings(1000)
    }

    /// Get console output as string
    pub fn get_output(&self) -> String {
        let console = self.console.lock();
        let messages = console.get_displayed_strings(1000);
        messages.join("\n")
    }

    /// Get recent messages
    pub fn get_recent_messages(&self, count: usize) -> Vec<String> {
        let console = self.console.lock();
        console.get_displayed_strings(count)
    }

    /// Clear console
    pub fn clear(&self) {
        {
            let mut console = self.console.lock();
            console.clear();
        }
        self.emit_event(ConsoleEvent::Cleared);
    }

    /// Set verbose output enabled
    pub fn set_verbose_enabled(&self, enabled: bool) {
        {
            *self.verbose_enabled.lock() = enabled;
        }
        self.emit_event(ConsoleEvent::SettingsChanged);
    }

    /// Get verbose output enabled state
    pub fn is_verbose_enabled(&self) -> bool {
        *self.verbose_enabled.lock()
    }

    /// Set auto-scroll enabled
    pub fn set_auto_scroll_enabled(&self, enabled: bool) {
        {
            let mut console = self.console.lock();
            console.auto_scroll = enabled;
            *self.auto_scroll_enabled.lock() = enabled;
        }
        self.emit_event(ConsoleEvent::SettingsChanged);
    }

    /// Get auto-scroll enabled state
    pub fn is_auto_scroll_enabled(&self) -> bool {
        *self.auto_scroll_enabled.lock()
    }

    /// Set maximum number of console lines to keep
    pub fn set_max_lines(&self, max_lines: usize) {
        {
            let mut console = self.console.lock();
            console.max_messages = max_lines;
        }
    }

    /// Get maximum number of console lines
    pub fn get_max_lines(&self) -> usize {
        let console = self.console.lock();
        console.max_messages
    }

    /// Toggle auto-scroll
    pub fn toggle_auto_scroll(&self) {
        let enabled = !self.is_auto_scroll_enabled();
        self.set_auto_scroll_enabled(enabled);
    }

    /// Get command history
    pub fn get_history(&self) -> Vec<String> {
        let console = self.console.lock();
        console.get_history()
    }

    /// Get total message count
    pub fn message_count(&self) -> usize {
        let console = self.console.lock();
        console.message_count()
    }

    /// Simulate connection message
    pub fn on_connection(&self) {
        self.add_message(DeviceMessageType::Success, "Device connected");
    }

    /// Simulate disconnection message
    pub fn on_disconnection(&self) {
        self.add_message(DeviceMessageType::Output, "Device disconnected");
    }

    /// Simulate error message
    pub fn on_error(&self, error: impl Into<String>) {
        self.add_message(DeviceMessageType::Error, error);
    }

    /// Emit console event
    fn emit_event(&self, event: ConsoleEvent) {
        {
            let callbacks = self.on_event.lock();
            for callback in callbacks.iter() {
                callback(event.clone());
            }
        }
    }

    /// Register event callback
    pub fn on_event<F>(&self, callback: F)
    where
        F: Fn(ConsoleEvent) + Send + Sync + 'static,
    {
        {
            let mut callbacks = self.on_event.lock();
            callbacks.push(Box::new(callback));
        }
    }

    /// Activar/desactivar modo silencioso (para grabado de imágenes)
    pub fn set_silent_mode(&self, enabled: bool) {
        *self.silent_mode.lock() = enabled;
    }

    /// Obtener estado del modo silencioso
    pub fn is_silent_mode(&self) -> bool {
        *self.silent_mode.lock()
    }
}

impl Default for DeviceConsoleManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Global console manager instance
static CONSOLE_MANAGER: std::sync::OnceLock<Arc<DeviceConsoleManager>> = std::sync::OnceLock::new();

/// Get or initialize global console manager
pub fn get_console_manager() -> Arc<DeviceConsoleManager> {
    Arc::clone(CONSOLE_MANAGER.get_or_init(|| Arc::new(DeviceConsoleManager::new())))
}

/// Listener that connects communicator events to the device console
pub struct ConsoleListener {
    console_manager: Arc<DeviceConsoleManager>,
    /// Buffer for accumulating multi-line responses (like $I)
    _response_buffer: ThreadSafe<String>,

}

impl ConsoleListener {
    /// Create new console listener connected to a console manager
    pub fn new(console_manager: Arc<DeviceConsoleManager>) -> Arc<Self> {
        Arc::new(Self {
            console_manager,
            _response_buffer: thread_safe(String::new()),
        })
    }

    /// Create with firmware detection state sharing
    pub fn new_with_firmware_state(
        console_manager: Arc<DeviceConsoleManager>,

    ) -> Arc<Self> {
        Arc::new(Self {
            console_manager,
            _response_buffer: thread_safe(String::new()),
        })
    }
}

impl CommunicatorListener for ConsoleListener {
    fn on_connected(&self) {
        self.console_manager
            .add_message(DeviceMessageType::Success, "Device connected");
    }

    fn on_disconnected(&self) {
        self.console_manager
            .add_message(DeviceMessageType::Output, "Device disconnected");
    }

    fn on_error(&self, error: &str) {
        self.console_manager
            .add_message(DeviceMessageType::Error, format!("Error: {}", error));
    }

    fn on_data_received(&self, data: &[u8]) {
        if let Ok(text) = std::str::from_utf8(data) {
            let trimmed = text.trim();

            // Suppress status polling responses (starts with '<')
            // Also suppress if it only contains status and 'ok' responses
            if trimmed.starts_with('<')
                || (trimmed.contains('<') && trimmed.contains('>') && !trimmed.contains('['))
            {
                // Status response - don't log to console
                return;
            }

            // Suppress raw "ok" messages as they are now handled by the machine callback
            // which pairs them with the sent command.
            if trimmed == "ok"
                || trimmed == "ok\nok"
                || trimmed == "ok\nok\nok"
                || trimmed == "ok\n\nok"
                || trimmed == "ok\n\nok\n\nok"
                || trimmed == "ok\n\nok\n\nok\n\nok"
            {
                return;
            }

            if !trimmed.is_empty() {
                self.console_manager
                    .add_message(DeviceMessageType::Output, trimmed);
            }
        }
    }

    fn on_data_sent(&self, data: &[u8]) {
        if let Ok(text) = std::str::from_utf8(data) {
            let trimmed = text.trim();

            // Parse for F and S commands to update commanded values
            let mut clean_line = trimmed.to_string();
            // Remove comments
            if let Some(idx) = clean_line.find(';') {
                clean_line.truncate(idx);
            }
            if let Some(idx) = clean_line.find('(') {
                clean_line.truncate(idx);
            }

            for token in clean_line.split_whitespace() {
                if let Some(first_char) = token.chars().next() {
                    if token.len() > 1 {
                        if first_char == 'F' || first_char == 'f' {
                            if let Ok(val) = token[1..].parse::<f32>() {
                                device_status::update_commanded_feed_rate(val);
                            }
                        } else if first_char == 'S' || first_char == 's' {
                            if let Ok(val) = token[1..].parse::<f32>() {
                                device_status::update_commanded_spindle_speed(val);
                            }
                        }
                    }
                }
            }

            // Suppress status polling queries (just '?')
            if trimmed == "?" {
                // Status query - don't log to console
            }
        }
    }

    fn on_timeout(&self) {
        self.console_manager
            .add_message(DeviceMessageType::Verbose, "Connection timeout");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_manager_creation() {
        let manager = DeviceConsoleManager::new();
        assert_eq!(manager.message_count(), 0);
    }

    #[test]
    fn test_add_message() {
        let manager = DeviceConsoleManager::new();
        manager.add_message(DeviceMessageType::Output, "Test message");
        assert_eq!(manager.message_count(), 1);
    }

    #[test]
    fn test_add_error_message() {
        let manager = DeviceConsoleManager::new();
        manager.add_message(DeviceMessageType::Error, "Test error");
        let output = manager.get_output();
        assert!(output.contains("Test error"));
        assert!(output.contains("ERR"));
    }

    #[test]
    fn test_verbose_filtering() {
        let manager = DeviceConsoleManager::new();
        manager.set_verbose_enabled(false);

        manager.add_message(DeviceMessageType::Verbose, "Verbose message");
        manager.add_message(DeviceMessageType::Output, "Regular message");

        let output = manager.get_output();
        assert!(!output.contains("Verbose message"));
        assert!(output.contains("Regular message"));
    }

    #[test]
    fn test_verbose_enabled() {
        let manager = DeviceConsoleManager::new();
        manager.set_verbose_enabled(true);

        manager.add_message(DeviceMessageType::Verbose, "Verbose message");
        manager.add_message(DeviceMessageType::Output, "Regular message");

        let output = manager.get_output();
        assert!(output.contains("Verbose message"));
        assert!(output.contains("Regular message"));
    }

    #[test]
    fn test_clear_console() {
        let manager = DeviceConsoleManager::new();
        manager.add_message(DeviceMessageType::Output, "Message 1");
        manager.add_message(DeviceMessageType::Output, "Message 2");
        assert_eq!(manager.message_count(), 2);

        manager.clear();
        assert_eq!(manager.message_count(), 0);
    }

    #[test]
    fn test_command_history() {
        let manager = DeviceConsoleManager::new();
        manager.add_command_to_history("G0 X10");
        manager.add_command_to_history("G1 Y20");

        let history = manager.get_history();
        assert_eq!(history.len(), 2);
        assert_eq!(history[0], "G0 X10");
        assert_eq!(history[1], "G1 Y20");
    }

    #[test]
    fn test_toggle_verbose() {
        let manager = DeviceConsoleManager::new();
        assert!(!manager.is_verbose_enabled());

        manager.set_verbose_enabled(true);
        assert!(manager.is_verbose_enabled());

        manager.set_verbose_enabled(false);
        assert!(!manager.is_verbose_enabled());
    }

    #[test]
    fn test_auto_scroll() {
        let manager = DeviceConsoleManager::new();
        assert!(manager.is_auto_scroll_enabled());

        manager.set_auto_scroll_enabled(false);
        assert!(!manager.is_auto_scroll_enabled());

        manager.set_auto_scroll_enabled(true);
        assert!(manager.is_auto_scroll_enabled());
    }

    #[test]
    fn test_recent_messages() {
        let manager = DeviceConsoleManager::new();
        for i in 0..10 {
            manager.add_message(DeviceMessageType::Output, format!("Message {}", i));
        }

        let recent = manager.get_recent_messages(5);
        assert_eq!(recent.len(), 5);
    }
}
