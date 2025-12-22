use gcodekit5_designer::model::Point;
use gcodekit5_designer::viewport::Viewport;

#[test]
fn test_viewport_creation() {
    let vp = Viewport::new(1200.0, 800.0);
    assert_eq!(vp.zoom(), 1.0);
    assert_eq!(vp.pan_x(), 5.0); // Initial margin
    assert_eq!(vp.pan_y(), 5.0); // Initial margin
}

#[test]
fn test_pixel_to_world_origin_at_bottom_left() {
    let vp = Viewport::new(1200.0, 600.0);
    // With margin of 5px, pixel (5, 595) should map to world (0, 0)
    // pixel_y=595 is 5px from bottom of 600px canvas
    let world = vp.pixel_to_world(5.0, 595.0);
    assert!((world.x - 0.0).abs() < 0.01);
    assert!((world.y - 0.0).abs() < 0.01);
}

#[test]
fn test_world_to_pixel_origin_at_bottom_left() {
    let vp = Viewport::new(1200.0, 600.0);
    // World (0, 0) should map to pixel (5, 595) with margin
    let (pixel_x, pixel_y) = vp.world_to_pixel(0.0, 0.0);
    assert!((pixel_x - 5.0).abs() < 0.01);
    assert!((pixel_y - 595.0).abs() < 0.01);
}

#[test]
fn test_world_to_pixel_positive_y_goes_up() {
    let vp = Viewport::new(1200.0, 800.0);
    // Positive Y should go up the screen (lower pixel Y)
    let (_, py0) = vp.world_to_pixel(0.0, 0.0);
    let (_, py100) = vp.world_to_pixel(0.0, 100.0);
    assert!(py100 < py0); // Higher world Y = lower pixel Y (up on screen)
}

#[test]
fn test_world_to_pixel_positive_x_goes_right() {
    let vp = Viewport::new(1200.0, 800.0);
    // Positive X should go right (higher pixel X)
    let (px0, _) = vp.world_to_pixel(0.0, 0.0);
    let (px100, _) = vp.world_to_pixel(100.0, 0.0);
    assert!(px100 > px0); // Higher world X = higher pixel X (right on screen)
}

#[test]
fn test_pixel_to_world_with_zoom() {
    let mut vp = Viewport::new(1200.0, 600.0);
    vp.reset(); // Clear margin for simpler math
    vp.set_pan(0.0, 0.0); // Explicitly clear margin
    vp.set_zoom(2.0);
    // At zoom 2.0, 200 pixels = 100 world units
    let world = vp.pixel_to_world(200.0, 400.0);
    assert!((world.x - 100.0).abs() < 0.01);
    assert!((world.y - 100.0).abs() < 0.01);
}

#[test]
fn test_roundtrip_conversion() {
    let mut vp = Viewport::new(1200.0, 800.0);
    vp.reset(); // Clear margin for simpler math
    vp.set_zoom(2.5);
    vp.set_pan(75.0, 125.0);

    let original = Point::new(123.45, 456.78);
    let (pixel_x, pixel_y) = vp.world_to_pixel(original.x, original.y);
    let roundtrip = vp.pixel_to_world(pixel_x, pixel_y);

    assert!((roundtrip.x - original.x).abs() < 0.01);
    assert!((roundtrip.y - original.y).abs() < 0.01);
}

#[test]
fn test_zoom_constraints() {
    let mut vp = Viewport::new(1200.0, 800.0);
    vp.set_zoom(0.05); // Too small
    assert!(vp.zoom() > 0.05);

    vp.set_zoom(60.0); // Too large
    assert!(vp.zoom() < 60.0);
}

#[test]
fn test_zoom_in_out() {
    let mut vp = Viewport::new(1200.0, 800.0);
    let initial = vp.zoom();
    vp.zoom_in();
    assert!(vp.zoom() > initial);

    vp.zoom_out();
    assert!((vp.zoom() - initial).abs() < 0.01);
}

#[test]
fn test_center_on_point() {
    let mut vp = Viewport::new(800.0, 600.0);
    vp.set_zoom(1.0);
    vp.center_on(100.0, 200.0);

    let world = vp.pixel_to_world(400.0, 300.0);
    assert!((world.x - 100.0).abs() < 0.01);
    assert!((world.y - 200.0).abs() < 0.01);
}

#[test]
fn test_fit_to_bounds() {
    let mut vp = Viewport::new(1200.0, 800.0);
    vp.fit_to_bounds(0.0, 0.0, 100.0, 100.0, 0.05);

    assert!(vp.zoom() > 1.0); // Should zoom in to fit small content
}

#[test]
fn test_reset() {
    let mut vp = Viewport::new(1200.0, 800.0);
    vp.set_zoom(2.5);
    vp.set_pan(100.0, 200.0);
    vp.reset();

    assert_eq!(vp.zoom(), 1.0);
    assert_eq!(vp.pan_x(), 5.0);
    assert_eq!(vp.pan_y(), 5.0);
}
