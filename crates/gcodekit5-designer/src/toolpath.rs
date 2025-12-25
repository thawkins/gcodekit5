//! Toolpath generation from design shapes.

use super::pocket_operations::{PocketGenerator, PocketOperation, PocketStrategy};
use crate::font_manager;
use crate::model::{
    rotate_point, DesignCircle as Circle, DesignGear, DesignLine as Line, DesignPath as PathShape,
    DesignPolygon as Polygon, DesignRectangle as Rectangle, DesignSprocket, DesignTabbedBox,
    DesignText as TextShape, DesignTriangle as Triangle, DesignerShape, Point,
};
use lyon::path::iterator::PathIterator;
use rusttype::{GlyphId, OutlineBuilder, Scale};

/// Types of toolpath segments.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolpathSegmentType {
    RapidMove,
    LinearMove,
    ArcCW,
    ArcCCW,
}

/// A single segment of a toolpath.
#[derive(Debug, Clone)]
pub struct ToolpathSegment {
    pub segment_type: ToolpathSegmentType,
    pub start: Point,
    pub end: Point,
    pub center: Option<Point>,
    pub feed_rate: f64,
    pub spindle_speed: u32,
    /// Z depth for this segment (negative = below stock top)
    pub z_depth: Option<f64>,
    /// Start Z depth for this segment (if different from current Z)
    pub start_z: Option<f64>,
}

impl ToolpathSegment {
    /// Creates a new toolpath segment.
    pub fn new(
        segment_type: ToolpathSegmentType,
        start: Point,
        end: Point,
        feed_rate: f64,
        spindle_speed: u32,
    ) -> Self {
        Self {
            segment_type,
            start,
            end,
            center: None,
            feed_rate,
            spindle_speed,
            z_depth: None,
            start_z: None,
        }
    }

    /// Creates a new arc segment.
    pub fn new_arc(
        segment_type: ToolpathSegmentType,
        start: Point,
        end: Point,
        center: Point,
        feed_rate: f64,
        spindle_speed: u32,
    ) -> Self {
        Self {
            segment_type,
            start,
            end,
            center: Some(center),
            feed_rate,
            spindle_speed,
            z_depth: None,
            start_z: None,
        }
    }

    /// Set the Z depth for this segment
    pub fn with_z_depth(mut self, z: f64) -> Self {
        self.z_depth = Some(z);
        self
    }
}

/// A complete toolpath made up of multiple segments.
#[derive(Debug, Clone)]
pub struct Toolpath {
    pub segments: Vec<ToolpathSegment>,
    pub tool_diameter: f64,
    pub depth: f64,
}

impl Toolpath {
    /// Creates a new empty toolpath.
    pub fn new(tool_diameter: f64, depth: f64) -> Self {
        Self {
            segments: Vec::new(),
            tool_diameter,
            depth,
        }
    }

    /// Adds a segment to the toolpath.
    pub fn add_segment(&mut self, segment: ToolpathSegment) {
        self.segments.push(segment);
    }

    /// Gets the total length of the toolpath.
    pub fn total_length(&self) -> f64 {
        self.segments
            .iter()
            .map(|seg| seg.start.distance_to(&seg.end))
            .sum()
    }
}

/// Generates toolpaths from design shapes.
#[derive(Debug, Clone)]
pub struct ToolpathGenerator {
    feed_rate: f64,
    spindle_speed: u32,
    tool_diameter: f64,
    cut_depth: f64,
    start_depth: f64,
    step_in: f64,
    pocket_strategy: PocketStrategy,
    ramp_angle: f64,
    raster_fill_ratio: f64,
}

impl ToolpathGenerator {
    /// Creates a new toolpath generator with default parameters.
    pub fn new() -> Self {
        Self {
            feed_rate: 100.0,
            spindle_speed: 1000,
            tool_diameter: 3.175, // 1/8 inch
            cut_depth: 5.0,       // 5mm deep (magnitude)
            start_depth: 0.0,     // Top of stock
            step_in: 1.0,
            pocket_strategy: PocketStrategy::ContourParallel,
            ramp_angle: 0.0,
            raster_fill_ratio: 0.5,
        }
    }

    /// Sets the pocket strategy.
    pub fn set_pocket_strategy(&mut self, strategy: PocketStrategy) {
        self.pocket_strategy = strategy;
    }

    /// Sets the ramp angle in degrees.
    pub fn set_ramp_angle(&mut self, angle: f64) {
        self.ramp_angle = angle;
    }

    /// Sets the feed rate in mm/min.
    pub fn set_feed_rate(&mut self, feed_rate: f64) {
        self.feed_rate = feed_rate;
    }

    /// Sets the spindle speed in RPM.
    pub fn set_spindle_speed(&mut self, speed: u32) {
        self.spindle_speed = speed;
    }

    /// Sets the tool diameter in mm.
    pub fn set_tool_diameter(&mut self, diameter: f64) {
        self.tool_diameter = diameter;
    }

    /// Sets the cut depth in mm (positive magnitude for downward cut).
    pub fn set_cut_depth(&mut self, depth: f64) {
        self.cut_depth = depth;
    }

    /// Sets the start depth (top of stock) in mm.
    pub fn set_start_depth(&mut self, depth: f64) {
        self.start_depth = depth;
    }

    /// Sets the step in (step over) in mm.
    pub fn set_step_in(&mut self, step_in: f64) {
        self.step_in = step_in;
    }

    /// Sets raster fill ratio (0.0-1.0, where 1.0 keeps full strokes).
    pub fn set_raster_fill_ratio(&mut self, ratio: f64) {
        self.raster_fill_ratio = ratio.clamp(0.0, 1.0);
    }

    pub fn raster_fill_ratio(&self) -> f64 {
        self.raster_fill_ratio
    }

    /// Creates an empty toolpath with current settings.
    pub fn empty_toolpath(&self) -> Toolpath {
        Toolpath::new(self.tool_diameter, self.start_depth - self.cut_depth.abs())
    }

