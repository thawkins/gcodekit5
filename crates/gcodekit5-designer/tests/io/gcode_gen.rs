use gcodekit5_designer::gcode_gen::ToolpathToGcode;
use gcodekit5_designer::shapes::Rectangle;
use gcodekit5_designer::toolpath::{Toolpath, ToolpathGenerator};
use gcodekit5_core::Units;

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
