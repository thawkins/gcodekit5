//! Pocket operations for CAM toolpath generation.
//!
//! Implements pocket milling operations with island detection and offset path generation.
//! Supports outline pocket and island preservation.

use super::toolpath::{Toolpath, ToolpathSegment, ToolpathSegmentType};
use crate::model::{DesignCircle as Circle, DesignRectangle as Rectangle, Point};
use crate::ops::clean_polyline;
use cavalier_contours::polyline::{PlineSource, PlineSourceMut, PlineVertex, Polyline};
use std::f64::consts::PI;
use std::panic;

fn add_center_cleanup(
    toolpath: &mut Toolpath,
    center: Point,
    tool_diameter: f64,
    depth: f64,
    feed_rate: f64,
    spindle_speed: u32,
) {
    let radius = (tool_diameter * 0.5).max(0.25);
    // Cross pattern that passes through the exact center to clear any core.
    let cleanup_points = [
        Point::new(center.x + radius, center.y),
        center,
        Point::new(center.x - radius, center.y),
        center,
        Point::new(center.x, center.y + radius),
        Point::new(center.x, center.y - radius),
        Point::new(center.x + radius, center.y),
    ];

    toolpath.add_segment(ToolpathSegment::new(
        ToolpathSegmentType::RapidMove,
        Point::new(0.0, 0.0),
        cleanup_points[0],
        feed_rate,
        spindle_speed,
    ));

    for window in cleanup_points.windows(2) {
        let mut seg = ToolpathSegment::new(
            ToolpathSegmentType::LinearMove,
            window[0],
            window[1],
            feed_rate,
            spindle_speed,
        );
        seg.start_z = Some(depth);
        seg.z_depth = Some(depth);
        toolpath.add_segment(seg);
    }

    // Add a small circular sweep to fully wipe the center.
    let start_pt = Point::new(center.x + radius, center.y);

    // Ensure we start the loop at the right point
    let mut lead_in = ToolpathSegment::new(
        ToolpathSegmentType::LinearMove,
        cleanup_points.last().copied().unwrap_or(start_pt),
        start_pt,
        feed_rate,
        spindle_speed,
    );
    lead_in.start_z = Some(depth);
    lead_in.z_depth = Some(depth);
    toolpath.add_segment(lead_in);

    let mut current = start_pt;
    for target in [
        Point::new(center.x, center.y + radius),
        Point::new(center.x - radius, center.y),
        Point::new(center.x, center.y - radius),
        start_pt,
    ] {
        let mut seg = ToolpathSegment::new_arc(
            ToolpathSegmentType::ArcCCW,
            current,
            target,
            center,
            feed_rate,
            spindle_speed,
        );
        seg.start_z = Some(depth);
        seg.z_depth = Some(depth);
        toolpath.add_segment(seg);
        current = target;
    }
}

fn polygon_centroid(points: &[Point]) -> Option<Point> {
    if points.len() < 3 {
        return None;
    }

    let mut area = 0.0;
    let mut cx = 0.0;
    let mut cy = 0.0;

    for i in 0..points.len() - 1 {
        let p1 = points[i];
        let p2 = points[i + 1];
        let cross = p1.x * p2.y - p2.x * p1.y;
        area += cross;
        cx += (p1.x + p2.x) * cross;
        cy += (p1.y + p2.y) * cross;
    }

    if area.abs() < 1e-6 {
        return None;
    }

    area *= 0.5;
    cx /= 6.0 * area;
    cy /= 6.0 * area;
    Some(Point::new(cx, cy))
}

fn point_in_polygon(poly: &[Point], test: Point) -> bool {
    // Standard ray casting test.
    let mut inside = false;
    let mut j = poly.len().saturating_sub(1);
    for i in 0..poly.len() {
        let pi = poly[i];
        let pj = poly[j];
        if ((pi.y > test.y) != (pj.y > test.y))
            && (test.x < (pj.x - pi.x) * (test.y - pi.y) / (pj.y - pi.y + f64::EPSILON) + pi.x)
        {
            inside = !inside;
        }
        j = i;
    }
    inside
}

/// Strategy for pocket milling.
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum PocketStrategy {
    /// Zig-Zag or Raster milling.
    Raster { angle: f64, bidirectional: bool },
    /// Contour-parallel (offset) milling.
    ContourParallel,
    /// Adaptive clearing (trochoidal-like).
    Adaptive,
}

impl Default for PocketStrategy {
    fn default() -> Self {
        PocketStrategy::ContourParallel
    }
}

/// Represents a pocket operation configuration.
#[derive(Debug, Clone)]
pub struct PocketOperation {
    pub id: String,
    pub depth: f64,
    pub start_depth: f64,
    pub tool_diameter: f64,
    pub stepover: f64,
    pub feed_rate: f64,
    pub spindle_speed: u32,
    pub climb_milling: bool,
    pub strategy: PocketStrategy,
    pub ramp_angle: f64,
    pub raster_fill_ratio: f64,
}