    /// Generates a contour toolpath for a rectangle.
    pub fn generate_rectangle_contour(&self, rect: &Rectangle, step_down: f64) -> Vec<Toolpath> {
        let mut segments = Vec::new();

        // Use dimensions from the rectangle struct
        let w = rect.width;
        let h = rect.height;
        let cx = rect.center.x;
        let cy = rect.center.y;

        // Unrotated bottom-left corner
        let x = cx - w / 2.0;
        let y = cy - h / 2.0;

        let r = rect.corner_radius.min(w / 2.0).min(h / 2.0);
        let rotation = rect.rotation;

        // Helper to rotate a point if needed
        let transform_point = |p: Point| -> Point {
            if rotation.abs() > 1e-6 {
                rotate_point(p, rect.center, rotation.to_degrees())
            } else {
                p
            }
        };

        if r < 0.001 {
            // Sharp corners
            // Define corners in unrotated space
            let corners = [
                Point::new(x, y),         // BL
                Point::new(x + w, y),     // BR
                Point::new(x + w, y + h), // TR
                Point::new(x, y + h),     // TL
            ];

            // Transform corners
            let t_corners: Vec<Point> = corners.iter().map(|&p| transform_point(p)).collect();

            // Start at first corner with rapid move
            segments.push(ToolpathSegment::new(
                ToolpathSegmentType::RapidMove,
                Point::new(0.0, 0.0),
                t_corners[0],
                self.feed_rate,
                self.spindle_speed,
            ));

            // Move around the rectangle
            for i in 0..4 {
                let next_i = (i + 1) % 4;
                segments.push(ToolpathSegment::new(
                    ToolpathSegmentType::LinearMove,
                    t_corners[i],
                    t_corners[next_i],
                    self.feed_rate,
                    self.spindle_speed,
                ));
            }

            // Return to origin with rapid move
            segments.push(ToolpathSegment::new(
                ToolpathSegmentType::RapidMove,
                t_corners[0],
                Point::new(0.0, 0.0),
                self.feed_rate,
                self.spindle_speed,
            ));
        } else {
            // Rounded corners
            // Start point: (x + r, y)
            let start_pt_raw = Point::new(x + r, y);
            let start_pt = transform_point(start_pt_raw);

            // Rapid to start
            segments.push(ToolpathSegment::new(
                ToolpathSegmentType::RapidMove,
                Point::new(0.0, 0.0),
                start_pt,
                self.feed_rate,
                self.spindle_speed,
            ));

            let mut current_pt = start_pt;

            // 1. Bottom Edge
            let p1_raw = Point::new(x + w - r, y);
            let p1 = transform_point(p1_raw);

            if current_pt.distance_to(&p1) > 0.001 {
                segments.push(ToolpathSegment::new(
                    ToolpathSegmentType::LinearMove,
                    current_pt,
                    p1,
                    self.feed_rate,
                    self.spindle_speed,
                ));
                current_pt = p1;
            }

            // 2. BR Corner (CCW Arc)
            let p_br_end_raw = Point::new(x + w, y + r);
            let center_br_raw = Point::new(x + w - r, y + r);
            let p_br_end = transform_point(p_br_end_raw);
            let center_br = transform_point(center_br_raw);

            segments.push(ToolpathSegment::new_arc(
                ToolpathSegmentType::ArcCCW,
                current_pt,
                p_br_end,
                center_br,
                self.feed_rate,
                self.spindle_speed,
            ));
            current_pt = p_br_end;

            // 3. Right Edge
            let p2_raw = Point::new(x + w, y + h - r);
            let p2 = transform_point(p2_raw);

            if current_pt.distance_to(&p2) > 0.001 {
                segments.push(ToolpathSegment::new(
                    ToolpathSegmentType::LinearMove,
                    current_pt,
                    p2,
                    self.feed_rate,
                    self.spindle_speed,
                ));
                current_pt = p2;
            }

            // 4. TR Corner (CCW Arc)
            let p_tr_end_raw = Point::new(x + w - r, y + h);
            let center_tr_raw = Point::new(x + w - r, y + h - r);
            let p_tr_end = transform_point(p_tr_end_raw);
            let center_tr = transform_point(center_tr_raw);

            segments.push(ToolpathSegment::new_arc(
                ToolpathSegmentType::ArcCCW,
                current_pt,
                p_tr_end,
                center_tr,
                self.feed_rate,
                self.spindle_speed,
            ));
            current_pt = p_tr_end;

            // 5. Top Edge
            let p3_raw = Point::new(x + r, y + h);
            let p3 = transform_point(p3_raw);

            if current_pt.distance_to(&p3) > 0.001 {
                segments.push(ToolpathSegment::new(
                    ToolpathSegmentType::LinearMove,
                    current_pt,
                    p3,
                    self.feed_rate,
                    self.spindle_speed,
                ));
                current_pt = p3;
            }

            // 6. TL Corner (CCW Arc)
            let p_tl_end_raw = Point::new(x, y + h - r);
            let center_tl_raw = Point::new(x + r, y + h - r);
            let p_tl_end = transform_point(p_tl_end_raw);
            let center_tl = transform_point(center_tl_raw);

            segments.push(ToolpathSegment::new_arc(
                ToolpathSegmentType::ArcCCW,
                current_pt,
                p_tl_end,
                center_tl,
                self.feed_rate,
                self.spindle_speed,
            ));
            current_pt = p_tl_end;

            // 7. Left Edge
            let p4_raw = Point::new(x, y + r);
            let p4 = transform_point(p4_raw);

            if current_pt.distance_to(&p4) > 0.001 {
                segments.push(ToolpathSegment::new(
                    ToolpathSegmentType::LinearMove,
                    current_pt,
                    p4,
                    self.feed_rate,
                    self.spindle_speed,
                ));
                current_pt = p4;
            }

            // 8. BL Corner (CCW Arc)
            let p_bl_end_raw = Point::new(x + r, y);
            let center_bl_raw = Point::new(x + r, y + r);
            let p_bl_end = transform_point(p_bl_end_raw);
            let center_bl = transform_point(center_bl_raw);

            segments.push(ToolpathSegment::new_arc(
                ToolpathSegmentType::ArcCCW,
                current_pt,
                p_bl_end,
                center_bl,
                self.feed_rate,
                self.spindle_speed,
            ));
            current_pt = p_bl_end;

            // Return to origin
            segments.push(ToolpathSegment::new(
                ToolpathSegmentType::RapidMove,
                current_pt,
                Point::new(0.0, 0.0),
                self.feed_rate,
                self.spindle_speed,
            ));
        }

        self.create_multipass_toolpaths(segments, step_down)
    }

