//! Stock Removal Simulation Module
//!
//! This module provides data structures and algorithms for simulating material removal
//! during CNC machining operations. It supports both 2D height-map based simulation
//! and 3D voxel-based simulation.

/// Represents the stock material dimensions and position
#[derive(Debug, Clone)]
pub struct StockMaterial {
    /// Width in X dimension (mm)
    pub width: f32,
    /// Height in Y dimension (mm)
    pub height: f32,
    /// Thickness in Z dimension (mm)
    pub thickness: f32,
    /// Origin point (bottom-left corner) in world coordinates
    pub origin: (f32, f32, f32),
    /// Safe Z height for rapid moves (mm)
    pub safe_z: f32,
}

impl StockMaterial {
    /// Create a new stock material definition
    pub fn new(width: f32, height: f32, thickness: f32, origin: (f32, f32, f32)) -> Self {
        Self {
            width,
            height,
            thickness,
            origin,
            safe_z: 10.0,
        }
    }

    /// Create a new stock material definition with custom safe Z height
    pub fn with_safe_z(
        width: f32,
        height: f32,
        thickness: f32,
        origin: (f32, f32, f32),
        safe_z: f32,
    ) -> Self {
        Self {
            width,
            height,
            thickness,
            origin,
            safe_z,
        }
    }

    /// Get the center point of the stock
    pub fn center(&self) -> (f32, f32, f32) {
        (
            self.origin.0 + self.width / 2.0,
            self.origin.1 + self.height / 2.0,
            self.origin.2 + self.thickness / 2.0,
        )
    }

    /// Get the top surface Z coordinate
    pub fn top_z(&self) -> f32 {
        self.origin.2 + self.thickness
    }

    /// Check if a point is within stock bounds
    pub fn contains(&self, x: f32, y: f32, z: f32) -> bool {
        x >= self.origin.0
            && x <= self.origin.0 + self.width
            && y >= self.origin.1
            && y <= self.origin.1 + self.height
            && z >= self.origin.2
            && z <= self.origin.2 + self.thickness
    }
}

/// 2D height map for representing the top surface of the stock after machining
#[derive(Debug, Clone)]
pub struct HeightMap2D {
    /// Resolution in mm per pixel (e.g., 0.1mm means each pixel represents 0.1mm x 0.1mm)
    pub resolution: f32,
    /// Width in pixels
    pub width_px: usize,
    /// Height in pixels
    pub height_px: usize,
    /// Z heights at each XY position (stored row-major: y * width + x)
    pub heights: Vec<f32>,
    /// Origin point in world coordinates (bottom-left corner)
    pub origin: (f32, f32),
}

impl HeightMap2D {
    /// Create a new height map from stock material dimensions
    pub fn new(stock: &StockMaterial, resolution: f32) -> Self {
        let width_px = (stock.width / resolution).ceil() as usize;
        let height_px = (stock.height / resolution).ceil() as usize;
        let initial_height = stock.origin.2 + stock.thickness;

        Self {
            resolution,
            width_px,
            height_px,
            heights: vec![initial_height; width_px * height_px],
            origin: (stock.origin.0, stock.origin.1),
        }
    }

    /// Get height at world coordinates (x, y)
    /// Returns None if coordinates are outside the height map
    pub fn get_height(&self, x: f32, y: f32) -> Option<f32> {
        let px = ((x - self.origin.0) / self.resolution) as isize;
        let py = ((y - self.origin.1) / self.resolution) as isize;

        if px < 0 || py < 0 || px >= self.width_px as isize || py >= self.height_px as isize {
            return None;
        }

        let index = (py as usize) * self.width_px + (px as usize);
        Some(self.heights[index])
    }

    /// Set height at world coordinates (x, y)
    /// Does nothing if coordinates are outside the height map
    pub fn set_height(&mut self, x: f32, y: f32, z: f32) {
        let px = ((x - self.origin.0) / self.resolution) as isize;
        let py = ((y - self.origin.1) / self.resolution) as isize;

        if px < 0 || py < 0 || px >= self.width_px as isize || py >= self.height_px as isize {
            return;
        }

        let index = (py as usize) * self.width_px + (px as usize);
        self.heights[index] = z;
    }

    /// Get height at pixel coordinates
    pub fn get_height_at_pixel(&self, px: usize, py: usize) -> Option<f32> {
        if px >= self.width_px || py >= self.height_px {
            return None;
        }
        let index = py * self.width_px + px;
        Some(self.heights[index])
    }

