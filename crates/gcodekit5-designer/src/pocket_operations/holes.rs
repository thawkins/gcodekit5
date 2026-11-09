//! Hole-aware (island-preserving) pocketing for [`PocketGenerator`].
//!
//! A pocket region is an outer boundary minus a set of holes (islands of material that must
//! be left uncut). The region is offset as a whole with
//! [`cavalier_contours::shape_algorithms::Shape`], so every generated tool-centre path keeps
//! at least one tool radius of clearance from the outer wall *and* from every hole.
//!
//! All three [`PocketStrategy`] variants are supported:
//! - **Contour-parallel**: successive shape offsets, cut outside-in.
//! - **Adaptive**: the same offset levels, cut inside-out.
//! - **Raster**: scanlines clipped (even-odd) against the tool-centre region, i.e. the shape
//!   offset by one tool radius, so spans stop short of holes as well as the outer wall.
//!
//! Moves between loops or spans stay at cutting depth only when the straight link remains
//! inside the tool-centre region; otherwise the tool retracts (rapid move) and re-enters,
//! using the same ramp/plunge rules as the hole-free pocket.
//!
//! Dependencies: `cavalier_contours` for polyline/shape offsetting.

use std::collections::BTreeSet;
use std::f64::consts::PI;
use std::panic;

use cavalier_contours::polyline::{
    seg_arc_radius_and_center, PlineSourceMut, PlineVertex, Polyline,
};
use cavalier_contours::shape_algorithms::{Shape, ShapeOffsetOptions};

use super::{add_center_cleanup, polygon_centroid, PocketGenerator, PocketStrategy};
use crate::model::Point;
use crate::toolpath::{Toolpath, ToolpathSegment, ToolpathSegmentType};

/// Closed loops (first point not repeated at the end) describing a region with even-odd fill.
type Loops = Vec<Vec<Point>>;

/// Distance (mm) under which a point is treated as lying on a region boundary.
const BOUNDARY_EPS: f64 = 1e-3;
/// Maximum chord deviation (mm) used when flattening offset arcs.
const ARC_TOLERANCE: f64 = 0.005;
/// Duplicate-vertex tolerance (mm) when building input polylines.
const DUPLICATE_TOLERANCE: f64 = 0.01;
/// Safety cap on the number of offset levels generated for one region.
const MAX_OFFSET_LEVELS: usize = 100_000;

/// Builds a closed polyline from `points` with the requested orientation.
///
/// Returns `None` for degenerate input (fewer than three distinct vertices or zero area).
fn to_pline(points: &[Point], ccw: bool) -> Option<Polyline> {
    let mut clean: Vec<Point> = Vec::with_capacity(points.len());
    for p in points {
        if clean
            .last()
            .is_none_or(|last: &Point| last.distance_to(p) > DUPLICATE_TOLERANCE)
        {
            clean.push(*p);
        }
    }
    while clean.len() > 1
        && clean
            .first()
            .zip(clean.last())
            .is_some_and(|(f, l)| f.distance_to(l) <= DUPLICATE_TOLERANCE)
    {
        clean.pop();
    }
    if clean.len() < 3 {
        return None;
    }

    let signed_area: f64 = (0..clean.len())
        .map(|i| {
            let a = clean[i];
            let b = clean[(i + 1) % clean.len()];
            a.x * b.y - b.x * a.y
        })
        .sum::<f64>()
        * 0.5;
    if signed_area.abs() < 1e-9 {
        return None;
    }
    if (signed_area > 0.0) != ccw {
        clean.reverse();
    }

    let mut pline = Polyline::new();
    for p in clean {
        pline.add_vertex(PlineVertex::new(p.x, p.y, 0.0));
    }
    pline.set_is_closed(true);
    Some(pline)
}

/// Offsets `shape` inwards (shrinking filled area, growing holes) by `distance`.
///
/// A panic inside the offset algorithm is treated as an empty result.
fn offset_shape(shape: &Shape<f64>, distance: f64) -> Shape<f64> {
    panic::catch_unwind(panic::AssertUnwindSafe(|| {
        shape.parallel_offset(distance, ShapeOffsetOptions::default())
    }))
    .unwrap_or_else(|_| {
        tracing::warn!(
            distance,
            "Shape offset failed while pocketing a region with holes"
        );
        Shape::empty()
    })
}