    /// Helper to create multiple toolpaths from segments based on depth settings
    fn create_multipass_toolpaths(
        &self,
        segments: Vec<ToolpathSegment>,
        step_down: f64,
    ) -> Vec<Toolpath> {
        let mut toolpaths = Vec::new();
        let start_z = self.start_depth;
        // Treat cut_depth as magnitude (positive distance downwards)
        let target_z = start_z - self.cut_depth.abs();
        let total_dist = (start_z - target_z).abs();

        if self.ramp_angle > 0.001 && !segments.is_empty() {
            // Ramping logic
            let contour_length: f64 = segments
                .iter()
                .map(|s| {
                    match s.segment_type {
                        ToolpathSegmentType::LinearMove | ToolpathSegmentType::RapidMove => {
                            s.start.distance_to(&s.end)
                        }
                        ToolpathSegmentType::ArcCW | ToolpathSegmentType::ArcCCW => {
                            s.start.distance_to(&s.end) // Approximation using chord length
                        }
                    }
                })
                .sum();

            if contour_length > 0.001 {
                let mut current_z = start_z;

                // Loop until we reach target_z
                let mut loop_guard = 0;
                let max_segments = 20000; // Limit total segments to prevent freeze
                let path_segment_count = segments.len();

                while current_z > target_z {
                    loop_guard += 1;

                    // Check if we are generating too much geometry
                    if loop_guard * path_segment_count > max_segments {
                        // Too much geometry, stop ramping and plunge/cut at target
                        // Add a final pass at target_z and break
                        let mut final_pass = Toolpath::new(self.tool_diameter, target_z);
                        final_pass.segments = segments
                            .iter()
                            .map(|s| {
                                let mut ns = s.clone();
                                ns.start_z = Some(target_z);
                                ns.z_depth = Some(target_z);
                                ns
                            })
                            .collect();
                        toolpaths.push(final_pass);
                        return toolpaths;
                    }

                    if loop_guard > 1000 {
                        break;
                    }

                    let mut pass_segments = Vec::new();
                    let mut segment_start_z = current_z;
                    let pass_start_z = current_z;

                    for seg in &segments {
                        if seg.segment_type == ToolpathSegmentType::RapidMove {
                            let mut new_seg = seg.clone();
                            new_seg.start_z = Some(segment_start_z);
                            new_seg.z_depth = Some(segment_start_z);
                            pass_segments.push(new_seg);
                            continue;
                        }

                        let seg_len = seg.start.distance_to(&seg.end);
                        let z_drop = seg_len * self.ramp_angle.to_radians().tan();

                        let mut next_z = segment_start_z - z_drop;
                        if next_z < target_z {
                            next_z = target_z;
                        }

                        let mut new_seg = seg.clone();
                        new_seg.start_z = Some(segment_start_z);
                        new_seg.z_depth = Some(next_z);
                        pass_segments.push(new_seg);

                        segment_start_z = next_z;
                    }

                    current_z = segment_start_z;

                    let mut tp = Toolpath::new(self.tool_diameter, current_z);
                    tp.segments = pass_segments;
                    toolpaths.push(tp);

                    if (current_z - target_z).abs() < 0.001 {
                        break;
                    }

                    // If we didn't make significant progress, force a step down to avoid infinite loop
                    if (pass_start_z - current_z).abs() < 0.001 {
                        // Fallback: just step down by standard amount if ramping isn't working (e.g. geometry too small)
                        let fallback_step = if step_down > 0.0 {
                            step_down
                        } else {
                            self.tool_diameter
                        };
                        current_z = (current_z - fallback_step).max(target_z);
                    }
                }

                // Add a final flat pass at the bottom
                let mut final_pass = Toolpath::new(self.tool_diameter, target_z);
                final_pass.segments = segments
                    .iter()
                    .map(|s| {
                        let mut ns = s.clone();
                        ns.start_z = Some(target_z);
                        ns.z_depth = Some(target_z);
                        ns
                    })
                    .collect();
                toolpaths.push(final_pass);

                return toolpaths;
            }
        }

        let step = if step_down <= 0.001 {
            total_dist
        } else {
            step_down
        };
        let num_passes = (total_dist / step).ceil() as usize;
        let num_passes = if num_passes == 0 { 1 } else { num_passes };

        for i in 1..=num_passes {
            let depth_step = (i as f64 * step).min(total_dist);
            // Always cut downwards from start_z
            let z = start_z - depth_step;

            let mut tp = Toolpath::new(self.tool_diameter, z);
            tp.segments = segments.clone();
            toolpaths.push(tp);
        }

        toolpaths
    }

    /// Generates a contour toolpath for a circle.
    pub fn generate_circle_contour(&self, circle: &Circle, step_down: f64) -> Vec<Toolpath> {
        let mut segments = Vec::new();
        let rotation = circle.rotation;

        let transform_point = |p: Point| -> Point {
            if rotation.abs() > 1e-6 {
                rotate_point(p, circle.center, rotation.to_degrees())
            } else {
                p
            }
        };

        // Start at rightmost point of circle (unrotated) with rapid move
        let start_point_raw = Point::new(circle.center.x + circle.radius, circle.center.y);
        let start_point = transform_point(start_point_raw);

        segments.push(ToolpathSegment::new(
            ToolpathSegmentType::RapidMove,
            Point::new(0.0, 0.0),
            start_point,
            self.feed_rate,
            self.spindle_speed,
        ));

        // Generate 4 arc segments (90 degrees each) for full circle
        // CCW direction
        let points_raw = [
            Point::new(circle.center.x, circle.center.y + circle.radius), // Top
            Point::new(circle.center.x - circle.radius, circle.center.y), // Left
            Point::new(circle.center.x, circle.center.y - circle.radius), // Bottom
            Point::new(circle.center.x + circle.radius, circle.center.y), // Right (Start)
        ];

        let mut current = start_point;
        for p_raw in points_raw.iter() {
            let p = transform_point(*p_raw);
            segments.push(ToolpathSegment::new_arc(
                ToolpathSegmentType::ArcCCW,
                current,
                p,
                circle.center,
                self.feed_rate,
                self.spindle_speed,
            ));
            current = p;
        }

        // Return to origin with rapid move
        segments.push(ToolpathSegment::new(
            ToolpathSegmentType::RapidMove,
            start_point,
            Point::new(0.0, 0.0),
            self.feed_rate,
            self.spindle_speed,
        ));

        self.create_multipass_toolpaths(segments, step_down)
    }

    /// Generates a contour toolpath for a line.
    pub fn generate_line_contour(&self, line: &Line, step_down: f64) -> Vec<Toolpath> {
        let mut segments = Vec::new();

        // Rapid move to start
        segments.push(ToolpathSegment::new(
            ToolpathSegmentType::RapidMove,
            Point::new(0.0, 0.0),
            line.start,
            self.feed_rate,
            self.spindle_speed,
        ));

        // Linear move along the line
        segments.push(ToolpathSegment::new(
            ToolpathSegmentType::LinearMove,
            line.start,
            line.end,
            self.feed_rate,
            self.spindle_speed,
        ));

        // Return to origin
        segments.push(ToolpathSegment::new(
            ToolpathSegmentType::RapidMove,
            line.end,
            Point::new(0.0, 0.0),
            self.feed_rate,
            self.spindle_speed,
        ));

        self.create_multipass_toolpaths(segments, step_down)
    }

