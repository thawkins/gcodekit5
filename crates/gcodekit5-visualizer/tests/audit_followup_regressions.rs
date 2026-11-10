//! Regression tests for the visualizer audit follow-up fixes: default motion
//! mode, laser on/off tracking, content-replacement detection, shared arc
//! maths, stock removal Z convention, processors, file decoding, stream errors,
//! backup naming and UTC timestamps.

use std::fs;
use std::sync::Arc;

use gcodekit5_visualizer::gcode::arc::arc_sweep;
use gcodekit5_visualizer::gcode::processors::{ArcExpander, PatternRemover};
use gcodekit5_visualizer::utils::phase7::{
    CalibrationStepType, CalibrationWizard, CommunicationDiagnostics, DiagnosticReport,
};
use gcodekit5_visualizer::visualizer::{
    detect_stock_zero_reference, is_content_replacement, StockZeroReference, ToolpathCache,
};
use gcodekit5_visualizer::{
    BackupManager, CommandProcessor, DecimalProcessor, FileEncoding, FileStreamReader,
    GCodeCommand, GcodeCommand, GcodeFileReader, GcodeState, GcodeStreamReader, Point3D,
    StockSimulator3D, ToolpathSegment, ToolpathSegmentType, Visualizer,
};

fn parse(gcode: &str) -> Visualizer {
    let mut vis = Visualizer::new();
    vis.parse_gcode(gcode);
    vis
}

fn move_intensities(vis: &Visualizer) -> Vec<Option<f32>> {
    vis.commands()
        .iter()
        .filter_map(|c| match c {
            GCodeCommand::Move { intensity, .. } | GCodeCommand::Arc { intensity, .. } => {
                Some(*intensity)
            }
            GCodeCommand::Dwell { .. } => None,
        })
        .collect()
}

fn process(processor: &dyn CommandProcessor, line: &str) -> Vec<String> {
    processor
        .process(&GcodeCommand::new(line), &GcodeState::default())
        .expect("process")
        .into_iter()
        .map(|c| c.command)
        .collect()
}

fn linear(start: (f32, f32, f32), end: (f32, f32, f32)) -> ToolpathSegment {
    ToolpathSegment {
        segment_type: ToolpathSegmentType::LinearMove,
        start,
        end,
        center: None,
        feed_rate: 100.0,
        spindle_speed: 1000.0,
    }
}

// ---------------------------------------------------------------- motion mode

#[test]
fn test_visualizer_axis_words_before_first_motion_word_are_rapid() {
    let vis = parse("X10 Y5\nG1 X20");
    assert_eq!(vis.commands().len(), 2);
    match &vis.commands()[0] {
        GCodeCommand::Move { rapid, to, .. } => {
            assert!(*rapid, "GRBL powers up in G0");
            assert_eq!((to.x, to.y), (10.0, 5.0));
        }
        other => panic!("unexpected {:?}", other),
    }
}

#[test]
fn test_visualizer_g80_still_cancels_motion() {
    let vis = parse("G1 X1\nG80\nX5");
    assert_eq!(vis.commands().len(), 1);
}

// ---------------------------------------------------------------- laser state

#[test]
fn test_visualizer_m5_moves_have_no_intensity() {
    let vis = parse("M3 S500\nG1 X10\nM5\nG1 X20\nG2 X30 I5\nM4 S200\nG1 X40");
    assert_eq!(
        move_intensities(&vis),
        vec![Some(500.0), None, None, Some(200.0)]
    );
}

#[test]
fn test_visualizer_without_spindle_words_keeps_intensity() {
    let vis = parse("S100\nG1 X10");
    assert_eq!(move_intensities(&vis), vec![Some(100.0)]);
}

// -------------------------------------------------------- content replacement

#[test]
fn test_visualizer_content_replacement_detects_edits_and_new_files() {
    let program = "G21\nG90\nG0 X0 Y0\nG1 X10 Y10 F500\nG1 X20 Y0\nM5\n";
    let edited = "G21\nG90\nG0 X0 Y0\nG1 X10 Y15 F500\nG1 X20 Y0\nM5\n";
    let other = "; other file\nG0 X100 Y100\nG1 X200 Y150 F900\nG1 X300 Y100\n";

    assert!(is_content_replacement("", program), "first load fits");
    assert!(!is_content_replacement(program, edited), "edit keeps view");
    assert!(!is_content_replacement(
        program,
        &program[..program.len() - 3]
    ));
    assert!(is_content_replacement(program, other), "new file fits");
    assert!(
        !is_content_replacement(program, ""),
        "cleared buffer keeps view"
    );
}

