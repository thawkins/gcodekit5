use gcodekit5_communication::firmware::grbl::response_parser::*;

#[test]
fn test_parse_ok() {
    let parser = GrblResponseParser::new();
    assert_eq!(parser.parse("ok"), Some(GrblResponse::Ok));
}

#[test]
fn test_parse_error() {
    let parser = GrblResponseParser::new();
    assert_eq!(parser.parse("error:1"), Some(GrblResponse::Error(1)));
    assert_eq!(parser.parse("error:23"), Some(GrblResponse::Error(23)));
}

#[test]
fn test_parse_alarm() {
    let parser = GrblResponseParser::new();
    assert_eq!(parser.parse("alarm:1"), Some(GrblResponse::Alarm(1)));
    assert_eq!(parser.parse("alarm:6"), Some(GrblResponse::Alarm(6)));
}

#[test]
fn test_parse_status_report() {
    let parser = GrblResponseParser::new();
    let response = parser.parse("<Idle|MPos:0.000,0.000,0.000|WPos:0.000,0.000,0.000>");

    assert!(matches!(response, Some(GrblResponse::Status(_))));

    if let Some(GrblResponse::Status(status)) = response {
        assert_eq!(status.state, "Idle");
        assert_eq!(status.machine_pos.x, 0.0);
        assert_eq!(status.work_pos.y, 0.0);
    }
}

#[test]
fn test_parse_status_with_buffer() {
    let parser = GrblResponseParser::new();
    let response =
        parser.parse("<Run|MPos:10.000,5.000,2.500|WPos:10.000,5.000,2.500|Buf:15:128>");

    if let Some(GrblResponse::Status(status)) = response {
        assert_eq!(status.state, "Run");
        assert_eq!(
            status.buffer_state,
            Some(BufferState {
                plan: 15,
                exec: 128
            })
        );
    }
}

#[test]
fn test_parse_status_with_feedrate() {
    let parser = GrblResponseParser::new();
    let response = parser.parse("<Run|MPos:0,0,0|WPos:0,0,0|F:1500.0|S:1200>");

    if let Some(GrblResponse::Status(status)) = response {
        assert_eq!(status.feed_rate, Some(1500.0));
        assert_eq!(status.spindle_speed, Some(1200));
    }
}

#[test]
fn test_parse_setting() {
    let parser = GrblResponseParser::new();
    assert!(matches!(
        parser.parse("$110=1000.000"),
        Some(GrblResponse::Setting { .. })
    ));
}

#[test]
fn test_parse_version() {
    let parser = GrblResponseParser::new();
    assert!(matches!(
        parser.parse("Grbl 1.1h ['$' for help]"),
        Some(GrblResponse::Version(_))
    ));
}

#[test]
fn test_parse_build_info() {
    let parser = GrblResponseParser::new();
    assert!(matches!(
        parser.parse("[GrblHAL 1.1 STM32F4xx]"),
        Some(GrblResponse::BuildInfo(_))
    ));
}

#[test]
fn test_error_description() {
    assert_eq!(
        GrblResponseParser::error_description(1),
        "Expected command letter"
    );
    assert_eq!(
        GrblResponseParser::error_description(23),
        "Failed to execute startup block"
    );
}

#[test]
fn test_alarm_description() {
    assert_eq!(
        GrblResponseParser::alarm_description(1),
        "Hard limit triggered"
    );
    assert_eq!(GrblResponseParser::alarm_description(6), "Homing fail");
}

#[test]
fn test_parse_empty_line() {
    let parser = GrblResponseParser::new();
    assert_eq!(parser.parse(""), None);
}

#[test]
fn test_parse_multiaxis_position() {
    let parser = GrblResponseParser::new();
    let response =
        parser.parse("<Idle|MPos:10.000,20.000,30.000,5.000|WPos:10.000,20.000,30.000,5.000>");

    if let Some(GrblResponse::Status(status)) = response {
        assert_eq!(status.machine_pos.x, 10.0);
        assert_eq!(status.machine_pos.y, 20.0);
        assert_eq!(status.machine_pos.z, 30.0);
        assert_eq!(status.machine_pos.a, 5.0);
    }
}
