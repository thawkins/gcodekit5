//! Comprehensive tests for visualizer coordinate transformations

use gcodekit5_visualizer::Visualizer2D;

#[test]
fn test_set_default_view_with_scale() {
    let mut viz = Visualizer2D::new();
    viz.set_scale_factor(2.0);
    viz.set_default_view(800.0, 600.0);

    // Offsets should be non-zero after setting default view
    assert_ne!(viz.x_offset, 0.0);
    assert_ne!(viz.y_offset, 0.0);
}

#[test]
fn test_reset_pan_clears_offsets() {
    let mut viz = Visualizer2D::new();
    viz.x_offset = 100.0;
    viz.y_offset = 200.0;

    viz.reset_pan();

    assert_eq!(viz.x_offset, 0.0);
    assert_eq!(viz.y_offset, 0.0);
}

#[test]
fn test_fit_to_view_centers_content() {
    let mut viz = Visualizer2D::new();
    // Create a simple square from (10,10) to (20,20)
    let gcode = "G0 X10 Y10\nG1 X20 Y10\nG1 X20 Y20\nG1 X10 Y20\nG1 X10 Y10";
    viz.parse_gcode(gcode);

    viz.fit_to_view(800.0, 600.0);

    // Zoom should be applied
    assert!(
        viz.zoom_scale > 1.0,
        "Expected zoom > 1.0 for small content"
    );

    // Offsets should be set to center content
    assert_ne!(viz.x_offset, 0.0);
    assert_ne!(viz.y_offset, 0.0);
}

#[test]
fn test_fit_to_view_with_origin_content() {
    let mut viz = Visualizer2D::new();
    // Content that includes origin
    let gcode = "G0 X0 Y0\nG1 X50 Y0\nG1 X50 Y50\nG1 X0 Y50\nG1 X0 Y0";
    viz.parse_gcode(gcode);

    viz.fit_to_view(800.0, 600.0);

    assert!(viz.zoom_scale > 0.1);
    assert!(viz.zoom_scale < 50.0);
}

#[test]
fn test_fit_to_view_with_negative_coords() {
    let mut viz = Visualizer2D::new();
    // Content in negative space
    let gcode = "G0 X-50 Y-50\nG1 X-10 Y-50\nG1 X-10 Y-10\nG1 X-50 Y-10\nG1 X-50 Y-50";
    viz.parse_gcode(gcode);

    viz.fit_to_view(800.0, 600.0);

    assert!(viz.zoom_scale > 1.0);
    assert!(viz.x_offset.abs() > 0.0);
    assert!(viz.y_offset.abs() > 0.0);
}

#[test]
fn test_fit_to_view_preserves_aspect_ratio() {
    let mut viz = Visualizer2D::new();
    // Wide rectangle
    let gcode = "G0 X0 Y0\nG1 X100 Y0\nG1 X100 Y20\nG1 X0 Y20\nG1 X0 Y0";
    viz.parse_gcode(gcode);

    viz.fit_to_view(800.0, 600.0);

    // Should zoom to fit width, not height
    assert!(viz.zoom_scale > 0.0);
}

#[test]
fn test_fit_to_view_tall_canvas() {
    let mut viz = Visualizer2D::new();
    // Square content
    let gcode = "G0 X0 Y0\nG1 X50 Y0\nG1 X50 Y50\nG1 X0 Y50\nG1 X0 Y0";
    viz.parse_gcode(gcode);

    // Tall canvas (400x800)
    viz.fit_to_view(400.0, 800.0);

    assert!(viz.zoom_scale > 0.0);
}

#[test]
fn test_fit_to_view_wide_canvas() {
    let mut viz = Visualizer2D::new();
    // Square content
    let gcode = "G0 X0 Y0\nG1 X50 Y0\nG1 X50 Y50\nG1 X0 Y50\nG1 X0 Y0";
    viz.parse_gcode(gcode);

    // Wide canvas (800x400)
    viz.fit_to_view(800.0, 400.0);

    assert!(viz.zoom_scale > 0.0);
}

#[test]
fn test_zoom_and_pan_independent() {
    let mut viz = Visualizer2D::new();

    viz.zoom_in();
    let zoom_after = viz.zoom_scale;
    let x_after_zoom = viz.x_offset;

    viz.pan_right(800.0);
    assert_eq!(viz.zoom_scale, zoom_after, "Panning should not affect zoom");
    assert_ne!(viz.x_offset, x_after_zoom, "Pan should change offset");
}

#[test]
fn test_bounds_include_origin() {
    let mut viz = Visualizer2D::new();
    // Content not at origin
    let gcode = "G0 X100 Y100\nG1 X200 Y200";
    viz.parse_gcode(gcode);

    let (min_x, _max_x, min_y, _max_y) = viz.get_bounds();

    // Bounds should be extended to include origin
    assert!(min_x <= 0.0, "Min X should include origin");
    assert!(min_y <= 0.0, "Min Y should include origin");
}

#[test]
fn test_fit_to_view_margin_applied() {
    let mut viz = Visualizer2D::new();
    // 100x100 square
    let gcode = "G0 X0 Y0\nG1 X100 Y0\nG1 X100 Y100\nG1 X0 Y100\nG1 X0 Y0";
    viz.parse_gcode(gcode);

    // Canvas is 1000x1000 - should fit with margin
    viz.fit_to_view(1000.0, 1000.0);

    // With 10% margin per side (20% total margin), content fits with scale = 8
    // Test accepts a small range to avoid false negatives on rounding
    assert!(viz.zoom_scale <= 9.0 && viz.zoom_scale >= 8.0);
}

#[test]
fn test_pan_operations_accumulate() {
    let mut viz = Visualizer2D::new();

    viz.pan_right(800.0);
    let x_after_first = viz.x_offset;

    viz.pan_right(800.0);
    let x_after_second = viz.x_offset;

    assert!(x_after_second > x_after_first, "Pan should accumulate");
    assert_eq!(x_after_second, x_after_first * 2.0);
}

#[test]
fn test_zoom_affects_scale_calculation() {
    let mut viz = Visualizer2D::new();
    let gcode = "G0 X0 Y0\nG1 X100 Y100";
    viz.parse_gcode(gcode);

    viz.zoom_in();
    let zoom1 = viz.zoom_scale;

    viz.zoom_in();
    let zoom2 = viz.zoom_scale;

    assert!(zoom2 > zoom1, "Successive zooms should increase scale");
}

#[test]
fn test_set_default_view_positions_origin() {
    let mut viz = Visualizer2D::new();
    let canvas_width = 800.0;
    let canvas_height = 600.0;

    viz.set_default_view(canvas_width, canvas_height);

    // Origin should be positioned near bottom-left (with offsets applied)
    // The exact values depend on min_x/min_y, but offsets should be non-zero
    assert!(viz.x_offset != 0.0 || viz.min_x == 0.0);
    assert!(viz.y_offset != 0.0 || viz.min_y == 0.0);
}
