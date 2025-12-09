//! Designer state manager integration tests

use gcodekit5_designer::{DesignerState, DrawingMode, Point};

#[test]
fn test_designer_state_complete_workflow() {
    let mut state = DesignerState::new();

    // Add some shapes
    state.canvas.add_rectangle(0.0, 0.0, 50.0, 30.0);
    state
        .canvas
        .add_circle(Point::new(100.0, 100.0), 20.0);
    state.canvas.add_line(
        Point::new(0.0, 0.0),
        Point::new(100.0, 100.0),
    );

    assert_eq!(state.canvas.shape_count(), 3);

    // Test drawing modes
    state.set_mode(0); // Select
    state.set_mode(1); // Rectangle
    assert_eq!(state.canvas.mode(), DrawingMode::Rectangle);

    // Test zoom
    let initial_zoom = state.canvas.zoom();
    state.zoom_in();
    assert!(state.canvas.zoom() > initial_zoom);

    // Test selection
    state.canvas.select_at(&Point::new(25.0, 15.0), 0.0, false);
    assert!(state.canvas.selected_id().is_some());

    // Test deletion
    state.delete_selected();
    assert_eq!(state.canvas.shape_count(), 2);

    // Test G-code generation
    let gcode = state.generate_gcode();
    assert!(!gcode.is_empty());
    assert!(state.gcode_generated);
    assert!(gcode.contains("G00")); // Should have rapid moves
    assert!(gcode.contains("G01")); // Should have linear moves

    // Test tool parameters
    state.set_feed_rate(200.0);
    state.set_spindle_speed(5000);
    state.set_tool_diameter(4.0);
    state.set_cut_depth(-3.0);

    // Generate again with new parameters
    let gcode2 = state.generate_gcode();
    assert!(!gcode2.is_empty());

    // Test clear
    state.clear_canvas();
    assert_eq!(state.canvas.shape_count(), 0);
    assert!(!state.gcode_generated);
}

#[test]
fn test_designer_state_rectangle_workflow() {
    let mut state = DesignerState::new();

    // Design a rectangle
    state.set_mode(1); // Rectangle mode
    state.canvas.add_rectangle(10.0, 10.0, 100.0, 50.0);

    assert_eq!(state.canvas.shape_count(), 1);

    // Generate G-code
    let gcode = state.generate_gcode();
    assert!(state.gcode_generated);
    assert!(gcode.contains("G90"));
    assert!(gcode.contains("G21"));
    assert!(gcode.contains("M3")); // Spindle on
    assert!(gcode.contains("M5")); // Spindle off
}

#[test]
fn test_designer_state_multi_shape_design() {
    let mut state = DesignerState::new();

    // Create a complex design
    for i in 0..5 {
        state
            .canvas
            .add_rectangle((i as f64) * 20.0, 0.0, 15.0, 15.0);
    }

    assert_eq!(state.canvas.shape_count(), 5);

    // Generate G-code for all shapes
    let gcode = state.generate_gcode();
    assert!(state.gcode_generated);
    assert!(!gcode.is_empty());

    // Verify G-code has multiple sections
    let g00_count = gcode.matches("G00").count();
    let g01_count = gcode.matches("G01").count();

    assert!(g00_count > 5); // At least one rapid move per shape
    assert!(g01_count > 5); // At least one linear move per shape
}

#[test]
fn test_designer_state_polyline_update() {
    use gcodekit5_designer::shapes::PathShape;
    use gcodekit5_designer::{Point, Shape};
    let mut state = DesignerState::new();

    // Create a custom polyline (triangle)
    let vertices = vec![
        Point::new(0.0, 0.0),
        Point::new(100.0, 0.0),
        Point::new(50.0, 86.6),
    ];
    state.canvas.add_polyline(vertices.clone());
    
    // Select it
    state.canvas.select_at(&Point::new(50.0, 10.0), 0.0, false);
    assert!(state.canvas.selected_id().is_some());
    
    // Verify it's a PathShape
    if let Some(id) = state.canvas.selected_id() {
        if let Some(obj) = state.canvas.shapes().find(|o| o.id == id) {
            if let Some(path) = obj.shape.as_any().downcast_ref::<PathShape>() {
                let (x1, _y1, x2, _y2) = path.bounding_box();
                assert!((x1 - 0.0).abs() < 0.1);
                assert!((x2 - 100.0).abs() < 0.1);
            } else {
                panic!("Shape is not a PathShape");
            }
        }
    }

    // Update properties (move it)
    state.set_selected_position_and_size(10.0, 10.0, 100.0, 86.6);
    
    // Verify it moved
    if let Some(id) = state.canvas.selected_id() {
        if let Some(obj) = state.canvas.shapes().find(|o| o.id == id) {
            if let Some(path) = obj.shape.as_any().downcast_ref::<PathShape>() {
                let (x1, y1, _x2, _y2) = path.bounding_box();
                assert!((x1 - 10.0).abs() < 0.1);
                assert!((y1 - 10.0).abs() < 0.1);
            } else {
                panic!("Shape is not a PathShape after update");
            }
        }
    }
}