    /// Set height at pixel coordinates
    pub fn set_height_at_pixel(&mut self, px: usize, py: usize, z: f32) {
        if px >= self.width_px || py >= self.height_px {
            return;
        }
        let index = py * self.width_px + px;
        self.heights[index] = z;
    }

    /// Convert world coordinates to pixel coordinates
    pub fn world_to_pixel(&self, x: f32, y: f32) -> (isize, isize) {
        let px = ((x - self.origin.0) / self.resolution) as isize;
        let py = ((y - self.origin.1) / self.resolution) as isize;
        (px, py)
    }

    /// Convert pixel coordinates to world coordinates (center of pixel)
    pub fn pixel_to_world(&self, px: usize, py: usize) -> (f32, f32) {
        let x = self.origin.0 + (px as f32 + 0.5) * self.resolution;
        let y = self.origin.1 + (py as f32 + 0.5) * self.resolution;
        (x, y)
    }

    /// Get the minimum height in the map
    pub fn min_height(&self) -> f32 {
        self.heights.iter().copied().fold(f32::INFINITY, f32::min)
    }

    /// Get the maximum height in the map
    pub fn max_height(&self) -> f32 {
        self.heights
            .iter()
            .copied()
            .fold(f32::NEG_INFINITY, f32::max)
    }
}

/// Result of a stock removal simulation
#[derive(Debug, Clone)]
pub struct SimulationResult {
    /// The height map after simulation
    pub height_map: HeightMap2D,
    /// Total volume of material removed (mm³)
    pub material_removed: f32,
    /// Minimum Z value (deepest cut)
    pub min_z: f32,
    /// Maximum Z value (highest remaining surface)
    pub max_z: f32,
}

impl SimulationResult {
    /// Create a new simulation result from a height map
    pub fn from_height_map(height_map: HeightMap2D, _original_thickness: f32) -> Self {
        let min_z = height_map.min_height();
        let max_z = height_map.max_height();

        // Calculate volume removed
        let pixel_area = height_map.resolution * height_map.resolution;
        let mut material_removed = 0.0;

        for &height in &height_map.heights {
            let depth = max_z - height;
            if depth > 0.0 {
                material_removed += depth * pixel_area;
            }
        }

        Self {
            height_map,
            material_removed,
            min_z,
            max_z,
        }
    }

    /// Get the percentage of material removed
    pub fn removal_percentage(&self, stock: &StockMaterial) -> f32 {
        let total_volume = stock.width * stock.height * stock.thickness;
        if total_volume > 0.0 {
            (self.material_removed / total_volume) * 100.0
        } else {
            0.0
        }
    }
}

/// 2D Stock simulator that processes toolpaths and updates height map
pub struct StockSimulator2D {
    /// The stock material being simulated
    pub stock: StockMaterial,
    /// Height map tracking the material surface
    pub height_map: HeightMap2D,
    /// Tool radius in mm
    pub tool_radius: f32,
}

impl StockSimulator2D {
    /// Create a new 2D stock simulator
    pub fn new(stock: StockMaterial, resolution: f32, tool_radius: f32) -> Self {
        let height_map = HeightMap2D::new(&stock, resolution);
        Self {
            stock,
            height_map,
            tool_radius,
        }
    }

    /// Simulate a toolpath and update the height map
    pub fn simulate_toolpath(&mut self, segments: &[crate::toolpath::ToolpathSegment]) {
        // Track coordinate ranges for debugging
        let mut min_x = f32::INFINITY;
        let mut max_x = f32::NEG_INFINITY;
        let mut min_y = f32::INFINITY;
        let mut max_y = f32::NEG_INFINITY;
        let mut processed_count = 0;
        let mut skipped_count = 0;

        for segment in segments {
            // Update ranges
            min_x = min_x.min(segment.start.x as f32).min(segment.end.x as f32);
            max_x = max_x.max(segment.start.x as f32).max(segment.end.x as f32);
            min_y = min_y.min(segment.start.y as f32).min(segment.end.y as f32);
            max_y = max_y.max(segment.start.y as f32).max(segment.end.y as f32);

            match segment.segment_type {
                crate::toolpath::ToolpathSegmentType::LinearMove => {
                    self.simulate_linear_move(segment);
                    processed_count += 1;
                }
                crate::toolpath::ToolpathSegmentType::ArcCW
                | crate::toolpath::ToolpathSegmentType::ArcCCW => {
                    self.simulate_arc(segment);
                    processed_count += 1;
                }
                crate::toolpath::ToolpathSegmentType::RapidMove => {
                    // Rapid moves don't cut material
                    skipped_count += 1;
                }
            }
        }

        eprintln!(
            "DEBUG: Toolpath coordinate range: X:[{:.2}, {:.2}], Y:[{:.2}, {:.2}]",
            min_x, max_x, min_y, max_y
        );
        eprintln!(
            "DEBUG: Stock bounds: X:[{:.2}, {:.2}], Y:[{:.2}, {:.2}]",
            self.stock.origin.0,
            self.stock.origin.0 + self.stock.width,
            self.stock.origin.1,
            self.stock.origin.1 + self.stock.height
        );
        eprintln!(
            "DEBUG: Processed {} cutting moves, skipped {} rapid moves",
            processed_count, skipped_count
        );
    }

