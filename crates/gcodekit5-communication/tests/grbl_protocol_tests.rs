//! Regression tests for GRBL protocol handling: status report parsing,
//! response parsing, real-time override bytes and command formatting.

use gcodekit5_communication::firmware::device_status::{DeviceStatus, MachineStateType};
use gcodekit5_communication::firmware::grbl::{
    CommandCreator, GrblResponse, GrblResponseParser, RealTimeOverrideCommand, StatusParser,
};

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
fn test_response_parser_uppercase_alarm() {
    let parser = GrblResponseParser::new();
    assert_eq!(parser.parse("ALARM:1"), Some(GrblResponse::Alarm(1)));
    assert_eq!(parser.parse("alarm:9"), Some(GrblResponse::Alarm(9)));
}

#[test]
fn test_response_parser_status_fs_and_bf() {
    let parser = GrblResponseParser::new();
    match parser.parse("<Run|MPos:1.000,2.000,3.000|Bf:15,128|FS:750,12000>") {
        Some(GrblResponse::Status(report)) => {
            assert_eq!(report.feed_rate, Some(750.0));
            assert_eq!(report.spindle_speed, Some(12000));
            let buf = report.buffer_state.expect("buffer state");
            assert_eq!((buf.plan, buf.exec), (15, 128));
        }
        other => panic!("expected status, got {:?}", other),
    }
}

#[test]
fn test_response_parser_error_descriptions() {
    assert_eq!(
        GrblResponseParser::error_description(7),
        "EEPROM read failed"
    );
    assert_eq!(
        GrblResponseParser::error_description(23),
        "Command requires integer value"
    );
    assert_eq!(
        GrblResponseParser::alarm_description(8),
        "Homing fail: pull-off failed to clear limit switch"
    );
}

#[test]
fn test_override_bytes_match_grbl_1_1() {
    assert_eq!(RealTimeOverrideCommand::FeedIncrease10.as_byte(), 0x91);
    assert_eq!(RealTimeOverrideCommand::FeedDecrease10.as_byte(), 0x92);
    assert_eq!(RealTimeOverrideCommand::FeedIncrease1.as_byte(), 0x93);
    assert_eq!(RealTimeOverrideCommand::FeedDecrease1.as_byte(), 0x94);
    assert_eq!(RealTimeOverrideCommand::RapidOv100.as_byte(), 0x95);
    assert_eq!(RealTimeOverrideCommand::RapidOv50.as_byte(), 0x96);
    assert_eq!(RealTimeOverrideCommand::RapidOv25.as_byte(), 0x97);
}

#[test]
fn test_jog_commands_have_no_motion_mode() {
    let cmd = CommandCreator::jog_incremental("X", -5.0, 1000.0);
    assert_eq!(cmd, "$J=G91 X-5.000 F1000\n");
    assert!(!cmd.contains("G0"));
}

#[test]
fn test_set_work_offset_uses_l20() {
    assert_eq!(
        CommandCreator::set_work_offset(&["X", "Y"]),
        "G10 L20 P0 X0 Y0\n"
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
