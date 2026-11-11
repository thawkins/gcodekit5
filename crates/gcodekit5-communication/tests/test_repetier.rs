//! Regression tests for the Repetier protocol translation and streaming
//! (audit car 1, issue #72).
//!
//! Each test pins one defect found by the audit: the translated line must
//! keep the meaning of the GRBL-style input, and the streaming helpers must
//! frame, acknowledge, rewind and shut down correctly.

use gcodekit5_communication::SerialCommunicator;
use gcodekit5_communication::firmware::Sender;
use gcodekit5_communication::firmware::repetier::RepetierSender;
use gcodekit5_communication::firmware::repetier::processor::{RepetierEvent, RepetierProcessor};
use parking_lot::Mutex;
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Translates one outgoing line and returns the G-code body of each emitted
/// line, without the `N<n>` prefix and `*<checksum>` suffix.
fn translate(p: &mut RepetierProcessor, line: &str) -> Vec<String> {
    let out = p
        .process_outgoing(line.as_bytes())
        .map(|b| String::from_utf8_lossy(&b).into_owned())
        .unwrap_or_default();
    out.lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| {
            let body = l.split('*').next().unwrap_or("").trim();
            match body.split_once(' ') {
                Some((n, rest)) if n.starts_with('N') => rest.trim().to_string(),
                _ => body.to_string(),
            }
        })
        .collect()
}

#[test]
fn test_m400_is_not_translated_to_m3() {
    let mut p = RepetierProcessor::new();
    assert_eq!(translate(&mut p, "M400"), vec!["M400"]);
    assert_eq!(translate(&mut p, "M42 P4"), vec!["M42 P4"]);
}

#[test]
fn test_m4_is_still_translated_to_m3() {
    let mut p = RepetierProcessor::new();
    assert_eq!(translate(&mut p, "M4 S500"), vec!["M3 S500"]);
}

#[test]
fn test_preformatted_line_is_renumbered_and_m400_kept() {
    let mut p = RepetierProcessor::new();
    let out = p.process_outgoing(b"N7 M400*99").expect("line is sent");
    let text = String::from_utf8_lossy(&out);
    assert!(text.starts_with("N1 M400 "), "got {text:?}");
    assert_eq!(p.line_counter, 1);
    assert_eq!(p.resend_lines(1), Some(vec![text.into_owned()]));
}

#[test]
fn test_compact_words_are_tokenized() {
    let mut p = RepetierProcessor::new();
    assert_eq!(translate(&mut p, "M4S500"), vec!["M3 S500"]);
    assert_eq!(translate(&mut p, "M400"), vec!["M400"]);
    assert_eq!(translate(&mut p, "N10G1X10Y5"), vec!["G1 X10 Y5"]);
    assert!(status(&mut p).contains("MPos:10.000,5.000,0.000"));
}

#[test]
fn test_g28_keeps_axis_words() {
    let mut p = RepetierProcessor::new();
    assert_eq!(translate(&mut p, "G28 Z0"), vec!["G28 Z0.00"]);
}

#[test]
fn test_g28_1_is_not_turned_into_home_all() {
    let mut p = RepetierProcessor::new();
    assert_eq!(translate(&mut p, "G28.1"), vec!["G28.1"]);
}

#[test]
fn test_dollar_h_homes_all_axes() {
    let mut p = RepetierProcessor::new();
    assert_eq!(translate(&mut p, "$H"), vec!["G28"]);
}

#[test]
fn test_g10_l20_keeps_values_and_all_axes() {
    let mut p = RepetierProcessor::new();
    assert_eq!(
        translate(&mut p, "G10 L20 P1 X0.5 Y20"),
        vec!["G92 X0.5 Y20"]
    );
}

#[test]
fn test_g10_l20_zero_all_axes() {
    let mut p = RepetierProcessor::new();
    assert_eq!(
        translate(&mut p, "G10 L20 P1 X0 Y0 Z0"),
        vec!["G92 X0 Y0 Z0"]
    );
}