impl PocketOperation {
    /// Creates a new pocket operation with default parameters.
    pub fn new(id: String, depth: f64, tool_diameter: f64) -> Self {
        Self {
            id,
            depth,
            start_depth: 0.0,
            tool_diameter,
            stepover: tool_diameter / 2.0,
            feed_rate: 100.0,
            spindle_speed: 10000,
            climb_milling: false,
            strategy: PocketStrategy::ContourParallel,
            ramp_angle: 0.0,
            raster_fill_ratio: 0.5,
        }
    }

    /// Sets the start depth (top of stock).
    pub fn set_start_depth(&mut self, start_depth: f64) {
        self.start_depth = start_depth;
    }

    /// Sets the cutting parameters for this pocket operation.
    pub fn set_parameters(&mut self, stepover: f64, feed_rate: f64, spindle_speed: u32) {
        self.stepover = stepover;
        self.feed_rate = feed_rate;
        self.spindle_speed = spindle_speed;
    }

    /// Enables or disables climb milling.
    pub fn set_climb_milling(&mut self, enable: bool) {
        self.climb_milling = enable;
    }

    /// Sets the pocket strategy.
    pub fn set_strategy(&mut self, strategy: PocketStrategy) {
        self.strategy = strategy;
    }

    /// Sets the ramp angle in degrees.
    pub fn set_ramp_angle(&mut self, angle: f64) {
        self.ramp_angle = angle;
    }

    /// Sets raster fill percentage where 100 keeps full strokes and 0 removes them.
    pub fn set_raster_fill_percent(&mut self, percent: f64) {
        self.raster_fill_ratio = (percent / 100.0).clamp(0.0, 1.0);
    }

    /// Calculates the offset distance for the given pass number.
    pub fn calculate_offset(&self, pass: u32) -> f64 {
        self.stepover * pass as f64
    }
}

/// Represents an island within a pocket.
#[derive(Debug, Clone)]
pub struct Island {
    pub center: Point,
    pub radius: f64,
}

impl Island {
    /// Creates a new island.
    pub fn new(center: Point, radius: f64) -> Self {
        Self { center, radius }
    }

    /// Checks if a point is inside the island.
    pub fn contains_point(&self, point: &Point) -> bool {
        self.center.distance_to(point) <= self.radius
    }
}

/// Generates pocket toolpaths with island detection.
pub struct PocketGenerator {
    pub operation: PocketOperation,
    pub islands: Vec<Island>,
}

impl PocketGenerator {
    /// Creates a new pocket generator.
    pub fn new(operation: PocketOperation) -> Self {
        Self {
            operation,
            islands: Vec::new(),
        }
    }

    fn generate_raster_cleanup(
        &self,
        vertices: &[Point],
        depth: f64,
        angle: f64,
    ) -> Option<Toolpath> {
        if vertices.is_empty() {
            return None;
        }

        let tool_radius = self.operation.tool_diameter / 2.0;
        let step = (self.operation.stepover).max(0.1);
        let inset = step * 0.25; // 25% inset of stroke width on each side to stay off walls

        // Rotate vertices to align with X axis for scanlines.
        let cos_a = (-angle).to_radians().cos();
        let sin_a = (-angle).to_radians().sin();

        let rotate = |p: Point| -> Point {
            Point::new(p.x * cos_a - p.y * sin_a, p.x * sin_a + p.y * cos_a)
        };

        let inv_rotate = |p: Point| -> Point {
            let cos_inv = angle.to_radians().cos();
            let sin_inv = angle.to_radians().sin();
            Point::new(p.x * cos_inv - p.y * sin_inv, p.x * sin_inv + p.y * cos_inv)
        };

        let rotated: Vec<Point> = vertices.iter().map(|&p| rotate(p)).collect();
        if rotated.len() < 3 {
            return None;
        }

        let mut min_x = rotated[0].x;
        let mut min_y = rotated[0].y;
        let mut max_x = rotated[0].x;
        let mut max_y = rotated[0].y;
        for v in &rotated {
            min_x = min_x.min(v.x);
            min_y = min_y.min(v.y);
            max_x = max_x.max(v.x);
            max_y = max_y.max(v.y);
        }

        let mut toolpath = Toolpath::new(self.operation.tool_diameter, depth);
        let mut current_y = min_y + tool_radius + inset;
        let limit_y = max_y - tool_radius - inset;
        if current_y > limit_y {
            return None;
        }
        let mut forward = true;

        while current_y <= limit_y {
            let mut intersections = Vec::new();
            for i in 0..rotated.len() {
                let p1 = rotated[i];
                let p2 = rotated[(i + 1) % rotated.len()];
                if (p1.y <= current_y && p2.y > current_y)
                    || (p2.y <= current_y && p1.y > current_y)
                {
                    let denom = (p2.y - p1.y).abs();
                    if denom > 1e-9 {
                        let x = p1.x + (current_y - p1.y) * (p2.x - p1.x) / (p2.y - p1.y);
                        intersections.push(x);
                    }
                }
            }
            intersections.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

            let mut segments = Vec::new();
            for i in (0..intersections.len()).step_by(2) {
                if i + 1 < intersections.len() {
                    let x_start = intersections[i] + tool_radius;
                    let x_end = intersections[i + 1] - tool_radius;
                    if x_start < x_end {
                        let span = x_end - x_start;
                        let trim = span * (1.0 - self.operation.raster_fill_ratio) / 2.0;
                        let adj_start = x_start + trim;
                        let adj_end = x_end - trim;
                        if adj_start < adj_end {
                            segments.push((adj_start, adj_end));
                        }
                    }
                }
            }

            if !forward {
                segments.reverse();
            }

            for (start_x, end_x) in segments {
                let start_rot = Point::new(start_x, current_y);
                let end_rot = Point::new(end_x, current_y);
                let start_pt = inv_rotate(start_rot);
                let end_pt = inv_rotate(end_rot);

                // Skip if outside original polygon (safety check for concave cases)
                if !point_in_polygon(vertices, start_pt) && !point_in_polygon(vertices, end_pt) {
                    continue;
                }

                let needs_rapid = toolpath
                    .segments
                    .last()
                    .map(|s| s.end.distance_to(&start_pt) > self.operation.tool_diameter * 1.5)
                    .unwrap_or(true);

                if needs_rapid {
                    let mut rapid = ToolpathSegment::new(
                        ToolpathSegmentType::RapidMove,
                        toolpath.segments.last().map(|s| s.end).unwrap_or(start_pt),
                        start_pt,
                        self.operation.feed_rate,
                        self.operation.spindle_speed,
                    );
                    rapid.start_z = Some(depth);
                    rapid.z_depth = Some(depth);
                    toolpath.add_segment(rapid);
                }

                let mut cut = ToolpathSegment::new(
                    ToolpathSegmentType::LinearMove,
                    start_pt,
                    end_pt,
                    self.operation.feed_rate,
                    self.operation.spindle_speed,
                );
                cut.start_z = Some(depth);
                cut.z_depth = Some(depth);
                toolpath.add_segment(cut);
            }

            current_y += step;
            forward = !forward;
        }

        if toolpath.segments.is_empty() {
            None
        } else {
            Some(toolpath)
        }
    }

