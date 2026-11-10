//! Regression tests for the optional homing preamble and GRBL `$` system commands.
//!
//! - After `$H` the generators keep every G54 offset (no `G10`), so work Z0 is not machine
//!   Z0 and X/Y run from the user's work zero, inside the travel (issue #70). Every Z
//!   travel/retract move after homing must be a `G53` machine-coordinate move at Z <= 0, so
//!   it can never rise above the homed position whatever the Z offset is (simulated here with
//!   G54 Z offsets of +10 mm, 0 and -40 mm). This covers the tabbed box, bitmap
//!   engraver, spoilboard surfacing, spoilboard grid and drill press.
//! - With homing off no generator emits `$H`, `G10` or `G53`.
//! - The validator, stats calculator and optimizer must treat GRBL system commands (`$H`,
//!   `$X`, `$$`, `$J=...`) as commands, not as G-code words with unparsable values.

use gcodekit5_camtools::comment_processor::{system_command, SystemCommand};
use gcodekit5_camtools::drill_press::{DrillPressGenerator, DrillPressParameters};
use gcodekit5_camtools::job_preamble::{travel_z_line, HOMED_TRAVEL_Z_MACHINE};
use gcodekit5_camtools::laser_engraver::{BitmapImageEngraver, EngravingParameters};
use gcodekit5_camtools::optimizer::GCodeOptimizer;
use gcodekit5_camtools::spoilboard_grid::{SpoilboardGridGenerator, SpoilboardGridParameters};
use gcodekit5_camtools::spoilboard_surfacing::{
    SpoilboardSurfacingGenerator, SpoilboardSurfacingParameters,
};
use gcodekit5_camtools::stats::StatsCalculator;
use gcodekit5_camtools::tabbed_box::{BoxParameters, TabbedBoxMaker};
use gcodekit5_camtools::validator::GCodeValidator;
use image::{DynamicImage, GrayImage, Luma};

/// One Z move: its line, whether it is a `G53` machine-coordinate move, and its Z value.
struct ZMove<'a> {
    line: &'a str,
    machine: bool,
    z: f32,
}

/// Every line that commands Z, in program order.
fn z_moves(gcode: &str) -> Vec<ZMove<'_>> {
    gcode
        .lines()
        .filter_map(|line| {
            let code = line.split(';').next().unwrap_or_default();
            let tokens: Vec<&str> = code.split_whitespace().collect();
            let z = tokens
                .iter()
                .find(|w| w.starts_with('Z'))
                .map(|w| w[1..].parse::<f32>().expect("numeric Z word"))?;
            Some(ZMove {
                line,
                machine: tokens.contains(&"G53"),
                z,
            })
        })
        .collect()
}

/// Machine Z reached by a move, given the G54 Z offset (machine Z of work Z0).
fn machine_z(m: &ZMove<'_>, g54_z_offset: f32) -> f32 {
    if m.machine {
        m.z
    } else {
        m.z + g54_z_offset
    }
}

/// `true` for a cutting / focus height move (measured from the work Z0 the user set): a feed
/// move (`G1`/`G2`/`G3`), a laser pass height, or a drill press in-hole chip-clearing rapid.
fn is_cut_height(line: &str) -> bool {
    let first = line.split_whitespace().next().unwrap_or_default();
    matches!(first, "G1" | "G2" | "G3")
        || line.contains("Plunge")
        || line.contains("pass depth")
        || line.contains("pass height")
        || line.contains("clear chips")
        || line.contains("above last cut")
}

/// Check the homing guarantee for one program and several G54 Z offsets.
fn assert_homed_z_safe(gcode: &str) {
    assert!(gcode.contains("$H"), "homing preamble missing:\n{gcode}");
    let moves = z_moves(gcode);
    let first_cut = moves
        .iter()
        .position(|m| is_cut_height(m.line))
        .expect("program has a plunge / pass-height move");

    for offset in [10.0_f32, 0.0, -40.0] {
        for m in &moves[..first_cut] {
            assert!(m.machine, "pre-plunge Z move is not G53: {}", m.line);
            let z = machine_z(m, offset);
            assert!(
                z <= 0.0,
                "G54 Z offset {offset}: '{}' reaches machine Z{z}",
                m.line
            );
        }
    }

    // Every non-cutting Z move (travel, retract, end of job) is a G53 move at machine Z <= 0.
    for m in moves.iter().filter(|m| !is_cut_height(m.line)) {
        assert!(
            m.machine,
            "travel/retract is not G53 after homing: {}",
            m.line
        );
        assert!(m.z <= 0.0, "travel above machine Z0: {}", m.line);
    }
}

