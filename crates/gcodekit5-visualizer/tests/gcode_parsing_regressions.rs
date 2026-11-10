//! Regression tests for G-code parsing, arc handling, bounds and statistics.

use std::fs;
use std::sync::Arc;

use gcodekit5_visualizer::gcode::processors::ArcExpander;
use gcodekit5_visualizer::gcode::words::{parse_line, strip_comments};
use gcodekit5_visualizer::visualizer::{ArcSegment, Vector3};
use gcodekit5_visualizer::{
    CommandProcessor, CommentProcessor, FileStreamReader, GCodeCommand, GcodeCommand, GcodeParser,
    GcodeState, GcodeStreamReader, ProcessorPipeline, Visualizer,
};

fn parse(gcode: &str) -> Visualizer {
    let mut vis = Visualizer::new();
    vis.parse_gcode(gcode);
    vis
}

fn approx(a: f32, b: f32) -> bool {
    (a - b).abs() < 1e-3
}

#[test]
fn test_words_strip_paren_comment_keeps_following_words() {
    assert_eq!(strip_comments("G1 (move) X10").trim(), "G1   X10".trim());
    let words = parse_line("g1(c)x-1.5 ; Y99");
    assert_eq!(words.len(), 2);
    assert_eq!(words[1].letter, 'X');
    assert_eq!(words[1].value, -1.5);
}

#[test]
fn test_parser_multiple_g_words_on_one_line() {
    let mut parser = GcodeParser::new();
    parser.parse("G90 G1 X10 F500").expect("parse");
    let state = parser.get_state();
    assert_eq!(state.motion_mode, 1);
    assert_eq!(state.distance_mode, 90);
    assert_eq!(state.feed_rate, 500.0);

    parser.parse("G1X5F1200").expect("parse");
    assert_eq!(parser.get_state().feed_rate, 1200.0);
}

#[test]
fn test_parser_paren_comment_mid_line_keeps_axis_words() {
    let mut parser = GcodeParser::new();
    let cmd = parser.parse("G1 (move) X10").expect("parse");
    assert!(cmd.command.contains("X10"));
}

#[test]
fn test_pipeline_updates_motion_mode_for_short_codes() {
    let pipeline = ProcessorPipeline::new();
    let mut state = GcodeState::default();
    pipeline
        .process_commands(&[GcodeCommand::new("G1 X1 F100")], &mut state)
        .expect("process");
    assert_eq!(state.motion_mode, 1);
}

#[test]
fn test_visualizer_g90_prefix_updates_motion_mode() {
    let vis = parse("G90 G0 X10\nG1 Y10");
    match &vis.commands()[0] {
        GCodeCommand::Move { rapid, to, .. } => {
            assert!(*rapid);
            assert!(approx(to.x, 10.0));
        }
        other => panic!("unexpected {:?}", other),
    }
    assert!(matches!(
        vis.commands()[1],
        GCodeCommand::Move { rapid: false, .. }
    ));
}

#[test]
fn test_visualizer_incremental_and_inch_modes() {
    let vis = parse("G91\nG1 X10\nX10");
    assert!(approx(vis.current_pos.x, 20.0));

    let vis = parse("G20 G1 X1");
    assert!(approx(vis.current_pos.x, 25.4));
}

#[test]
fn test_visualizer_paren_comment_axis_ignored() {
    let vis = parse("G0 X10 (to X99)");
    assert!(approx(vis.current_pos.x, 10.0));
}

#[test]
fn test_visualizer_arc_with_only_i_is_arc() {
    let vis = parse("G1 X0 Y0\nG2 X10 Y0 I5");
    let arc = vis
        .commands()
        .iter()
        .find_map(|c| match c {
            GCodeCommand::Arc { center, .. } => Some(*center),
            _ => None,
        })
        .expect("arc command");
    assert!(approx(arc.x, 5.0) && approx(arc.y, 0.0));
}

#[test]
fn test_visualizer_full_circle_is_split_and_bounded() {
    let vis = parse("G0 X10 Y0\nG2 X10 Y0 I-10 J0");
    let arcs: Vec<_> = vis
        .commands()
        .iter()
        .filter(|c| matches!(c, GCodeCommand::Arc { .. }))
        .collect();
    assert_eq!(arcs.len(), 2);
    for arc in arcs {
        if let GCodeCommand::Arc { from, to, .. } = arc {
            assert!(!(approx(from.x, to.x) && approx(from.y, to.y)));
        }
    }
    let (min_x, max_x, min_y, max_y, _, _) = vis.get_cutting_bounds().expect("bounds");
    assert!(approx(min_x, -10.0) && approx(max_x, 10.0));
    assert!(approx(min_y, -10.0) && approx(max_y, 10.0));
}

