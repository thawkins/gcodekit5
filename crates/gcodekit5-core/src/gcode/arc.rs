//! # Arc geometry helpers
//!
//! Plane-agnostic helpers for G2/G3 arcs shared by the visualizer and the file
//! statistics pipeline. All functions work in 2D "plane coordinates" (the two
//! axes of the active G17/G18/G19 plane) and follow GRBL's conventions:
//!
//! - R-format arcs: positive R selects the shorter arc, negative R the longer one.
//! - An arc whose start and end points coincide is a full circle.
//!
//! No dependencies beyond `std`.

use std::f64::consts::TAU;

/// Angular tolerance (radians) below which an arc is treated as a full circle
/// (matches GRBL's `ARC_ANGULAR_TRAVEL_EPSILON`).
const ARC_ANGULAR_TRAVEL_EPSILON: f64 = 5e-7;

/// Compute the centre offset (from the start point) of an R-format arc.
///
/// # Arguments
/// * `dx`, `dy` - Target minus start in plane coordinates.
/// * `r` - Signed radius (negative selects the arc longer than 180 degrees).
/// * `clockwise` - `true` for G2, `false` for G3.
///
/// # Returns
/// `Some((i, j))` offsets of the centre from the start point, or `None` when the
/// arc is undefined (zero radius, or start and end coincide). If the chord is
/// longer than the diameter the radius is stretched to a half circle.
pub fn center_offset_from_radius(dx: f64, dy: f64, r: f64, clockwise: bool) -> Option<(f64, f64)> {
    let chord_sq = dx * dx + dy * dy;
    if chord_sq <= f64::EPSILON || r == 0.0 || !r.is_finite() {
        return None;
    }

    let h_sq = (4.0 * r * r - chord_sq).max(0.0);
    let mut h_x2_div_d = -h_sq.sqrt() / chord_sq.sqrt();
    if !clockwise {
        h_x2_div_d = -h_x2_div_d;
    }
    if r < 0.0 {
        h_x2_div_d = -h_x2_div_d;
    }

    Some((0.5 * (dx - dy * h_x2_div_d), 0.5 * (dy + dx * h_x2_div_d)))
}

/// Signed angular travel of an arc (negative for clockwise).
///
/// # Arguments
/// * `start_rel` - Start point relative to the centre.
/// * `end_rel` - End point relative to the centre.
/// * `clockwise` - `true` for G2, `false` for G3.
///
/// # Returns
/// The sweep in radians, in `(-TAU, 0)` / `(0, TAU)`; coincident start and end
/// points yield a full circle (`-TAU` or `TAU`).
pub fn angular_travel(start_rel: (f64, f64), end_rel: (f64, f64), clockwise: bool) -> f64 {
    let (rx, ry) = start_rel;
    let (tx, ty) = end_rel;
    let mut travel = (rx * ty - ry * tx).atan2(rx * tx + ry * ty);
    if clockwise {
        if travel >= -ARC_ANGULAR_TRAVEL_EPSILON {
            travel -= TAU;
        }
    } else if travel <= ARC_ANGULAR_TRAVEL_EPSILON {
        travel += TAU;
    }
    travel
}