fn tabbed_box(home: bool, passes: i32) -> String {
    let mut maker = TabbedBoxMaker::new(BoxParameters {
        home_before_job: home,
        laser_passes: passes,
        ..Default::default()
    })
    .expect("box setup");
    maker.generate().expect("box generate");
    maker.to_gcode()
}

fn engraver(home: bool) -> String {
    let params = EngravingParameters {
        width_mm: 0.4,
        pixels_per_mm: 10.0,
        home_before_job: home,
        ..Default::default()
    };
    BitmapImageEngraver::from_image(
        DynamicImage::ImageLuma8(GrayImage::from_pixel(4, 4, Luma([0]))),
        params,
    )
    .expect("engraver setup")
    .generate_gcode()
    .expect("gcode")
}

fn surfacing(home: bool) -> String {
    SpoilboardSurfacingGenerator::new(SpoilboardSurfacingParameters {
        width: 60.0,
        height: 40.0,
        tool_diameter: 20.0,
        feed_rate: 1000.0,
        spindle_speed: 10000.0,
        cut_depth: 0.5,
        stepover_percent: 50.0,
        safe_z: 5.0,
        home_before_job: home,
    })
    .generate()
    .expect("surfacing")
}

fn grid(home: bool) -> String {
    SpoilboardGridGenerator::new(SpoilboardGridParameters {
        width: 20.0,
        height: 20.0,
        grid_spacing: 10.0,
        feed_rate: 1000.0,
        laser_power: 100.0,
        laser_mode: "M4".to_string(),
        home_before_job: home,
    })
    .generate()
    .expect("grid")
}

/// Drill press program; `helical` picks a hole larger than the tool.
fn drill(home: bool, helical: bool) -> String {
    DrillPressGenerator::new(DrillPressParameters {
        hole_diameter: if helical { 12.0 } else { 6.0 },
        tool_diameter: 6.0,
        top_z: 0.0,
        bottom_z: -6.0,
        peck_depth: 2.0,
        plunge_rate: 100.0,
        feed_rate: 500.0,
        spindle_speed: 10000.0,
        safe_z: 5.0,
        x: 10.0,
        y: 20.0,
        home_before_job: home,
    })
    .generate()
    .expect("drill")
}

/// Every generator's output with homing on or off.
fn all_programs(home: bool) -> Vec<(&'static str, String)> {
    vec![
        ("tabbed box", tabbed_box(home, 2)),
        ("engraver", engraver(home)),
        ("surfacing", surfacing(home)),
        ("grid", grid(home)),
        ("drill", drill(home, false)),
        ("drill helical", drill(home, true)),
    ]
}

fn validate_clean(gcode: &str) {
    let lines: Vec<String> = gcode.lines().map(str::to_string).collect();
    if let Err(errors) = GCodeValidator::default().validate(&lines) {
        let report: Vec<String> = errors
            .iter()
            .map(|e| format!("line {}: {} ('{}')", e.line + 1, e.message, lines[e.line]))
            .collect();
        panic!("validation errors:\n{}", report.join("\n"));
    }
}

fn validate_one(line: &str) -> bool {
    GCodeValidator::default()
        .validate(&[line.to_string()])
        .is_ok()
}

// ---- Homing: Z travel in machine coordinates ----

#[test]
fn test_travel_z_line_homed_uses_g53_below_machine_zero() {
    let homed = travel_z_line(true, 5.0, "Retract");
    assert!(
        homed.starts_with(&format!("G53 G0 Z{HOMED_TRAVEL_Z_MACHINE:.2} ")),
        "{homed}"
    );
    assert!(homed.starts_with("G53 G0 Z-1.00 "), "{homed}");
    let free = travel_z_line(false, 5.0, "Retract");
    assert!(free.starts_with("G0 Z5.00 "), "{free}");
    assert!(travel_z_line(false, -3.0, "Retract").starts_with("G0 Z0.00 "));
}

#[test]
fn test_tabbed_box_homed_travel_never_above_homed_position() {
    for passes in [1, 3] {
        let gcode = tabbed_box(true, passes);
        assert_homed_z_safe(&gcode);
        assert!(
            gcode.lines().any(|l| l.starts_with("G53 G0 Z-1.00")),
            "final retract missing:\n{gcode}"
        );
    }
}

#[test]
fn test_engraver_homed_never_moves_z() {
    let gcode = engraver(true);
    assert!(gcode.contains("$H"));
    assert!(z_moves(&gcode).is_empty(), "{gcode}");
}

#[test]
fn test_spoilboard_surfacing_homed_travel_never_above_homed_position() {
    let gcode = surfacing(true);
    assert_homed_z_safe(&gcode);
    // No offset is rewritten: the user's G54 X/Y zero and the spoilboard surface (Z work
    // offset) are both kept.
    assert!(!gcode.lines().any(|l| l.starts_with("G10")), "{gcode}");
    // Z is cleared before the first X/Y move.
    let first_z = gcode
        .lines()
        .position(|l| l.starts_with("G53 G0 Z"))
        .expect("travel Z");
    let first_xy = gcode
        .lines()
        .position(|l| l.starts_with("G0 X"))
        .expect("XY move");
    assert!(first_z < first_xy, "{gcode}");
}