// --------------------------------------------------------------- arc maths

#[test]
fn test_arc_sweep_quarter_and_full_circle() {
    let ccw = arc_sweep((0.0, 0.0), (1.0, 0.0), (0.0, 1.0), false).expect("arc");
    assert!((ccw.sweep - std::f64::consts::FRAC_PI_2).abs() < 1e-9);
    let cw = arc_sweep((0.0, 0.0), (1.0, 0.0), (0.0, 1.0), true).expect("arc");
    assert!((cw.sweep + 3.0 * std::f64::consts::FRAC_PI_2).abs() < 1e-9);

    let full = arc_sweep((0.0, 0.0), (1.0, 0.0), (1.0, 0.0), true).expect("arc");
    assert!((full.sweep + std::f64::consts::TAU).abs() < 1e-9);
    let (x, y) = full.point_at((0.0, 0.0), 0.5);
    assert!((x + 1.0).abs() < 1e-9 && y.abs() < 1e-9);

    assert_eq!(full.segment_count(std::f64::consts::PI / 36.0, 4, 128), 72);
    assert!(arc_sweep((0.0, 0.0), (0.0, 0.0), (1.0, 0.0), false).is_none());
}

// ----------------------------------------------------------- toolpath cache

#[test]
fn test_toolpath_cache_holds_commands_only() {
    let mut cache = ToolpathCache::new();
    assert!(cache.is_empty());
    assert!(cache.needs_update(1));
    let p = Point3D::new(0.0, 0.0, 0.0);
    cache.update(
        7,
        vec![GCodeCommand::Dwell {
            pos: p,
            duration: 1.0,
        }],
    );
    assert_eq!(cache.len(), 1);
    assert!(!cache.needs_update(7));
    assert!(cache.needs_update(8));
}

// ------------------------------------------------------------ stock removal

#[test]
fn test_stock_removal_top_at_z0_removes_below_zero() {
    // 10 x 10 x 5 mm stock, 0.5 mm voxels -> 20 x 20 x 10, top at work Z0.
    let mut sim = StockSimulator3D::new(10.0, 10.0, 5.0, 0.5, 1.0);
    assert_eq!(sim.stock_top_z(), 0.0);
    sim.simulate_toolpath(&[linear((2.0, 5.0, -2.0), (8.0, 5.0, -2.0))]);
    let grid = sim.get_grid();
    // Work Z -2 is grid Z 3 mm (voxel 6): everything from there up is cut away.
    assert_eq!(grid.get_at_position(10, 10, 9), 0);
    assert_eq!(grid.get_at_position(10, 10, 6), 0);
    assert_eq!(grid.get_at_position(10, 10, 5), 255);
    // Away from the cut the stock is untouched.
    assert_eq!(grid.get_at_position(10, 2, 9), 255);
}

#[test]
fn test_stock_removal_above_top_removes_nothing() {
    let mut sim = StockSimulator3D::new(10.0, 10.0, 5.0, 0.5, 1.0);
    sim.simulate_toolpath(&[linear((2.0, 5.0, 0.0), (8.0, 5.0, 0.5))]);
    assert!(sim.get_grid().data().iter().all(|&v| v == 255));
}

#[test]
fn test_stock_removal_with_bottom_zero_stock_top() {
    let mut sim = StockSimulator3D::new(10.0, 10.0, 5.0, 0.5, 1.0).with_stock_top_z(5.0);
    sim.simulate_toolpath(&[linear((2.0, 5.0, 3.0), (8.0, 5.0, 3.0))]);
    let grid = sim.get_grid();
    assert_eq!(grid.get_at_position(10, 10, 6), 0);
    assert_eq!(grid.get_at_position(10, 10, 5), 255);
}

