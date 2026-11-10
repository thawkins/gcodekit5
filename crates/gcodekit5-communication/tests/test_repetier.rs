//! Regression tests for the Repetier protocol translation and streaming
//! (audit car 1, issue #72).
//!
//! Each test pins one defect found by the audit: the translated line must
//! keep the meaning of the GRBL-style input, and the streaming helpers must
//! frame, acknowledge, rewind and shut down correctly.

use gcodekit5_communication::firmware::repetier::processor::{
    JobLineMap, RepetierEvent, RepetierProcessor,
};
use gcodekit5_communication::firmware::repetier::RepetierSender;
use gcodekit5_communication::firmware::Sender;
use gcodekit5_communication::SerialCommunicator;
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
fn test_preformatted_line_m400_is_not_translated_to_m3() {
    let mut p = RepetierProcessor::new();
    let out = p.process_outgoing(b"N7 M400*99").expect("line is sent");
    let text = String::from_utf8_lossy(&out);
    assert!(text.starts_with("N7 M400 "), "got {text:?}");
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
fn test_job_line_map_resolves_resend_to_file_line() {
    let mut map = JobLineMap::default();
    // A console command used N1..N4 before the job started.
    map.record(4, 5, 0);
    map.record(5, 6, 1);
    map.record(6, 8, 2); // one file line that emitted two numbered lines
    assert_eq!(map.file_index(5), Some(0));
    assert_eq!(map.file_index(6), Some(1));
    assert_eq!(map.file_index(7), Some(2));
    assert_eq!(map.file_index(8), Some(2));
    assert_eq!(map.first_number(2), Some(7));
    assert_eq!(map.file_index(3), None);
    map.clear();
    assert_eq!(map.file_index(5), None);
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