    /// Adds an island to the pocket.
    pub fn add_island(&mut self, island: Island) {
        self.islands.push(island);
    }

    /// Adds a circular island.
    pub fn add_circular_island(&mut self, center: Point, radius: f64) {
        self.add_island(Island::new(center, radius));
    }

    /// Clears all islands.
    pub fn clear_islands(&mut self) {
        self.islands.clear();
    }

    /// Checks if a point is in any island.
    fn is_in_island(&self, point: &Point) -> bool {
        self.islands
            .iter()
            .any(|island| island.contains_point(point))
    }

    fn add_helical_ramp(&self, toolpath: &mut Toolpath, center: Point, start_z: f64, end_z: f64) {
        let radius = self.operation.tool_diameter / 4.0;
        let ramp_angle_rad = self.operation.ramp_angle.to_radians();
        let z_drop_per_rev = 2.0 * PI * radius * ramp_angle_rad.tan();

        if z_drop_per_rev < 0.001 {
            // Angle too shallow or radius too small, just plunge
            // Rapid to center
            toolpath.add_segment(ToolpathSegment::new(
                ToolpathSegmentType::RapidMove,
                Point::new(0.0, 0.0),
                center,
                self.operation.feed_rate,
                self.operation.spindle_speed,
            ));
            // Plunge
            let mut plunge = ToolpathSegment::new(
                ToolpathSegmentType::LinearMove,
                center,
                center,
                self.operation.feed_rate / 2.0,
                self.operation.spindle_speed,
            );
            plunge.start_z = Some(start_z);
            plunge.z_depth = Some(end_z);
            toolpath.add_segment(plunge);
            return;
        }

        let total_drop = start_z - end_z; // Positive
        let revs = (total_drop / z_drop_per_rev).ceil() as u32;
        let actual_drop_per_rev = total_drop / revs as f64;

        // Start point of helix
        let start_pt = Point::new(center.x + radius, center.y);

        // Rapid to start of helix at start_z
        let mut rapid = ToolpathSegment::new(
            ToolpathSegmentType::RapidMove,
            Point::new(0.0, 0.0),
            start_pt,
            self.operation.feed_rate,
            self.operation.spindle_speed,
        );
        rapid.z_depth = Some(start_z);
        toolpath.add_segment(rapid);

        let mut current_z = start_z;
        let mut current_pt = start_pt;

        for _ in 0..revs {
            let next_z = current_z - actual_drop_per_rev;

            // Full circle arc (broken into 2 halves or handled by G-code gen?)
            // G-code gen handles full circle if start != end? No.
            // If start == end, it might be full circle.
            // But here we are spiraling, so Z changes.
            // Let's do 4 quadrants to be safe and consistent.

            let p1 = Point::new(center.x, center.y + radius);
            let p2 = Point::new(center.x - radius, center.y);
            let p3 = Point::new(center.x, center.y - radius);
            let p4 = Point::new(center.x + radius, center.y);

            let drop_per_quad = actual_drop_per_rev / 4.0;

            let points = [p1, p2, p3, p4];
            for i in 0..4 {
                let target = points[i];
                let target_z = current_z - drop_per_quad * (i as f64 + 1.0);

                let mut arc = ToolpathSegment::new_arc(
                    ToolpathSegmentType::ArcCCW,
                    current_pt,
                    target,
                    center,
                    self.operation.feed_rate,
                    self.operation.spindle_speed,
                );
                arc.start_z = Some(current_z - drop_per_quad * i as f64);
                arc.z_depth = Some(target_z);
                toolpath.add_segment(arc);
                current_pt = target;
            }
            current_z = next_z;
        }

        // Move to center at bottom
        let mut to_center = ToolpathSegment::new(
            ToolpathSegmentType::LinearMove,
            current_pt,
            center,
            self.operation.feed_rate,
            self.operation.spindle_speed,
        );
        to_center.start_z = Some(end_z);
        to_center.z_depth = Some(end_z);
        toolpath.add_segment(to_center);
    }