    /// Generates a contour toolpath for a polyline.
    pub fn generate_polyline_contour(&self, vertices: &[Point], step_down: f64) -> Vec<Toolpath> {
        let mut segments = Vec::new();

        if vertices.is_empty() {
            return Vec::new();
        }

        // Start at first vertex with rapid move
        segments.push(ToolpathSegment::new(
            ToolpathSegmentType::RapidMove,
            Point::new(0.0, 0.0),
            vertices[0],
            self.feed_rate,
            self.spindle_speed,
        ));

        // Move along the polyline
        for i in 0..vertices.len() {
            let next_i = (i + 1) % vertices.len();
            segments.push(ToolpathSegment::new(
                ToolpathSegmentType::LinearMove,
                vertices[i],
                vertices[next_i],
                self.feed_rate,
                self.spindle_speed,
            ));
        }

        // Return to origin with rapid move
        segments.push(ToolpathSegment::new(
            ToolpathSegmentType::RapidMove,
            vertices[0],
            Point::new(0.0, 0.0),
            self.feed_rate,
            self.spindle_speed,
        ));

        self.create_multipass_toolpaths(segments, step_down)
    }

    /// Generates a pocket toolpath for a rectangle.
    pub fn generate_rectangle_pocket(
        &self,
        rect: &Rectangle,
        pocket_depth: f64,
        step_down: f64,
        step_in: f64,
    ) -> Vec<Toolpath> {
        let r = rect
            .corner_radius
            .min(rect.width.abs() / 2.0)
            .min(rect.height.abs() / 2.0);

        if r > 0.001 || rect.rotation.abs() > 1e-6 {
            // Convert rounded or rotated rectangle to polygon for pocketing
            let mut vertices = Vec::new();
            let x = rect.center.x - rect.width / 2.0;
            let y = rect.center.y - rect.height / 2.0;
            let w = rect.width;
            let h = rect.height;

            if r > 0.001 {
                // Use more segments for better approximation (32 instead of 8)
                let segments = 32;

                // Helper to add arc points (excluding start point to avoid duplicates)
                let mut add_arc_points =
                    |center: Point, start_angle: f64, end_angle: f64, include_start: bool| {
                        let start_rad = start_angle.to_radians();
                        let end_rad = end_angle.to_radians();
                        let step = (end_rad - start_rad) / segments as f64;

                        let start_i = if include_start { 0 } else { 1 };
                        for i in start_i..=segments {
                            let angle = start_rad + step * i as f64;
                            vertices.push(Point::new(
                                center.x + r * angle.cos(),
                                center.y + r * angle.sin(),
                            ));
                        }
                    };

                // Generate rounded rectangle corners (clockwise from bottom-right)
                // BR Corner (270 -> 360) - include start point
                add_arc_points(Point::new(x + w - r, y + r), 270.0, 360.0, true);

                // TR Corner (0 -> 90) - exclude start point (overlaps with BR end)
                add_arc_points(Point::new(x + w - r, y + h - r), 0.0, 90.0, false);

                // TL Corner (90 -> 180) - exclude start point (overlaps with TR end)
                add_arc_points(Point::new(x + r, y + h - r), 90.0, 180.0, false);

                // BL Corner (180 -> 270) - exclude start point (overlaps with TL end)
                add_arc_points(Point::new(x + r, y + r), 180.0, 270.0, false);
            } else {
                vertices.push(Point::new(x, y));
                vertices.push(Point::new(x + w, y));
                vertices.push(Point::new(x + w, y + h));
                vertices.push(Point::new(x, y + h));
            }

            // Apply rotation
            if rect.rotation.abs() > 1e-6 {
                let center = rect.center;
                let rotation_deg = rect.rotation.to_degrees();
                for p in &mut vertices {
                    *p = crate::model::rotate_point(*p, center, rotation_deg);
                }
            }

            // Debug print
            println!(
                "Generating pocket for rotated rect: rotation={}, vertices={:?}",
                rect.rotation, vertices
            );

            return self.generate_polyline_pocket(&vertices, pocket_depth, step_down, step_in);
        }

        let op = PocketOperation::new("rect_pocket".to_string(), pocket_depth, self.tool_diameter);
        let mut gen = PocketGenerator::new(op);
        gen.operation.set_start_depth(self.start_depth);
        gen.operation.set_ramp_angle(self.ramp_angle);
        gen.operation.raster_fill_ratio = self.raster_fill_ratio;
        let effective_step_in = if step_in > 0.0 { step_in } else { self.step_in };
        gen.operation
            .set_parameters(effective_step_in, self.feed_rate, self.spindle_speed);
        gen.generate_rectangular_pocket(rect, step_down)
    }

    /// Generates a pocket toolpath for a circle.
    pub fn generate_circle_pocket(
        &self,
        circle: &Circle,
        pocket_depth: f64,
        step_down: f64,
        step_in: f64,
    ) -> Vec<Toolpath> {
        let op = PocketOperation::new(
            "circle_pocket".to_string(),
            pocket_depth,
            self.tool_diameter,
        );
        let mut gen = PocketGenerator::new(op);
        gen.operation.set_start_depth(self.start_depth);
        gen.operation.set_ramp_angle(self.ramp_angle);
        gen.operation.raster_fill_ratio = self.raster_fill_ratio;
        let effective_step_in = if step_in > 0.0 { step_in } else { self.step_in };
        gen.operation
            .set_parameters(effective_step_in, self.feed_rate, self.spindle_speed);
        gen.generate_circular_pocket(circle, step_down)
    }

    /// Generates a pocket toolpath for a polyline.
    pub fn generate_polyline_pocket(
        &self,
        vertices: &[Point],
        pocket_depth: f64,
        step_down: f64,
        step_in: f64,
    ) -> Vec<Toolpath> {
        let op = PocketOperation::new(
            "polyline_pocket".to_string(),
            pocket_depth,
            self.tool_diameter,
        );
        let mut gen = PocketGenerator::new(op);
        gen.operation.set_start_depth(self.start_depth);
        gen.operation.set_ramp_angle(self.ramp_angle);
        let effective_step_in = if step_in > 0.0 { step_in } else { self.step_in };
        gen.operation
            .set_parameters(effective_step_in, self.feed_rate, self.spindle_speed);
        gen.operation.set_strategy(self.pocket_strategy);
        gen.operation.raster_fill_ratio = self.raster_fill_ratio;
        gen.generate_polygon_pocket(vertices, step_down)
    }

