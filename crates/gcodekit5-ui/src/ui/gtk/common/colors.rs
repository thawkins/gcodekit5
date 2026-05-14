//! # Shared Color Constants
//!
//! Centralized color definitions for consistent theming across the UI.
//! Uses GTK theme colors where possible, with fallback values for custom rendering.

use gtk4::gdk::RGBA;
use gtk4::prelude::StyleContextExt;

// =============================================================================
// GTK Theme Color Lookups (preferred)
// =============================================================================

/// Get the accent color from the GTK theme, with fallback
pub fn accent_color(style_context: &gtk4::StyleContext) -> RGBA {
    style_context
        .lookup_color("accent_color")
        .unwrap_or_else(|| RGBA::new(0.0, 0.5, 1.0, 1.0))
}

/// Get the success color from the GTK theme, with fallback
pub fn success_color(style_context: &gtk4::StyleContext) -> RGBA {
    style_context
        .lookup_color("success_color")
        .unwrap_or_else(|| RGBA::new(0.0, 0.8, 0.0, 1.0))
}

/// Get the warning color from the GTK theme, with fallback
pub fn warning_color(style_context: &gtk4::StyleContext) -> RGBA {
    style_context
        .lookup_color("warning_color")
        .unwrap_or_else(|| RGBA::new(1.0, 0.8, 0.0, 1.0))
}

/// Get the error color from the GTK theme, with fallback
pub fn error_color(style_context: &gtk4::StyleContext) -> RGBA {
    style_context
        .lookup_color("error_color")
        .unwrap_or_else(|| RGBA::new(1.0, 0.0, 0.0, 1.0))
}

// =============================================================================
// Fixed Color Constants (for rendering where theme colors aren't appropriate)
// =============================================================================

/// Primary axis colors (X, Y, Z)
pub const AXIS_X: RGBA = RGBA::new(1.0, 0.0, 0.0, 1.0); // Red
pub const AXIS_Y: RGBA = RGBA::new(0.0, 1.0, 0.0, 1.0); // Green
pub const AXIS_Z: RGBA = RGBA::new(0.0, 0.0, 1.0, 1.0); // Blue

/// Device/machine bounds color
pub const DEVICE_BOUNDS: RGBA = RGBA::new(0.0, 0.0, 1.0, 1.0); // Blue

/// Grid colors
pub const GRID_MAJOR: RGBA = RGBA::new(0.5, 0.5, 0.5, 1.0); // Gray
pub const GRID_MINOR: RGBA = RGBA::new(0.7, 0.7, 0.7, 1.0); // Light gray

/// Selection and highlight colors
pub const SELECTION: RGBA = RGBA::new(1.0, 0.0, 0.0, 1.0); // Red
pub const SELECTION_MULTI: RGBA = RGBA::new(1.0, 1.0, 0.0, 1.0); // Yellow
pub const PREVIEW: RGBA = RGBA::new(1.0, 1.0, 0.0, 1.0); // Yellow

/// Toolpath colors
pub const TOOLPATH_RAPID: RGBA = RGBA::new(1.0, 0.8, 0.0, 0.5); // Yellow with transparency
pub const TOOLPATH_CUT: RGBA = RGBA::new(0.0, 0.8, 0.0, 1.0); // Green
pub const TOOLPATH_ARC: RGBA = RGBA::new(0.0, 0.8, 0.0, 0.7); // Green with transparency

/// Background colors
pub const BACKGROUND_DARK: RGBA = RGBA::new(0.15, 0.15, 0.15, 1.0); // Dark gray
pub const BACKGROUND_LIGHT: RGBA = RGBA::new(1.0, 1.0, 1.0, 1.0); // White

/// Default/fallback colors
pub const BLACK: RGBA = RGBA::new(0.0, 0.0, 0.0, 1.0);
pub const WHITE: RGBA = RGBA::new(1.0, 1.0, 1.0, 1.0);
pub const GRAY_50: RGBA = RGBA::new(0.5, 0.5, 0.5, 1.0);
pub const GRAY_70: RGBA = RGBA::new(0.7, 0.7, 0.7, 1.0);

// =============================================================================
// Opacity Constants
// =============================================================================

pub const OPACITY_FULL: f64 = 1.0;
pub const OPACITY_HIGH: f64 = 0.8;
pub const OPACITY_MEDIUM: f64 = 0.5;
pub const OPACITY_LOW: f64 = 0.3;
pub const OPACITY_VERY_LOW: f64 = 0.15;

// =============================================================================
// Helper Functions
// =============================================================================

/// Convert RGBA to a tuple of (r, g, b) as f64
pub fn to_rgb_f64(color: &RGBA) -> (f64, f64, f64) {
    (
        color.red() as f64,
        color.green() as f64,
        color.blue() as f64,
    )
}

/// Convert RGBA to a tuple of (r, g, b, a) as f64
pub fn to_rgba_f64(color: &RGBA) -> (f64, f64, f64, f64) {
    (
        color.red() as f64,
        color.green() as f64,
        color.blue() as f64,
        color.alpha() as f64,
    )
}

/// Create a grayscale color from 0.0 (black) to 1.0 (white)
pub fn grayscale(value: f64) -> RGBA {
    let v = value.clamp(0.0, 1.0) as f32;
    RGBA::new(v, v, v, 1.0)
}

/// Apply opacity to a color
pub fn with_opacity(color: &RGBA, opacity: f64) -> RGBA {
    RGBA::new(
        color.red(),
        color.green(),
        color.blue(),
        (color.alpha() as f64 * opacity) as f32,
    )
}