/// Flattens a closed polyline (including arc segments) into points.
fn flatten_pline(pline: &Polyline) -> Vec<Point> {
    let n = pline.vertex_data.len();
    let mut points = Vec::with_capacity(n);
    for i in 0..n {
        let v1 = pline.vertex_data[i];
        let v2 = pline.vertex_data[(i + 1) % n];
        points.push(Point::new(v1.x, v1.y));
        if v1.bulge.abs() < 1e-12 {
            continue;
        }
        let (radius, center) = seg_arc_radius_and_center(v1, v2);
        if !radius.is_finite() || radius < 1e-9 {
            continue;
        }
        let sweep = 4.0 * v1.bulge.atan();
        let max_step = if radius > ARC_TOLERANCE {
            2.0 * (1.0 - ARC_TOLERANCE / radius).acos()
        } else {
            PI / 2.0
        };
        let steps = (sweep.abs() / max_step.max(1e-3)).ceil().max(1.0) as usize;
        let start_angle = (v1.y - center.y).atan2(v1.x - center.x);
        for k in 1..steps {
            let a = start_angle + sweep * (k as f64) / (steps as f64);
            points.push(Point::new(
                center.x + radius * a.cos(),
                center.y + radius * a.sin(),
            ));
        }
    }
    points
}

/// Returns every loop of `shape` (outer boundaries first, then holes) as flattened points.
fn shape_loops(shape: &Shape<f64>) -> Loops {
    shape
        .ccw_plines
        .iter()
        .chain(shape.cw_plines.iter())
        .map(|ip| flatten_pline(&ip.polyline))
        .filter(|pts| pts.len() >= 2)
        .collect()
}

/// Distance from `p` to segment `a`-`b`.
fn dist_to_segment(p: &Point, a: &Point, b: &Point) -> f64 {
    let (dx, dy) = (b.x - a.x, b.y - a.y);
    let len_sq = dx * dx + dy * dy;
    if len_sq < 1e-18 {
        return p.distance_to(a);
    }
    let t = (((p.x - a.x) * dx + (p.y - a.y) * dy) / len_sq).clamp(0.0, 1.0);
    p.distance_to(&Point::new(a.x + t * dx, a.y + t * dy))
}

/// Even-odd containment test against `loops`; points within `eps` of a boundary count as inside.
fn region_contains(p: &Point, loops: &[Vec<Point>], eps: f64) -> bool {
    let mut inside = false;
    for lp in loops {
        let n = lp.len();
        for i in 0..n {
            let a = lp[i];
            let b = lp[(i + 1) % n];
            if dist_to_segment(p, &a, &b) <= eps {
                return true;
            }
            if (a.y > p.y) != (b.y > p.y) && p.x < (b.x - a.x) * (p.y - a.y) / (b.y - a.y) + a.x {
                inside = !inside;
            }
        }
    }
    inside
}

/// Signed area of the triangle `a`, `b`, `c` (times two).
fn orient(a: &Point, b: &Point, c: &Point) -> f64 {
    (b.x - a.x) * (c.y - a.y) - (b.y - a.y) * (c.x - a.x)
}

/// Returns true if the straight move `a` -> `b` stays inside the region bounded by `loops`.
///
/// The move is rejected if it properly crosses any boundary edge or if any interior sample
/// point lies outside the region.
fn segment_stays_inside(a: &Point, b: &Point, loops: &[Vec<Point>]) -> bool {
    const ORIENT_EPS: f64 = 1e-9;
    for lp in loops {
        let n = lp.len();
        for i in 0..n {
            let c = lp[i];
            let d = lp[(i + 1) % n];
            let o1 = orient(a, b, &c);
            let o2 = orient(a, b, &d);
            let o3 = orient(&c, &d, a);
            let o4 = orient(&c, &d, b);
            if ((o1 > ORIENT_EPS && o2 < -ORIENT_EPS) || (o1 < -ORIENT_EPS && o2 > ORIENT_EPS))
                && ((o3 > ORIENT_EPS && o4 < -ORIENT_EPS) || (o3 < -ORIENT_EPS && o4 > ORIENT_EPS))
            {
                return false;
            }
        }
    }
    [0.25, 0.5, 0.75].iter().all(|t| {
        let p = Point::new(a.x + (b.x - a.x) * t, a.y + (b.y - a.y) * t);
        region_contains(&p, loops, BOUNDARY_EPS)
    })
}

