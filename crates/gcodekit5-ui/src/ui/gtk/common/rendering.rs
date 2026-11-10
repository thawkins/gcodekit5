//! # Shared Rendering Primitives
//!
//! Cairo drawing helpers shared by the 2D views.

use crate::ui::gtk::common::colors;
use gtk4::cairo::Context;

/// Draw X and Y axes at the origin with standard colors (Red for X, Green for Y)
///
/// # Arguments
/// * `cr` - Cairo context
/// * `extent` - How far to draw axes in each direction (mm)
/// * `zoom` - Current zoom level
/// * `line_width` - Desired line width on screen
pub fn draw_origin_axes(cr: &Context, extent: f64, zoom: f64, line_width: f64) {
    let _ = cr.save();

    let line_width_scaled = line_width / zoom;
    cr.set_line_width(line_width_scaled);

    // X Axis - Red
    cr.set_source_rgb(
        colors::AXIS_X.red() as f64,
        colors::AXIS_X.green() as f64,
        colors::AXIS_X.blue() as f64,
    );
    cr.move_to(-extent, 0.0);
    cr.line_to(extent, 0.0);
    let _ = cr.stroke();

    // Y Axis - Green
    cr.set_source_rgb(
        colors::AXIS_Y.red() as f64,
        colors::AXIS_Y.green() as f64,
        colors::AXIS_Y.blue() as f64,
    );
    cr.move_to(0.0, -extent);
    cr.line_to(0.0, extent);
    let _ = cr.stroke();

    let _ = cr.restore();
}