#[test]
fn test_jog_keeps_axis_and_feed_and_restores_absolute() {
    let mut p = RepetierProcessor::new();
    assert_eq!(
        translate(&mut p, "$J=G21 G91 X10 F1000"),
        vec!["G21", "G91", "G1 X10 F1000", "G90"]
    );
}

#[test]
fn test_jog_honours_g90_and_restores_prior_g91() {
    let mut p = RepetierProcessor::new();
    translate(&mut p, "G91");
    assert_eq!(
        translate(&mut p, "$J=G90 X10 F100"),
        vec!["G90", "G1 X10 F100", "G91"]
    );
}

#[test]
fn test_inch_jog_restores_mm() {
    let mut p = RepetierProcessor::new();
    assert_eq!(
        translate(&mut p, "$J=G20 G91 X1 F10"),
        vec!["G20", "G91", "G1 X1 F10", "G90", "G21"]
    );
}

#[test]
fn test_jog_with_multiple_axes() {
    let mut p = RepetierProcessor::new();
    assert_eq!(
        translate(&mut p, "$J=G91 X-1.5 Y2 F300"),
        vec!["G91", "G1 X-1.5 Y2 F300", "G90"]
    );
}

#[test]
fn test_user_line_number_is_replaced_not_dropped() {
    let mut p = RepetierProcessor::new();
    assert_eq!(translate(&mut p, "N10 G1 X5"), vec!["G1 X5"]);
}

#[test]
fn test_parameter_only_lines_are_sent() {
    let mut p = RepetierProcessor::new();
    assert_eq!(translate(&mut p, "F1000"), vec!["F1000"]);
    assert_eq!(translate(&mut p, "S1000"), vec!["S1000"]);
    assert_eq!(translate(&mut p, "E2.5"), vec!["E2.5"]);
}

#[test]
fn test_line_numbers_are_consecutive_with_valid_checksum() {
    let mut p = RepetierProcessor::new();
    let a = String::from_utf8(p.process_outgoing(b"G1 X1").expect("sent")).expect("utf8");
    let b = String::from_utf8(p.process_outgoing(b"G1 X2").expect("sent")).expect("utf8");
    assert!(a.starts_with("N1 "), "got {a:?}");
    assert!(b.starts_with("N2 "), "got {b:?}");
    let (body, ck) = b.trim_end().split_once('*').expect("checksum present");
    let expected = body.bytes().fold(0u8, |acc, x| acc ^ x);
    assert_eq!(ck.parse::<u8>().expect("numeric checksum"), expected);
}

#[test]
fn test_ack_split_across_reads_is_detected() {
    let mut p = RepetierProcessor::new();
    let first = p.process_incoming(b"o".to_vec());
    assert!(first.events.is_empty());
    let second = p.process_incoming(b"k\n".to_vec());
    assert_eq!(second.events, vec![RepetierEvent::Ack]);
}

#[test]
fn test_resend_split_across_reads_is_detected() {
    let mut p = RepetierProcessor::new();
    assert!(p.process_incoming(b"Resend:1".to_vec()).events.is_empty());
    let r = p.process_incoming(b"2\nok\n".to_vec());
    assert_eq!(
        r.events,
        vec![RepetierEvent::Resend(12), RepetierEvent::Ack]
    );
}

fn status(p: &mut RepetierProcessor) -> String {
    let r = p.process_incoming(Vec::new());
    String::from_utf8_lossy(&r.display).into_owned()
}

#[test]
fn test_status_tracks_absolute_and_relative_moves() {
    let mut p = RepetierProcessor::new();
    translate(&mut p, "G90");
    translate(&mut p, "G1 X10 Y5");
    assert!(status(&mut p).contains("MPos:10.000,5.000,0.000"));
    translate(&mut p, "$J=G91 X1 F500");
    assert!(status(&mut p).contains("MPos:11.000,5.000,0.000"));
    translate(&mut p, "G1 Z2");
    assert!(status(&mut p).contains("MPos:11.000,5.000,2.000"));
}

