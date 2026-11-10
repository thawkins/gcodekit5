//! Tests for `gcodekit5_communication::firmware::grbl::error_decoder` (moved from the inline test module).

use gcodekit5_communication::firmware::grbl::error_decoder::*;

#[test]
fn test_decode_error() {
    assert_eq!(
        decode_error(1),
        "G-code words consist of a letter and a value. Letter was not found."
    );
    assert_eq!(
        decode_error(9),
        "G-code locked out during alarm or jog state."
    );
    assert_eq!(
        decode_error(53),
        "Invalid file number. File number specified does not exist or is invalid."
    );
    assert_eq!(
        decode_error(69),
        "Homing is required. Command cannot execute until machine is homed."
    );
    assert!(decode_error(255).contains("Unknown error code"));
}

#[test]
fn test_decode_alarm() {
    assert!(decode_alarm(1).contains("Hard limit"));
    assert!(decode_alarm(2).contains("Soft limit"));
    assert!(decode_alarm(12).contains("E-stop"));
    assert!(decode_alarm(15).contains("Self-test failed"));
    assert!(decode_alarm(255).contains("Unknown alarm code"));
}

#[test]
fn test_format_error() {
    let msg = format_error(1);
    assert!(msg.starts_with("error:1"));
    assert!(msg.contains("Letter was not found"));
}

#[test]
fn test_format_alarm() {
    let msg = format_alarm(1);
    assert!(msg.starts_with("ALARM:1"));
    assert!(msg.contains("Hard limit"));
}