/// Rotates the closed loop `lp` so that it starts at the vertex nearest to `from`.
fn rotate_to_nearest(lp: &[Point], from: Option<Point>) -> Vec<Point> {
    let start = from
        .and_then(|f| {
            lp.iter()
                .enumerate()
                .min_by(|(_, a), (_, b)| {
                    a.distance_to(&f)
                        .partial_cmp(&b.distance_to(&f))
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
                .map(|(i, _)| i)
        })
        .unwrap_or(0);
    lp[start..]
        .iter()
        .chain(lp[..start].iter())
        .copied()
        .collect()
}

/// Orders loops level by level, greedily picking the loop nearest to the current position and
/// starting it at its nearest vertex. Each returned loop is closed (first point repeated).
fn order_loops(levels: &[Loops]) -> Vec<Vec<Point>> {
    let mut ordered = Vec::new();
    let mut pos: Option<Point> = None;
    for level in levels {
        let mut remaining: Vec<&Vec<Point>> = level.iter().collect();
        while !remaining.is_empty() {
            let idx = pos
                .map(|p| {
                    remaining
                        .iter()
                        .enumerate()
                        .map(|(i, lp)| {
                            let d = lp
                                .iter()
                                .map(|v| v.distance_to(&p))
                                .fold(f64::MAX, f64::min);
                            (i, d)
                        })
                        .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
                        .map(|(i, _)| i)
                        .unwrap_or(0)
                })
                .unwrap_or(0);
            let lp = remaining.remove(idx);
            let mut pts = rotate_to_nearest(lp, pos);
            if let Some(first) = pts.first().copied() {
                pts.push(first);
                pos = Some(first);
            }
            ordered.push(pts);
        }
    }
    ordered
}

/// Remaining raster spans indexed by scanline.
///
/// Spans keep their scan order inside each scanline bucket and are marked as taken instead
/// of being removed, so taking a span is O(1) and a connected link only has to look at the
/// two neighbouring buckets.
struct SpanIndex {
    /// Spans per scanline as `(start, end, taken)`.
    buckets: Vec<Vec<(Point, Point, bool)>>,
    /// Untaken spans per scanline.
    live_per_line: Vec<usize>,
    /// Scanlines that still have untaken spans, in scan order.
    live_lines: BTreeSet<usize>,
    /// Total untaken spans.
    remaining: usize,
}

impl SpanIndex {
    /// Builds the index from spans per scanline.
    fn new(lines: &[Vec<(Point, Point)>]) -> Self {
        let buckets: Vec<Vec<(Point, Point, bool)>> = lines
            .iter()
            .map(|spans| spans.iter().map(|&(a, b)| (a, b, false)).collect())
            .collect();
        let live_per_line: Vec<usize> = buckets.iter().map(Vec::len).collect();
        let live_lines = live_per_line
            .iter()
            .enumerate()
            .filter(|(_, &n)| n > 0)
            .map(|(li, _)| li)
            .collect();
        let remaining = live_per_line.iter().sum();
        Self {
            buckets,
            live_per_line,
            live_lines,
            remaining,
        }
    }

    /// Untaken spans on scanline `line` as `(span index, start, end)`, in scan order.
    fn live_on(&self, line: usize) -> impl Iterator<Item = (usize, Point, Point)> + '_ {
        self.buckets
            .get(line)
            .into_iter()
            .flatten()
            .enumerate()
            .filter(|(_, (_, _, taken))| !taken)
            .map(|(si, &(a, b, _))| (si, a, b))
    }

    /// Marks span `si` of scanline `line` as taken and returns it.
    fn take(&mut self, line: usize, si: usize) -> (Point, Point) {
        let span = &mut self.buckets[line][si];
        debug_assert!(!span.2, "raster span taken twice");
        span.2 = true;
        let (a, b) = (span.0, span.1);
        self.live_per_line[line] -= 1;
        if self.live_per_line[line] == 0 {
            self.live_lines.remove(&line);
        }
        self.remaining -= 1;
        (a, b)
    }
}

/// Orders raster spans so that each connected part of the region is finished before
/// moving to the next.
///
/// Scanlines that cross a hole split into several spans. Cutting them line by line
/// would jump across the hole (and retract) on every such line. Instead, from the
/// current position the next span is chosen on an adjacent scanline whose entry can
/// be reached by a straight link inside `allowed` (so no retract is needed); only when
/// no such span exists does the order jump to the nearest remaining span. With
/// `bidirectional` a span may be cut in either direction (whichever entry is closer);
/// otherwise every span keeps its original direction.
///
/// The remaining spans are indexed by scanline ([`SpanIndex`]), so a connected link only
/// examines the spans of the two adjacent scanlines; the scan over every remaining span is
/// done only for disconnected transitions. Ties are broken in scan order (lower scanline
/// first, then span order within a scanline).
///
/// # Arguments
/// * `lines` - Spans per scanline, in scan order.
/// * `allowed` - Tool-centre region loops used to validate links.
/// * `bidirectional` - Whether spans may be reversed.
///
/// # Returns
/// Every span exactly once, as `(start, end)` in cutting order.
fn order_raster_spans(
    lines: &[Vec<(Point, Point)>],
    allowed: &[Vec<Point>],
    bidirectional: bool,
) -> Vec<(Point, Point)> {
    let mut index = SpanIndex::new(lines);
    let mut ordered = Vec::with_capacity(index.remaining);
    let Some(&first_line) = index.live_lines.iter().next() else {
        return ordered;
    };

    // Entry options for a span: (distance, reversed).
    let entries = |pos: &Point, a: &Point, b: &Point| -> ([(f64, bool); 2], usize) {
        let options = [(pos.distance_to(a), false), (pos.distance_to(b), true)];
        (options, if bidirectional { 2 } else { 1 })
    };

    let (first_a, first_b) = index.take(first_line, 0);
    ordered.push((first_a, first_b));
    let mut pos = first_b;
    let mut line = first_line;

    while index.remaining > 0 {
        // Best linkable span on an adjacent scanline (lower scanline first, as in scan order).
        let mut best: Option<(usize, usize, bool, f64)> = None;
        let adjacent = [line.checked_sub(1), line.checked_add(1)];
        for li in adjacent.into_iter().flatten() {
            for (si, a, b) in index.live_on(li) {
                let (options, count) = entries(&pos, &a, &b);
                for &(d, rev) in &options[..count] {
                    if best.is_some_and(|(_, _, _, bd)| bd <= d) {
                        continue;
                    }
                    let entry = if rev { b } else { a };
                    if segment_stays_inside(&pos, &entry, allowed) {
                        best = Some((li, si, rev, d));
                    }
                }
            }
        }

        // Otherwise, the nearest remaining span anywhere (this may need a retract).
        let (li, si, rev) = match best {
            Some((li, si, rev, _)) => (li, si, rev),
            None => {
                let mut nearest: Option<(usize, usize, bool, f64)> = None;
                for &li in &index.live_lines {
                    for (si, a, b) in index.live_on(li) {
                        let (options, count) = entries(&pos, &a, &b);
                        for &(d, rev) in &options[..count] {
                            if nearest.is_none_or(|(_, _, _, nd)| d < nd) {
                                nearest = Some((li, si, rev, d));
                            }
                        }
                    }
                }
                match nearest {
                    Some((li, si, rev, _)) => (li, si, rev),
                    None => break,
                }
            }
        };

        let (a, b) = index.take(li, si);
        let (start, end) = if rev { (b, a) } else { (a, b) };
        ordered.push((start, end));
        pos = end;
        line = li;
    }
    ordered
}

/// Per-Z-pass bookkeeping shared by the hole-aware strategies.
struct PassState {
    /// Z of the previous pass (or the start depth) used for ramp entries.
    prev_z: f64,
    /// Z of the current pass.
    current_z: f64,
    /// Tool XY position at the end of the previous pass, if any.
    previous_pass_end: Option<Point>,
}

impl PocketGenerator {
    /// Generates a pocket for an outer boundary minus `holes`, keeping the tool at least one
    /// tool radius away from the outer wall and from every hole.
    ///
    /// Holes are treated as islands of material that must remain uncut. If no usable hole is
    /// supplied this is identical to [`PocketGenerator::generate_polygon_pocket`].
    ///
    /// # Arguments
    /// * `outer` - Outer boundary vertices (any orientation, closing vertex optional).
    /// * `holes` - Hole boundaries lying inside `outer` (any orientation).
    /// * `step_down` - Depth per Z pass in mm (`<= 0` cuts the full depth in one pass).
    ///
    /// # Returns
    /// One toolpath per Z pass.
    pub fn generate_polygon_pocket_with_holes(
        &self,
        outer: &[Point],
        holes: &[Vec<Point>],
        step_down: f64,
    ) -> Vec<Toolpath> {
        let hole_plines: Vec<Polyline> = holes.iter().filter_map(|h| to_pline(h, false)).collect();
        if hole_plines.is_empty() {
            return self.generate_polygon_pocket(outer, step_down);
        }
        let Some(outer_pline) = to_pline(outer, true) else {
            return Vec::new();
        };
        let shape = panic::catch_unwind(panic::AssertUnwindSafe(|| {
            Shape::from_plines(std::iter::once(outer_pline).chain(hole_plines))
        }));
        let Ok(shape) = shape else {
            tracing::warn!("Failed to build pocket region with holes; no pocket generated");
            return Vec::new();
        };

        match self.operation.strategy {
            PocketStrategy::ContourParallel => {
                self.offset_pocket_with_holes(&shape, step_down, false)
            }
            PocketStrategy::Adaptive => self.offset_pocket_with_holes(&shape, step_down, true),
            PocketStrategy::Raster {
                angle,
                bidirectional,
            } => self.raster_pocket_with_holes(&shape, step_down, angle, bidirectional),
        }
    }

    /// Computes the Z level of each pass, as the hole-free pocket strategies do.
    fn z_levels(&self, step_down: f64) -> Vec<f64> {
        let total_depth = self.operation.depth.abs();
        let z_step = if step_down > 0.0 {
            step_down
        } else {
            total_depth
        };
        if z_step <= 0.0 {
            return Vec::new();
        }
        let z_passes = (total_depth / z_step).ceil() as u32;
        (1..=z_passes)
            .map(|z_pass| self.operation.start_depth - (z_step * z_pass as f64).min(total_depth))
            .collect()
    }

    /// Generates successive inward offsets of `shape`, starting one tool radius in.
    fn offset_levels(&self, shape: &Shape<f64>) -> Vec<Loops> {
        let mut levels = Vec::new();
        let step = self.operation.stepover.max(0.01);
        let mut distance = self.operation.tool_diameter / 2.0;
        while levels.len() < MAX_OFFSET_LEVELS {
            let loops = shape_loops(&offset_shape(shape, distance));
            if loops.is_empty() {
                break;
            }
            levels.push(loops);
            distance += step;
        }
        levels
    }

    /// Moves the tool to `target` at cutting depth, linking from the current position when the
    /// straight move stays inside `allowed`, otherwise retracting and re-entering.
    fn link_or_enter(
        &self,
        toolpath: &mut Toolpath,
        target: Point,
        allowed: &[Vec<Point>],
        state: &PassState,
    ) {
        let first_in_pass = toolpath.segments.is_empty();
        let from = if first_in_pass {
            state.previous_pass_end
        } else {
            toolpath.segments.last().map(|s| s.end)
        };

        if first_in_pass && self.operation.ramp_angle > 0.0 {
            self.add_helical_ramp(toolpath, target, state.prev_z, state.current_z);
            return;
        }

        match from {
            Some(p) if segment_stays_inside(&p, &target, allowed) => {
                if !first_in_pass && p.distance_to(&target) < 1e-9 {
                    return;
                }
                // Linking at cutting depth (on a new pass the plunge happens at `p`, which is
                // inside the cleared region).
                toolpath.add_segment(ToolpathSegment::new(
                    ToolpathSegmentType::LinearMove,
                    p,
                    target,
                    self.operation.feed_rate,
                    self.operation.spindle_speed,
                ));
            }
            _ => {
                if self.operation.ramp_angle > 0.0 {
                    self.add_helical_ramp(toolpath, target, state.prev_z, state.current_z);
                } else {
                    toolpath.add_segment(ToolpathSegment::new(
                        ToolpathSegmentType::RapidMove,
                        Point::new(0.0, 0.0),
                        target,
                        self.operation.feed_rate,
                        self.operation.spindle_speed,
                    ));
                }
            }
        }
    }

    /// Adds the centre cleanup pattern only if it fits entirely inside `allowed`.
    fn add_center_cleanup_if_clear(
        &self,
        toolpath: &mut Toolpath,
        center: Point,
        allowed: &[Vec<Point>],
        depth: f64,
    ) {
        if self.operation.raster_fill_ratio <= 0.0 {
            return;
        }
        let radius = (self.operation.tool_diameter * 0.5).max(0.25);
        let fits = region_contains(&center, allowed, BOUNDARY_EPS)
            && (0..16).all(|k| {
                let a = 2.0 * PI * (k as f64) / 16.0;
                let p = Point::new(center.x + radius * a.cos(), center.y + radius * a.sin());
                region_contains(&p, allowed, BOUNDARY_EPS)
                    && segment_stays_inside(&center, &p, allowed)
            });
        if fits {
            add_center_cleanup(
                toolpath,
                center,
                self.operation.tool_diameter,
                depth,
                self.operation.feed_rate,
                self.operation.spindle_speed,
            );
        }
    }

    /// Contour-parallel (`inside_out == false`) or adaptive (`inside_out == true`) pocket of a
    /// region with holes.
    fn offset_pocket_with_holes(
        &self,
        shape: &Shape<f64>,
        step_down: f64,
        inside_out: bool,
    ) -> Vec<Toolpath> {
        let mut levels = self.offset_levels(shape);
        let Some(allowed) = levels.first().cloned() else {
            return Vec::new();
        };
        if inside_out {
            levels.reverse();
        }
        let loops = order_loops(&levels);

        // Contour-parallel cleans up around the last (innermost) loop, adaptive around the
        // first one, matching the hole-free strategies.
        let cleanup_center = if inside_out {
            loops.first()
        } else {
            loops.last()
        }
        .and_then(|pts| polygon_centroid(pts));

        let mut toolpaths = Vec::new();
        let mut state = PassState {
            prev_z: self.operation.start_depth,
            current_z: self.operation.start_depth,
            previous_pass_end: None,
        };

        for current_z in self.z_levels(step_down) {
            state.current_z = current_z;
            let mut toolpath = Toolpath::new(self.operation.tool_diameter, current_z);

            for pts in &loops {
                if pts.len() < 2 {
                    continue;
                }
                self.link_or_enter(&mut toolpath, pts[0], &allowed, &state);
                for window in pts.windows(2) {
                    toolpath.add_segment(ToolpathSegment::new(
                        ToolpathSegmentType::LinearMove,
                        window[0],
                        window[1],
                        self.operation.feed_rate,
                        self.operation.spindle_speed,
                    ));
                }
            }

            if let Some(center) = cleanup_center {
                self.add_center_cleanup_if_clear(&mut toolpath, center, &allowed, current_z);
            }

            state.previous_pass_end = toolpath.segments.last().map(|s| s.end);
            toolpaths.push(toolpath);
            state.prev_z = current_z;
        }
        toolpaths
    }

    /// Raster (zig-zag) pocket of a region with holes.
    ///
    /// Scanlines are clipped against the tool-centre region (the shape offset by one tool
    /// radius), so spans keep tool-radius clearance from the outer wall and from holes.
    fn raster_pocket_with_holes(
        &self,
        shape: &Shape<f64>,
        step_down: f64,
        angle: f64,
        bidirectional: bool,
    ) -> Vec<Toolpath> {
        let allowed = shape_loops(&offset_shape(shape, self.operation.tool_diameter / 2.0));
        if allowed.is_empty() {
            return Vec::new();
        }

        let (sin_r, cos_r) = (-angle).to_radians().sin_cos();
        let rotate = |p: &Point| Point::new(p.x * cos_r - p.y * sin_r, p.x * sin_r + p.y * cos_r);
        let (sin_i, cos_i) = angle.to_radians().sin_cos();
        let inv_rotate =
            |p: Point| Point::new(p.x * cos_i - p.y * sin_i, p.x * sin_i + p.y * cos_i);

        let rotated: Loops = allowed
            .iter()
            .map(|lp| lp.iter().map(rotate).collect())
            .collect();
        let (min_y, max_y) = rotated
            .iter()
            .flatten()
            .fold((f64::MAX, f64::MIN), |(lo, hi), p| {
                (lo.min(p.y), hi.max(p.y))
            });

        // Build the scanline spans once; they are identical for every Z pass.
        let step = self.operation.stepover.max(0.01);
        let mut lines: Vec<Vec<(Point, Point)>> = Vec::new();
        let mut y = min_y + BOUNDARY_EPS;
        let mut forward = true;
        while y <= max_y - BOUNDARY_EPS {
            let mut xs: Vec<f64> = Vec::new();
            for lp in &rotated {
                let n = lp.len();
                for i in 0..n {
                    let p1 = lp[i];
                    let p2 = lp[(i + 1) % n];
                    if ((p1.y <= y && p2.y > y) || (p2.y <= y && p1.y > y))
                        && (p2.y - p1.y).abs() > 1e-12
                    {
                        xs.push(p1.x + (y - p1.y) * (p2.x - p1.x) / (p2.y - p1.y));
                    }
                }
            }
            xs.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

            let mut spans: Vec<(Point, Point)> = xs
                .as_chunks::<2>()
                .0
                .iter()
                .filter(|c| c[1] - c[0] > 1e-6)
                .map(|c| {
                    let (sx, ex) = if bidirectional && !forward {
                        (c[1], c[0])
                    } else {
                        (c[0], c[1])
                    };
                    (inv_rotate(Point::new(sx, y)), inv_rotate(Point::new(ex, y)))
                })
                .filter(|(s, e)| !self.is_in_island(s) && !self.is_in_island(e))
                .collect();
            if bidirectional && !forward {
                spans.reverse();
            }
            lines.push(spans);
            y += step;
            forward = !forward;
        }

        let ordered = order_raster_spans(&lines, &allowed, bidirectional);

        let mut toolpaths = Vec::new();
        let mut state = PassState {
            prev_z: self.operation.start_depth,
            current_z: self.operation.start_depth,
            previous_pass_end: None,
        };

        for current_z in self.z_levels(step_down) {
            state.current_z = current_z;
            let mut toolpath = Toolpath::new(self.operation.tool_diameter, current_z);
            for (start, end) in &ordered {
                self.link_or_enter(&mut toolpath, *start, &allowed, &state);
                toolpath.add_segment(ToolpathSegment::new(
                    ToolpathSegmentType::LinearMove,
                    *start,
                    *end,
                    self.operation.feed_rate,
                    self.operation.spindle_speed,
                ));
            }
            state.previous_pass_end = toolpath.segments.last().map(|s| s.end);
            toolpaths.push(toolpath);
            state.prev_z = current_z;
        }
        toolpaths
    }
}