#[test]
fn test_status_uses_reported_position() {
    let mut p = RepetierProcessor::new();
    let r =
        p.process_incoming(b"X:3.00 Y:4.00 Z:5.00 E:0.00 Count X:3.00 Y:4.00 Z:5.00\n".to_vec());
    let text = String::from_utf8_lossy(&r.display);
    assert!(text.contains("MPos:3.000,4.000,5.000"), "got {text:?}");
}

#[test]
fn test_status_state_follows_flow_control() {
    let mut p = RepetierProcessor::new();
    assert!(status(&mut p).contains("<Idle|"));
    p.mark_buffer_full();
    assert!(status(&mut p).contains("<Run|"));
    p.mark_ack();
    assert!(status(&mut p).contains("<Idle|"));
}

#[test]
fn test_resend_lines_replays_exact_suffix() {
    let mut p = RepetierProcessor::new();
    let sent: Vec<String> = translate_raw(&mut p, "$J=G91 X1 F100");
    assert_eq!(sent.len(), 3);
    // N2 and N3 were not accepted: replay exactly those, not N1.
    assert_eq!(p.resend_lines(2), Some(sent[1..].to_vec()));
    // The next number not yet sent: nothing to replay.
    assert_eq!(p.resend_lines(4), Some(Vec::new()));
    // A number never sent cannot be honoured.
    assert_eq!(p.resend_lines(5), None);
    assert_eq!(p.resend_lines(0), None);
}

#[test]
fn test_resend_of_evicted_line_is_refused() {
    let mut p = RepetierProcessor::new();
    for i in 0..600 {
        translate(&mut p, &format!("G1 X{i}"));
    }
    assert_eq!(p.resend_lines(1), None);
    assert_eq!(p.resend_lines(600).map(|l| l.len()), Some(1));
}

#[test]
fn test_activate_resets_modal_state_and_position() {
    let mut p = RepetierProcessor::new();
    translate(&mut p, "G20");
    translate(&mut p, "G91");
    translate(&mut p, "G1 X1");
    p.activate();
    translate(&mut p, "G1 X10");
    assert!(status(&mut p).contains("MPos:10.000,0.000,0.000"));
    assert_eq!(p.resend_lines(1).map(|l| l.len()), Some(1));
}

#[test]
fn test_status_reports_g92_offset_as_wco() {
    let mut p = RepetierProcessor::new();
    translate(&mut p, "G1 X10 Y4");
    translate(&mut p, "G10 L20 P1 X0 Y0");
    let s = status(&mut p);
    assert!(s.contains("MPos:10.000,4.000,0.000"), "got {s:?}");
    assert!(s.contains("WCO:10.000,4.000,0.000"), "got {s:?}");
    // Absolute moves are now in the shifted work coordinates.
    translate(&mut p, "G1 X1");
    assert!(status(&mut p).contains("MPos:11.000,4.000,0.000"));
    // Homing clears the offset.
    translate(&mut p, "$H");
    assert!(status(&mut p).contains("WCO:0.000,0.000,0.000"));
}

#[test]
fn test_configuration_axis_words_do_not_move_position() {
    let mut p = RepetierProcessor::new();
    translate(&mut p, "M201 X1000 Y1000 Z100");
    translate(&mut p, "M92 X80");
    assert!(status(&mut p).contains("MPos:0.000,0.000,0.000"));
}

#[test]
fn test_ack_in_same_read_reports_idle() {
    let mut p = RepetierProcessor::new();
    p.mark_buffer_full();
    let r = p.process_incoming(b"ok\n".to_vec());
    let text = String::from_utf8_lossy(&r.display);
    assert!(text.contains("<Idle|"), "got {text:?}");
}

/// Translates one outgoing line and returns each emitted numbered line as
/// written to the port.
fn translate_raw(p: &mut RepetierProcessor, line: &str) -> Vec<String> {
    let out = p.process_outgoing(line.as_bytes()).expect("line is sent");
    String::from_utf8_lossy(&out)
        .split_inclusive('\n')
        .map(str::to_string)
        .collect()
}