    /// Generates a pocket toolpath for a rectangular outline.
    pub fn generate_rectangular_pocket(&self, rect: &Rectangle, step_down: f64) -> Vec<Toolpath> {
        if let PocketStrategy::ContourParallel = self.operation.strategy {
            let mut toolpaths = Vec::new();

            let half_tool = self.operation.tool_diameter / 2.0;
            let max_offset = rect.width.min(rect.height) / 2.0;

            // Calculate Z passes
            let total_depth = self.operation.depth.abs();
            let z_step = if step_down > 0.0 {
                step_down
            } else {
                total_depth
            };
            let z_passes = (total_depth / z_step).ceil() as u32;
            let mut prev_z = self.operation.start_depth;

            for z_pass in 1..=z_passes {
                let current_z =
                    self.operation.start_depth - (z_step * z_pass as f64).min(total_depth);
                let mut toolpath = Toolpath::new(self.operation.tool_diameter, current_z);

                // Start from the outside (boundary) and work inwards
                let mut current_offset = half_tool;

                while current_offset < max_offset {
                    let inset_x = (rect.center.x - rect.width / 2.0) + current_offset;
                    let inset_y = (rect.center.y - rect.height / 2.0) + current_offset;
                    let inset_width = (rect.width - 2.0 * current_offset).max(0.0);
                    let inset_height = (rect.height - 2.0 * current_offset).max(0.0);

                    if inset_width <= 0.0 || inset_height <= 0.0 {
                        break;
                    }

                    let points = [
                        Point::new(inset_x, inset_y),
                        Point::new(inset_x + inset_width, inset_y),
                        Point::new(inset_x + inset_width, inset_y + inset_height),
                        Point::new(inset_x, inset_y + inset_height),
                        Point::new(inset_x, inset_y),
                    ];

                    // Add rapid move to start of this loop to avoid cutting across
                    if let Some(first_point) = points.first() {
                        if self.operation.ramp_angle > 0.0 {
                            self.add_helical_ramp(&mut toolpath, *first_point, prev_z, current_z);
                        } else {
                            toolpath.add_segment(ToolpathSegment::new(
                                ToolpathSegmentType::RapidMove,
                                Point::new(0.0, 0.0), // Start point ignored for Rapid in current logic? No, it uses end.
                                *first_point,
                                self.operation.feed_rate,
                                self.operation.spindle_speed,
                            ));
                        }
                    }

                    for window in points.windows(2) {
                        let start = window[0];
                        let end = window[1];

                        if !self.is_in_island(&start) && !self.is_in_island(&end) {
                            let segment = ToolpathSegment::new(
                                ToolpathSegmentType::LinearMove,
                                start,
                                end,
                                self.operation.feed_rate,
                                self.operation.spindle_speed,
                            );
                            toolpath.add_segment(segment);
                        }
                    }

                    current_offset += self.operation.stepover;
                }
                toolpaths.push(toolpath);
                prev_z = current_z;
            }

            toolpaths
        } else {
            // Convert to polygon and use generic generator
            let vertices = vec![
                Point::new(
                    rect.center.x - rect.width / 2.0,
                    rect.center.y - rect.height / 2.0,
                ),
                Point::new(
                    (rect.center.x - rect.width / 2.0) + rect.width,
                    rect.center.y - rect.height / 2.0,
                ),
                Point::new(
                    (rect.center.x - rect.width / 2.0) + rect.width,
                    (rect.center.y - rect.height / 2.0) + rect.height,
                ),
                Point::new(
                    rect.center.x - rect.width / 2.0,
                    (rect.center.y - rect.height / 2.0) + rect.height,
                ),
            ];
            self.generate_polygon_pocket(&vertices, step_down)
        }
    }

