//! Integration tests for the Designer tool

use gcodekit5_designer::{
    Canvas, Circle, DrawingMode, Line, Point, Rectangle, ToolpathGenerator, ToolpathToGcode,
};
use gcodekit5_core::Units;

#[test]
fn test_designer_workflow_rectangle() {
    // Create a canvas
    let mut canvas = Canvas::new();

    // Switch to rectangle mode and draw a rectangle
    canvas.set_mode(DrawingMode::Rectangle);
    let rect_id = canvas.add_rectangle(10.0, 10.0, 50.0, 30.0);

    // Verify the rectangle was added
    assert_eq!(canvas.shape_count(), 1);
    assert!(canvas.selected_id().is_none());

    // Select the rectangle
    let select_point = Point::new(35.0, 25.0);
    let selected = canvas.select_at(&select_point, 0.0, false);
    assert_eq!(selected, Some(rect_id));
    assert_eq!(canvas.selected_id(), Some(rect_id));
}

#[test]
fn test_designer_workflow_circle() {
    let mut canvas = Canvas::new();

    // Add a circle
    canvas.set_mode(DrawingMode::Circle);
    let circle_id = canvas.add_circle(Point::new(50.0, 50.0), 20.0);

    // Verify the circle was added
    assert_eq!(canvas.shape_count(), 1);

    // Select the circle
    let select_point = Point::new(50.0, 50.0);
    let selected = canvas.select_at(&select_point, 0.0, false);
    assert_eq!(selected, Some(circle_id));
}

#[test]
fn test_designer_canvas_pan_zoom() {
    let mut canvas = Canvas::new();

    // Test zoom
    canvas.set_zoom(2.0);
    assert_eq!(canvas.zoom(), 2.0);

    canvas.set_zoom(0.5);
    assert_eq!(canvas.zoom(), 0.5);

    // Test pan - pan accumulates
    // Note: Canvas starts with 5.0 margin
    canvas.set_pan(0.0, 0.0); // Reset to 0 for test
    canvas.pan(10.0, 20.0);
    let (pan_x, pan_y) = canvas.pan_offset();
    assert_eq!(pan_x, 10.0);
    assert_eq!(pan_y, 20.0);

    canvas.pan(20.0, 10.0);
    let (pan_x, pan_y) = canvas.pan_offset();
    // Pan accumulates
    assert_eq!(pan_x, 30.0);
    assert_eq!(pan_y, 30.0);
}

#[test]
fn test_toolpath_generation_rectangle() {
    let mut gen = ToolpathGenerator::new();
    gen.set_feed_rate(150.0);
    gen.set_spindle_speed(5000);
    gen.set_tool_diameter(3.175);
    gen.set_cut_depth(-3.0);

    let rect = Rectangle::new(0.0, 0.0, 20.0, 10.0);
    let toolpath = gen.generate_rectangle_contour(&rect);

    // Verify toolpath properties
    assert!(toolpath.segments.len() > 0);
    assert_eq!(toolpath.tool_diameter, 3.175);
    assert_eq!(toolpath.depth, -3.0);

    // Verify toolpath has reasonable length
    let length = toolpath.total_length();
    assert!(length > 50.0); // At least around the perimeter
}

#[test]
fn test_toolpath_generation_circle() {
    let gen = ToolpathGenerator::new();
    let circle = Circle::new(Point::new(0.0, 0.0), 10.0);
    let toolpath = gen.generate_circle_contour(&circle);

    assert!(toolpath.segments.len() > 0);
    assert!(toolpath.total_length() > 50.0); // Circumference is ~62.8
}

#[test]
fn test_toolpath_generation_line() {
    let gen = ToolpathGenerator::new();
    let line = Line::new(Point::new(0.0, 0.0), Point::new(50.0, 50.0));
    let toolpath = gen.generate_line_contour(&line);

    assert!(toolpath.segments.len() > 0);
}

#[test]
fn test_gcode_export_from_rectangle() {
    // Create a rectangle and generate toolpath
    let gen = ToolpathGenerator::new();
    let rect = Rectangle::new(0.0, 0.0, 25.4, 25.4); // 1 inch square
    let toolpath = gen.generate_rectangle_contour(&rect);

    // Generate G-code
    let gcode_gen = ToolpathToGcode::new(Units::MM, 10.0);
    let gcode = gcode_gen.generate(&toolpath);

    // Verify G-code structure
    assert!(gcode.contains("Generated G-code from Designer tool"));
    assert!(gcode.contains("G90")); // Absolute positioning
    assert!(gcode.contains("G21")); // Millimeter units
    assert!(gcode.contains("G17")); // XY plane
    assert!(gcode.contains("M3")); // Spindle on
    assert!(gcode.contains("G00")); // Rapid moves
    assert!(gcode.contains("G01")); // Linear moves
    assert!(gcode.contains("M5")); // Spindle off
    assert!(gcode.contains("M30")); // End program
}

#[test]
fn test_canvas_multi_shapes() {
    let mut canvas = Canvas::new();

    // Add multiple shapes
    let rect_id = canvas.add_rectangle(0.0, 0.0, 10.0, 10.0);
    let circle_id = canvas.add_circle(Point::new(25.0, 25.0), 5.0);
    let line_id = canvas.add_line(Point::new(50.0, 0.0), Point::new(50.0, 50.0));

    // debug prints removed
    assert_eq!(canvas.shape_count(), 3);

    // Select each shape
    assert_eq!(canvas.select_at(&Point::new(5.0, 5.0), 0.0, false), Some(rect_id));
    assert_eq!(canvas.select_at(&Point::new(25.0, 25.0), 0.0, false), Some(circle_id));
    assert_eq!(canvas.select_at(&Point::new(50.0, 25.0), 0.0, false), Some(line_id));

    // Remove the selected shape (line)
    assert!(canvas.remove_shape(line_id));
    assert_eq!(canvas.shape_count(), 2);
    assert_eq!(canvas.selected_id(), None); // Removed selected shape
}

#[test]
fn test_designer_complete_workflow() {
    // Complete workflow: design -> toolpath -> gcode
    let mut canvas = Canvas::new();

    // Design a simple part
    canvas.set_mode(DrawingMode::Rectangle);
    canvas.add_rectangle(0.0, 0.0, 50.0, 30.0);

    // Generate toolpath from the design
    let mut gen = ToolpathGenerator::new();
    gen.set_feed_rate(120.0);

    let rect = Rectangle::new(0.0, 0.0, 50.0, 30.0);
    let toolpath = gen.generate_rectangle_contour(&rect);

    // Export to G-code
    let gcode_gen = ToolpathToGcode::new(Units::MM, 10.0);
    let gcode = gcode_gen.generate(&toolpath);

    // Verify we have complete G-code
    assert!(gcode.contains("G90"));
    assert!(gcode.len() > 100); // Should be a reasonable size
}
