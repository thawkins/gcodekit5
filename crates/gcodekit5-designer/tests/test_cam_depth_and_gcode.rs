//! Regression tests for CAM depth handling and G-code output.
//!
//! Covers the shared pocket-depth rule for every shape type, step-down limited
//! ramped contours, the machine boundary-violation check, and per-segment laser
//! power changes.

use gcodekit5_core::Units;
use gcodekit5_designer::designer_state::MachineMode;
use gcodekit5_designer::gcode_gen::MachineLimits;
use gcodekit5_designer::model::{DesignRectangle, Point};
use gcodekit5_designer::shapes::OperationType;
use gcodekit5_designer::{
    DesignerState, Toolpath, ToolpathGenerator, ToolpathSegment, ToolpathSegmentType,
    ToolpathToGcode,
};

/// Returns the lowest Z word in a G-code program (comments ignored).
fn min_z(gcode: &str) -> f64 {
    gcode
        .lines()
        .map(|l| l.split(';').next().unwrap_or(""))
        .flat_map(|l| l.split_whitespace())
        .filter_map(|w| w.strip_prefix('Z'))
        .filter_map(|v| v.parse::<f64>().ok())
        .fold(f64::INFINITY, f64::min)
}

/// Generates CNC G-code for a single pocketed shape with the given depths.
fn pocket_gcode(add: impl FnOnce(&mut DesignerState) -> u64, pocket_depth: f64) -> String {
    let mut state = DesignerState::new();
    state.set_machine_mode(MachineMode::Cnc3D);
    let id = add(&mut state);
    let obj = state
        .canvas
        .get_shape_mut(id)
        .expect("shape was just added");
    obj.use_custom_values = true;
    obj.operation_type = OperationType::Pocket;
    obj.pocket_depth = pocket_depth;
    obj.start_depth = 1.0;
    obj.step_down = 1.0;
    obj.step_in = 1.0;
    state.generate_gcode(None)
}

#[test]
fn test_pocket_depth_rule_is_shared_by_rectangle_and_other_shapes() {
    // Stock top is at Z10; a 3 mm pocket must bottom out at Z7 for every shape type,
    // including rectangles (which used to ignore `pocket_depth`).
    let rect = pocket_gcode(|s| s.canvas.add_rectangle(0.0, 0.0, 30.0, 20.0), 3.0);
    let circle = pocket_gcode(|s| s.canvas.add_circle(Point::new(15.0, 15.0), 10.0), 3.0);
    assert!((min_z(&rect) - 7.0).abs() < 1e-6, "rectangle:\n{rect}");
    assert!((min_z(&circle) - 7.0).abs() < 1e-6, "circle:\n{circle}");
}

#[test]
fn test_pocket_depth_falls_back_to_start_depth() {
    // Without a pocket depth the start depth (1 mm) is used, for every shape.
    let rect = pocket_gcode(|s| s.canvas.add_rectangle(0.0, 0.0, 30.0, 20.0), 0.0);
    let circle = pocket_gcode(|s| s.canvas.add_circle(Point::new(15.0, 15.0), 10.0), 0.0);
    assert!((min_z(&rect) - 9.0).abs() < 1e-6, "rectangle:\n{rect}");
    assert!((min_z(&circle) - 9.0).abs() < 1e-6, "circle:\n{circle}");
}

#[test]
fn test_ramped_contour_descends_at_most_step_down_per_lap() {
    // 25 x 25 mm square: a 100 mm contour. At 3 degrees a full lap would drop ~5.2 mm.
    let rect = DesignRectangle::new(0.0, 0.0, 25.0, 25.0);
    let mut gen = ToolpathGenerator::new();
    gen.set_feed_rate(500.0);
    gen.set_start_depth(0.0);
    gen.set_cut_depth(3.0);
    gen.set_ramp_angle(3.0);
    let step_down = 1.0;
    let toolpaths = gen.generate_rectangle_contour(&rect, step_down);
    assert!(toolpaths.len() >= 4, "expected 3 ramp laps + cleanup");

    let mut previous_floor = 0.0_f64;
    for tp in &toolpaths {
        let cuts: Vec<&ToolpathSegment> = tp
            .segments
            .iter()
            .filter(|s| s.segment_type != ToolpathSegmentType::RapidMove)
            .collect();
        let top = cuts
            .iter()
            .filter_map(|s| s.start_z)
            .fold(f64::NEG_INFINITY, f64::max);
        let floor = cuts
            .iter()
            .filter_map(|s| s.z_depth)
            .fold(f64::INFINITY, f64::min);
        assert!(
            top - floor <= step_down + 1e-6,
            "lap descends {} mm (> step-down {step_down})",
            top - floor
        );
        assert!(floor <= previous_floor + 1e-9, "laps must not climb");
        previous_floor = floor;

        // Ramp slope never exceeds the requested angle.
        let max_slope = 3.0_f64.to_radians().tan() + 1e-6;
        for s in &cuts {
            if let (Some(z0), Some(z1)) = (s.start_z, s.z_depth) {
                let len = s.start.distance_to(&s.end);
                if len > 1e-9 {
                    assert!((z0 - z1).abs() / len <= max_slope, "segment too steep");
                }
            }
        }
    }

    // Final cleanup lap at full depth.
    let last = toolpaths.last().expect("toolpaths");
    assert!(last
        .segments
        .iter()
        .all(|s| s.z_depth == Some(-3.0) && s.start_z == Some(-3.0)));
}

fn limits() -> MachineLimits {
    MachineLimits {
        x_min: 0.0,
        x_max: 100.0,
        y_min: 0.0,
        y_max: 100.0,
    }
}