#[test]
fn test_stock_removal_mesh_is_in_work_coordinates() {
    let sim = StockSimulator3D::new(2.0, 2.0, 5.0, 1.0, 0.5);
    let mesh = sim.get_mesh();
    let zs: Vec<f32> = mesh.as_chunks::<10>().0.iter().map(|v| v[2]).collect();
    let min = zs.iter().copied().fold(f32::INFINITY, f32::min);
    let max = zs.iter().copied().fold(f32::NEG_INFINITY, f32::max);
    assert!((min + 5.0).abs() < 1e-4, "bottom at -thickness, got {min}");
    assert!(max.abs() < 1e-4, "top at Z0, got {max}");
}

/// Count the voxels removed from a fresh 10 x 10 x 10 mm stock (1 mm voxels).
fn removed_voxels(reference: StockZeroReference, toolpath: &[ToolpathSegment]) -> usize {
    let mut sim =
        StockSimulator3D::new(10.0, 10.0, 10.0, 1.0, 1.0).with_stock_zero_reference(reference);
    sim.simulate_toolpath(toolpath);
    sim.get_grid().data().iter().filter(|&&v| v == 0).count()
}

#[test]
fn test_stock_removal_zero_reference_defaults_to_top() {
    assert_eq!(StockZeroReference::default(), StockZeroReference::Top);
    assert_eq!(StockZeroReference::Top.stock_top_z(10.0), 0.0);
    assert_eq!(StockZeroReference::Bottom.stock_top_z(10.0), 10.0);
    for r in [StockZeroReference::Top, StockZeroReference::Bottom] {
        assert_eq!(StockZeroReference::from_index(r.index()), r);
    }
    let sim = StockSimulator3D::new(10.0, 10.0, 10.0, 1.0, 1.0)
        .with_stock_zero_reference(StockZeroReference::Bottom);
    assert_eq!(sim.stock_top_z(), 10.0);
}

#[test]
fn test_stock_removal_top_zero_feed_at_z0_removes_nothing() {
    // A top-zeroed program that only feeds at Z0 never enters the stock.
    let toolpath = [linear((2.0, 5.0, 0.0), (8.0, 5.0, 0.0))];
    assert_eq!(removed_voxels(StockZeroReference::Top, &toolpath), 0);
}

#[test]
fn test_stock_removal_top_zero_air_cut_removes_nothing() {
    // Air-cutting at Z2 over 10 mm stock zeroed on top removes nothing.
    let toolpath = [
        linear((2.0, 5.0, 2.0), (8.0, 5.0, 2.0)),
        linear((8.0, 5.0, 2.0), (8.0, 8.0, 2.0)),
    ];
    assert_eq!(removed_voxels(StockZeroReference::Top, &toolpath), 0);
}

#[test]
fn test_stock_removal_bottom_zero_cuts_between_zero_and_thickness() {
    // The same Z2 pass in a bottom-zeroed program cuts 8 mm deep.
    let toolpath = [linear((2.0, 5.0, 2.0), (8.0, 5.0, 2.0))];
    let mut sim = StockSimulator3D::new(10.0, 10.0, 10.0, 1.0, 1.0)
        .with_stock_zero_reference(StockZeroReference::Bottom);
    sim.simulate_toolpath(&toolpath);
    let grid = sim.get_grid();
    assert_eq!(grid.get_at_position(5, 5, 9), 0);
    assert_eq!(grid.get_at_position(5, 5, 2), 0);
    assert_eq!(grid.get_at_position(5, 5, 1), 255);
}

#[test]
fn test_stock_removal_detects_recorded_zero_reference() {
    let designer = "; Generated G-code from Designer tool\n; Tool diameter: 3.18mm\n\nG90\n";
    assert_eq!(
        detect_stock_zero_reference(designer),
        Some(StockZeroReference::Bottom)
    );
    let laser = "; Generated Laser G-code from Designer tool\nG90\n";
    assert_eq!(detect_stock_zero_reference(laser), None);
    assert_eq!(
        detect_stock_zero_reference("%\n(Stock Z0: Top)\nG0 Z5\n"),
        Some(StockZeroReference::Top)
    );
    assert_eq!(
        detect_stock_zero_reference("; stock z0: bottom\nG1 Z2 F100\n"),
        Some(StockZeroReference::Bottom)
    );
}

