use gcodekit5_designer::Canvas;

#[test]
fn test_fit_to_view_empty_sets_zoom_1_and_pan_30() {
    let mut canvas = Canvas::new();
    // No shapes were added
    assert_eq!(canvas.shape_count(), 0);

    canvas.fit_all_shapes();

    assert!(
        (canvas.zoom() - 1.0).abs() < f64::EPSILON,
        "Zoom should be 1.0 on empty canvas"
    );
    assert!(
        (canvas.pan_x() - 30.0).abs() < 1e-6,
        "Pan X should be 30px on empty canvas"
    );
    assert!(
        (canvas.pan_y() - (-30.0)).abs() < 1e-6,
        "Pan Y should be -30px on empty canvas"
    );
}