    /// Generates a contour toolpath for a triangle.
    pub fn generate_triangle_contour(&self, triangle: &Triangle, step_down: f64) -> Vec<Toolpath> {
        let mut segments = Vec::new();
        let half_w = triangle.width / 2.0;
        let half_h = triangle.height / 2.0;

        // Local points
        let p1_local = Point::new(-half_w, -half_h);
        let p2_local = Point::new(half_w, -half_h);
        let p3_local = Point::new(-half_w, half_h);

        let rotation = triangle.rotation;
        let center = triangle.center;

        let transform_point = |p: Point| -> Point {
            let mut pt = p;
            if rotation.abs() > 1e-6 {
                pt = rotate_point(pt, Point::new(0.0, 0.0), rotation.to_degrees());
            }
            Point::new(pt.x + center.x, pt.y + center.y)
        };

        let p1 = transform_point(p1_local);
        let p2 = transform_point(p2_local);
        let p3 = transform_point(p3_local);

        // Rapid to start
        segments.push(ToolpathSegment::new(
            ToolpathSegmentType::RapidMove,
            Point::new(0.0, 0.0),
            p1,
            self.feed_rate,
            self.spindle_speed,
        ));

        // p1 -> p2
        segments.push(ToolpathSegment::new(
            ToolpathSegmentType::LinearMove,
            p1,
            p2,
            self.feed_rate,
            self.spindle_speed,
        ));

        // p2 -> p3
        segments.push(ToolpathSegment::new(
            ToolpathSegmentType::LinearMove,
            p2,
            p3,
            self.feed_rate,
            self.spindle_speed,
        ));

        // p3 -> p1
        segments.push(ToolpathSegment::new(
            ToolpathSegmentType::LinearMove,
            p3,
            p1,
            self.feed_rate,
            self.spindle_speed,
        ));

        // Return to origin
        segments.push(ToolpathSegment::new(
            ToolpathSegmentType::RapidMove,
            p1,
            Point::new(0.0, 0.0),
            self.feed_rate,
            self.spindle_speed,
        ));

        self.create_multipass_toolpaths(segments, step_down)
    }

    /// Generates a contour toolpath for a polygon.
    pub fn generate_polygon_contour(&self, polygon: &Polygon, step_down: f64) -> Vec<Toolpath> {
        let mut segments = Vec::new();
        let sides = polygon.sides.max(3);
        let rotation = polygon.rotation;
        let center = polygon.center;
        let radius = polygon.radius;

        let transform_point = |p: Point| -> Point {
            let mut pt = p;
            if rotation.abs() > 1e-6 {
                pt = rotate_point(pt, Point::new(0.0, 0.0), rotation.to_degrees());
            }
            Point::new(pt.x + center.x, pt.y + center.y)
        };

        let mut points = Vec::with_capacity(sides as usize);
        for i in 0..sides {
            let theta = 2.0 * std::f64::consts::PI * (i as f64) / (sides as f64);
            let x = radius * theta.cos();
            let y = radius * theta.sin();
            points.push(transform_point(Point::new(x, y)));
        }

        if points.is_empty() {
            return Vec::new();
        }

        // Rapid to start
        segments.push(ToolpathSegment::new(
            ToolpathSegmentType::RapidMove,
            Point::new(0.0, 0.0),
            points[0],
            self.feed_rate,
            self.spindle_speed,
        ));

        for i in 0..sides as usize {
            let next_i = (i + 1) % (sides as usize);
            segments.push(ToolpathSegment::new(
                ToolpathSegmentType::LinearMove,
                points[i],
                points[next_i],
                self.feed_rate,
                self.spindle_speed,
            ));
        }

        // Return to origin
        segments.push(ToolpathSegment::new(
            ToolpathSegmentType::RapidMove,
            points[0],
            Point::new(0.0, 0.0),
            self.feed_rate,
            self.spindle_speed,
        ));

        self.create_multipass_toolpaths(segments, step_down)
    }

    /// Generates a pocket toolpath for a triangle.
    pub fn generate_triangle_pocket(
        &self,
        triangle: &Triangle,
        pocket_depth: f64,
        step_down: f64,
        step_in: f64,
    ) -> Vec<Toolpath> {
        let half_w = triangle.width / 2.0;
        let half_h = triangle.height / 2.0;

        // Local points
        let p1_local = Point::new(-half_w, -half_h);
        let p2_local = Point::new(half_w, -half_h);
        let p3_local = Point::new(-half_w, half_h);

        let rotation = triangle.rotation;
        let center = triangle.center;

        let transform_point = |p: Point| -> Point {
            let mut pt = p;
            if rotation.abs() > 1e-6 {
                pt = rotate_point(pt, Point::new(0.0, 0.0), rotation.to_degrees());
            }
            Point::new(pt.x + center.x, pt.y + center.y)
        };

        let vertices = vec![
            transform_point(p1_local),
            transform_point(p2_local),
            transform_point(p3_local),
        ];

        self.generate_polyline_pocket(&vertices, pocket_depth, step_down, step_in)
    }

    /// Generates a pocket toolpath for a polygon.
    pub fn generate_polygon_pocket(
        &self,
        polygon: &Polygon,
        pocket_depth: f64,
        step_down: f64,
        step_in: f64,
    ) -> Vec<Toolpath> {
        let sides = polygon.sides.max(3);
        let rotation = polygon.rotation;
        let center = polygon.center;
        let radius = polygon.radius;

        let transform_point = |p: Point| -> Point {
            let mut pt = p;
            if rotation.abs() > 1e-6 {
                pt = rotate_point(pt, Point::new(0.0, 0.0), rotation.to_degrees());
            }
            Point::new(pt.x + center.x, pt.y + center.y)
        };

        let mut vertices = Vec::with_capacity(sides as usize);
        for i in 0..sides {
            let theta = 2.0 * std::f64::consts::PI * (i as f64) / (sides as f64);
            let x = radius * theta.cos();
            let y = radius * theta.sin();
            vertices.push(transform_point(Point::new(x, y)));
        }

        self.generate_polyline_pocket(&vertices, pocket_depth, step_down, step_in)
    }

    /// Generates a contour toolpath for a PathShape.
    pub fn generate_path_contour(&self, path_shape: &PathShape, step_down: f64) -> Vec<Toolpath> {
        let mut segments = Vec::new();
        let tolerance = 0.05; // mm

        let mut current_point = Point::new(0.0, 0.0);
        let mut start_point = Point::new(0.0, 0.0);

        // Calculate center for rotation (unrotated bounding box)
        let rect = lyon::algorithms::aabb::bounding_box(&path_shape.render());
        let center = Point::new(
            (rect.min.x + rect.max.x) as f64 / 2.0,
            (rect.min.y + rect.max.y) as f64 / 2.0,
        );
        let rotation = path_shape.rotation;

        for event in path_shape.render().iter().flattened(tolerance) {
            match event {
                lyon::path::Event::Begin { at } => {
                    let mut p = Point::new(at.x as f64, at.y as f64);
                    if rotation.abs() > 1e-6 {
                        p = crate::model::rotate_point(p, center, rotation.to_degrees());
                    }
                    segments.push(ToolpathSegment::new(
                        ToolpathSegmentType::RapidMove,
                        current_point,
                        p,
                        self.feed_rate,
                        self.spindle_speed,
                    ));
                    current_point = p;
                    start_point = p;
                }
                lyon::path::Event::Line { from: _, to } => {
                    let mut p = Point::new(to.x as f64, to.y as f64);
                    if rotation.abs() > 1e-6 {
                        p = crate::model::rotate_point(p, center, rotation.to_degrees());
                    }
                    segments.push(ToolpathSegment::new(
                        ToolpathSegmentType::LinearMove,
                        current_point,
                        p,
                        self.feed_rate,
                        self.spindle_speed,
                    ));
                    current_point = p;
                }
                lyon::path::Event::End {
                    last: _,
                    first: _,
                    close,
                } => {
                    if close {
                        segments.push(ToolpathSegment::new(
                            ToolpathSegmentType::LinearMove,
                            current_point,
                            start_point,
                            self.feed_rate,
                            self.spindle_speed,
                        ));
                        current_point = start_point;
                    }
                }
                _ => {}
            }
        }

        // Return to origin
        segments.push(ToolpathSegment::new(
            ToolpathSegmentType::RapidMove,
            current_point,
            Point::new(0.0, 0.0),
            self.feed_rate,
            self.spindle_speed,
        ));

        self.create_multipass_toolpaths(segments, step_down)
    }