#[test]
fn test_stock_removal_detection_ignores_toolpath_z() {
    // No recorded convention: Z values (air cut / Z0 feed) must not be used to guess.
    assert_eq!(detect_stock_zero_reference("G0 Z5\nG1 X10 Z2 F100\n"), None);
    assert_eq!(detect_stock_zero_reference("G1 X10 Z0 F100\n"), None);
    assert_eq!(detect_stock_zero_reference(""), None);
    // Markers after the header are not trusted.
    assert_eq!(
        detect_stock_zero_reference("G0 Z5\n; Generated G-code from Designer tool\n"),
        None
    );
}

// --------------------------------------------------------------- processors

#[test]
fn test_pattern_remover_compiles_once_and_handles_invalid_pattern() {
    let remover = PatternRemover::new("^M3");
    assert!(remover.is_valid());
    assert!(process(&remover, "M3 S100").is_empty());
    assert_eq!(process(&remover, "G1 X1"), vec!["G1 X1"]);

    let invalid = PatternRemover::new("(");
    assert!(!invalid.is_valid());
    assert_eq!(process(&invalid, "M3 S100"), vec!["M3 S100"]);
}

#[test]
fn test_decimal_processor_leaves_comments_untouched() {
    let processor = DecimalProcessor::with_precision(2);
    assert_eq!(
        process(
            &processor,
            "g1 x1.23456 (Keep 1.23456 here) y2.5 ; note 3.14159"
        ),
        vec!["G1 X1.23 (Keep 1.23456 here) Y2.5 ; note 3.14159"]
    );
    assert_eq!(
        process(&processor, "G0 X1.005 (unclosed 9.87654"),
        vec!["G0 X1 (unclosed 9.87654"]
    );
}

#[test]
fn test_arc_expander_description_matches_behaviour() {
    let expander: Arc<dyn CommandProcessor> = Arc::new(ArcExpander::new());
    assert!(!expander.description().starts_with("Expands"));
    assert_eq!(
        process(expander.as_ref(), "G2 X10 Y0 I5 J0"),
        vec!["G2 X10 Y0 I5 J0"]
    );
}

// ------------------------------------------------------------- input files

#[test]
fn test_file_reader_decodes_non_utf8_files() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("latin1.nc");
    fs::write(&path, b"G21 (caf\xe9)\r\nG1 X1\n").expect("write");
    let reader = GcodeFileReader::new(&path).expect("reader");

    let mut lines = Vec::new();
    let stats = reader
        .read_lines(|line| {
            lines.push(line.to_string());
            Ok(())
        })
        .expect("read_lines");
    assert_eq!(lines, vec!["G21 (caf\u{e9})", "G1 X1"]);
    assert_eq!(stats.encoding, FileEncoding::Latin1);
    assert_eq!(stats.lines_read, 2);

    let all = reader.read_all().expect("read_all");
    assert!(all.contains("caf\u{e9}"));
    let (limited, limited_stats) = reader.read_lines_limited(1).expect("limited");
    assert_eq!(limited.len(), 1);
    assert_eq!(limited_stats.encoding, FileEncoding::Latin1);
}

#[test]
fn test_file_reader_strips_utf8_bom() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("bom.nc");
    fs::write(&path, b"\xEF\xBB\xBFG0 X1\n").expect("write");
    let reader = GcodeFileReader::new(&path).expect("reader");
    let mut first = String::new();
    let stats = reader
        .read_lines(|line| {
            if first.is_empty() {
                first = line.to_string();
            }
            Ok(())
        })
        .expect("read_lines");
    assert_eq!(first, "G0 X1");
    assert_eq!(stats.encoding, FileEncoding::Utf8);
}

#[test]
fn test_file_stream_read_error_is_not_eof() {
    // Reading a directory fails with EISDIR on Linux; that must surface as an
    // error, not be mistaken for the end of the file.
    let dir = tempfile::tempdir().expect("tempdir");
    if let Ok(mut reader) = FileStreamReader::new(dir.path()) {
        assert!(reader.read_line().is_err());
        assert!(!reader.is_eof());
    }
}