    /// Generates a pocket toolpath for a circular outline.
    pub fn generate_circular_pocket(&self, circle: &Circle, step_down: f64) -> Vec<Toolpath> {
        if let PocketStrategy::ContourParallel = self.operation.strategy {
            let mut toolpaths = Vec::new();

            let half_tool = self.operation.tool_diameter / 2.0;
            let max_offset = circle.radius;

            // Calculate Z passes
            let total_depth = self.operation.depth.abs();
            let z_step = if step_down > 0.0 {
                step_down
            } else {
                total_depth
            };
            let z_passes = (total_depth / z_step).ceil() as u32;
            let mut prev_z = self.operation.start_depth;

            for z_pass in 1..=z_passes {
                let current_z =
                    self.operation.start_depth - (z_step * z_pass as f64).min(total_depth);
                let mut toolpath = Toolpath::new(self.operation.tool_diameter, current_z);

                let segments = 36;
                let mut current_offset = half_tool;

                while current_offset < max_offset {
                    let inset_radius = circle.radius - current_offset;
                    if inset_radius <= 0.0 {
                        break;
                    }

                    // Add rapid move to start of circle
                    let start_angle: f64 = 0.0;
                    let start_x = circle.center.x + inset_radius * start_angle.cos();
                    let start_y = circle.center.y + inset_radius * start_angle.sin();
                    let start_pt = Point::new(start_x, start_y);

                    if self.operation.ramp_angle > 0.0 {
                        self.add_helical_ramp(&mut toolpath, start_pt, prev_z, current_z);
                    } else {
                        toolpath.add_segment(ToolpathSegment::new(
                            ToolpathSegmentType::RapidMove,
                            Point::new(0.0, 0.0),
                            start_pt,
                            self.operation.feed_rate,
                            self.operation.spindle_speed,
                        ));
                    }

                    for i in 0..segments {
                        let angle1 = (i as f64 / segments as f64) * 2.0 * PI;
                        let angle2 = ((i + 1) as f64 / segments as f64) * 2.0 * PI;

                        let x1 = circle.center.x + inset_radius * angle1.cos();
                        let y1 = circle.center.y + inset_radius * angle1.sin();
                        let x2 = circle.center.x + inset_radius * angle2.cos();
                        let y2 = circle.center.y + inset_radius * angle2.sin();

                        let start = Point::new(x1, y1);
                        let end = Point::new(x2, y2);

                        if !self.is_in_island(&start) && !self.is_in_island(&end) {
                            let segment = ToolpathSegment::new(
                                ToolpathSegmentType::LinearMove,
                                start,
                                end,
                                self.operation.feed_rate,
                                self.operation.spindle_speed,
                            );
                            toolpath.add_segment(segment);
                        }
                    }

                    current_offset += self.operation.stepover;
                }
                // Final center cleanup to eliminate residual core
                add_center_cleanup(
                    &mut toolpath,
                    circle.center,
                    self.operation.tool_diameter,
                    current_z,
                    self.operation.feed_rate,
                    self.operation.spindle_speed,
                );
                toolpaths.push(toolpath);
                prev_z = current_z;
            }

            toolpaths
        } else {
            // Convert to polygon (approximate circle)
            let segments = 64;
            let mut vertices = Vec::with_capacity(segments);
            for i in 0..segments {
                let angle = (i as f64 / segments as f64) * 2.0 * PI;
                let x = circle.center.x + circle.radius * angle.cos();
                let y = circle.center.y + circle.radius * angle.sin();
                vertices.push(Point::new(x, y));
            }
            self.generate_polygon_pocket(&vertices, step_down)
        }
    }

    /// Generates a pocket toolpath for a polygon defined by vertices.
    pub fn generate_polygon_pocket(&self, vertices: &[Point], step_down: f64) -> Vec<Toolpath> {
        match self.operation.strategy {
            PocketStrategy::Raster {
                angle,
                bidirectional,
            } => self.generate_raster_pocket(vertices, step_down, angle, bidirectional),
            PocketStrategy::ContourParallel => {
                self.generate_contour_parallel_pocket(vertices, step_down)
            }
            PocketStrategy::Adaptive => self.generate_adaptive_pocket(vertices, step_down),
        }
    }

    /// Prepares a polygon for offsetting by removing duplicates and enforcing CW orientation.
    fn prepare_polygon(vertices: &[Point]) -> Polyline {
        let mut clean_vertices = Vec::new();
        let tolerance = 0.01; // 0.01mm tolerance for duplicate detection

        if let Some(first) = vertices.first() {
            clean_vertices.push(*first);
            for p in vertices.iter().skip(1) {
                let last = clean_vertices.last().unwrap();
                let dist = ((p.x - last.x).powi(2) + (p.y - last.y).powi(2)).sqrt();
                if dist > tolerance {
                    clean_vertices.push(*p);
                }
            }

            // Remove closing vertex if it's the same as the first
            if clean_vertices.len() > 1 {
                let first = clean_vertices.first().unwrap();
                let last = clean_vertices.last().unwrap();
                let dist = ((last.x - first.x).powi(2) + (last.y - first.y).powi(2)).sqrt();
                if dist < tolerance {
                    clean_vertices.pop();
                }
            }
        }

        let mut signed_area = 0.0;
        if !clean_vertices.is_empty() {
            for i in 0..clean_vertices.len() {
                let p1 = clean_vertices[i];
                let p2 = clean_vertices[(i + 1) % clean_vertices.len()];
                signed_area += p1.x * p2.y - p2.x * p1.y;
            }
        }

        if signed_area > 0.0 {
            clean_vertices.reverse();
        }

        let mut polyline = Polyline::new();
        for p in clean_vertices {
            polyline.add_vertex(PlineVertex::new(p.x, p.y, 0.0));
        }
        polyline.set_is_closed(true);
        polyline
    }

