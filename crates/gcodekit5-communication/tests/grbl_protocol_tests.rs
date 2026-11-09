//! Regression tests for GRBL status report parsing.

use gcodekit5_communication::firmware::device_status::{DeviceStatus, MachineStateType};
use gcodekit5_communication::firmware::grbl::StatusParser;

#[test]
fn test_status_parser_fs_field_spindle_speed() {
    // "S:" must not be matched inside "FS:"
    let line = "<Run|MPos:1.000,2.000,3.000|FS:500,8000|Ov:100,100,100>";
    assert_eq!(StatusParser::parse_feed_rate(line), Some(500.0));
    assert_eq!(StatusParser::parse_spindle_speed(line), Some(8000));
    let full = StatusParser::parse_full(line);
    assert_eq!(full.spindle_speed, Some(8000));
    assert_eq!(full.feed_rate, Some(500.0));
}

#[test]
fn test_status_parser_bf_buffer_field() {
    let line = "<Idle|MPos:0.000,0.000,0.000|Bf:15,128|FS:0,0>";
    let buf = StatusParser::parse_buffer(line).expect("Bf: field should parse");
    assert_eq!(buf.plan, 15);
    assert_eq!(buf.rx, 128);

    // grblHAL reports RX buffers larger than 255 bytes
    let line = "<Idle|MPos:0.000,0.000,0.000|Bf:35,1023|FS:0,0>";
    assert_eq!(StatusParser::parse_buffer(line).map(|b| b.rx), Some(1023));
}

#[test]
fn test_status_parser_bare_state() {
    assert_eq!(
        StatusParser::parse_machine_state("<Idle>").as_deref(),
        Some("Idle")
    );
    assert_eq!(
        StatusParser::parse_machine_state("<Hold:0|MPos:0,0,0>").as_deref(),
        Some("Hold:0")
    );
}

#[test]
fn test_device_status_fs_and_override_order() {
    let status =
        DeviceStatus::parse_grbl_status("<Hold:0|MPos:1.0,2.0,3.0|FS:500,8000|Ov:110,50,90>")
            .expect("status should parse");
    assert_eq!(status.state, MachineStateType::Hold);
    assert_eq!(status.feed_rate, Some(500.0));
    assert_eq!(status.spindle_speed, Some(8000));
    let ov = status.overrides.expect("overrides");
    assert_eq!((ov.feed_rate, ov.rapid, ov.spindle_speed), (110, 50, 90));
}
