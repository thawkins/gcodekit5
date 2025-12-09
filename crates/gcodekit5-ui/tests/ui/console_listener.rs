#![cfg(feature = "slint_legacy_tests")]
/// Tests for ConsoleListener integration with DeviceConsoleManager
use gcodekit5_ui::{ConsoleListener, DeviceConsoleManager};
use gcodekit5_communication::CommunicatorListener;
use std::sync::Arc;

#[test]
fn test_console_listener_on_connected() {
    let console_manager = Arc::new(DeviceConsoleManager::new());
    let listener: Arc<ConsoleListener> = ConsoleListener::new(console_manager.clone());

    listener.as_ref().on_connected();

    let output = console_manager.get_output();
    assert!(output.contains("Device connected"));
}

#[test]
fn test_console_listener_on_disconnected() {
    let console_manager = Arc::new(DeviceConsoleManager::new());
    let listener: Arc<ConsoleListener> = ConsoleListener::new(console_manager.clone());

    listener.as_ref().on_disconnected();

    let output = console_manager.get_output();
    assert!(output.contains("Device disconnected"));
}

#[test]
fn test_console_listener_on_error() {
    let console_manager = Arc::new(DeviceConsoleManager::new());
    let listener: Arc<ConsoleListener> = ConsoleListener::new(console_manager.clone());

    listener.as_ref().on_error("Test error");

    let output = console_manager.get_output();
    assert!(output.contains("Test error"));
    assert!(output.contains("ERR"));
}

#[test]
fn test_console_listener_on_data_received() {
    let console_manager = Arc::new(DeviceConsoleManager::new());
    let listener: Arc<ConsoleListener> = ConsoleListener::new(console_manager.clone());

    listener.as_ref().on_data_received(b"ok\n");

    let output = console_manager.get_output();
    assert!(output.contains("ok"));
}

#[test]
fn test_console_listener_on_data_sent() {
    let console_manager = Arc::new(DeviceConsoleManager::new());
    let listener: Arc<ConsoleListener> = ConsoleListener::new(console_manager.clone());

    listener.as_ref().on_data_sent(b"G0 X10\n");

    let output = console_manager.get_output();
    assert!(output.contains("G0 X10"));
}

#[test]
fn test_console_listener_on_timeout() {
    let console_manager = Arc::new(DeviceConsoleManager::new());
    console_manager.set_verbose_enabled(true);
    let listener: Arc<ConsoleListener> = ConsoleListener::new(console_manager.clone());

    listener.as_ref().on_timeout();

    let output = console_manager.get_output();
    assert!(output.contains("Connection timeout"));
}

#[test]
fn test_console_listener_multiple_events() {
    let console_manager = Arc::new(DeviceConsoleManager::new());
    let listener: Arc<ConsoleListener> = ConsoleListener::new(console_manager.clone());

    listener.as_ref().on_connected();
    listener.as_ref().on_data_sent(b"G0 X10\n");
    listener.as_ref().on_data_received(b"ok\n");
    listener.as_ref().on_disconnected();

    let output = console_manager.get_output();
    assert!(output.contains("Device connected"));
    assert!(output.contains("G0 X10"));
    assert!(output.contains("ok"));
    assert!(output.contains("Device disconnected"));
}