    /// Generates a pocket toolpath for a PathShape.
    pub fn generate_path_pocket(
        &self,
        path_shape: &PathShape,
        pocket_depth: f64,
        step_down: f64,
        step_in: f64,
    ) -> Vec<Toolpath> {
        // Flatten path to polyline and use polyline pocket generation
        let tolerance = 0.1; // mm
        let mut vertices = Vec::new();

        // Calculate center for rotation
        let rect = lyon::algorithms::aabb::bounding_box(&path_shape.render());
        let center = Point::new(
            (rect.min.x + rect.max.x) as f64 / 2.0,
            (rect.min.y + rect.max.y) as f64 / 2.0,
        );
        let rotation = path_shape.rotation;

        for event in path_shape.render().iter().flattened(tolerance) {
            match event {
                lyon::path::Event::Begin { at } => {
                    let mut p = Point::new(at.x as f64, at.y as f64);
                    if rotation.abs() > 1e-6 {
                        p = crate::model::rotate_point(p, center, rotation.to_degrees());
                    }
                    vertices.push(p);
                }
                lyon::path::Event::Line { from: _, to } => {
                    let mut p = Point::new(to.x as f64, to.y as f64);
                    if rotation.abs() > 1e-6 {
                        p = crate::model::rotate_point(p, center, rotation.to_degrees());
                    }
                    vertices.push(p);
                }
                _ => {}
            }
        }

        let polyline_vertices = vertices;
        self.generate_polyline_pocket(&polyline_vertices, pocket_depth, step_down, step_in)
    }

    fn build_text_outline_segments(&self, text_shape: &TextShape) -> Vec<ToolpathSegment> {
        let mut segments = Vec::new();

        let font =
            font_manager::get_font_for(&text_shape.font_family, text_shape.bold, text_shape.italic);
        let scale = Scale::uniform(text_shape.font_size as f32);
        let v_metrics = font.v_metrics(scale);
        let line_height = v_metrics.ascent - v_metrics.descent + v_metrics.line_gap;

        // Match the designer's text rotation behaviour: rotate around the *unrotated* text bounds center.
        let (min_x, min_y, max_x, max_y) = text_shape.bounds();
        let baseline_y0 = (text_shape.y as f32) + v_metrics.ascent;
        let rotation_center_raw = Point::new((min_x + max_x) / 2.0, (min_y + max_y) / 2.0);
        let rotation_center = Point::new(
            rotation_center_raw.x,
            2.0 * baseline_y0 as f64 - rotation_center_raw.y,
        );

        let mut caret_x = text_shape.x as f32;
        let mut baseline_y = baseline_y0;
        let mut prev: Option<GlyphId> = None;

        let mut pen = Point::new(0.0, 0.0);

        for ch in text_shape.text.chars() {
            if ch == '\n' {
                caret_x = text_shape.x as f32;
                baseline_y -= line_height;
                prev = None;
                continue;
            }

            let base = font.glyph(ch);
            let base_id = base.id();

            if let Some(prev_id) = prev {
                caret_x += font.pair_kerning(scale, prev_id, base_id);
            }

            let scaled = base.scaled(scale);
            let advance = scaled.h_metrics().advance_width;

            // Build outline in glyph-local coordinates and apply baseline offset + rotation in the builder.
            let mut builder = ToolpathBuilder::new(
                self.feed_rate,
                self.spindle_speed,
                pen,
                Point::new(caret_x as f64, baseline_y as f64),
                rotation_center,
                text_shape.rotation.to_degrees(),
            );
            scaled.build_outline(&mut builder);
            pen = builder.current_point;
            segments.extend(builder.segments);

            caret_x += advance;
            prev = Some(base_id);
        }

        segments
    }

