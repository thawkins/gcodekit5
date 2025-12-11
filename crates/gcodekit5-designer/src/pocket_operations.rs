//! Pocket operations for CAM toolpath generation.
//!
//! Implements pocket milling operations with island detection and offset path generation.
//! Supports outline pocket and island preservation.

use super::shapes::{Circle, Point, Rectangle};
use super::toolpath::{Toolpath, ToolpathSegment, ToolpathSegmentType};
use std::f64::consts::PI;
use cavalier_contours::polyline::{Polyline, PlineSource, PlineVertex, PlineSourceMut};

/// Strategy for pocket milling.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PocketStrategy {
    /// Zig-Zag or Raster milling.
    Raster {
        angle: f64,
        bidirectional: bool,
    },
    /// Contour-parallel (offset) milling.
    ContourParallel,
    /// Adaptive clearing (trochoidal-like).
    Adaptive,
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

    /// Generates a pocket toolpath for a rectangular outline.
    pub fn generate_rectangular_pocket(&self, rect: &Rectangle, step_down: f64) -> Vec<Toolpath> {
        if let PocketStrategy::ContourParallel = self.operation.strategy {
            let mut toolpaths = Vec::new();

            let half_tool = self.operation.tool_diameter / 2.0;
            let max_offset = rect.width.min(rect.height) / 2.0;
            
            // Calculate Z passes
            let total_depth = self.operation.depth.abs();
            let z_step = if step_down > 0.0 { step_down } else { total_depth };
            let z_passes = (total_depth / z_step).ceil() as u32;
            
            for z_pass in 1..=z_passes {
                let current_z = self.operation.start_depth - (z_step * z_pass as f64).min(total_depth);
                let mut toolpath = Toolpath::new(self.operation.tool_diameter, current_z);
                
                // Start from the outside (boundary) and work inwards
                let mut current_offset = half_tool;

                while current_offset < max_offset {
                    let inset_x = rect.x + current_offset;
                    let inset_y = rect.y + current_offset;
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
                         toolpath.add_segment(ToolpathSegment::new(
                            ToolpathSegmentType::RapidMove,
                            Point::new(0.0, 0.0), // Start point ignored for Rapid in current logic? No, it uses end.
                            *first_point,
                            self.operation.feed_rate,
                            self.operation.spindle_speed,
                        ));
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
            }

            toolpaths
        } else {
             // Convert to polygon and use generic generator
             let vertices = vec![
                 Point::new(rect.x, rect.y),
                 Point::new(rect.x + rect.width, rect.y),
                 Point::new(rect.x + rect.width, rect.y + rect.height),
                 Point::new(rect.x, rect.y + rect.height),
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
            let z_step = if step_down > 0.0 { step_down } else { total_depth };
            let z_passes = (total_depth / z_step).ceil() as u32;
            
            for z_pass in 1..=z_passes {
                let current_z = self.operation.start_depth - (z_step * z_pass as f64).min(total_depth);
                let mut toolpath = Toolpath::new(self.operation.tool_diameter, current_z);
                
                let mut current_offset = half_tool;

                while current_offset < max_offset {
                    let inset_radius = circle.radius - current_offset;
                    if inset_radius <= 0.0 {
                        break;
                    }

                    let segments = 36;
                    
                    // Add rapid move to start of circle
                    let start_angle: f64 = 0.0;
                    let start_x = circle.center.x + inset_radius * start_angle.cos();
                    let start_y = circle.center.y + inset_radius * start_angle.sin();
                    
                    toolpath.add_segment(ToolpathSegment::new(
                        ToolpathSegmentType::RapidMove,
                        Point::new(0.0, 0.0),
                        Point::new(start_x, start_y),
                        self.operation.feed_rate,
                        self.operation.spindle_speed,
                    ));

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
                toolpaths.push(toolpath);
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
            PocketStrategy::Raster { angle, bidirectional } => {
                self.generate_raster_pocket(vertices, step_down, angle, bidirectional)
            },
            PocketStrategy::ContourParallel => {
                self.generate_contour_parallel_pocket(vertices, step_down)
            },
            PocketStrategy::Adaptive => {
                self.generate_adaptive_pocket(vertices, step_down)
            }
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

    fn generate_contour_parallel_pocket(&self, vertices: &[Point], step_down: f64) -> Vec<Toolpath> {
        let mut toolpaths = Vec::new();
        if vertices.is_empty() { return toolpaths; }

        let polyline = Self::prepare_polygon(vertices);

        // Calculate Z passes
        let total_depth = self.operation.depth.abs();
        let z_step = if step_down > 0.0 { step_down } else { total_depth };
        let z_passes = (total_depth / z_step).ceil() as u32;
        let tool_radius = self.operation.tool_diameter / 2.0;

        for z_pass in 1..=z_passes {
            let current_z = self.operation.start_depth - (z_step * z_pass as f64).min(total_depth);
            let mut toolpath = Toolpath::new(self.operation.tool_diameter, current_z);
            
            let mut current_offset = tool_radius;
            let has_paths = true;


            while has_paths {
                // Offset inwards
                let offsets = polyline.parallel_offset(-current_offset);
                
                if offsets.is_empty() {
                    break;
                }

                for (_path_idx, offset_path) in offsets.iter().enumerate() {
                    let mut points = Vec::new();
                    for v in &offset_path.vertex_data {
                        points.push(Point::new(v.x, v.y));
                    }
                    
                    if points.len() < 2 { continue; }

                    // Close the loop
                    points.push(points[0]);

                    // Add rapid to start
                    toolpath.add_segment(ToolpathSegment::new(
                        ToolpathSegmentType::RapidMove,
                        Point::new(0.0, 0.0),
                        points[0],
                        self.operation.feed_rate,
                        self.operation.spindle_speed,
                    ));

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
            toolpaths.push(toolpath);
        }
        toolpaths
    }

    fn generate_adaptive_pocket(&self, vertices: &[Point], step_down: f64) -> Vec<Toolpath> {
        let mut toolpaths = Vec::new();
        if vertices.is_empty() { return toolpaths; }

        let polyline = Self::prepare_polygon(vertices);

        // Calculate Z passes
        let total_depth = self.operation.depth.abs();
        let z_step = if step_down > 0.0 { step_down } else { total_depth };
        let z_passes = (total_depth / z_step).ceil() as u32;
        let tool_radius = self.operation.tool_diameter / 2.0;

        // Generate all offset levels first (Outside-In)
        let mut levels = Vec::new();
        let mut current_offset = tool_radius;
        
        loop {
            let offsets = polyline.parallel_offset(-current_offset);
            if offsets.is_empty() {
                break;
            }
            levels.push(offsets);
            current_offset += self.operation.stepover;
        }

        // Reverse levels to go Inside-Out
        levels.reverse();

        for z_pass in 1..=z_passes {
            let current_z = self.operation.start_depth - (z_step * z_pass as f64).min(total_depth);
            let mut toolpath = Toolpath::new(self.operation.tool_diameter, current_z);
            
            // Helical Entry for the first (innermost) level
            if let Some(first_level) = levels.first() {
                if let Some(first_path) = first_level.first() {
                    if !first_path.vertex_data.is_empty() {
                        let start_pt = Point::new(first_path.vertex_data[0].x, first_path.vertex_data[0].y);
                        
                        // Generate helix
                        // let helix_radius = self.operation.tool_diameter * 0.25;
                        // let helix_center = start_pt; 
                        // Ideally helix should be inside the pocket. 
                        // But start_pt is on the path.
                        // For now, just ramp down to start_pt
                        
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

            for level in &levels {
                for offset_path in level {
                    let mut points = Vec::new();
                    for v in &offset_path.vertex_data {
                        points.push(Point::new(v.x, v.y));
                    }
                    
                    if points.len() < 2 { continue; }

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
            toolpaths.push(toolpath);
        }
        toolpaths
    }

    fn generate_raster_pocket(&self, vertices: &[Point], step_down: f64, angle: f64, bidirectional: bool) -> Vec<Toolpath> {
        let mut toolpaths = Vec::new();
        
        if vertices.is_empty() {
            return toolpaths;
        }

        // Rotate vertices to align with X axis
        let cos_a = (-angle).to_radians().cos();
        let sin_a = (-angle).to_radians().sin();
        
        let rotate = |p: Point| -> Point {
            Point::new(
                p.x * cos_a - p.y * sin_a,
                p.x * sin_a + p.y * cos_a
            )
        };

        let inv_rotate = |p: Point| -> Point {
            let cos_inv = angle.to_radians().cos();
            let sin_inv = angle.to_radians().sin();
            Point::new(
                p.x * cos_inv - p.y * sin_inv,
                p.x * sin_inv + p.y * cos_inv
            )
        };

        let rotated_vertices: Vec<Point> = vertices.iter().map(|&p| rotate(p)).collect();

        // Calculate bounding box of rotated vertices
        let mut min_x = rotated_vertices[0].x;
        let mut min_y = rotated_vertices[0].y;
        let mut max_x = rotated_vertices[0].x;
        let mut max_y = rotated_vertices[0].y;

        for v in &rotated_vertices {
            if v.x < min_x { min_x = v.x; }
            if v.x > max_x { max_x = v.x; }
            if v.y < min_y { min_y = v.y; }
            if v.y > max_y { max_y = v.y; }
        }
        
        // Calculate Z passes
        let total_depth = self.operation.depth.abs();
        let z_step = if step_down > 0.0 { step_down } else { total_depth };
        let z_passes = (total_depth / z_step).ceil() as u32;
        
        let tool_radius = self.operation.tool_diameter / 2.0;
        
        for z_pass in 1..=z_passes {
            let current_z = self.operation.start_depth - (z_step * z_pass as f64).min(total_depth);
            let mut toolpath = Toolpath::new(self.operation.tool_diameter, current_z);
            
            // Scanline fill
            let mut current_y = min_y + tool_radius;
            let limit_y = max_y - tool_radius;
            let mut forward = true;
            
            while current_y <= limit_y {
                let mut intersections = Vec::new();
                
                if rotated_vertices.len() < 3 { break; }
                
                for i in 0..rotated_vertices.len() {
                    let p1 = rotated_vertices[i];
                    let p2 = rotated_vertices[(i + 1) % rotated_vertices.len()];
                    
                    if (p1.y <= current_y && p2.y > current_y) || (p2.y <= current_y && p1.y > current_y) {
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
                        let x_end = intersections[i+1];
                        
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
                            toolpath.add_segment(ToolpathSegment::new(
                                ToolpathSegmentType::RapidMove,
                                Point::new(0.0, 0.0),
                                start_pt,
                                self.operation.feed_rate,
                                self.operation.spindle_speed,
                            ));
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

            let inset_x = rect.x + offset;
            let inset_y = rect.y + offset;
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


