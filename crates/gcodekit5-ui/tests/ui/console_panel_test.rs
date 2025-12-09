#![cfg(feature = "slint_legacy_tests")]
use gcodekit5_ui::ui::console_panel::{ConsoleMessage, MessageLevel, MessageFilter, ConsolePanel, HistoryEntry};

#[test]
fn test_message_creation() {
    let msg = ConsoleMessage::new(MessageLevel::Info, "Test message");
    assert_eq!(msg.level, MessageLevel::Info);
    assert_eq!(msg.text, "Test message");
    assert!(!msg.is_command);
}

#[test]
fn test_command_message() {
    let msg = ConsoleMessage::command("G0 X10");
    assert!(msg.is_command);
    assert_eq!(msg.level, MessageLevel::Info);
}

#[test]
fn test_message_formatting() {
    let msg = ConsoleMessage::new(MessageLevel::Error, "Test error");
    let formatted = msg.formatted();
    assert!(formatted.contains("ERR"));
    assert!(formatted.contains("Test error"));
}

#[test]
fn test_filter_show_all() {
    let filter = MessageFilter::show_all();
    assert!(filter.debug);
    assert!(filter.info);
    assert!(filter.error);
}

#[test]
fn test_filter_errors_only() {
    let filter = MessageFilter::errors_only();
    assert!(!filter.debug);
    assert!(filter.error);
    assert!(!filter.info);
}

#[test]
fn test_filter_matches() {
    let filter = MessageFilter::errors_only();
    let error_msg = ConsoleMessage::new(MessageLevel::Error, "Error");
    let info_msg = ConsoleMessage::new(MessageLevel::Info, "Info");

    assert!(filter.matches(&error_msg));
    assert!(!filter.matches(&info_msg));
}

#[test]
fn test_filter_text() {
    let mut filter = MessageFilter::show_all();
    filter.text_filter = Some("G0".to_string());

    let msg1 = ConsoleMessage::new(MessageLevel::Info, "G0 X10");
    let msg2 = ConsoleMessage::new(MessageLevel::Info, "G1 Y20");

    assert!(filter.matches(&msg1));
    assert!(!filter.matches(&msg2));
}

#[test]
fn test_console_add_message() {
    let mut console = ConsolePanel::new();
    console.add_message(MessageLevel::Info, "Test");
    assert_eq!(console.message_count(), 1);
}

#[test]
fn test_console_add_command() {
    let mut console = ConsolePanel::new();
    console.add_command("G0 X10");
    assert_eq!(console.message_count(), 1);
}

#[test]
fn test_console_history() {
    let mut console = ConsolePanel::new();
    console.add_to_history("G0 X10");
    console.add_to_history("G1 Y20");
    assert_eq!(console.history_count(), 2);
}

#[test]
fn test_console_clear() {
    let mut console = ConsolePanel::new();
    console.add_message(MessageLevel::Info, "Test");
    console.clear();
    assert_eq!(console.message_count(), 0);
}

#[test]
fn test_console_filtered() {
    let mut console = ConsolePanel::new();
    console.add_message(MessageLevel::Info, "Info");
    console.add_message(MessageLevel::Error, "Error");

    let mut filter = MessageFilter::show_all();
    filter.error = false;
    console.set_filter(filter);

    assert_eq!(console.filtered_count(), 1);
}

#[test]
fn test_console_scroll() {
    let mut console = ConsolePanel::new();
    for i in 0..10 {
        console.add_message(MessageLevel::Info, format!("Message {}", i));
    }

    console.scroll_up();
    assert_eq!(console.scroll_position, 1);
    console.scroll_down();
    assert_eq!(console.scroll_position, 0);
}

#[test]
fn test_history_entry() {
    let entry = HistoryEntry::new("G0 X10");
    assert_eq!(entry.command, "G0 X10");
}
