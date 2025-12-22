use gcodekit5_designer::Canvas;

#[test]
fn test_fit_to_view_padding_5pct() {
    let mut canvas = Canvas::new();
    // Viewport default is 1200x600 in Canvas::new() -> Viewport::new
    let view_width = canvas.viewport().canvas_width();
    let view_height = canvas.viewport().canvas_height();

    // Create simple rectangle within world coords
    let min_x = 100.0;
    let min_y = 50.0;
    let w = 200.0;
    let h = 100.0;
    canvas.add_rectangle(min_x, min_y, w, h);

    // Fit to view (this calls viewport.fit_to_bounds with padding 0.05 in the designer canvas implementation)
    canvas.fit_all_shapes();

    // Padding expected: 5% per edge
    let pad_x = view_width * 0.05;
    let pad_y = view_height * 0.05;

    // Get pixel coords of content bounding box after fit
    let (min_px, min_py) = canvas.world_to_pixel(min_x, min_y);
    let (max_px, max_py) = canvas.world_to_pixel(min_x + w, min_y + h);

    // Left edge should be >= pad_x
    assert!(
        min_px >= pad_x - 1.0,
        "left edge {} < pad {}",
        min_px,
        pad_x
    );
    // Right edge should be <= width - pad_x
    assert!(
        max_px <= view_width - pad_x + 1.0,
        "right edge {} > width-pad {}",
        max_px,
        pad_x
    );
    // Top edge should be >= pad_y
    assert!(max_py >= pad_y - 1.0, "top edge {} < pad {}", max_py, pad_y);
    // Bottom edge should be <= height - pad_y
    assert!(
        min_py <= view_height - pad_y + 1.0,
        "bottom edge {} > height-pad {}",
        min_py,
        pad_y
    );
}
