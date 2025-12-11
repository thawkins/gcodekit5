//! Toolpath generation from design shapes.

use super::shapes::{Circle, Line, Point, Rectangle, PathShape};
use lyon::path::iterator::PathIterator;
use super::pocket_operations::{PocketGenerator, PocketOperation, PocketStrategy};
use rusttype::{OutlineBuilder, Scale, point as rt_point};
use crate::font_manager;

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
        }
    }

    /// Sets the pocket strategy.
    pub fn set_pocket_strategy(&mut self, strategy: PocketStrategy) {
        self.pocket_strategy = strategy;
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

    /// Creates an empty toolpath with current settings.
    pub fn empty_toolpath(&self) -> Toolpath {
        Toolpath::new(self.tool_diameter, self.start_depth - self.cut_depth.abs())
    }

    /// Generates a contour toolpath for a rectangle.
    pub fn generate_rectangle_contour(&self, rect: &Rectangle, step_down: f64) -> Vec<Toolpath> {
        let mut segments = Vec::new();

        // Normalize coordinates
        let (x1, y1, x2, y2) = rect.bounding_box();
        let min_x = x1.min(x2);
        let max_x = x1.max(x2);
        let min_y = y1.min(y2);
        let max_y = y1.max(y2);
        let w = max_x - min_x;
        let h = max_y - min_y;
        let x = min_x;
        let y = min_y;

        let r = rect.corner_radius.min(w / 2.0).min(h / 2.0);

        if r < 0.001 {
            // Sharp corners
            let corners = [
                Point::new(x, y),
                Point::new(x + w, y),
                Point::new(x + w, y + h),
                Point::new(x, y + h),
            ];

            // Start at first corner with rapid move
            segments.push(ToolpathSegment::new(
                ToolpathSegmentType::RapidMove,
                Point::new(0.0, 0.0),
                corners[0],
                self.feed_rate,
                self.spindle_speed,
            ));

            // Move around the rectangle
            for i in 0..4 {
                let next_i = (i + 1) % 4;
                segments.push(ToolpathSegment::new(
                    ToolpathSegmentType::LinearMove,
                    corners[i],
                    corners[next_i],
                    self.feed_rate,
                    self.spindle_speed,
                ));
            }

            // Return to origin with rapid move
            segments.push(ToolpathSegment::new(
                ToolpathSegmentType::RapidMove,
                corners[0],
                Point::new(0.0, 0.0),
                self.feed_rate,
                self.spindle_speed,
            ));
        } else {
            // Rounded corners
            // Start point: (x + r, y)
            let start_pt = Point::new(x + r, y);
            
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
            let p1 = Point::new(x + w - r, y);
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
            
            // 2. BR Corner (CW Arc)
            let p_br_end = Point::new(x + w, y + r);
            let center_br = Point::new(x + w - r, y + r);
            segments.push(ToolpathSegment::new_arc(
                ToolpathSegmentType::ArcCCW, // Y-up: Bottom edge (y) to Right edge (x+w) is CCW?
                // Wait, coordinate system.
                // If (0,0) is bottom-left.
                // Bottom edge is y=min_y. Moving right (x increasing).
                // Right edge is x=max_x. Moving up (y increasing).
                // Turn is Left (CCW).
                // Rectangle contour is usually CCW for climb milling (tool on left).
                // Or CW for conventional.
                // Let's check the order of points in sharp corners:
                // (x, y) -> (x+w, y) -> (x+w, y+h) -> (x, y+h) -> (x, y)
                // (0,0) -> (10,0) -> (10,10) -> (0,10) -> (0,0)
                // This is CCW.
                // So arcs should be CCW (G03).
                current_pt,
                p_br_end,
                center_br,
                self.feed_rate,
                self.spindle_speed
            ));
            current_pt = p_br_end;
            
            // 3. Right Edge
            let p2 = Point::new(x + w, y + h - r);
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
            let p_tr_end = Point::new(x + w - r, y + h);
            let center_tr = Point::new(x + w - r, y + h - r);
            segments.push(ToolpathSegment::new_arc(
                ToolpathSegmentType::ArcCCW,
                current_pt,
                p_tr_end,
                center_tr,
                self.feed_rate,
                self.spindle_speed
            ));
            current_pt = p_tr_end;
            
            // 5. Top Edge
            let p3 = Point::new(x + r, y + h);
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
            let p_tl_end = Point::new(x, y + h - r);
            let center_tl = Point::new(x + r, y + h - r);
            segments.push(ToolpathSegment::new_arc(
                ToolpathSegmentType::ArcCCW,
                current_pt,
                p_tl_end,
                center_tl,
                self.feed_rate,
                self.spindle_speed
            ));
            current_pt = p_tl_end;
            
            // 7. Left Edge
            let p4 = Point::new(x, y + r);
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
            let p_bl_end = Point::new(x + r, y);
            let center_bl = Point::new(x + r, y + r);
            segments.push(ToolpathSegment::new_arc(
                ToolpathSegmentType::ArcCCW,
                current_pt,
                p_bl_end,
                center_bl,
                self.feed_rate,
                self.spindle_speed
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
    fn create_multipass_toolpaths(&self, segments: Vec<ToolpathSegment>, step_down: f64) -> Vec<Toolpath> {
        let mut toolpaths = Vec::new();
        let start_z = self.start_depth;
        // Treat cut_depth as magnitude (positive distance downwards)
        let target_z = start_z - self.cut_depth.abs();
        let total_dist = (start_z - target_z).abs();
        
        let step = if step_down <= 0.001 { total_dist } else { step_down };
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

        // Start at rightmost point of circle with rapid move
        let start_point = Point::new(circle.center.x + circle.radius, circle.center.y);
        segments.push(ToolpathSegment::new(
            ToolpathSegmentType::RapidMove,
            Point::new(0.0, 0.0),
            start_point,
            self.feed_rate,
            self.spindle_speed,
        ));

        // Generate 4 arc segments (90 degrees each) for full circle
        // CCW direction
        let points = [
            Point::new(circle.center.x, circle.center.y + circle.radius), // Top
            Point::new(circle.center.x - circle.radius, circle.center.y), // Left
            Point::new(circle.center.x, circle.center.y - circle.radius), // Bottom
            Point::new(circle.center.x + circle.radius, circle.center.y), // Right (Start)
        ];
        
        let mut current = start_point;
        for p in points.iter() {
            segments.push(ToolpathSegment::new_arc(
                ToolpathSegmentType::ArcCCW,
                current,
                *p,
                circle.center,
                self.feed_rate,
                self.spindle_speed
            ));
            current = *p;
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
    pub fn generate_rectangle_pocket(&self, rect: &Rectangle, pocket_depth: f64, step_down: f64, step_in: f64) -> Vec<Toolpath> {
        let r = rect.corner_radius.min(rect.width.abs() / 2.0).min(rect.height.abs() / 2.0);
        
        if r > 0.001 || rect.rotation.abs() > 1e-6 {
            // Convert rounded or rotated rectangle to polygon for pocketing
            let mut vertices = Vec::new();
            let x = rect.x;
            let y = rect.y;
            let w = rect.width;
            let h = rect.height;
            
            if r > 0.001 {
                // Use more segments for better approximation (32 instead of 8)
                let segments = 32;
                
                // Helper to add arc points (excluding start point to avoid duplicates)
                let mut add_arc_points = |center: Point, start_angle: f64, end_angle: f64, include_start: bool| {
                    let start_rad = start_angle.to_radians();
                    let end_rad = end_angle.to_radians();
                    let step = (end_rad - start_rad) / segments as f64;
                    
                    let start_i = if include_start { 0 } else { 1 };
                    for i in start_i..=segments {
                        let angle = start_rad + step * i as f64;
                        vertices.push(Point::new(
                            center.x + r * angle.cos(),
                            center.y + r * angle.sin()
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
                let center = Point::new(x + w / 2.0, y + h / 2.0);
                for p in &mut vertices {
                    *p = crate::shapes::rotate_point(*p, center, rect.rotation);
                }
            }
            
            return self.generate_polyline_pocket(&vertices, pocket_depth, step_down, step_in);
        }

        let op = PocketOperation::new("rect_pocket".to_string(), pocket_depth, self.tool_diameter);
        let mut gen = PocketGenerator::new(op);
        gen.operation.set_start_depth(self.start_depth);
        let effective_step_in = if step_in > 0.0 { step_in } else { self.step_in };
        gen.operation.set_parameters(effective_step_in, self.feed_rate, self.spindle_speed);
        gen.generate_rectangular_pocket(rect, step_down)
    }

    /// Generates a pocket toolpath for a circle.
    pub fn generate_circle_pocket(&self, circle: &Circle, pocket_depth: f64, step_down: f64, step_in: f64) -> Vec<Toolpath> {
        let op = PocketOperation::new("circle_pocket".to_string(), pocket_depth, self.tool_diameter);
        let mut gen = PocketGenerator::new(op);
        gen.operation.set_start_depth(self.start_depth);
        let effective_step_in = if step_in > 0.0 { step_in } else { self.step_in };
        gen.operation.set_parameters(effective_step_in, self.feed_rate, self.spindle_speed);
        gen.generate_circular_pocket(circle, step_down)
    }

    /// Generates a pocket toolpath for a polyline.
    pub fn generate_polyline_pocket(&self, vertices: &[Point], pocket_depth: f64, step_down: f64, step_in: f64) -> Vec<Toolpath> {
        let op = PocketOperation::new("polyline_pocket".to_string(), pocket_depth, self.tool_diameter);
        let mut gen = PocketGenerator::new(op);
        gen.operation.set_start_depth(self.start_depth);
        let effective_step_in = if step_in > 0.0 { step_in } else { self.step_in };
        gen.operation.set_parameters(effective_step_in, self.feed_rate, self.spindle_speed);
        gen.operation.set_strategy(self.pocket_strategy);
        gen.generate_polygon_pocket(vertices, step_down)
    }

    /// Generates a contour toolpath for a PathShape.
    pub fn generate_path_contour(&self, path_shape: &PathShape, step_down: f64) -> Vec<Toolpath> {
        let mut segments = Vec::new();
        let tolerance = 0.05; // mm
        
        let mut current_point = Point::new(0.0, 0.0);
        let mut start_point = Point::new(0.0, 0.0);
        
        // Calculate center for rotation (unrotated bounding box)
        let rect = lyon::algorithms::aabb::bounding_box(&path_shape.path);
        let center = Point::new(
            (rect.min.x + rect.max.x) as f64 / 2.0,
            (rect.min.y + rect.max.y) as f64 / 2.0
        );
        let rotation = path_shape.rotation;
        
        for event in path_shape.path.iter().flattened(tolerance) {
            match event {
                lyon::path::Event::Begin { at } => {
                    let mut p = Point::new(at.x as f64, at.y as f64);
                    if rotation.abs() > 1e-6 {
                        p = crate::shapes::rotate_point(p, center, rotation);
                    }
                    segments.push(ToolpathSegment::new(
                        ToolpathSegmentType::RapidMove,
                        current_point,
                        p,
                        self.feed_rate,
                        self.spindle_speed
                    ));
                    current_point = p;
                    start_point = p;
                },
                lyon::path::Event::Line { from: _, to } => {
                    let mut p = Point::new(to.x as f64, to.y as f64);
                    if rotation.abs() > 1e-6 {
                        p = crate::shapes::rotate_point(p, center, rotation);
                    }
                    segments.push(ToolpathSegment::new(
                        ToolpathSegmentType::LinearMove,
                        current_point,
                        p,
                        self.feed_rate,
                        self.spindle_speed
                    ));
                    current_point = p;
                },
                lyon::path::Event::End { last: _, first: _, close } => {
                    if close {
                        segments.push(ToolpathSegment::new(
                            ToolpathSegmentType::LinearMove,
                            current_point,
                            start_point,
                            self.feed_rate,
                            self.spindle_speed
                        ));
                        current_point = start_point;
                    }
                },
                _ => {}
            }
        }
        
        // Return to origin
        segments.push(ToolpathSegment::new(
            ToolpathSegmentType::RapidMove,
            current_point,
            Point::new(0.0, 0.0),
            self.feed_rate,
            self.spindle_speed
        ));
        
        self.create_multipass_toolpaths(segments, step_down)
    }

    /// Generates a pocket toolpath for a PathShape.
    pub fn generate_path_pocket(&self, path_shape: &PathShape, pocket_depth: f64, step_down: f64, step_in: f64) -> Vec<Toolpath> {
        // Flatten path to polyline and use polyline pocket generation
        let tolerance = 0.1; // mm
        let mut vertices = Vec::new();
        
        // Calculate center for rotation
        let rect = lyon::algorithms::aabb::bounding_box(&path_shape.path);
        let center = Point::new(
            (rect.min.x + rect.max.x) as f64 / 2.0,
            (rect.min.y + rect.max.y) as f64 / 2.0
        );
        let rotation = path_shape.rotation;
        
        for event in path_shape.path.iter().flattened(tolerance) {
             match event {
                 lyon::path::Event::Begin { at } => {
                     let mut p = Point::new(at.x as f64, at.y as f64);
                     if rotation.abs() > 1e-6 {
                         p = crate::shapes::rotate_point(p, center, rotation);
                     }
                     vertices.push(p);
                 },
                 lyon::path::Event::Line { from: _, to } => {
                     let mut p = Point::new(to.x as f64, to.y as f64);
                     if rotation.abs() > 1e-6 {
                         p = crate::shapes::rotate_point(p, center, rotation);
                     }
                     vertices.push(p);
                 },
                 _ => {} 
             }
        }
        
        let polyline_vertices = vertices;
        self.generate_polyline_pocket(&polyline_vertices, pocket_depth, step_down, step_in)
    }

    /// Generates a toolpath for text.
    pub fn generate_text_toolpath(&self, text_shape: &crate::shapes::TextShape, step_down: f64) -> Vec<Toolpath> {
        let mut segments = Vec::new();
        let font = font_manager::get_font();
        let scale = Scale::uniform(text_shape.font_size as f32);
        let v_metrics = font.v_metrics(scale);
        let start = rt_point(text_shape.x as f32, text_shape.y as f32 + v_metrics.ascent);
        
        for glyph in font.layout(&text_shape.text, scale, start) {
             let mut builder = ToolpathBuilder::new(self.feed_rate, self.spindle_speed);
             glyph.build_outline(&mut builder);
             segments.extend(builder.segments);
        }
        
        self.create_multipass_toolpaths(segments, step_down)
    }
}

struct ToolpathBuilder {
    segments: Vec<ToolpathSegment>,
    current_point: Point,
    start_point: Point,
    feed_rate: f64,
    spindle_speed: u32,
}

impl ToolpathBuilder {
    fn new(feed_rate: f64, spindle_speed: u32) -> Self {
        Self {
            segments: Vec::new(),
            current_point: Point::new(0.0, 0.0),
            start_point: Point::new(0.0, 0.0),
            feed_rate,
            spindle_speed,
        }
    }
}

impl OutlineBuilder for ToolpathBuilder {
    fn move_to(&mut self, x: f32, y: f32) {
        let p = Point::new(x as f64, y as f64);
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
        let p = Point::new(x as f64, y as f64);
        self.segments.push(ToolpathSegment::new(
            ToolpathSegmentType::LinearMove,
            self.current_point,
            p,
            self.feed_rate,
            self.spindle_speed,
        ));
        self.current_point = p;
    }

    fn quad_to(&mut self, _x1: f32, _y1: f32, x: f32, y: f32) {
        // Approximate quadratic bezier with line for now
        self.line_to(x, y);
    }

    fn curve_to(&mut self, _x1: f32, _y1: f32, _x2: f32, _y2: f32, x: f32, y: f32) {
        // Approximate cubic bezier with line for now
        self.line_to(x, y);
    }

    fn close(&mut self) {
        self.segments.push(ToolpathSegment::new(
            ToolpathSegmentType::LinearMove,
            self.current_point,
            self.start_point,
            self.feed_rate,
            self.spindle_speed,
        ));
        self.current_point = self.start_point;
    }
}

impl Default for ToolpathGenerator {
    fn default() -> Self {
        Self::new()
    }
}