    /// Generates a pocket (area clearing) toolpath for text.
    pub fn generate_text_pocket_toolpath(
        &self,
        text_shape: &TextShape,
        step_down: f64,
    ) -> Vec<Toolpath> {
        let outline_segments = self.build_text_outline_segments(text_shape);
        let contours = contours_from_outline_segments(&outline_segments);
        if contours.is_empty() {
            return Vec::new();
        }

        let stepover = if self.step_in > 1e-6 {
            self.step_in
        } else {
            (self.tool_diameter * 0.4).max(0.1)
        };

        fn centroid(poly: &[Point]) -> Point {
            if poly.is_empty() {
                return Point::new(0.0, 0.0);
            }
            let (mut sx, mut sy) = (0.0, 0.0);
            for p in poly {
                sx += p.x;
                sy += p.y;
            }
            let n = poly.len() as f64;
            Point::new(sx / n, sy / n)
        }

        fn clean_contour(contour: &[Point], tol: f64) -> Vec<Point> {
            let mut out: Vec<Point> = Vec::new();
            for &p in contour {
                let should_push = match out.last() {
                    None => true,
                    Some(last) => last.distance_to(&p) > tol,
                };
                if should_push {
                    out.push(p);
                }
            }

            if out.len() > 2 {
                let first = out[0];
                let last = *out.last().unwrap();
                if last.distance_to(&first) <= tol {
                    out.pop();
                }
            }

            out
        }

        fn point_in_polygon(p: Point, poly: &[Point]) -> bool {
            if poly.len() < 3 {
                return false;
            }
            let mut inside = false;
            let mut j = poly.len() - 1;
            for i in 0..poly.len() {
                let pi = poly[i];
                let pj = poly[j];
                let intersect = ((pi.y > p.y) != (pj.y > p.y))
                    && (p.x < (pj.x - pi.x) * (p.y - pi.y) / (pj.y - pi.y + 1e-12) + pi.x);
                if intersect {
                    inside = !inside;
                }
                j = i;
            }
            inside
        }

        // Classify contours by nesting depth (even=solid, odd=hole)
        let mut holes: Vec<Vec<Point>> = Vec::new();
        let mut solids: Vec<Vec<Point>> = Vec::new();
        for (idx, c) in contours.iter().enumerate() {
            let clean = clean_contour(c, 0.01);
            if clean.len() < 3 {
                continue;
            }
            // Use a boundary-adjacent point for nesting tests; centroids can fall in holes (e.g. 'O').
            let test_pt = Point::new(clean[0].x + 1e-6, clean[0].y);
            let mut depth = 0usize;
            for (j, other) in contours.iter().enumerate() {
                if idx == j {
                    continue;
                }
                let other_clean = clean_contour(other, 0.01);
                if other_clean.len() < 3 {
                    continue;
                }
                if point_in_polygon(test_pt, &other_clean) {
                    depth += 1;
                }
            }
            if depth % 2 == 1 {
                holes.push(clean);
            } else {
                solids.push(clean);
            }
        }

        let mut segments = Vec::new();
        let mut current = Point::new(0.0, 0.0);

        fn intersections_at_y(poly: &[Point], y: f64) -> Vec<f64> {
            let mut xs = Vec::new();
            if poly.len() < 3 {
                return xs;
            }

            for i in 0..poly.len() {
                let p1 = poly[i];
                let p2 = poly[(i + 1) % poly.len()];

                if (p1.y <= y && p2.y > y) || (p2.y <= y && p1.y > y) {
                    let dy = p2.y - p1.y;
                    if dy.abs() > 1e-12 {
                        let t = (y - p1.y) / dy;
                        xs.push(p1.x + t * (p2.x - p1.x));
                    }
                }
            }

            xs.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
            xs
        }

        fn pair_intervals(mut xs: Vec<f64>) -> Vec<(f64, f64)> {
            xs.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
            let mut out = Vec::new();
            for i in (0..xs.len()).step_by(2) {
                if i + 1 < xs.len() {
                    out.push((xs[i], xs[i + 1]));
                }
            }
            out
        }

        fn merge_intervals(mut ivals: Vec<(f64, f64)>) -> Vec<(f64, f64)> {
            ivals.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
            let mut out: Vec<(f64, f64)> = Vec::new();
            for (a, b) in ivals {
                if let Some(last) = out.last_mut() {
                    if a <= last.1 {
                        last.1 = last.1.max(b);
                        continue;
                    }
                }
                out.push((a, b));
            }
            out
        }

        fn subtract_intervals(
            mut allowed: Vec<(f64, f64)>,
            forbidden: &[(f64, f64)],
        ) -> Vec<(f64, f64)> {
            if forbidden.is_empty() {
                return allowed;
            }

            allowed.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));

            let mut out = Vec::new();
            for (mut a0, a1) in allowed {
                for (f0, f1) in forbidden {
                    if *f1 <= a0 || *f0 >= a1 {
                        continue;
                    }
                    if *f0 > a0 {
                        out.push((a0, (*f0).min(a1)));
                    }
                    a0 = a0.max(*f1);
                    if a0 >= a1 {
                        break;
                    }
                }
                if a0 < a1 {
                    out.push((a0, a1));
                }
            }
            out
        }

        for solid in solids {
            if solid.len() < 3 {
                continue;
            }

            let (mut min_x, mut max_x) = (f64::INFINITY, f64::NEG_INFINITY);
            let (mut min_y, mut max_y) = (f64::INFINITY, f64::NEG_INFINITY);
            for p in &solid {
                min_x = min_x.min(p.x);
                max_x = max_x.max(p.x);
                min_y = min_y.min(p.y);
                max_y = max_y.max(p.y);
            }

            let mut y = min_y;
            let y_limit = max_y;
            let mut forward = true;

            while y <= y_limit {
                let solid_xs = intersections_at_y(&solid, y);
                let mut allowed = Vec::new();
                for (x0, x1) in pair_intervals(solid_xs) {
                    let a0 = x0;
                    let a1 = x1;
                    if a0 < a1 {
                        allowed.push((a0, a1));
                    }
                }

                if !allowed.is_empty() {
                    let mut forbidden = Vec::new();
                    for h in &holes {
                        if h.len() < 3 {
                            continue;
                        }
                        // Only subtract holes that are inside this solid.
                        if !point_in_polygon(centroid(h), &solid) {
                            continue;
                        }
                        let hole_xs = intersections_at_y(h, y);
                        for (hx0, hx1) in pair_intervals(hole_xs) {
                            forbidden.push((hx0, hx1));
                        }
                    }
                    let forbidden = merge_intervals(forbidden);
                    allowed = subtract_intervals(allowed, &forbidden);
                }

                if !allowed.is_empty() {
                    if !forward {
                        allowed.reverse();
                    }

                    for (a0, a1) in allowed {
                        let (start_x, end_x) = if forward { (a0, a1) } else { (a1, a0) };
                        let start = Point::new(start_x, y);
                        let end = Point::new(end_x, y);

                        segments.push(ToolpathSegment::new(
                            ToolpathSegmentType::RapidMove,
                            current,
                            start,
                            self.feed_rate,
                            self.spindle_speed,
                        ));
                        segments.push(ToolpathSegment::new(
                            ToolpathSegmentType::LinearMove,
                            start,
                            end,
                            self.feed_rate,
                            self.spindle_speed,
                        ));
                        current = end;
                    }
                }

                forward = !forward;
                y += stepover.max(0.05);
            }
        }

        segments.push(ToolpathSegment::new(
            ToolpathSegmentType::RapidMove,
            current,
            Point::new(0.0, 0.0),
            self.feed_rate,
            self.spindle_speed,
        ));

        self.create_multipass_toolpaths(segments, step_down)
    }

    /// Generates a contour (profile) toolpath for text.
    pub fn generate_text_toolpath(&self, text_shape: &TextShape, step_down: f64) -> Vec<Toolpath> {
        let segments = self.build_text_outline_segments(text_shape);
        self.create_multipass_toolpaths(segments, step_down)
    }

    /// Generates a contour toolpath for a gear.
    pub fn generate_gear_contour(&self, gear: &DesignGear, step_down: f64) -> Vec<Toolpath> {
        let path = gear.render();
        let path_shape = PathShape::from_lyon_path(&path);
        self.generate_path_contour(&path_shape, step_down)
    }

    /// Generates a pocket toolpath for a gear.
    pub fn generate_gear_pocket(
        &self,
        gear: &DesignGear,
        pocket_depth: f64,
        step_down: f64,
        step_in: f64,
    ) -> Vec<Toolpath> {
        let path = gear.render();
        let path_shape = PathShape::from_lyon_path(&path);
        self.generate_path_pocket(&path_shape, pocket_depth, step_down, step_in)
    }

    /// Generates a contour toolpath for a sprocket.
    pub fn generate_sprocket_contour(
        &self,
        sprocket: &DesignSprocket,
        step_down: f64,
    ) -> Vec<Toolpath> {
        let path = sprocket.render();
        let path_shape = PathShape::from_lyon_path(&path);
        self.generate_path_contour(&path_shape, step_down)
    }

    /// Generates a pocket toolpath for a sprocket.
    pub fn generate_sprocket_pocket(
        &self,
        sprocket: &DesignSprocket,
        pocket_depth: f64,
        step_down: f64,
        step_in: f64,
    ) -> Vec<Toolpath> {
        let path = sprocket.render();
        let path_shape = PathShape::from_lyon_path(&path);
        self.generate_path_pocket(&path_shape, pocket_depth, step_down, step_in)
    }

    /// Generates a contour toolpath for a tabbed box.
    pub fn generate_tabbed_box_contour(
        &self,
        tbox: &DesignTabbedBox,
        step_down: f64,
    ) -> Vec<Toolpath> {
        let paths = tbox.render_all();
        let mut all_toolpaths = Vec::new();
        for path in paths {
            let path_shape = PathShape::from_lyon_path(&path);
            all_toolpaths.extend(self.generate_path_contour(&path_shape, step_down));
        }
        all_toolpaths
    }
}

