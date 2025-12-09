#![cfg(feature = "slint_legacy_tests")]
/// Debug test to verify console output formatting
use gcodekit5_ui::{DeviceConsoleManager, DeviceMessageType};

#[test]
fn test_console_output_format() {
    let console_manager = DeviceConsoleManager::new();

    console_manager.add_message(DeviceMessageType::Success, "GCodeKit4 initialized");
    console_manager.add_message(DeviceMessageType::Output, "Ready for operation");

    let output = console_manager.get_output();

    println!("\n=== Console Output Debug ===");
    println!("Output:\n{}", output);
    println!("Output length: {}", output.len());
    println!("Contains 'GCodeKit4': {}", output.contains("GCodeKit4"));
    println!("Contains 'Ready': {}", output.contains("Ready"));
    println!("Output lines: {}", output.lines().count());

    assert!(!output.is_empty(), "Console output should not be empty");
    assert!(
        output.contains("GCodeKit4"),
        "Output should contain 'GCodeKit4'"
    );
    assert!(output.contains("Ready"), "Output should contain 'Ready'");
}

#[test]
fn test_console_output_with_connection() {
    let console_manager = DeviceConsoleManager::new();

    console_manager.add_message(
        DeviceMessageType::Output,
        "Connecting to /dev/ttyUSB0 at 115200 baud",
    );
    console_manager.add_message(
        DeviceMessageType::Success,
        "Successfully connected to /dev/ttyUSB0 at 115200 baud",
    );

    let output = console_manager.get_output();

    println!("\n=== Connection Messages ===");
    println!("Output:\n{}", output);

    assert!(
        output.contains("Connecting"),
        "Should contain connection message"
    );
    assert!(
        output.contains("Successfully connected"),
        "Should contain success message"
    );
}