#[test]
fn test_spoilboard_grid_homed_never_moves_z() {
    let gcode = grid(true);
    assert!(gcode.contains("$H"));
    assert!(z_moves(&gcode).is_empty(), "{gcode}");
}

#[test]
fn test_drill_press_homed_travel_never_above_homed_position() {
    for helical in [false, true] {
        assert_homed_z_safe(&drill(true, helical));
    }
}

#[test]
fn test_no_homing_preamble_without_homing() {
    for (name, gcode) in all_programs(false) {
        assert!(!gcode.contains("$H"), "{name}: $H without homing");
        assert!(
            !gcode.lines().any(|l| l.starts_with("G10")),
            "{name}: G10 without homing"
        );
        assert!(!gcode.contains("G53"), "{name}: G53 without homing");
    }
    // Without homing, surfacing still travels at a positive work safe height.
    assert!(z_moves(&surfacing(false))
        .iter()
        .any(|m| !m.machine && m.z > 0.0));
}

#[test]
fn test_homed_programs_keep_g54_offsets() {
    // Clearing G54 X/Y after `$H` put work zero at the homing switches, outside GRBL's
    // negative travel range (issue #70). Every homed program must leave G54 untouched.
    for (name, gcode) in all_programs(true) {
        assert!(gcode.contains("$H"), "{name}: no $H");
        assert!(
            !gcode.lines().any(|l| l.starts_with("G10")),
            "{name}: homed program rewrites a work offset"
        );
        assert!(
            gcode.lines().any(|l| l.starts_with("G54")),
            "{name}: no G54"
        );
    }
}

// ---- GRBL `$` commands ----

#[test]
fn test_system_command_classification() {
    assert_eq!(system_command("G0 X1"), None);
    assert_eq!(
        system_command("$H ; Home all axes"),
        Some(SystemCommand::Known("$H".to_string()))
    );
    assert_eq!(
        system_command(" $j = g91 x10 f500"),
        Some(SystemCommand::Jog("G91X10F500".to_string()))
    );
    assert_eq!(
        system_command("$FOO"),
        Some(SystemCommand::Unknown("$FOO".to_string()))
    );
}

#[test]
fn test_validator_accepts_grbl_system_commands() {
    for cmd in [
        "$",
        "$$",
        "$#",
        "$G",
        "$I",
        "$N",
        "$C",
        "$X",
        "$H",
        "$HZ",
        "$SLP",
        "$RST=$",
        "$RST=#",
        "$RST=*",
        "$100=80.0",
        "$32=1",
        "$N0=G20",
        "$I=my machine",
        "$H ; Home all axes",
        "$J=G91 X10 F500",
        "$J=G21 G53 Z-1 F300",
    ] {
        assert!(validate_one(cmd), "'{cmd}' rejected");
    }
}

#[test]
fn test_validator_rejects_bad_system_commands_and_jogs() {
    for cmd in [
        "$J=G91 X F500",
        "$J=G91 X10",
        "$J=X99999 F500",
        "$J=G91 X10 F0",
        "$J=",
        "$FOO",
        "$100=abc",
    ] {
        assert!(!validate_one(cmd), "'{cmd}' accepted");
    }
}

#[test]
fn test_validator_accepts_homed_generator_output() {
    for (name, gcode) in all_programs(true) {
        assert!(gcode.contains("$H"), "{name}: homing preamble missing");
        validate_clean(&gcode);
    }
}

#[test]
fn test_stats_ignore_system_command_letters() {
    let lines: Vec<String> = ["$H", "$X", "$J=G91 X1000 F100", "G0 X5 Y6", "G1 X7 Y8 F100"]
        .iter()
        .map(|s| s.to_string())
        .collect();
    let stats = StatsCalculator::calculate(&lines);
    assert_eq!(stats.total_commands, 5);
    assert_eq!(stats.rapid_count, 1);
    assert_eq!(stats.linear_count, 1);
    assert_eq!(stats.max_x, 7.0);
    assert_eq!(stats.min_x, 5.0);
}

#[test]
fn test_optimizer_keeps_system_commands() {
    let lines: Vec<String> = ["M5", "$H", "M5", "$X", "T1", "$T", "T1"]
        .iter()
        .map(|s| s.to_string())
        .collect();
    let out = GCodeOptimizer::optimize(&lines);
    assert_eq!(out, vec!["M5", "$H", "M5", "$X", "T1", "$T"]);
}