fn contours_from_outline_segments(segments: &[ToolpathSegment]) -> Vec<Vec<Point>> {
    let mut contours: Vec<Vec<Point>> = Vec::new();
    let mut current: Vec<Point> = Vec::new();

    for seg in segments {
        match seg.segment_type {
            ToolpathSegmentType::RapidMove => {
                if current.len() >= 2 {
                    if current
                        .last()
                        .unwrap()
                        .distance_to(current.first().unwrap())
                        <= 1e-6
                    {
                        current.pop();
                    }
                    if current.len() >= 2 {
                        contours.push(std::mem::take(&mut current));
                    } else {
                        current.clear();
                    }
                }
                current.clear();
                current.push(seg.end);
            }
            _ => {
                if current.is_empty() {
                    current.push(seg.start);
                }
                current.push(seg.end);
            }
        }
    }

    if current.len() >= 2 {
        if current
            .last()
            .unwrap()
            .distance_to(current.first().unwrap())
            <= 1e-6
        {
            current.pop();
        }
        if current.len() >= 2 {
            contours.push(current);
        }
    }

    contours
}

struct ToolpathBuilder {
    segments: Vec<ToolpathSegment>,
    current_point: Point,
    start_point: Point,
    started: bool,
    feed_rate: f64,
    spindle_speed: u32,
    offset: Point,
    rotation_center: Point,
    rotation_deg: f64,
}

impl ToolpathBuilder {
    fn new(
        feed_rate: f64,
        spindle_speed: u32,
        initial_point: Point,
        offset: Point,
        rotation_center: Point,
        rotation_deg: f64,
    ) -> Self {
        Self {
            segments: Vec::new(),
            current_point: initial_point,
            start_point: Point::new(0.0, 0.0),
            started: false,
            feed_rate,
            spindle_speed,
            offset,
            rotation_center,
            rotation_deg,
        }
    }

    fn map_point(&self, x: f32, y: f32) -> Point {
        // rusttype/cairo glyph outlines are Y-down; convert to designer world (Y-up).
        let p = Point::new(x as f64 + self.offset.x, self.offset.y - y as f64);
        rotate_point(p, self.rotation_center, self.rotation_deg)
    }
}

impl OutlineBuilder for ToolpathBuilder {
    fn move_to(&mut self, x: f32, y: f32) {
        let p = self.map_point(x, y);

        if !self.started {
            self.started = true;
        }

        // Rapid move to start of contour (assumed safe height handling in G-code gen)
        self.segments.push(ToolpathSegment::new(
            ToolpathSegmentType::RapidMove,
            self.current_point,
            p,
            self.feed_rate,
            self.spindle_speed,
        ));
        self.current_point = p;
        self.start_point = p;
    }

    fn line_to(&mut self, x: f32, y: f32) {
        let p = self.map_point(x, y);
        self.segments.push(ToolpathSegment::new(
            ToolpathSegmentType::LinearMove,
            self.current_point,
            p,
            self.feed_rate,
            self.spindle_speed,
        ));
        self.current_point = p;
    }

    fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) {
        let p0 = self.current_point;
        let p1 = self.map_point(x1, y1);
        let p2 = self.map_point(x, y);

        let approx_len = (p0.x - p1.x).hypot(p0.y - p1.y) + (p1.x - p2.x).hypot(p1.y - p2.y);
        let max_seg_len = 0.5_f64;
        let steps = ((approx_len / max_seg_len).ceil() as usize).clamp(4, 64);

        for i in 1..=steps {
            let t = (i as f64) / (steps as f64);
            let mt = 1.0 - t;
            let px = mt * mt * p0.x + 2.0 * mt * t * p1.x + t * t * p2.x;
            let py = mt * mt * p0.y + 2.0 * mt * t * p1.y + t * t * p2.y;
            let p = Point::new(px, py);
            self.segments.push(ToolpathSegment::new(
                ToolpathSegmentType::LinearMove,
                self.current_point,
                p,
                self.feed_rate,
                self.spindle_speed,
            ));
            self.current_point = p;
        }
    }

    fn curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32) {
        let p0 = self.current_point;
        let p1 = self.map_point(x1, y1);
        let p2 = self.map_point(x2, y2);
        let p3 = self.map_point(x, y);

        let approx_len = (p0.x - p1.x).hypot(p0.y - p1.y)
            + (p1.x - p2.x).hypot(p1.y - p2.y)
            + (p2.x - p3.x).hypot(p2.y - p3.y);
        let max_seg_len = 0.5_f64;
        let steps = ((approx_len / max_seg_len).ceil() as usize).clamp(8, 128);

        for i in 1..=steps {
            let t = (i as f64) / (steps as f64);
            let mt = 1.0 - t;
            let px = mt * mt * mt * p0.x
                + 3.0 * mt * mt * t * p1.x
                + 3.0 * mt * t * t * p2.x
                + t * t * t * p3.x;
            let py = mt * mt * mt * p0.y
                + 3.0 * mt * mt * t * p1.y
                + 3.0 * mt * t * t * p2.y
                + t * t * t * p3.y;
            let p = Point::new(px, py);
            self.segments.push(ToolpathSegment::new(
                ToolpathSegmentType::LinearMove,
                self.current_point,
                p,
                self.feed_rate,
                self.spindle_speed,
            ));
            self.current_point = p;
        }
    }

    fn close(&mut self) {
        if !self.started {
            return;
        }

        if self.current_point.distance_to(&self.start_point) > 1e-6 {
            self.segments.push(ToolpathSegment::new(
                ToolpathSegmentType::LinearMove,
                self.current_point,
                self.start_point,
                self.feed_rate,
                self.spindle_speed,
            ));
        }

        self.current_point = self.start_point;
    }
}

impl Default for ToolpathGenerator {
    fn default() -> Self {
        Self::new()
    }
}