    /// Simulate a linear cutting move
    fn simulate_linear_move(&mut self, segment: &crate::toolpath::ToolpathSegment) {
        let start = &segment.start;
        let end = &segment.end;
        // Convert Z depth to height in stock (stock top is at thickness, Z=0 means cutting to stock top)
        let z_depth = segment.z_depth.unwrap_or(0.0) as f32;
        let z = self.stock.thickness - z_depth.abs();

        // Debug first few moves
        static MOVE_COUNT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let count = MOVE_COUNT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        if count < 5 {
            eprintln!(
                "DEBUG: Linear move {} - z_depth={}, calculated z={}, stock.thickness={}",
                count, z_depth, z, self.stock.thickness
            );
        }

        let dx = end.x - start.x;
        let dy = end.y - start.y;
        let distance = (dx * dx + dy * dy).sqrt();

        if distance < 0.001 {
            // Very short move, just process as single point
            self.apply_tool_footprint(end.x as f32, end.y as f32, z);
            return;
        }

        // Interpolate along the line with step size equal to half the resolution
        let step_size = self.height_map.resolution * 0.5;
        let num_steps = (distance / step_size as f64).ceil() as usize + 1;

        for i in 0..num_steps {
            let t = (i as f64) / (num_steps - 1) as f64;
            let x = start.x + dx * t;
            let y = start.y + dy * t;

            self.apply_tool_footprint(x as f32, y as f32, z);
        }
    }

    /// Simulate an arc move
    fn simulate_arc(&mut self, segment: &crate::toolpath::ToolpathSegment) {
        let center = match segment.center {
            Some(c) => c,
            None => return, // Invalid arc, skip
        };

        let start = &segment.start;
        let end = &segment.end;
        // Convert Z depth to height in stock (stock top is at thickness, Z=0 means cutting to stock top)
        let z_depth = segment.z_depth.unwrap_or(0.0) as f32;
        let z = self.stock.thickness - z_depth.abs();

        // Calculate arc parameters
        let radius = ((start.x - center.x).powi(2) + (start.y - center.y).powi(2)).sqrt();
        let start_angle = (start.y - center.y).atan2(start.x - center.x);
        let end_angle = (end.y - center.y).atan2(end.x - center.x);

        let mut angle_diff = end_angle - start_angle;

        // Normalize angle difference based on arc direction
        if segment.segment_type == crate::toolpath::ToolpathSegmentType::ArcCW {
            while angle_diff > 0.0 {
                angle_diff -= 2.0 * std::f64::consts::PI;
            }
        } else {
            while angle_diff < 0.0 {
                angle_diff += 2.0 * std::f64::consts::PI;
            }
        }

        let arc_length = radius * angle_diff.abs();
        let step_size = self.height_map.resolution as f64 * 0.5;
        let num_steps = (arc_length / step_size).ceil() as usize + 1;

        for i in 0..num_steps {
            let t = (i as f64) / (num_steps - 1) as f64;
            let angle = start_angle + angle_diff * t;
            let x = center.x + radius * angle.cos();
            let y = center.y + radius * angle.sin();

            self.apply_tool_footprint(x as f32, y as f32, z);
        }
    }

