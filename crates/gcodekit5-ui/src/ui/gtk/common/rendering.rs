//! # Shared Rendering Primitives
//!
//! Common rendering functions used by both designer_canvas and visualizer.
//! Provides consistent grid drawing, axis drawing, and coordinate transformations.

use crate::ui::gtk::common::colors;
use gtk4::cairo::Context;
use gtk4::gdk::RGBA;

/// Grid spacing options based on zoom level
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GridSpacing {
    /// 0.1mm spacing for very high zoom
    Fine(f64),
    /// 1mm spacing for medium zoom
    Medium(f64),
    /// 10mm spacing for normal zoom
    Normal(f64),
    /// 100mm spacing for zoomed out
    Coarse(f64),
}

impl GridSpacing {
    /// Calculate appropriate grid spacing based on zoom level
    pub fn from_zoom(zoom: f64, base_spacing: f64) -> Self {
        let pixels_per_unit = zoom * base_spacing;

        if pixels_per_unit < 5.0 {
            // Too crowded, use coarser spacing
            if pixels_per_unit * 10.0 < 5.0 {
                GridSpacing::Coarse(base_spacing * 100.0)
            } else {
                GridSpacing::Coarse(base_spacing * 10.0)
            }
        } else if pixels_per_unit > 50.0 {
            // Too sparse, use finer spacing
            if pixels_per_unit / 10.0 > 50.0 {
                GridSpacing::Fine(base_spacing / 100.0)
            } else {
                GridSpacing::Fine(base_spacing / 10.0)
            }
        } else {
            GridSpacing::Normal(base_spacing)
        }
    }

    /// Get the spacing value in mm
    pub fn value(self) -> f64 {
        match self {
            GridSpacing::Fine(v)
            | GridSpacing::Medium(v)
            | GridSpacing::Normal(v)
            | GridSpacing::Coarse(v) => v,
        }
    }

    /// Whether this spacing should use major line styling
    pub fn is_major(self) -> bool {
        matches!(self, GridSpacing::Normal(_) | GridSpacing::Coarse(_))
    }
}

/// Draw a Cartesian grid with major and minor lines
///
/// # Arguments
/// * `cr` - Cairo context
/// * `width` - Viewport width in pixels
/// * `height` - Viewport height in pixels
/// * `grid_spacing_mm` - Base grid spacing in mm
/// * `fg_color` - Foreground color from theme
/// * `zoom` - Current zoom level
/// * `major_line_width` - Line width for major grid lines
/// * `minor_line_width` - Line width for minor grid lines
/// * `view_offset_x` - X offset of view in mm (optional, default 0)
/// * `view_offset_y` - Y offset of view in mm (optional, default 0)
/// * `flip_y` - Whether to flip Y axis (Cairo default is Y-down)
#[allow(clippy::too_many_arguments)]
pub fn draw_cartesian_grid(
    cr: &Context,
    width: f64,
    height: f64,
    grid_spacing_mm: f64,
    fg_color: &RGBA,
    zoom: f64,
    major_line_width: f64,
    minor_line_width: f64,
    view_offset_x: f64,
    view_offset_y: f64,
    flip_y: bool,
) {
    let _ = cr.save();

    // Determine grid spacing based on zoom
    let effective_spacing = GridSpacing::from_zoom(zoom, grid_spacing_mm).value();

    // Calculate visible bounds in world coordinates
    let y_scale = if flip_y { -1.0 } else { 1.0 };
    let view_width_mm = width / zoom;
    let view_height_mm = height / zoom;
    let x0 = -view_offset_x;
    let x1 = x0 + view_width_mm;
    let y0 = if flip_y {
        -view_offset_y - view_height_mm
    } else {
        -view_offset_y
    };
    let y1 = if flip_y {
        -view_offset_y
    } else {
        -view_offset_y + view_height_mm
    };

    // Round to grid boundaries
    let x_start = (x0 / effective_spacing).floor() * effective_spacing;
    let x_end = (x1 / effective_spacing).ceil() * effective_spacing;
    let y_start = (y0 / effective_spacing).floor() * effective_spacing;
    let y_end = (y1 / effective_spacing).ceil() * effective_spacing;

    // Draw vertical lines
    let mut x = x_start;
    while x <= x_end {
        cr.move_to(x, y0);
        cr.line_to(x, y1);
        x += effective_spacing;
    }

    // Draw horizontal lines
    let mut y = y_start;
    while y <= y_end {
        cr.move_to(x0, y);
        cr.line_to(x1, y);
        y += effective_spacing;
    }

    // Draw with appropriate color
    let gray = (fg_color.red() + fg_color.green() + fg_color.blue()) / 3.0;
    let minor_alpha = 0.15f64;
    let major_alpha = 0.25f64;
    let alpha = if effective_spacing >= 10.0 {
        major_alpha
    } else {
        minor_alpha
    };

    cr.set_source_rgba(gray as f64, gray as f64, gray as f64, alpha);
    cr.set_line_width(if effective_spacing >= 10.0 {
        major_line_width / zoom
    } else {
        minor_line_width / zoom
    });
    let _ = cr.stroke();

    // Draw axes (thicker, more visible) - only if they're visible
    cr.set_source_rgba(
        fg_color.red() as f64,
        fg_color.green() as f64,
        fg_color.blue() as f64,
        0.8,
    );
    cr.set_line_width(major_line_width / zoom);

    // X-axis (y=0)
    if y_scale * y_start <= 0.0 && y_scale * y_end >= 0.0 {
        cr.move_to(x0, 0.0);
        cr.line_to(x1, 0.0);
    }

    // Y-axis (x=0)
    if x_start <= 0.0 && x_end >= 0.0 {
        cr.move_to(0.0, y_start);
        cr.line_to(0.0, y_end);
    }
    let _ = cr.stroke();

    let _ = cr.restore();
}

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

