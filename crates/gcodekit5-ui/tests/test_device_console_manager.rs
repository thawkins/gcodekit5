//! Tests for `gcodekit5_ui::ui::device_console_manager` (moved from the inline test module).

use gcodekit5_ui::ui::device_console_manager::*;

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