    /// Apply the circular tool footprint at a given position
    /// Updates all pixels within the tool radius to the cutting depth
    fn apply_tool_footprint(&mut self, cx: f32, cy: f32, cz: f32) {
        let radius_px = (self.tool_radius / self.height_map.resolution).ceil() as isize;
        let (center_px, center_py) = self.height_map.world_to_pixel(cx, cy);

        // Debug first few footprint calls
        static FOOTPRINT_COUNT: std::sync::atomic::AtomicUsize =
            std::sync::atomic::AtomicUsize::new(0);
        let count = FOOTPRINT_COUNT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        if count < 5 {
            eprintln!("DEBUG: apply_tool_footprint #{} - pos:({:.2},{:.2}), z:{:.2}, tool_radius:{:.2}, radius_px:{}, center_px:({},{})", 
                count, cx, cy, cz, self.tool_radius, radius_px, center_px, center_py);
        }

        // Iterate over a square bounding box around the tool
        for py in (center_py - radius_px)..=(center_py + radius_px) {
            for px in (center_px - radius_px)..=(center_px + radius_px) {
                if px < 0
                    || py < 0
                    || px >= self.height_map.width_px as isize
                    || py >= self.height_map.height_px as isize
                {
                    continue;
                }

                // Get world coordinates of this pixel
                let (world_x, world_y) = self.height_map.pixel_to_world(px as usize, py as usize);

                // Check if pixel is within tool radius
                let dx = world_x - cx;
                let dy = world_y - cy;
                let distance = (dx * dx + dy * dy).sqrt();

                if distance <= self.tool_radius {
                    // Update height to minimum of current height and cutting depth
                    let current_height = self
                        .height_map
                        .get_height_at_pixel(px as usize, py as usize)
                        .unwrap_or(cz);
                    let new_height = current_height.min(cz);
                    self.height_map
                        .set_height_at_pixel(px as usize, py as usize, new_height);
                }
            }
        }
    }

    /// Get the simulation result
    pub fn get_simulation_result(&self) -> SimulationResult {
        SimulationResult::from_height_map(self.height_map.clone(), self.stock.thickness)
    }
}

/// Visualization module for rendering stock removal simulation results
pub mod visualization {
    use super::*;

    /// A 2D point
    #[derive(Debug, Clone, Copy)]
    pub struct Point2D {
        pub x: f32,
        pub y: f32,
    }

    impl Point2D {
        pub fn new(x: f32, y: f32) -> Self {
            Self { x, y }
        }
    }

    /// Generate contour lines at a specified Z level using marching squares
    /// Returns a list of closed polygons representing the material boundary
    pub fn generate_2d_contours(height_map: &HeightMap2D, z_level: f32) -> Vec<Vec<Point2D>> {
        let mut contours = Vec::new();

        // Marching squares lookup table
        // Each cell has 4 corners, each can be above or below threshold (16 cases)
        // We'll generate line segments based on which edges the contour crosses

        for py in 0..(height_map.height_px - 1) {
            for px in 0..(height_map.width_px - 1) {
                // Get the 4 corner heights
                let h00 = height_map.get_height_at_pixel(px, py).unwrap_or(z_level);
                let h10 = height_map
                    .get_height_at_pixel(px + 1, py)
                    .unwrap_or(z_level);
                let h01 = height_map
                    .get_height_at_pixel(px, py + 1)
                    .unwrap_or(z_level);
                let h11 = height_map
                    .get_height_at_pixel(px + 1, py + 1)
                    .unwrap_or(z_level);

                // Create a case index (4-bit number representing which corners are below z_level)
                let mut case = 0;
                if h00 <= z_level {
                    case |= 1;
                }
                if h10 <= z_level {
                    case |= 2;
                }
                if h01 <= z_level {
                    case |= 4;
                }
                if h11 <= z_level {
                    case |= 8;
                }

                // Generate segments based on case
                // For simplicity, we're generating individual segments
                // A full implementation would trace complete contours
                let segments = get_marching_squares_segments(case, px, py, height_map, z_level);

                if !segments.is_empty() {
                    // Convert segments to a polygon (simplified - just add all points)
                    let mut polygon = Vec::new();
                    for (p1, p2) in segments {
                        polygon.push(p1);
                        polygon.push(p2);
                    }
                    if !polygon.is_empty() {
                        contours.push(polygon);
                    }
                }
            }
        }

        contours
    }

