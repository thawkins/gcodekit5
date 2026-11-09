//! Regression tests for G-code parsing, arc handling, bounds and statistics.

use std::fs;
use std::sync::Arc;

use gcodekit5_visualizer::gcode::processors::ArcExpander;
use gcodekit5_visualizer::gcode::words::{parse_line, strip_comments};
use gcodekit5_visualizer::visualizer::{ArcSegment, Vector3};
use gcodekit5_visualizer::{
    AdvancedProber, BackupManager, CommandProcessor, CommentProcessor, FileProcessingPipeline,
    FileStreamReader, GCodeCommand, GcodeCommand, GcodeFileReader, GcodeParser, GcodeState,
    GcodeStreamReader, ProcessorPipeline, Visualizer,
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
fn test_file_statistics_modal_units_and_time() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("stats.nc");
    fs::write(
        &path,
        "(header)\nG21 G90\nM3 S1000\nG0 X10\nG1 X20 F600\nG92 X0\nG91 G1 X10\nG2 X10 Y0 I5\n",
    )
    .expect("write");

    let mut pipeline = FileProcessingPipeline::new();
    let result = pipeline.process_file(&path).expect("process");
    let stats = &result.statistics;
    assert_eq!(stats.rapid_moves, 1);
    assert_eq!(stats.linear_moves, 2);
    assert_eq!(stats.arc_moves, 1);
    assert_eq!(stats.comment_lines, 1);
    assert_eq!(result.original_lines, 8);
    assert!(stats.command_counts.contains_key("M3"));
    // 10 + 10 + 10 straight, plus a half circle of radius 5
    let expected = 30.0 + std::f32::consts::PI * 5.0;
    assert!((stats.total_distance - expected).abs() < 1e-2);
    // 0.2 s rapid + 20 mm / 600 mm/min (2 s) + 15.7 mm / 600 mm/min (1.57 s)
    assert_eq!(stats.estimated_time, 4);
}

#[test]
fn test_file_validation_does_not_count_g21_g91_as_moves() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("v.nc");
    fs::write(&path, "G21 G90\nG91 G1 X1\nG92 X0\n").expect("write");
    let validation = GcodeFileReader::new(&path)
        .expect("reader")
        .validate()
        .expect("validate");
    assert_eq!(validation.linear_moves, 1);
    assert_eq!(validation.rapid_moves, 0);
    assert_eq!(validation.arc_moves, 0);
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
fn test_backup_cleanup_keeps_newest() {
    let dir = tempfile::tempdir().expect("tempdir");
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("time")
        .as_secs();
    for age in [1u64, 2, 3, 10_000_000] {
        fs::write(dir.path().join((now - age).to_string()), "x").expect("write");
    }
    let mut manager = BackupManager::new(dir.path());
    manager.set_max_backups(2);
    manager.cleanup().expect("cleanup");
    let mut remaining: Vec<u64> = manager
        .list_backups()
        .expect("list")
        .iter()
        .map(|b| b.timestamp)
        .collect();
    remaining.sort_unstable();
    assert_eq!(remaining, vec![now - 2, now - 1]);
}

#[test]
fn test_probe_sequence_retracts_between_points() {
    let mut prober = AdvancedProber::new();
    prober.add_probe_point(0.0, 0.0);
    prober.add_probe_point(10.0, 10.0);
    let seq = prober.generate_probe_sequence();
    let probe = seq.find("G38.2").expect("probe");
    let next_xy = seq.find("G0 X10").expect("second point");
    assert!(seq[probe..next_xy].contains("G0 Z"));
}
