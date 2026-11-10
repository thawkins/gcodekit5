//! Regression tests for GRBL status report parsing.

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
fn test_malformed_axis_token_rejects_the_report() {
    // Audit B-11: a bad token used to be skipped, moving Z onto the Y axis.
    use gcodekit5_communication::firmware::grbl::status_parser::{
        MachinePosition, WorkCoordinateOffset, WorkPosition,
    };
    assert_eq!(MachinePosition::parse("1.000,2.0x0,3.000,4.000"), None);
    assert_eq!(WorkPosition::parse("1.000,2.0x0,3.000"), None);
    assert_eq!(WorkCoordinateOffset::parse("1,,3"), None);
    let ok = MachinePosition::parse("1.000,2.000,3.000,4.000").expect("valid");
    assert_eq!((ok.x, ok.y, ok.z, ok.a), (1.0, 2.0, 3.0, Some(4.0)));
}

#[test]
fn test_grblhal_and_fluidnc_are_grbl_family() {
    // Audit B-10: these banners were detected as Unknown, hiding the GRBL match.
    use gcodekit5_communication::firmware::protocol_registry::detect_protocol;
    use gcodekit5_communication::firmware::ControllerProtocol;
    for name in ["grblHAL", "GrblHal", "FluidNC"] {
        assert_eq!(
            ControllerProtocol::from_name(name),
            ControllerProtocol::Grbl
        );
    }
    assert_eq!(
        detect_protocol("GrblHAL 1.1f ['$' or '$HELP' for help]"),
        Some(ControllerProtocol::Grbl)
    );
    assert_eq!(
        detect_protocol("Grbl 3.7 [FluidNC v3.7.8 (wifi) '$' for help]"),
        Some(ControllerProtocol::Grbl)
    );
}

#[test]
fn test_inch_reports_are_converted_to_mm() {
    // Owner decision B-22: honour `$13=1` (positions reported in inches).
    use gcodekit5_communication::firmware::grbl::status_parser::parse_probe_report_in;
    let line = "<Idle|MPos:1.000,2.000,-0.500|FS:10,0|WCO:0.500,0.000,0.000>";
    let full = StatusParser::parse_full(line).into_mm(true);
    let m = full.mpos.expect("mpos");
    assert!((m.x - 25.4).abs() < 1e-9 && (m.y - 50.8).abs() < 1e-9 && (m.z + 12.7).abs() < 1e-9);
    let w = full.wco.expect("wco");
    assert!((w.x - 12.7).abs() < 1e-9);
    assert_eq!(full.feed_rate, Some(254.0));
    assert_eq!(
        StatusParser::parse_full(line).into_mm(false).mpos,
        StatusParser::parse_full(line).mpos
    );

    let probe = parse_probe_report_in("[PRB:1.000,0.000,-0.250:1]", true).expect("PRB");
    assert!((probe.position.x - 25.4).abs() < 1e-4);
    assert!((probe.position.z + 6.35).abs() < 1e-4);
}