    /// Get line segments for a marching squares case
    fn get_marching_squares_segments(
        case: u8,
        px: usize,
        py: usize,
        height_map: &HeightMap2D,
        _z_level: f32,
    ) -> Vec<(Point2D, Point2D)> {
        let mut segments = Vec::new();

        // Get world coordinates of cell corners
        let (x0, y0) = height_map.pixel_to_world(px, py);
        let (x1, y1) = height_map.pixel_to_world(px + 1, py + 1);

        // Midpoint of edges
        let left = Point2D::new(x0, (y0 + y1) / 2.0);
        let right = Point2D::new(x1, (y0 + y1) / 2.0);
        let top = Point2D::new((x0 + x1) / 2.0, y1);
        let bottom = Point2D::new((x0 + x1) / 2.0, y0);

        // Marching squares cases (simplified - connects edge midpoints)
        match case {
            1 | 14 => segments.push((left, bottom)),
            2 | 13 => segments.push((bottom, right)),
            3 | 12 => segments.push((left, right)),
            4 | 11 => segments.push((left, top)),
            5 => {
                segments.push((left, bottom));
                segments.push((right, top));
            }
            6 | 9 => segments.push((bottom, top)),
            7 | 8 => segments.push((top, right)),
            10 => {
                segments.push((left, top));
                segments.push((bottom, right));
            }
            _ => {} // Case 0 and 15 have no intersections
        }

        segments
    }

    /// Generate a removal overlay showing depths
    /// Returns vertices with depth values for visualization
    pub fn generate_removal_overlay(
        height_map: &HeightMap2D,
        original_height: f32,
    ) -> Vec<(Point2D, f32)> {
        let mut vertices = Vec::new();

        // Generate a vertex for each pixel that has material removed
        for py in 0..height_map.height_px {
            for px in 0..height_map.width_px {
                if let Some(height) = height_map.get_height_at_pixel(px, py) {
                    let depth = original_height - height;

                    // Only include pixels where material was removed
                    if depth > 0.001 {
                        let (x, y) = height_map.pixel_to_world(px, py);
                        vertices.push((Point2D::new(x, y), depth));
                    }
                }
            }
        }

        vertices
    }