#[test]
fn test_visualizer_negative_r_selects_long_arc() {
    let vis = parse("G0 X0 Y0\nG2 X10 Y0 R10\nG0 X0 Y0\nG2 X10 Y0 R-10");
    let centers: Vec<_> = vis
        .commands()
        .iter()
        .filter_map(|c| match c {
            GCodeCommand::Arc { center, .. } => Some(*center),
            _ => None,
        })
        .collect();
    assert!(approx(centers[0].x, 5.0) && approx(centers[0].y, -8.660));
    assert!(approx(centers[1].x, 5.0) && approx(centers[1].y, 8.660));
}

#[test]
fn test_visualizer_dwell_and_g92_do_not_move() {
    let vis = parse("G1 X5 F100\nG4 P1.5\nG4 X2\nG92 X0 Y0");
    let moves = vis
        .commands()
        .iter()
        .filter(|c| matches!(c, GCodeCommand::Move { .. }))
        .count();
    assert_eq!(moves, 1);
    assert!(vis
        .commands()
        .iter()
        .any(|c| matches!(c, GCodeCommand::Dwell { duration, .. } if approx(*duration, 1.5))));
    assert!(approx(vis.current_pos.x, 0.0));
}

#[test]
fn test_visualizer_g18_arc_is_tessellated() {
    let vis = parse("G18\nG0 X0 Z0\nG2 X10 Z0 I5 K0");
    assert!(!vis
        .commands()
        .iter()
        .any(|c| matches!(c, GCodeCommand::Arc { .. })));
    assert!(vis.commands().len() > 3);
    assert!(approx(vis.current_pos.x, 10.0));
}

#[test]
fn test_visualizer_cutting_bounds_include_cut_start() {
    let vis = parse("G0 X10 Y10\nG1 X20 Y10");
    let (min_x, max_x, ..) = vis.get_cutting_bounds().expect("bounds");
    assert!(approx(min_x, 10.0) && approx(max_x, 20.0));
}

#[test]
fn test_arc_segment_full_circle_length() {
    let p = Vector3::new(10.0, 0.0, 0.0);
    let arc = ArcSegment::new(p, p, Vector3::new(0.0, 0.0, 0.0), true);
    assert!((arc.length() - std::f32::consts::TAU * 10.0).abs() < 1e-2);
}

#[test]
fn test_comment_processor_removes_comment_text() {
    let out = CommentProcessor::new()
        .process(
            &GcodeCommand::new("G1 X10 (move fast) Y5"),
            &GcodeState::default(),
        )
        .expect("process");
    assert!(!out[0].command.contains("move"));
    assert!(out[0].command.contains("Y5"));
}

#[test]
fn test_arc_expander_does_not_duplicate_arcs() {
    let processor: Arc<dyn CommandProcessor> = Arc::new(ArcExpander::new());
    let out = processor
        .process(
            &GcodeCommand::new("G02 X10 Y0 I5 J0"),
            &GcodeState::default(),
        )
        .expect("process");
    assert_eq!(out.len(), 1);
}

#[test]
fn test_file_stream_seek_mid_file_not_eof() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("s.nc");
    fs::write(&path, "L1\nL2\nL3\n").expect("write");
    let mut reader = FileStreamReader::new(&path).expect("reader");
    reader.seek_to_line(2).expect("seek");
    assert!(!reader.is_eof());
    assert_eq!(
        reader.read_line().expect("read").as_deref().map(str::trim),
        Some("L3")
    );
}

#[test]
fn test_arc_segment_short_arc_at_large_radius_is_not_a_full_circle() {
    // Clockwise from (10000, 0) to (10000, -0.005) about the origin: a ~0.005 mm arc whose
    // angle is tiny. It must not be mistaken for a full circle (~62832 mm).
    let arc = ArcSegment::new(
        Vector3::new(10000.0, 0.0, 0.0),
        Vector3::new(10000.0, -0.005, 0.0),
        Vector3::new(0.0, 0.0, 0.0),
        true,
    );
    assert!(arc.length() < 1.0, "length {}", arc.length());
}
