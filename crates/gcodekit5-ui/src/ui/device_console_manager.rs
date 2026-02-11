//! Device Console Manager - Handles message routing and command execution
//!
//! This module manages the device console, routing messages from the backend,
//! handling command input, and maintaining console state. Inspired by UGS
//! ConsolePanel and CommandPanel architecture.

use crate::device_status;
use crate::ui::console_panel::{ConsolePanel, MessageLevel};
use gcodekit5_communication::CommunicatorListener;
use std::sync::{Arc, Mutex};

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
    console: Arc<Mutex<ConsolePanel>>,
    /// Whether verbose output is enabled
    verbose_enabled: Arc<Mutex<bool>>,
    /// Whether auto-scroll is enabled
    auto_scroll_enabled: Arc<Mutex<bool>>,
    /// Event callbacks (with interior mutability)
    on_event: Arc<Mutex<Vec<Box<dyn Fn(ConsoleEvent) + Send + Sync>>>>,
}

impl DeviceConsoleManager {
    /// Create new device console manager
    pub fn new() -> Self {
        Self {
            console: Arc::new(Mutex::new(ConsolePanel::new())),
            verbose_enabled: Arc::new(Mutex::new(false)),
            auto_scroll_enabled: Arc::new(Mutex::new(true)),
            on_event: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Add message to console
    pub fn add_message(&self, msg_type: DeviceMessageType, content: impl Into<String>) {
        let content = content.into();

        let level = match msg_type {
            DeviceMessageType::Output => MessageLevel::Info,
            DeviceMessageType::Error => MessageLevel::Error,
            DeviceMessageType::Verbose => MessageLevel::Debug,
            DeviceMessageType::Success => MessageLevel::Success,
            DeviceMessageType::Command => MessageLevel::Info,
        };

        // Check if verbose filtering applies - recover from poisoned lock
        let verbose_check = self
            .verbose_enabled
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if msg_type == DeviceMessageType::Verbose && !*verbose_check {
            return;
        }

        {
            let mut console = self
                .console
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
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
        let mut console = self
            .console
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        console.add_to_history(&command);
    }

    /// Get console output as model
    pub fn get_model(&self) -> Vec<String> {
        let console = self
            .console
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());

        console.get_displayed_strings(1000)
    }

    /// Get console output as string
    pub fn get_output(&self) -> String {
        let console = self
            .console
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let messages = console.get_displayed_strings(1000);
        messages.join("\n")
    }

    /// Get recent messages
    pub fn get_recent_messages(&self, count: usize) -> Vec<String> {
        let console = self
            .console
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        console.get_displayed_strings(count)
    }

    /// Clear console
    pub fn clear(&self) {
        {
            let mut console = self
                .console
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            console.clear();
        }
        self.emit_event(ConsoleEvent::Cleared);
    }

    /// Set verbose output enabled
    pub fn set_verbose_enabled(&self, enabled: bool) {
        {
            *self
                .verbose_enabled
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner()) = enabled;
        }
        self.emit_event(ConsoleEvent::SettingsChanged);
    }

    /// Get verbose output enabled state
    pub fn is_verbose_enabled(&self) -> bool {
        *self
            .verbose_enabled
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    /// Set auto-scroll enabled
    pub fn set_auto_scroll_enabled(&self, enabled: bool) {
        {
            let mut console = self
                .console
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            console.auto_scroll = enabled;
            *self
                .auto_scroll_enabled
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner()) = enabled;
        }
        self.emit_event(ConsoleEvent::SettingsChanged);
    }

    /// Get auto-scroll enabled state
    pub fn is_auto_scroll_enabled(&self) -> bool {
        *self
            .auto_scroll_enabled
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    /// Set maximum number of console lines to keep
    pub fn set_max_lines(&self, max_lines: usize) {
        if let Ok(mut console) = self.console.lock() {
            console.max_messages = max_lines;
        }
    }

    /// Get maximum number of console lines
    pub fn get_max_lines(&self) -> usize {
        if let Ok(console) = self.console.lock() {
            console.max_messages
        } else {
            500
        }
    }

    /// Toggle auto-scroll
    pub fn toggle_auto_scroll(&self) {
        let enabled = !self.is_auto_scroll_enabled();
        self.set_auto_scroll_enabled(enabled);
    }

    /// Get command history
    pub fn get_history(&self) -> Vec<String> {
        let console = self
            .console
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        console.get_history()
    }

    /// Get total message count
    pub fn message_count(&self) -> usize {
        let console = self
            .console
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
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
        if let Ok(callbacks) = self.on_event.lock() {
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
        if let Ok(mut callbacks) = self.on_event.lock() {
            callbacks.push(Box::new(callback));
        }
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
    _response_buffer: Arc<Mutex<String>>,
    /// Shared firmware detection result (optional)
    detected_firmware:
        Option<Arc<Mutex<Option<gcodekit5_communication::firmware::FirmwareDetectionResult>>>>,
}

impl ConsoleListener {
    /// Create new console listener connected to a console manager
    pub fn new(console_manager: Arc<DeviceConsoleManager>) -> Arc<Self> {
        Arc::new(Self {
            console_manager,
            _response_buffer: Arc::new(Mutex::new(String::new())),
            detected_firmware: None,
        })
    }

    /// Create with firmware detection state sharing
    pub fn new_with_firmware_state(
        console_manager: Arc<DeviceConsoleManager>,
        detected_firmware: Arc<
            Mutex<Option<gcodekit5_communication::firmware::FirmwareDetectionResult>>,
        >,
    ) -> Arc<Self> {
        Arc::new(Self {
            console_manager,
            _response_buffer: Arc::new(Mutex::new(String::new())),
            detected_firmware: Some(detected_firmware),
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

            // Check for startup message (single line, immediate detection)
            if (trimmed.contains("Grbl") || trimmed.contains("grbl")) && trimmed.contains("help") {
                use gcodekit5_communication::FirmwareDetector;
                match FirmwareDetector::parse_grbl_startup(trimmed) {
                    Ok(detection) => {
                        // Store in shared state if available
                        if let Some(ref fw_state) = self.detected_firmware {
                            if let Ok(mut guard) = fw_state.lock() {
                                *guard = Some(detection.clone());
                            }
                        }

                        // Update global device status (so UI doesn't stay "Unknown")
                        device_status::update_firmware_info(
                            format!("{:?}", detection.firmware_type),
                            detection.version_string.clone(),
                            None,
                        );

                        self.console_manager.add_message(
                            DeviceMessageType::Success,
                            format!(
                                "Detected firmware: {} {}",
                                detection.firmware_type, detection.version_string
                            ),
                        );

                        // Keep the raw startup banner visible in the console.
                        self.console_manager
                            .add_message(DeviceMessageType::Output, trimmed);
                        return;
                    }
                    Err(_) => {
                        // Ignore parsing errors, just treat as normal output
                    }
                }
            }

            // Handle multi-line $I response which may come as one chunk
            // Format: [VER:...]\n[OPT:...]\nok
            if trimmed.contains("[VER:") && trimmed.contains("[OPT:") && trimmed.contains("ok") {
                // Complete $I response in one chunk

                use gcodekit5_communication::FirmwareDetector;
                match FirmwareDetector::parse_grbl_version_info(trimmed) {
                    Ok(detection) => {
                        // Store in shared state if available
                        if let Some(ref fw_state) = self.detected_firmware {
                            if let Ok(mut guard) = fw_state.lock() {
                                *guard = Some(detection.clone());
                            }
                        }

                        // Update global device status (so UI doesn't stay "Unknown")
                        device_status::update_firmware_info(
                            format!("{:?}", detection.firmware_type),
                            detection.version_string.clone(),
                            None,
                        );

                        self.console_manager.add_message(
                            DeviceMessageType::Success,
                            format!(
                                "Detected firmware: {} {} (build: {})",
                                detection.firmware_type,
                                detection.version_string,
                                detection.build_date.as_deref().unwrap_or("unknown")
                            ),
                        );
                        // Show just the firmware info lines without the status pollution
                        if let Some(ver_line) = trimmed.lines().find(|l| l.starts_with("[VER:")) {
                            self.console_manager
                                .add_message(DeviceMessageType::Output, ver_line);
                        }
                        if let Some(opt_line) = trimmed.lines().find(|l| l.starts_with("[OPT:")) {
                            self.console_manager
                                .add_message(DeviceMessageType::Output, opt_line);
                        }
                        self.console_manager
                            .add_message(DeviceMessageType::Output, "ok");
                        return; // Don't show the whole chunk
                    }
                    Err(e) => {
                        tracing::warn!("Failed to parse $I response: {}", e);
                    }
                }
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

            // Suppress streamed G-code commands here as they are logged by the machine callback
            // when the response is received (Command => Result format).
            // We only want to log manual commands or commands not tracked by the send state.
            // However, since we can't easily distinguish here, we'll rely on the fact that
            // the machine callback logs the "Command => Result" pair.
            // But wait, if we suppress here, manual commands might not show up until acknowledged?
            // Actually, manual commands (via buttons) are also sent via the same mechanism now.
            // The issue is that `on_data_sent` logs immediately, while `machine.rs` logs on ack.
            // If we want to avoid duplicates for streamed commands, we should suppress here if it's part of a stream.
            // But `ConsoleListener` doesn't know about `GcodeSendState`.

            // For now, let's just suppress all sent data logging here, and rely on:
            // 1. `machine.rs` logging `Command => Result` for everything sent via `send_command` or the stream.
            // 2. `main.rs` logging `>>> Command` for manual commands sent via the console input.

            // Actually, `machine.rs` only logs if it finds the command in `sent_lines`.
            // Manual commands sent via `send_command` in `main.rs` are NOT added to `sent_lines` currently?
            // Let's check `main.rs`.

            // In `main.rs`:
            // main_window.on_send_command(move |command: slint::SharedString| {
            //     ...
            //     console_manager_clone.add_message(DeviceMessageType::Command, format!(">>> {}", cmd));
            //     ...
            // });

            // So manual console commands are logged with `>>>`.

            // Jog commands in `machine.rs` are now added to `sent_lines`.

            // Streamed commands in `machine.rs` are added to `sent_lines`.

            // So `on_data_sent` here is likely redundant and causing double logging or noise.
            // Let's disable it or make it very selective.

            // If we disable it, we lose logging for anything sent that ISN'T tracked elsewhere.
            // But most things are tracked.

            // Let's just return for now to reduce noise as requested.

            /*
            if !trimmed.is_empty() {
                self.console_manager
                    .add_message(DeviceMessageType::Command, trimmed);
            }
            */
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