    fn generate_contour_parallel_pocket(
        &self,
        vertices: &[Point],
        step_down: f64,
    ) -> Vec<Toolpath> {
        let mut toolpaths = Vec::new();
        if vertices.is_empty() {
            return toolpaths;
        }

        let polyline = Self::prepare_polygon(vertices);

        // Calculate Z passes
        let total_depth = self.operation.depth.abs();
        let z_step = if step_down > 0.0 {
            step_down
        } else {
            total_depth
        };
        let z_passes = (total_depth / z_step).ceil() as u32;
        let tool_radius = self.operation.tool_diameter / 2.0;
        let mut prev_z = self.operation.start_depth;

        for z_pass in 1..=z_passes {
            let current_z = self.operation.start_depth - (z_step * z_pass as f64).min(total_depth);
            let mut toolpath = Toolpath::new(self.operation.tool_diameter, current_z);

            let mut current_offset = tool_radius;
            let has_paths = true;
            let mut last_loop_centroid: Option<Point> = None;

            while has_paths {
                // Offset inwards
                let polyline = clean_polyline(polyline.clone());
                let offsets = panic::catch_unwind(panic::AssertUnwindSafe(|| {
                    polyline.parallel_offset(-current_offset)
                }))
                .unwrap_or_default();

                if offsets.is_empty() {
                    break;
                }

                for (_path_idx, offset_path) in offsets.iter().enumerate() {
                    let mut points = Vec::new();
                    for v in &offset_path.vertex_data {
                        points.push(Point::new(v.x, v.y));
                    }

                    if points.len() < 2 {
                        continue;
                    }

                    // Close the loop
                    points.push(points[0]);

                    last_loop_centroid = polygon_centroid(&points);

                    if self.operation.ramp_angle > 0.0 {
                        self.add_helical_ramp(&mut toolpath, points[0], prev_z, current_z);
                    } else {
                        // Add rapid to start
                        toolpath.add_segment(ToolpathSegment::new(
                            ToolpathSegmentType::RapidMove,
                            Point::new(0.0, 0.0),
                            points[0],
                            self.operation.feed_rate,
                            self.operation.spindle_speed,
                        ));
                    }

                    // Add segments
                    for window in points.windows(2) {
                        toolpath.add_segment(ToolpathSegment::new(
                            ToolpathSegmentType::LinearMove,
                            window[0],
                            window[1],
                            self.operation.feed_rate,
                            self.operation.spindle_speed,
                        ));
                    }
                }

                current_offset += self.operation.stepover;
            }

            // Final cleanup to remove residual core at the center
            if let Some(center_pt) = last_loop_centroid {
                add_center_cleanup(
                    &mut toolpath,
                    center_pt,
                    self.operation.tool_diameter,
                    current_z,
                    self.operation.feed_rate,
                    self.operation.spindle_speed,
                );
            }

            // Sweep a raster cleanup over the whole polygon to catch any voids
            if let Some(raster) = self.generate_raster_cleanup(vertices, current_z, 0.0) {
                toolpaths.push(raster);
            }
            toolpaths.push(toolpath);
            prev_z = current_z;
        }
        toolpaths
    }

