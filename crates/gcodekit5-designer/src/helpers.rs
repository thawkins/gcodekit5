//! # Designer Helpers
//!
//! Utility functions for the designer module, including coordinate snapping.

/// Snap world coordinates to whole millimeters (round to nearest mm)
pub fn snap_to_mm(value: f64) -> f64 {
    (value + 0.5).floor()
}