/// Axis-aligned bounds of an arc in plane coordinates.
///
/// # Arguments
/// * `center` - Arc centre.
/// * `radius` - Arc radius.
/// * `start_angle` - Angle of the start point (radians).
/// * `sweep` - Signed angular travel (radians).
///
/// # Returns
/// `(min_x, min_y, max_x, max_y)` covering the endpoints and every cardinal
/// extreme the arc passes through.
pub fn arc_bounds(
    center: (f64, f64),
    radius: f64,
    start_angle: f64,
    sweep: f64,
) -> (f64, f64, f64, f64) {
    let end_angle = start_angle + sweep;
    let (sx, sy) = (
        center.0 + radius * start_angle.cos(),
        center.1 + radius * start_angle.sin(),
    );
    let (ex, ey) = (
        center.0 + radius * end_angle.cos(),
        center.1 + radius * end_angle.sin(),
    );
    let mut min_x = sx.min(ex);
    let mut max_x = sx.max(ex);
    let mut min_y = sy.min(ey);
    let mut max_y = sy.max(ey);

    for k in 0..4 {
        let angle = k as f64 * std::f64::consts::FRAC_PI_2;
        let rel = if sweep >= 0.0 {
            (angle - start_angle).rem_euclid(TAU)
        } else {
            (start_angle - angle).rem_euclid(TAU)
        };
        if rel <= sweep.abs() {
            let px = center.0 + radius * angle.cos();
            let py = center.1 + radius * angle.sin();
            min_x = min_x.min(px);
            max_x = max_x.max(px);
            min_y = min_y.min(py);
            max_y = max_y.max(py);
        }
    }

    (min_x, min_y, max_x, max_y)
}

/// Radius, start angle and signed sweep of an arc given by its centre and end points.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ArcSweep {
    /// Arc radius, measured from the centre to the start point.
    pub radius: f64,
    /// Angle of the start point around the centre (radians).
    pub start_angle: f64,
    /// Signed angular travel (radians, negative for clockwise).
    pub sweep: f64,
}

impl ArcSweep {
    /// Point on the arc at parameter `t` (0 = start, 1 = end) in plane coordinates.
    ///
    /// # Arguments
    /// * `center` - Arc centre.
    /// * `t` - Fraction of the sweep, normally in `[0, 1]`.
    ///
    /// # Returns
    /// The `(x, y)` point on the circle at that fraction of the sweep.
    pub fn point_at(&self, center: (f64, f64), t: f64) -> (f64, f64) {
        let angle = self.start_angle + self.sweep * t;
        (
            center.0 + self.radius * angle.cos(),
            center.1 + self.radius * angle.sin(),
        )
    }

    /// Number of straight segments needed to draw this arc so that no segment
    /// spans more than `max_segment_angle` radians.
    ///
    /// # Arguments
    /// * `max_segment_angle` - Largest angle (radians) a single segment may cover.
    /// * `min_segments` / `max_segments` - Clamp for the result.
    ///
    /// # Returns
    /// The segment count, clamped to `[min_segments, max_segments]`.
    pub fn segment_count(
        &self,
        max_segment_angle: f64,
        min_segments: usize,
        max_segments: usize,
    ) -> usize {
        let step = if max_segment_angle.is_finite() && max_segment_angle > 0.0 {
            max_segment_angle
        } else {
            std::f64::consts::PI / 18.0
        };
        let wanted = (self.sweep.abs() / step).ceil();
        let wanted = if wanted.is_finite() {
            wanted as usize
        } else {
            0
        };
        wanted.clamp(min_segments.max(1), max_segments.max(min_segments.max(1)))
    }
}

/// Compute the sweep of an arc from its centre, start and end points.
///
/// Uses [`angular_travel`], so coincident start and end points describe a full
/// circle, matching GRBL.
///
/// # Arguments
/// * `center` - Arc centre in plane coordinates.
/// * `start` - Start point in plane coordinates.
/// * `end` - End point in plane coordinates.
/// * `clockwise` - `true` for G2, `false` for G3.
///
/// # Returns
/// `Some(ArcSweep)`, or `None` when the radius is zero or not finite.
pub fn arc_sweep(
    center: (f64, f64),
    start: (f64, f64),
    end: (f64, f64),
    clockwise: bool,
) -> Option<ArcSweep> {
    let start_rel = (start.0 - center.0, start.1 - center.1);
    let end_rel = (end.0 - center.0, end.1 - center.1);
    let radius = (start_rel.0 * start_rel.0 + start_rel.1 * start_rel.1).sqrt();
    if !radius.is_finite() || radius < 1e-6 {
        return None;
    }
    Some(ArcSweep {
        radius,
        start_angle: start_rel.1.atan2(start_rel.0),
        sweep: angular_travel(start_rel, end_rel, clockwise),
    })
}