    fn generate_adaptive_pocket(&self, vertices: &[Point], step_down: f64) -> Vec<Toolpath> {
        let mut toolpaths = Vec::new();
        if vertices.is_empty() {
            return toolpaths;
        }

        let polyline = Self::prepare_polygon(vertices);

        // Calculate Z passes
        let total_depth = self.operation.depth.abs();
        let z_step = if step_down > 0.0 {
            step_down
        } else {
            total_depth
        };
        let z_passes = (total_depth / z_step).ceil() as u32;
        let tool_radius = self.operation.tool_diameter / 2.0;

        // Generate all offset levels first (Outside-In)
        let mut levels = Vec::new();
        let mut current_offset = tool_radius;
        let mut innermost_centroid: Option<Point> = None;

        loop {
            let polyline = clean_polyline(polyline.clone());
            let offsets = panic::catch_unwind(panic::AssertUnwindSafe(|| {
                polyline.parallel_offset(-current_offset)
            }))
            .unwrap_or_default();

            if offsets.is_empty() {
                break;
            }
            levels.push(offsets);
            current_offset += self.operation.stepover;
        }

        // Reverse levels to go Inside-Out
        levels.reverse();

        if let Some(first_level) = levels.first() {
            if let Some(first_path) = first_level.first() {
                if !first_path.vertex_data.is_empty() {
                    let mut pts: Vec<Point> = first_path
                        .vertex_data
                        .iter()
                        .map(|v| Point::new(v.x, v.y))
                        .collect();
                    if let Some(first) = pts.first().copied() {
                        pts.push(first);
                    }
                    innermost_centroid = polygon_centroid(&pts);
                }
            }
        }

        let mut prev_z = self.operation.start_depth;

        for z_pass in 1..=z_passes {
            let current_z = self.operation.start_depth - (z_step * z_pass as f64).min(total_depth);
            let mut toolpath = Toolpath::new(self.operation.tool_diameter, current_z);

            // Helical Entry for the first (innermost) level
            if let Some(first_level) = levels.first() {
                if let Some(first_path) = first_level.first() {
                    if !first_path.vertex_data.is_empty() {
                        let start_pt =
                            Point::new(first_path.vertex_data[0].x, first_path.vertex_data[0].y);

                        if self.operation.ramp_angle > 0.0 {
                            self.add_helical_ramp(&mut toolpath, start_pt, prev_z, current_z);
                        } else {
                            // Rapid to start XY, Safe Z
                            toolpath.add_segment(ToolpathSegment::new(
                                ToolpathSegmentType::RapidMove,
                                Point::new(0.0, 0.0),
                                start_pt,
                                self.operation.feed_rate,
                                self.operation.spindle_speed,
                            ));
                        }
                    }
                }
            }

            for level in &levels {
                for offset_path in level {
                    let mut points = Vec::new();
                    for v in &offset_path.vertex_data {
                        points.push(Point::new(v.x, v.y));
                    }

                    if points.len() < 2 {
                        continue;
                    }

                    // Close the loop
                    points.push(points[0]);

                    // If not connected to previous point, rapid move
                    // (In a real adaptive path, we would link these smoothly)
                    let start_pt = points[0];
                    let needs_rapid = if let Some(last_seg) = toolpath.segments.last() {
                        last_seg.end.distance_to(&start_pt) > 0.1
                    } else {
                        true
                    };

                    if needs_rapid {
                        toolpath.add_segment(ToolpathSegment::new(
                            ToolpathSegmentType::RapidMove,
                            Point::new(0.0, 0.0),
                            start_pt,
                            self.operation.feed_rate,
                            self.operation.spindle_speed,
                        ));
                    }

                    // Add segments
                    for window in points.windows(2) {
                        toolpath.add_segment(ToolpathSegment::new(
                            ToolpathSegmentType::LinearMove,
                            window[0],
                            window[1],
                            self.operation.feed_rate,
                            self.operation.spindle_speed,
                        ));
                    }
                }
            }
            if let Some(center_pt) = innermost_centroid {
                add_center_cleanup(
                    &mut toolpath,
                    center_pt,
                    self.operation.tool_diameter,
                    current_z,
                    self.operation.feed_rate,
                    self.operation.spindle_speed,
                );
            }

            if let Some(center_pt) = innermost_centroid {
                add_center_cleanup(
                    &mut toolpath,
                    center_pt,
                    self.operation.tool_diameter,
                    current_z,
                    self.operation.feed_rate,
                    self.operation.spindle_speed,
                );
            }

            if let Some(raster) = self.generate_raster_cleanup(vertices, current_z, 0.0) {
                toolpaths.push(raster);
            }

            toolpaths.push(toolpath);
            prev_z = current_z;
        }
        toolpaths
    }

