//! Tests for the GTK-free GRBL streaming helpers used by the Machine Control panel:
//! character-counting flow control, response line splitting/classification, probe
//! reports, axis detection, jog step presets and jog feed validation.

use gcodekit5_communication::ConnectionDriver;
use gcodekit5_communication::firmware::grbl::status_parser::MachinePosition;
use gcodekit5_core::units::{FeedRateUnits, MeasurementSystem};
use gcodekit5_ui::ui::gtk::machine_control::grbl_stream::{
    CharCountWindow, GRBL_RX_BUFFER_BYTES, GrblResponseLine, ResponseLineSplitter,
    classify_response, format_probe_move, jog_step_presets, line_cost, nearest_jog_step_preset,
    network_driver_for, parse_jog_feed, parse_probe_report, reported_axis_count,
};

#[test]
fn test_char_count_line_cost_includes_newline() {
    assert_eq!(line_cost("G0 X1"), 6);
    assert_eq!(line_cost(""), 1);
}

#[test]
fn test_char_count_never_exceeds_rx_buffer() {
    let mut window = CharCountWindow::default();
    let line = "G1 X10.000 Y20.000 F1000"; // 24 bytes + newline = 25
    let mut sent = 0;
    while window.can_send(line) {
        window.on_sent(line);
        sent += 1;
    }
    // 5 * 25 = 125 <= 127, a sixth line would need 150.
    assert_eq!(sent, 5);
    assert_eq!(window.in_flight_bytes(), 125);
    assert!(window.in_flight_bytes() <= GRBL_RX_BUFFER_BYTES);
}

#[test]
fn test_char_count_exact_fit_is_allowed() {
    let mut window = CharCountWindow::new(127);
    let first = "a".repeat(99); // 100 bytes
    window.on_sent(&first);
    let exact = "b".repeat(26); // 27 bytes -> 127 total
    assert!(window.can_send(&exact));
    let too_big = "b".repeat(27); // 28 bytes -> 128 total
    assert!(!window.can_send(&too_big));
}

#[test]
fn test_char_count_ack_frees_oldest_line_first() {
    let mut window = CharCountWindow::default();
    window.on_sent(&"a".repeat(59)); // 60 bytes
    window.on_sent(&"b".repeat(9)); // 10 bytes
    assert_eq!(window.in_flight_bytes(), 70);
    assert!(window.on_ack());
    assert_eq!(window.in_flight_bytes(), 10);
    assert_eq!(window.in_flight_lines(), 1);
    assert!(window.on_ack());
    assert!(window.is_empty());
    // An unexpected extra ack is reported and does not underflow.
    assert!(!window.on_ack());
    assert_eq!(window.in_flight_bytes(), 0);
}

#[test]
fn test_char_count_rejects_line_larger_than_buffer() {
    let window = CharCountWindow::default();
    assert!(window.fits_at_all(&"x".repeat(126)));
    assert!(!window.fits_at_all(&"x".repeat(127)));
}

#[test]
fn test_splitter_carries_partial_lines_over() {
    let mut splitter = ResponseLineSplitter::new();
    assert!(splitter.push(b"o").is_empty());
    assert_eq!(splitter.push(b"k\r\nok\n<Idle|MPos:0"), vec!["ok", "ok"]);
    assert_eq!(splitter.pending(), b"<Idle|MPos:0");
    assert_eq!(
        splitter.push(b".000,0.000,0.000>\r\n"),
        vec!["<Idle|MPos:0.000,0.000,0.000>"]
    );
    assert!(splitter.pending().is_empty());
}

#[test]
fn test_splitter_handles_utf8_split_across_reads() {
    let mut splitter = ResponseLineSplitter::new();
    let text = "[MSG:Température]\n".as_bytes();
    let (a, b) = text.split_at(11); // splits the two-byte 'é'
    assert!(splitter.push(a).is_empty());
    assert_eq!(splitter.push(b), vec!["[MSG:Température]"]);
}

#[test]
fn test_classify_only_exact_ok_is_an_ack() {
    assert_eq!(classify_response("ok"), GrblResponseLine::Ok);
    assert_eq!(classify_response(" OK "), GrblResponseLine::Ok);
    // Text containing "ok" must not be counted as an acknowledgement.
    assert!(matches!(
        classify_response("[MSG:Probe ok]"),
        GrblResponseLine::Other(_)
    ));
    assert!(matches!(
        classify_response("[GC:G0 G54 G17 G21 G90 G94 M5 M9 T0 F0 S0 token]"),
        GrblResponseLine::Other(_)
    ));
}