fn cut(z: f64, from: Point, to: Point) -> Toolpath {
    let mut tp = Toolpath::new(3.0, z);
    let mut s = ToolpathSegment::new(ToolpathSegmentType::LinearMove, from, to, 1000.0, 12000);
    s.start_z = Some(z);
    s.z_depth = Some(z);
    tp.add_segment(s);
    tp
}

#[test]
fn test_boundary_violation_ignores_normal_negative_z_cut() {
    let gen = ToolpathToGcode::new(Units::MM, 10.0).with_machine_limits(limits());
    let tp = cut(-3.0, Point::new(10.0, 10.0), Point::new(20.0, 20.0));
    assert!(!gen.has_boundary_violation(&tp));
    let mut whole = Toolpath::new(3.0, -7.0);
    whole.add_segment(ToolpathSegment::new(
        ToolpathSegmentType::LinearMove,
        Point::new(10.0, 10.0),
        Point::new(20.0, 20.0),
        1000.0,
        12000,
    ));
    assert!(!gen.has_boundary_violation(&whole));
}

#[test]
fn test_boundary_violation_checks_xy_limits() {
    let gen = ToolpathToGcode::new(Units::MM, 10.0).with_machine_limits(limits());
    let tp = cut(-1.0, Point::new(10.0, 10.0), Point::new(120.0, 20.0));
    assert!(gen.has_boundary_violation(&tp));
}

#[test]
fn test_boundary_violation_checks_z_against_configured_floor() {
    let gen = ToolpathToGcode::new(Units::MM, 10.0)
        .with_machine_limits(limits())
        .with_z_min_limit(0.0);
    assert!(!gen.has_boundary_violation(&cut(2.0, Point::new(1.0, 1.0), Point::new(5.0, 5.0))));
    assert!(gen.has_boundary_violation(&cut(-0.5, Point::new(1.0, 1.0), Point::new(5.0, 5.0))));
}

#[test]
fn test_designer_cnc_cut_within_stock_has_no_warning() {
    let mut state = DesignerState::new();
    state.set_machine_mode(MachineMode::Cnc3D);
    state.canvas.add_rectangle(10.0, 10.0, 20.0, 20.0);
    let (gcode, warn) = state.generate_gcode_with_warning_info(Some(limits()));
    assert!(!warn, "{gcode}");
    assert!(!gcode.contains("OUT OF LIMITS"));
}

#[test]
fn test_designer_cnc_cut_below_stock_bottom_warns() {
    let mut state = DesignerState::new();
    state.set_machine_mode(MachineMode::Cnc3D);
    let id = state.canvas.add_rectangle(10.0, 10.0, 20.0, 20.0);
    let obj = state.canvas.get_shape_mut(id).expect("shape");
    // Stock is 10 mm thick; cutting 12 mm deep goes below the stock bottom.
    obj.start_depth = 12.0;
    let (_, warn) = state.generate_gcode_with_warning_info(Some(limits()));
    assert!(warn);
}

fn laser_segment(from: Point, to: Point, power: u32) -> ToolpathSegment {
    ToolpathSegment::new(ToolpathSegmentType::LinearMove, from, to, 800.0, power)
}

#[test]
fn test_laser_power_changes_are_emitted_per_segment() {
    let gen = ToolpathToGcode::new(Units::MM, 5.0).with_laser_2d();
    let mut tp = Toolpath::new(0.1, 0.0);
    tp.add_segment(ToolpathSegment::new(
        ToolpathSegmentType::RapidMove,
        Point::new(0.0, 0.0),
        Point::new(0.0, 0.0),
        800.0,
        100,
    ));
    tp.add_segment(laser_segment(
        Point::new(0.0, 0.0),
        Point::new(10.0, 0.0),
        100,
    ));
    tp.add_segment(laser_segment(
        Point::new(10.0, 0.0),
        Point::new(20.0, 0.0),
        500,
    ));
    tp.add_segment(laser_segment(
        Point::new(20.0, 0.0),
        Point::new(20.0, 10.0),
        500,
    ));

    // Collinear segments with different power must not be merged by the optimiser.
    let optimized = gen.optimize_toolpath_for_laser(&tp);
    for path in [&tp, &optimized] {
        let body = gen.generate_body(path, 10);
        let s_words: Vec<&str> = body
            .lines()
            .map(|l| l.split(';').next().unwrap_or(""))
            .flat_map(|l| l.split_whitespace())
            .filter(|w| w.starts_with('S'))
            .collect();
        assert_eq!(s_words, vec!["S100", "S500"], "{body}");
        let on = body.find("S100").expect("initial power");
        let change = body.find("S500").expect("power change");
        let first_cut_end = body.find("X10").expect("first cut");
        assert!(on < first_cut_end && first_cut_end < change, "{body}");
    }
}

#[test]
fn test_generate_header_records_stock_z0_for_cnc_only() {
    // The designer's CNC workflow zeroes Z on the stock bottom; the header must say so
    // explicitly so the visualizer's stock simulation uses the right origin.
    let cnc = ToolpathToGcode::new(Units::MM, 10.0).generate_header(3000, 1000.0, 3.175, -5.0, 0.0);
    assert!(cnc.contains("; Stock Z0: bottom"), "{cnc}");

    // Laser output has no Z stock origin.
    let laser = ToolpathToGcode::new(Units::MM, 5.0)
        .with_laser_2d()
        .generate_header(500, 1000.0, 0.1, 0.0, 0.0);
    assert!(!laser.contains("Stock Z0"), "{laser}");
}