    fn generate_raster_pocket(
        &self,
        vertices: &[Point],
        step_down: f64,
        angle: f64,
        bidirectional: bool,
    ) -> Vec<Toolpath> {
        let mut toolpaths = Vec::new();

        if vertices.is_empty() {
            return toolpaths;
        }

        // Rotate vertices to align with X axis
        let cos_a = (-angle).to_radians().cos();
        let sin_a = (-angle).to_radians().sin();

        let rotate = |p: Point| -> Point {
            Point::new(p.x * cos_a - p.y * sin_a, p.x * sin_a + p.y * cos_a)
        };

        let inv_rotate = |p: Point| -> Point {
            let cos_inv = angle.to_radians().cos();
            let sin_inv = angle.to_radians().sin();
            Point::new(p.x * cos_inv - p.y * sin_inv, p.x * sin_inv + p.y * cos_inv)
        };

        let rotated_vertices: Vec<Point> = vertices.iter().map(|&p| rotate(p)).collect();

        // Calculate bounding box of rotated vertices
        let mut min_x = rotated_vertices[0].x;
        let mut min_y = rotated_vertices[0].y;
        let mut max_x = rotated_vertices[0].x;
        let mut max_y = rotated_vertices[0].y;

        for v in &rotated_vertices {
            if v.x < min_x {
                min_x = v.x;
            }
            if v.x > max_x {
                max_x = v.x;
            }
            if v.y < min_y {
                min_y = v.y;
            }
            if v.y > max_y {
                max_y = v.y;
            }
        }

        // Calculate Z passes
        let total_depth = self.operation.depth.abs();
        let z_step = if step_down > 0.0 {
            step_down
        } else {
            total_depth
        };
        let z_passes = (total_depth / z_step).ceil() as u32;

        let tool_radius = self.operation.tool_diameter / 2.0;
        let mut prev_z = self.operation.start_depth;

        for z_pass in 1..=z_passes {
            let current_z = self.operation.start_depth - (z_step * z_pass as f64).min(total_depth);
            let mut toolpath = Toolpath::new(self.operation.tool_diameter, current_z);

            // Scanline fill
            let mut current_y = min_y + tool_radius;
            let limit_y = max_y - tool_radius;
            let mut forward = true;

            while current_y <= limit_y {
                let mut intersections = Vec::new();

                if rotated_vertices.len() < 3 {
                    break;
                }

                for i in 0..rotated_vertices.len() {
                    let p1 = rotated_vertices[i];
                    let p2 = rotated_vertices[(i + 1) % rotated_vertices.len()];

                    if (p1.y <= current_y && p2.y > current_y)
                        || (p2.y <= current_y && p1.y > current_y)
                    {
                        if (p2.y - p1.y).abs() > 1e-9 {
                            let x = p1.x + (current_y - p1.y) * (p2.x - p1.x) / (p2.y - p1.y);
                            intersections.push(x);
                        }
                    }
                }

                intersections.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

                let mut segments = Vec::new();
                for i in (0..intersections.len()).step_by(2) {
                    if i + 1 < intersections.len() {
                        let x_start = intersections[i];
                        let x_end = intersections[i + 1];

                        let seg_start_x = x_start + tool_radius;
                        let seg_end_x = x_end - tool_radius;

                        if seg_start_x < seg_end_x {
                            segments.push((seg_start_x, seg_end_x));
                        }
                    }
                }

                if !forward && bidirectional {
                    segments.reverse();
                }

                for (seg_start_x, seg_end_x) in segments {
                    let (start_x, end_x) = if bidirectional && !forward {
                        (seg_end_x, seg_start_x)
                    } else {
                        (seg_start_x, seg_end_x)
                    };

                    let start_pt = inv_rotate(Point::new(start_x, current_y));
                    let end_pt = inv_rotate(Point::new(end_x, current_y));

                    if !self.is_in_island(&start_pt) && !self.is_in_island(&end_pt) {
                        // If bidirectional and close to previous end, linear move, else rapid
                        let is_connected = if let Some(last_seg) = toolpath.segments.last() {
                            last_seg.end.distance_to(&start_pt) < self.operation.tool_diameter * 1.5
                        } else {
                            false
                        };

                        if bidirectional && is_connected {
                            toolpath.add_segment(ToolpathSegment::new(
                                ToolpathSegmentType::LinearMove,
                                toolpath.segments.last().unwrap().end,
                                start_pt,
                                self.operation.feed_rate,
                                self.operation.spindle_speed,
                            ));
                        } else {
                            let last_point = toolpath
                                .segments
                                .last()
                                .map(|s| s.end)
                                .unwrap_or(Point::new(0.0, 0.0));
                            if self.operation.ramp_angle > 0.0 {
                                self.add_helical_ramp(&mut toolpath, start_pt, prev_z, current_z);
                            } else {
                                toolpath.add_segment(ToolpathSegment::new(
                                    ToolpathSegmentType::RapidMove,
                                    last_point,
                                    start_pt,
                                    self.operation.feed_rate,
                                    self.operation.spindle_speed,
                                ));
                            }
                        }

                        toolpath.add_segment(ToolpathSegment::new(
                            ToolpathSegmentType::LinearMove,
                            start_pt,
                            end_pt,
                            self.operation.feed_rate,
                            self.operation.spindle_speed,
                        ));
                    }
                }

                current_y += self.operation.stepover;
                forward = !forward;
            }

            toolpaths.push(toolpath);
            prev_z = current_z;
        }

        toolpaths
    }

    /// Generates offset paths for the pocket boundary.
    pub fn generate_offset_paths(&self, rect: &Rectangle, offset_count: u32) -> Vec<Vec<Point>> {
        let mut paths = Vec::new();

        for offset_idx in 1..=offset_count {
            let offset = self.operation.calculate_offset(offset_idx);
            if offset > (rect.width.min(rect.height) / 2.0) {
                break;
            }

            let inset_x = (rect.center.x - rect.width / 2.0) + offset;
            let inset_y = (rect.center.y - rect.height / 2.0) + offset;
            let inset_width = (rect.width - 2.0 * offset).max(0.0);
            let inset_height = (rect.height - 2.0 * offset).max(0.0);

            if inset_width <= 0.0 || inset_height <= 0.0 {
                break;
            }

            let path = vec![
                Point::new(inset_x, inset_y),
                Point::new(inset_x + inset_width, inset_y),
                Point::new(inset_x + inset_width, inset_y + inset_height),
                Point::new(inset_x, inset_y + inset_height),
            ];

            paths.push(path);
        }

        paths
    }
}