/// Draw device/machine bounds rectangle
///
/// # Arguments
/// * `cr` - Cairo context
/// * `min_x`, `min_y`, `max_x`, `max_y` - Bounds in mm
/// * `zoom` - Current zoom level
/// * `line_width` - Line width on screen
/// * `color` - Optional color (defaults to DEVICE_BOUNDS)
#[allow(clippy::too_many_arguments)]
pub fn draw_device_bounds(
    cr: &Context,
    min_x: f64,
    min_y: f64,
    max_x: f64,
    max_y: f64,
    zoom: f64,
    line_width: f64,
    color: Option<&RGBA>,
) {
    let _ = cr.save();

    let width = max_x - min_x;
    let height = max_y - min_y;
    let color = color.unwrap_or(&colors::DEVICE_BOUNDS);

    cr.set_source_rgb(
        color.red() as f64,
        color.green() as f64,
        color.blue() as f64,
    );
    cr.set_line_width(line_width / zoom);
    cr.rectangle(min_x, min_y, width, height);
    let _ = cr.stroke();

    let _ = cr.restore();
}

/// Draw selection handles around a rectangular bounds
///
/// # Arguments
/// * `cr` - Cairo context
/// * `min_x`, `min_y`, `max_x`, `max_y` - Selection bounds
/// * `zoom` - Current zoom level
/// * `handle_size` - Size of handles on screen
/// * `color` - Handle color
#[allow(clippy::too_many_arguments)]
pub fn draw_selection_handles(
    cr: &Context,
    min_x: f64,
    min_y: f64,
    max_x: f64,
    max_y: f64,
    zoom: f64,
    handle_size: f64,
    color: &RGBA,
) {
    let _ = cr.save();

    let size = handle_size / zoom;
    let half = size / 2.0;

    // Define handle positions (corners + midpoints)
    let handles = [
        (min_x - half, min_y - half),                 // Bottom-left
        ((min_x + max_x) / 2.0 - half, min_y - half), // Bottom-center
        (max_x - half, min_y - half),                 // Bottom-right
        (max_x - half, (min_y + max_y) / 2.0 - half), // Right-center
        (max_x - half, max_y - half),                 // Top-right
        ((min_x + max_x) / 2.0 - half, max_y - half), // Top-center
        (min_x - half, max_y - half),                 // Top-left
        (min_x - half, (min_y + max_y) / 2.0 - half), // Left-center
    ];

    cr.set_source_rgb(
        color.red() as f64,
        color.green() as f64,
        color.blue() as f64,
    );

    for (x, y) in &handles {
        cr.rectangle(*x, *y, size, size);
    }
    let _ = cr.fill();

    let _ = cr.restore();
}

/// Apply standard Cairo transformations for canvas rendering
///
/// Sets up the coordinate system with:
/// - Origin at bottom-left (Y-up for CNC coordinates)
/// - Pan offset applied
/// - Zoom scale applied
///
/// # Arguments
/// * `cr` - Cairo context
/// * `width` - Widget width
/// * `height` - Widget height
/// * `pan_x` - X pan offset in pixels
/// * `pan_y` - Y pan offset in pixels
/// * `zoom` - Zoom scale
pub fn apply_canvas_transform(
    cr: &Context,
    _width: f64,
    height: f64,
    pan_x: f64,
    pan_y: f64,
    zoom: f64,
) {
    // Transform to bottom-left origin (Y-up)
    cr.translate(0.0, height);
    cr.scale(1.0, -1.0);

    // Apply pan and zoom
    cr.translate(pan_x, pan_y);
    cr.scale(zoom, zoom);
}

/// Apply visualizer-style transformations (center-based)
///
/// Sets up the coordinate system with:
/// - Origin at center of viewport
/// - Zoom scale applied
/// - Pan offsets applied in world coordinates
///
/// # Arguments
/// * `cr` - Cairo context
/// * `width` - Widget width
/// * `height` - Widget height
/// * `center_x` - X center offset in world coordinates
/// * `center_y` - Y center offset in world coordinates
/// * `zoom` - Zoom scale
/// * `flip_y` - Whether to flip Y axis (true for visualizer)
pub fn apply_visualizer_transform(
    cr: &Context,
    width: f64,
    height: f64,
    center_x: f64,
    center_y: f64,
    zoom: f64,
    flip_y: bool,
) {
    let y_scale = if flip_y { -1.0 } else { 1.0 };

    cr.translate(width / 2.0, height / 2.0);
    cr.scale(zoom, zoom * y_scale);
    cr.translate(center_x, center_y);
}

/// Calculate viewport bounds in world coordinates
///
/// Returns (min_x, min_y, max_x, max_y) in world coordinates
pub fn calculate_viewport_bounds(
    width: f64,
    height: f64,
    zoom: f64,
    offset_x: f64,
    offset_y: f64,
    flip_y: bool,
) -> (f64, f64, f64, f64) {
    let half_width = width / (2.0 * zoom);
    let half_height = height / (2.0 * zoom);

    let min_x = -offset_x - half_width;
    let max_x = -offset_x + half_width;

    let (min_y, max_y) = if flip_y {
        (-offset_y - half_height, -offset_y + half_height)
    } else {
        (-offset_y + half_height, -offset_y - half_height)
    };

    (min_x, min_y.min(max_y), max_x, min_y.max(max_y))
}