    /// Map depth to a blue color with alpha
    /// Returns (r, g, b, a) in range 0.0-1.0
    pub fn depth_to_color(depth: f32, max_depth: f32) -> (f32, f32, f32, f32) {
        let normalized = if max_depth > 0.0 {
            (depth / max_depth).clamp(0.0, 1.0)
        } else {
            0.0
        };

        // Light blue (#ADD8E6) base color
        let base_r = 0.678;
        let base_g = 0.847;
        let base_b = 0.902;

        // Darken based on depth
        let factor = 1.0 - (normalized * 0.5); // Darker for deeper cuts

        (
            base_r * factor,
            base_g * factor,
            base_b * factor,
            0.5, // 50% transparency
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stock_material_creation() {
        let stock = StockMaterial::new(100.0, 200.0, 10.0, (0.0, 0.0, 0.0));
        assert_eq!(stock.width, 100.0);
        assert_eq!(stock.height, 200.0);
        assert_eq!(stock.thickness, 10.0);
        assert_eq!(stock.center(), (50.0, 100.0, 5.0));
        assert_eq!(stock.top_z(), 10.0);
    }

    #[test]
    fn test_stock_contains() {
        let stock = StockMaterial::new(100.0, 100.0, 10.0, (0.0, 0.0, 0.0));
        assert!(stock.contains(50.0, 50.0, 5.0));
        assert!(!stock.contains(-1.0, 50.0, 5.0));
        assert!(!stock.contains(50.0, 101.0, 5.0));
        assert!(!stock.contains(50.0, 50.0, 11.0));
    }

    #[test]
    fn test_height_map_creation() {
        let stock = StockMaterial::new(100.0, 100.0, 10.0, (0.0, 0.0, 0.0));
        let height_map = HeightMap2D::new(&stock, 1.0);

        assert_eq!(height_map.resolution, 1.0);
        assert_eq!(height_map.width_px, 100);
        assert_eq!(height_map.height_px, 100);
        assert_eq!(height_map.heights.len(), 10000);

        // All heights should be initialized to top of stock
        assert_eq!(height_map.get_height(50.0, 50.0), Some(10.0));
    }

    #[test]
    fn test_height_map_set_get() {
        let stock = StockMaterial::new(100.0, 100.0, 10.0, (0.0, 0.0, 0.0));
        let mut height_map = HeightMap2D::new(&stock, 1.0);

        height_map.set_height(50.0, 50.0, 5.0);
        assert_eq!(height_map.get_height(50.0, 50.0), Some(5.0));

        // Out of bounds should return None
        assert_eq!(height_map.get_height(101.0, 50.0), None);
    }

    #[test]
    fn test_world_to_pixel_conversion() {
        let stock = StockMaterial::new(100.0, 100.0, 10.0, (0.0, 0.0, 0.0));
        let height_map = HeightMap2D::new(&stock, 1.0);

        let (px, py) = height_map.world_to_pixel(50.5, 75.3);
        assert_eq!((px, py), (50, 75));

        let (x, y) = height_map.pixel_to_world(50, 75);
        assert!((x - 50.5).abs() < 0.1);
        assert!((y - 75.5).abs() < 0.1);
    }

    #[test]
    fn test_min_max_height() {
        let stock = StockMaterial::new(10.0, 10.0, 10.0, (0.0, 0.0, 0.0));
        let mut height_map = HeightMap2D::new(&stock, 1.0);

        height_map.set_height(5.0, 5.0, 3.0);
        height_map.set_height(7.0, 7.0, 12.0);

        assert_eq!(height_map.min_height(), 3.0);
        assert_eq!(height_map.max_height(), 12.0);
    }

    #[test]
    fn test_stock_simulator_creation() {
        let stock = StockMaterial::new(100.0, 100.0, 10.0, (0.0, 0.0, 0.0));
        let simulator = StockSimulator2D::new(stock, 1.0, 3.0);

        assert_eq!(simulator.tool_radius, 3.0);
        assert_eq!(simulator.height_map.width_px, 100);
    }

    #[test]
    fn test_linear_move_simulation() {
        use crate::toolpath::{ToolpathSegment, ToolpathSegmentType};
        use crate::Point;

        let stock = StockMaterial::new(100.0, 100.0, 10.0, (0.0, 0.0, 0.0));
        let mut simulator = StockSimulator2D::new(stock, 1.0, 3.0);

        // Create a linear move from (20, 20) to (80, 80) at Z=5
        let segment = ToolpathSegment::new(
            ToolpathSegmentType::LinearMove,
            Point { x: 20.0, y: 20.0 },
            Point { x: 80.0, y: 80.0 },
            100.0,
            10000,
        )
        .with_z_depth(5.0);

        simulator.simulate_toolpath(&[segment]);

        // Check that material was removed along the path
        let height_at_start = simulator.height_map.get_height(20.0, 20.0);
        assert!(height_at_start.is_some());
        assert!(height_at_start.unwrap() <= 5.0);

        let height_at_end = simulator.height_map.get_height(80.0, 80.0);
        assert!(height_at_end.is_some());
        assert!(height_at_end.unwrap() <= 5.0);
    }

    #[test]
    fn test_visualization_contours() {
        use visualization::generate_2d_contours;

        let stock = StockMaterial::new(10.0, 10.0, 10.0, (0.0, 0.0, 0.0));
        let mut height_map = HeightMap2D::new(&stock, 1.0);

        // Cut a simple valley in the middle
        for px in 3..7 {
            for py in 3..7 {
                height_map.set_height_at_pixel(px, py, 5.0);
            }
        }

        let contours = generate_2d_contours(&height_map, 7.5);
        assert!(
            !contours.is_empty(),
            "Should generate contours at intermediate level"
        );
    }

    #[test]
    fn test_visualization_overlay() {
        use visualization::generate_removal_overlay;

        let stock = StockMaterial::new(10.0, 10.0, 10.0, (0.0, 0.0, 0.0));
        let mut height_map = HeightMap2D::new(&stock, 1.0);

        // Remove some material
        height_map.set_height(5.0, 5.0, 5.0);

        let overlay = generate_removal_overlay(&height_map, 10.0);
        assert!(
            !overlay.is_empty(),
            "Should generate overlay vertices for removed material"
        );

        // Check that depth is calculated correctly
        let has_correct_depth = overlay.iter().any(|(_, depth)| (*depth - 5.0).abs() < 0.1);
        assert!(has_correct_depth, "Should have vertices with correct depth");
    }

    #[test]
    fn test_depth_to_color() {
        use visualization::depth_to_color;

        // Shallow cut should be lighter
        let (r1, g1, b1, a1) = depth_to_color(1.0, 10.0);
        // Deep cut should be darker
        let (r2, g2, b2, a2) = depth_to_color(10.0, 10.0);

        assert!(r1 > r2, "Shallow cuts should be lighter");
        assert!(g1 > g2, "Shallow cuts should be lighter");
        assert!(b1 > b2, "Shallow cuts should be lighter");
        assert_eq!(a1, 0.5, "Alpha should be 0.5");
        assert_eq!(a2, 0.5, "Alpha should be 0.5");
    }
}
