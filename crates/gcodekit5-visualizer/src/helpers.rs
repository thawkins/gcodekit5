//! # Visualizer Helpers
//!
//! Utility functions for the visualizer crate, including
//! coordinate transformations and geometry calculations.

/// Transform screen coordinates to canvas coordinates
/// Used when the displayed image size doesn't match the rendered image size
#[allow(dead_code)]
pub fn transform_screen_to_canvas(
    screen_x: f32,
    screen_y: f32,
    display_width: f32,
    display_height: f32,
    canvas_width: f32,
    canvas_height: f32,
) -> (f32, f32) {
    // Calculate scaling factors
    let scale_x = canvas_width / display_width;
    let scale_y = canvas_height / display_height;

    // Transform coordinates
    let canvas_x = screen_x * scale_x;
    let canvas_y = screen_y * scale_y;

    (canvas_x, canvas_y)
}