fn new_sender() -> (RepetierSender, std::sync::mpsc::Receiver<String>) {
    let shared = |v: bool| Arc::new(Mutex::new(v));
    RepetierSender::new(
        Arc::new(Mutex::new(SerialCommunicator::new())),
        shared(false),
        shared(false),
        shared(false),
    )
}

#[test]
fn test_sender_thread_exits_when_sender_is_dropped() {
    let (sender, rx) = new_sender();
    drop(sender);
    assert_eq!(
        rx.recv_timeout(Duration::from_secs(3)),
        Err(std::sync::mpsc::RecvTimeoutError::Disconnected)
    );
}

#[test]
fn test_sender_reports_lost_connection_and_stops() {
    let (sender, rx) = new_sender();
    sender.send_gcode("G1 X1\nG1 X2\n");
    let deadline = Instant::now() + Duration::from_secs(3);
    let mut messages = Vec::new();
    while Instant::now() < deadline {
        if let Ok(m) = rx.recv_timeout(Duration::from_millis(50)) {
            let done = m.contains("Completed");
            messages.push(m);
            if done {
                break;
            }
        }
    }
    assert!(
        messages
            .iter()
            .any(|m| m.starts_with('>') && m.contains("not connected")),
        "messages: {messages:?}"
    );
    assert!(messages.iter().any(|m| m.contains("Completed")));
    assert!(!sender.is_streaming());
}

/// Streams `program` with the Repetier sender to a local TCP controller that
/// acknowledges every line, and returns the text the controller received.
fn stream_to_fake_controller(program: &str, pen_mode: bool) -> String {
    use gcodekit5_communication::firmware::ControllerProtocol;
    use gcodekit5_communication::{Communicator, ConnectionParams};
    use std::io::{Read, Write};

    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind");
    let port = listener.local_addr().expect("addr").port();
    let server = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept");
        stream
            .set_read_timeout(Some(Duration::from_secs(5)))
            .expect("timeout");
        let mut received = String::new();
        let mut buf = [0u8; 1024];
        while let Ok(n) = stream.read(&mut buf) {
            if n == 0 {
                break;
            }
            let chunk = String::from_utf8_lossy(&buf[..n]).into_owned();
            for _ in chunk.matches('\n') {
                stream.write_all(b"ok\n").expect("ack");
            }
            received.push_str(&chunk);
        }
        received
    });

    let mut comm = SerialCommunicator::new();
    comm.connect(&ConnectionParams::tcp("127.0.0.1", port))
        .expect("connect");
    comm.set_configured_protocol(ControllerProtocol::Repetier);
    let comm = Arc::new(Mutex::new(comm));
    let shared = |v: bool| Arc::new(Mutex::new(v));
    let (sender, rx) =
        RepetierSender::new(comm.clone(), shared(false), shared(false), shared(false));
    sender.set_pen_mode(pen_mode);
    sender.send_gcode(program);

    let deadline = Instant::now() + Duration::from_secs(5);
    while Instant::now() < deadline {
        if let Ok(m) = rx.recv_timeout(Duration::from_millis(50))
            && m.contains("Completed")
        {
            break;
        }
    }
    drop(sender);
    let _ = comm.lock().disconnect();
    drop(comm);
    server.join().expect("server")
}

#[test]
fn test_pen_mode_off_sends_spindle_commands_unchanged() {
    let sent = stream_to_fake_controller("M3 S1000\nG1 X1 F100\nM5\n", false);
    assert!(sent.contains(" M3 S1000 "), "{sent}");
    assert!(sent.contains(" M5 "), "{sent}");
    assert!(!sent.contains("Z5"), "{sent}");
}

#[test]
fn test_pen_mode_on_turns_spindle_commands_into_z_moves() {
    let sent = stream_to_fake_controller("M5\nM3 S1000\nG1 X1 F100\nM5\n", true);
    assert!(!sent.contains(" M3"), "{sent}");
    assert!(sent.contains("G0 Z0.00"), "{sent}");
    assert!(sent.contains("G0 Z5"), "{sent}");
}
