use gcodekit5_core::constants as core_constants;
use gcodekit5_designer::viewport::Viewport;

#[test]
fn test_fit_to_default_bbox() {
    let mut vp = Viewport::new(1200.0, 800.0);
    let width = core_constants::DEFAULT_WORK_WIDTH_MM;
    let height = core_constants::DEFAULT_WORK_HEIGHT_MM;
    let padding = core_constants::VIEW_PADDING;
    // Fit default bbox (0..width, 0..height) into viewport
    vp.fit_to_bounds(0.0, 0.0, width, height, padding);

    let padding_factor = 1.0 - (padding * 2.0);
    let expected_zoom_x = (vp.canvas_width() * padding_factor) / width;
    let expected_zoom_y = (vp.canvas_height() * padding_factor) / height;
    let expected_zoom = expected_zoom_x.min(expected_zoom_y).max(0.1).min(50.0);

    assert!(
        (vp.zoom() - expected_zoom).abs() < 1e-10,
        "zoom {} expected {}",
        vp.zoom(),
        expected_zoom
    );
}
