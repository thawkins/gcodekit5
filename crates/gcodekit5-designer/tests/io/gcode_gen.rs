use gcodekit5_core::Units;
use gcodekit5_designer::gcode_gen::ToolpathToGcode;
use gcodekit5_designer::toolpath::{Toolpath, ToolpathGenerator};
use gcodekit5_designer::Rectangle;

#[test]
fn test_gcode_generation() {
    let gen = ToolpathGenerator::new();
    let rect = Rectangle::new(0.0, 0.0, 10.0, 10.0);
    let toolpaths = gen.generate_rectangle_contour(&rect, 0.0);
    let toolpath = &toolpaths[0];

    let gcode_gen = ToolpathToGcode::new(Units::MM, 10.0);
    let gcode = gcode_gen.generate(toolpath);

    assert!(gcode.contains("G90"));
    assert!(gcode.contains("G21"));
    assert!(gcode.contains("G00"));
    assert!(gcode.contains("G01"));
    assert!(gcode.contains("M30"));
}

#[test]
fn test_gcode_header() {
    let toolpath = Toolpath::new(3.175, -5.0);
    let gcode_gen = ToolpathToGcode::new(Units::MM, 10.0);
    let gcode = gcode_gen.generate(&toolpath);

    assert!(gcode.contains("Generated G-code from Designer tool"));
    assert!(gcode.contains("Tool diameter"));
    assert!(gcode.contains("Cut depth"));
}

#[test]
fn test_rapid_moves_retract_before_xy() {
    let mut toolpath = Toolpath::new(3.175, -1.0);

    // Simulate end of a cut at Z -1 heading to origin for next layer
    toolpath.add_segment(
        gcodekit5_designer::toolpath::ToolpathSegment::new(
            gcodekit5_designer::toolpath::ToolpathSegmentType::LinearMove,
            gcodekit5_designer::model::Point::new(5.0, 5.0),
            gcodekit5_designer::model::Point::new(6.0, 6.0),
            200.0,
            1000,
        )
        .with_z_depth(-1.0),
    );

    toolpath.add_segment(gcodekit5_designer::toolpath::ToolpathSegment::new(
        gcodekit5_designer::toolpath::ToolpathSegmentType::RapidMove,
        gcodekit5_designer::model::Point::new(6.0, 6.0),
        gcodekit5_designer::model::Point::new(0.0, 0.0),
        200.0,
        1000,
    ));

    let gcode_gen = ToolpathToGcode::new(Units::MM, 5.0);
    let gcode = gcode_gen.generate(&toolpath);
    let lines: Vec<&str> = gcode.lines().collect();

    // Find the rapid move to origin
    let origin_idx = lines
        .iter()
        .position(|l| l.contains("G00 X0.000 Y0.000 Z5.000"))
        .expect("expected rapid move to origin");

    // The line before must be a retract-only move
    assert!(origin_idx > 0);
    let prev_line = lines[origin_idx - 1];
    assert!(prev_line.contains("G00 Z5.000"));
}