#[test]
fn test_file_stream_reads_non_utf8_lossily() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("s.nc");
    fs::write(&path, b"G1 (\xff)\nG0 X1\n").expect("write");
    let mut reader = FileStreamReader::new(&path).expect("reader");
    assert_eq!(reader.total_lines(), Some(2));
    let first = reader.read_line().expect("read").expect("line");
    assert!(first.starts_with("G1 ("));
    let second = reader.read_line().expect("read").expect("line");
    assert_eq!(second.trim(), "G0 X1");
    assert!(reader.read_line().expect("read").is_none());
    assert!(reader.is_eof());
}

#[test]
fn test_backup_same_second_does_not_overwrite() {
    let dir = tempfile::tempdir().expect("tempdir");
    let source = dir.path().join("prog.nc");
    let backups = dir.path().join("backups");
    let manager = BackupManager::new(&backups);

    fs::write(&source, "first").expect("write");
    let a = manager.backup(&source, "a").expect("backup a");
    fs::write(&source, "second").expect("write");
    let b = manager.backup(&source, "b").expect("backup b");
    fs::write(&source, "third").expect("write");
    let c = manager.backup(&source, "c").expect("backup c");

    assert_ne!(a.backup_path, b.backup_path);
    assert_ne!(b.backup_path, c.backup_path);
    assert_eq!(fs::read_to_string(&a.backup_path).expect("a"), "first");
    assert_eq!(fs::read_to_string(&b.backup_path).expect("b"), "second");
    assert_eq!(fs::read_to_string(&c.backup_path).expect("c"), "third");

    let listed = manager.list_backups().expect("list");
    assert_eq!(listed.len(), 3);
    assert_eq!(listed[0].id, c.id, "newest first");
}

// --------------------------------------------------------------- timestamps

#[test]
fn test_phase7_timestamps_are_utc() {
    let mut wizard = CalibrationWizard::new(CalibrationStepType::StepCalibration);
    wizard.record_measurement(10.0).expect("measure");
    let ts: chrono::DateTime<chrono::Utc> = wizard.results()[0].timestamp;
    assert!((chrono::Utc::now() - ts).num_seconds().abs() < 60);

    let report = DiagnosticReport::new(CommunicationDiagnostics::default());
    let _: chrono::DateTime<chrono::Utc> = report.timestamp;
    assert!(report.format_report().contains(" UTC"));
}

#[test]
fn test_stock_origin_explicit_marker_overrides_designer_header() {
    // An edited/re-zeroed designer program that records its own origin must keep it.
    let gcode = "; Generated G-code from Designer tool\n; Stock Z0: top\nG21\nG1 Z-1 F100\n";
    assert_eq!(
        detect_stock_zero_reference(gcode),
        Some(StockZeroReference::Top)
    );
    // Without a marker the designer title still means bottom-zero.
    let gcode = "; Generated G-code from Designer tool\nG21\n";
    assert_eq!(
        detect_stock_zero_reference(gcode),
        Some(StockZeroReference::Bottom)
    );
}

/// Lowest Z (mm) of any parsed move end point.
fn min_move_z(vis: &Visualizer) -> f32 {
    vis.commands()
        .iter()
        .filter_map(|c| match c {
            GCodeCommand::Move { to, .. } => Some(to.z),
            _ => None,
        })
        .fold(f32::INFINITY, f32::min)
}