#[test]
fn test_classify_errors_alarms_status_and_reset() {
    assert_eq!(
        classify_response("error:20"),
        GrblResponseLine::Error("20".to_string())
    );
    assert_eq!(
        classify_response("ALARM:4"),
        GrblResponseLine::Alarm("4".to_string())
    );
    assert_eq!(
        classify_response("<Run|MPos:1.000,2.000,3.000|FS:500,0>"),
        GrblResponseLine::Status("<Run|MPos:1.000,2.000,3.000|FS:500,0>".to_string())
    );
    assert!(matches!(
        classify_response("Grbl 1.1h ['$' for help]"),
        GrblResponseLine::Reset(_)
    ));
}

#[test]
fn test_probe_report_success_and_failure() {
    assert_eq!(parse_probe_report("[PRB:0.000,0.000,-1.234:1]"), Some(true));
    assert_eq!(parse_probe_report("[PRB:0.000,0.000,0.000:0]"), Some(false));
    assert_eq!(parse_probe_report("[MSG:Reset to continue]"), None);
    assert_eq!(parse_probe_report("ok"), None);
}

#[test]
fn test_reported_axis_count_from_position() {
    let mut pos = MachinePosition {
        x: 0.0,
        y: 0.0,
        z: 0.0,
        a: None,
        b: None,
        c: None,
    };
    assert_eq!(reported_axis_count(&pos), 3);
    pos.a = Some(0.0);
    assert_eq!(reported_axis_count(&pos), 4);
    pos.b = Some(0.0);
    assert_eq!(reported_axis_count(&pos), 5);
    pos.c = Some(0.0);
    assert_eq!(reported_axis_count(&pos), 6);
}

#[test]
fn test_jog_presets_inch_mode_uses_inch_values_stored_in_mm() {
    let presets = jog_step_presets(MeasurementSystem::Imperial);
    let labels: Vec<&str> = presets.iter().map(|p| p.label.as_str()).collect();
    assert_eq!(
        labels,
        vec!["0.0001 in", "0.001 in", "0.01 in", "0.1 in", "1 in"]
    );
    assert!((presets[0].mm - 0.00254).abs() < 1e-6);
    assert!((presets[4].mm - 25.4).abs() < 1e-4);
}

#[test]
fn test_jog_presets_metric_mode() {
    let presets = jog_step_presets(MeasurementSystem::Metric);
    assert_eq!(presets.len(), 6);
    assert_eq!(presets[3].label, "1 mm");
    assert!((presets[3].mm - 1.0).abs() < f32::EPSILON);
}

#[test]
fn test_jog_presets_nearest_on_unit_switch() {
    let inch = jog_step_presets(MeasurementSystem::Imperial);
    // 1 mm (0.039 in) lies between 0.01 in and 0.1 in; by ratio it is closer to 0.1 in.
    let idx = nearest_jog_step_preset(&inch, 1.0).expect("presets are not empty");
    assert_eq!(inch[idx].label, "0.1 in");
    let idx = nearest_jog_step_preset(&inch, 25.4).expect("presets are not empty");
    assert_eq!(inch[idx].label, "1 in");

    let metric = jog_step_presets(MeasurementSystem::Metric);
    let idx = nearest_jog_step_preset(&metric, 2.54).expect("presets are not empty");
    assert_eq!(metric[idx].label, "1 mm");
    assert_eq!(nearest_jog_step_preset(&[], 1.0), None);
}

#[test]
fn test_jog_feed_validation() {
    let v = parse_jog_feed("2000", FeedRateUnits::MmPerMin).expect("valid feed");
    assert!((v - 2000.0).abs() < f32::EPSILON);
    let v = parse_jog_feed("10", FeedRateUnits::InPerMin).expect("valid feed");
    assert!((v - 254.0).abs() < 1e-3);
    assert!(parse_jog_feed("abc", FeedRateUnits::MmPerMin).is_err());
    assert!(parse_jog_feed("", FeedRateUnits::MmPerMin).is_err());
    assert!(parse_jog_feed("0", FeedRateUnits::MmPerMin).is_err());
    assert!(parse_jog_feed("-5", FeedRateUnits::MmPerMin).is_err());
}

#[test]
fn test_probe_move_display_respects_units() {
    let metric = format_probe_move(
        ('Z', -20.0),
        100.0,
        MeasurementSystem::Metric,
        FeedRateUnits::MmPerMin,
    );
    assert_eq!(metric, "Z-20.00 mm at 100.00 mm/min");
    let inch = format_probe_move(
        ('X', 10.0),
        100.0,
        MeasurementSystem::Imperial,
        FeedRateUnits::InPerMin,
    );
    assert_eq!(inch, "X0.39 in at 3.94 in/min");
}

#[test]
fn test_connection_type_selects_network_driver() {
    assert_eq!(network_driver_for("TCP/IP"), Some(ConnectionDriver::Tcp));
    assert_eq!(network_driver_for("tcp"), Some(ConnectionDriver::Tcp));
    assert_eq!(
        network_driver_for("WebSocket"),
        Some(ConnectionDriver::WebSocket)
    );
    assert_eq!(network_driver_for("Serial"), None);
    assert_eq!(network_driver_for(""), None);
}
