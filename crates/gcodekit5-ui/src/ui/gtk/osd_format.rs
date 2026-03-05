//! # On-Screen Display Formatting
//!
//! Formats machine status values (coordinates, feed rate, spindle speed)
//! for on-screen display overlays in the visualizer view.

use gcodekit5_core::units::{format_length, MeasurementSystem};

use crate::t;

pub fn format_zoom_center_cursor(
    zoom_scale: f64,
    center_x: f32,
    center_y: f32,
    cursor_x: f32,
    cursor_y: f32,
    system: MeasurementSystem,
) -> String {
    let center_x_str = format_length(center_x, system);
    let center_y_str = format_length(center_y, system);
    let cursor_x_str = format_length(cursor_x, system);
    let cursor_y_str = format_length(cursor_y, system);

    format!(
        "{}: {:.0}%  {}: X {} Y {}  {}: X {} Y {}",
        t!("Zoom"),
        zoom_scale * 100.0,
        t!("Center"),
        center_x_str,
        center_y_str,
        t!("Cursor"),
        cursor_x_str,
        cursor_y_str,
    )
}