#[test]
fn test_visualizer_g53_retract_is_not_plotted_at_work_z() {
    use gcodekit5_camtools::spoilboard_surfacing::{
        SpoilboardSurfacingGenerator, SpoilboardSurfacingParameters,
    };

    // A homed surfacing program retracts with `G53 G0 Z-1` (machine coordinates)
    // and cuts at work Z-0.5. The retract must not be drawn (or simulated) at work
    // Z-1, which would show a false cut 0.5 mm below the programmed depth.
    let gcode = SpoilboardSurfacingGenerator::new(SpoilboardSurfacingParameters {
        width: 60.0,
        height: 40.0,
        tool_diameter: 20.0,
        feed_rate: 1000.0,
        spindle_speed: 10000.0,
        cut_depth: 0.5,
        stepover_percent: 50.0,
        safe_z: 5.0,
        home_before_job: true,
    })
    .generate()
    .expect("surfacing program");
    assert!(
        gcode.contains("G53"),
        "homed program should use G53:\n{gcode}"
    );

    let vis = parse(&gcode);
    let min_z = min_move_z(&vis);
    assert!(
        (min_z - -0.5).abs() < 1e-3,
        "lowest plotted Z should be the cut depth -0.5, got {min_z}"
    );

    // G53 moves are drawn as travel, never as cuts.
    let vis = parse("G0 Z5\nG1 Z-2 F100\nG53 G0 Z-1\nG0 X10\n");
    let g53_move = vis
        .commands()
        .iter()
        .filter_map(|c| match c {
            GCodeCommand::Move {
                to,
                rapid,
                intensity,
                ..
            } => Some((*to, *rapid, *intensity)),
            _ => None,
        })
        .nth(2)
        .expect("third move is the G53 retract");
    assert!(g53_move.1, "G53 move should be rapid");
    assert!(g53_move.2.is_none(), "G53 move should carry no intensity");
    assert!(
        (g53_move.0.z - 5.0).abs() < 1e-3,
        "G53 retract should rise to the highest work Z seen (5), got {}",
        g53_move.0.z
    );
}

/// Runs a stock simulation on a worker thread and fails if it does not
/// finish within a few seconds (the old code looped ~1.8e19 times).
fn simulate_within_deadline(width: f32, height: f32, segments: Vec<ToolpathSegment>) {
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let mut sim = StockSimulator3D::new(width, height, 5.0, 0.5, 1.0);
        sim.simulate_toolpath(&segments);
        let _ = tx.send(());
    });
    assert!(
        rx.recv_timeout(std::time::Duration::from_secs(10)).is_ok(),
        "stock simulation did not finish"
    );
}

#[test]
fn test_stock_removal_cut_left_of_stock_finishes() {
    // Audit D-01: a cut at negative X/Y made the voxel range wrap to usize::MAX.
    simulate_within_deadline(
        10.0,
        10.0,
        vec![linear((-10.0, -10.0, -2.0), (-8.0, -10.0, -2.0))],
    );
}

#[test]
fn test_stock_removal_zero_size_stock_finishes() {
    simulate_within_deadline(0.0, 0.0, vec![linear((1.0, 1.0, -2.0), (2.0, 1.0, -2.0))]);
}

#[test]
fn test_stock_dimensions_are_checked_before_allocating() {
    // Audit E1-06: a full sheet needed ~15.7 GB and aborted the process.
    use gcodekit5_visualizer::VoxelGrid;
    assert!(VoxelGrid::check_dimensions(200.0, 200.0, 10.0, 0.19).is_ok());
    assert!(VoxelGrid::check_dimensions(1220.0, 2440.0, 18.0, 0.19).is_err());
    assert!(VoxelGrid::check_dimensions(0.0, 10.0, 10.0, 0.19).is_err());
    assert!(VoxelGrid::check_dimensions(f32::NAN, 10.0, 10.0, 0.19).is_err());
}

#[test]
fn test_stock_resolution_must_be_positive_and_finite() {
    // #89 review: a bad resolution gave zero cells and passed the check.
    use gcodekit5_visualizer::VoxelGrid;
    for resolution in [0.0, -0.19, f32::NAN, f32::INFINITY] {
        assert!(
            VoxelGrid::check_dimensions(200.0, 200.0, 10.0, resolution).is_err(),
            "{resolution}"
        );
    }
}

#[test]
fn test_probe_move_is_drawn_as_travel_not_a_cut() {
    // Audit D-02: G38.2 kept the previous G1 mode, so the probe and the
    // following modal line were simulated as feed cuts.
    let vis = parse("G1 X0 Y0 F100\nG38.2 Z-20 F100\nX10\n");
    let moves: Vec<bool> = vis
        .commands()
        .iter()
        .filter_map(|c| match c {
            GCodeCommand::Move { rapid, .. } => Some(*rapid),
            _ => None,
        })
        .collect();
    assert_eq!(moves.len(), 3, "{moves:?}");
    assert!(moves[1], "probe move must be travel");
    assert!(
        moves[2],
        "the modal line after G38.2 is still a probe, not a cut"
    );
}
