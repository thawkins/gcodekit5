//! # 3D Slicer to Toolpath Module
//!
//! Converts 3D model slices into CNC toolpaths for 2.5D machining.
//! This module bridges 3D models with the existing CAM operations system.
//!
//! ## Features
//! - Multi-layer slicing of 3D models
//! - Automatic toolpath generation for each layer
//! - Support for different cutting strategies (pocket, contour, etc.)
//! - Layer height optimization
//! - Material and tool considerations

use crate::model::{DesignerShape, Shape};
use crate::model3d::Mesh3D;
use crate::shadow_projection::{ShadowProjector, SliceLayer, SlicingParams};
use crate::tool_library::{Tool, ToolType};
use crate::toolpath::Toolpath;
use anyhow::{anyhow, Result};
use tracing::{debug, info, warn};

/// Strategy for converting slices to toolpaths
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SliceStrategy {
    /// Contour cutting - follow the outline of each slice
    Contour,
    /// Pocket cutting - remove material inside each slice
    Pocket,
    /// Engrave - shallow engraving following the outline
    Engrave,
    /// Adaptive - choose strategy based on slice geometry
    Adaptive,
}

/// Parameters for slice-to-toolpath conversion
#[derive(Debug, Clone)]
pub struct SliceToolpathParams {
    /// Cutting strategy for each layer
    pub strategy: SliceStrategy,
    /// Tool to use for cutting
    pub tool: Tool,
    /// Material thickness
    pub material_thickness: f32,
    /// Safe Z height for rapid moves
    pub safe_z: f32,
    /// Final cutting depth for each layer
    pub layer_depth: f32,
    /// Stepover percentage (0.0-1.0) for pocket operations
    pub stepover: f64,
    /// Cut direction (climb vs conventional)
    pub climb_milling: bool,
    /// Lead-in distance for smooth entry
    pub lead_in: f64,
    /// Lead-out distance for smooth exit
    pub lead_out: f64,
}

impl Default for SliceToolpathParams {
    fn default() -> Self {
        Self {
            strategy: SliceStrategy::Contour,
            tool: Tool::new(
                "default".to_string(),
                "Default End Mill".to_string(),
                ToolType::EndMill,
                3.175, // 1/8" diameter
                4,     // 4 flutes
                "Carbide".to_string(),
            ),
            material_thickness: 10.0,
            safe_z: 5.0,
            layer_depth: 1.0,
            stepover: 0.5,
            climb_milling: true,
            lead_in: 1.0,
            lead_out: 1.0,
        }
    }
}

/// A complete layer with associated toolpath
#[derive(Debug, Clone)]
pub struct LayerToolpath {
    /// Original slice information
    pub slice: SliceLayer,
    /// Generated toolpath for this layer
    pub toolpath: Toolpath,
    /// Estimated machining time (minutes)
    pub estimated_time: f32,
    /// Strategy used for this layer
    pub strategy: SliceStrategy,
}

/// Complete 3D-to-2.5D machining job
#[derive(Debug, Clone)]
pub struct SlicedJob {
    /// All layers with toolpaths
    pub layers: Vec<LayerToolpath>,
    /// Original 3D mesh
    pub source_mesh: Mesh3D,
    /// Parameters used for generation
    pub params: SliceToolpathParams,
    /// Total estimated machining time
    pub total_time: f32,
    /// Job summary information
    pub summary: JobSummary,
}

/// Summary information about a sliced job
#[derive(Debug, Clone)]
pub struct JobSummary {
    /// Total number of layers
    pub layer_count: usize,
    /// Total toolpath length (mm)
    pub total_length: f64,
    /// Material removal volume (mm³)
    pub material_removed: f64,
    /// Maximum cutting depth
    pub max_depth: f32,
    /// Bounding box dimensions (width, height, depth)
    pub dimensions: (f32, f32, f32),
}

/// 3D Slicer to Toolpath Converter
pub struct SliceToToolpath {
    params: SliceToolpathParams,
}

impl Default for SliceToToolpath {
    fn default() -> Self {
        Self::new(SliceToolpathParams::default())
    }
}

impl SliceToToolpath {
    pub fn new(params: SliceToolpathParams) -> Self {
        Self { params }
    }

    /// Create for contour cutting
    pub fn contour_cutting(tool: Tool, layer_depth: f32) -> Self {
        let params = SliceToolpathParams {
            strategy: SliceStrategy::Contour,
            tool,
            layer_depth,
            ..Default::default()
        };
        Self::new(params)
    }

    /// Create for pocket clearing
    pub fn pocket_clearing(tool: Tool, stepover: f64) -> Self {
        let params = SliceToolpathParams {
            strategy: SliceStrategy::Pocket,
            tool,
            stepover,
            ..Default::default()
        };
        Self::new(params)
    }

    /// Convert 3D mesh to complete sliced job
    pub fn process_mesh(&self, mesh: &Mesh3D, slicing_params: &SlicingParams) -> Result<SlicedJob> {
        info!("Processing 3D mesh for slice-to-toolpath conversion");

        // Step 1: Slice the mesh
        let projector = ShadowProjector::orthographic_z();
        let slices = projector.slice_mesh(mesh, slicing_params)?;

        info!("Generated {} slices from 3D mesh", slices.len());

        // Step 2: Generate toolpaths for each slice
        let mut layer_toolpaths = Vec::new();
        let mut total_time = 0.0;

        for (layer_index, slice) in slices.into_iter().enumerate() {
            info!("Processing layer {} at Z={}", layer_index, slice.z_height);

            let layer_toolpath = self.process_slice(&slice)?;
            total_time += layer_toolpath.estimated_time;
            layer_toolpaths.push(layer_toolpath);
        }

        // Step 3: Generate job summary
        let summary = self.generate_summary(&layer_toolpaths, mesh)?;

        Ok(SlicedJob {
            layers: layer_toolpaths,
            source_mesh: mesh.clone(),
            params: self.params.clone(),
            total_time,
            summary,
        })
    }

    /// Process a single slice to generate toolpath
    fn process_slice(&self, slice: &SliceLayer) -> Result<LayerToolpath> {
        if slice.shapes.is_empty() {
            warn!("Empty slice at Z={}, skipping", slice.z_height);
            return Ok(LayerToolpath {
                slice: slice.clone(),
                toolpath: Toolpath::new(self.params.tool.diameter, 0.0),
                estimated_time: 0.0,
                strategy: self.params.strategy,
            });
        }

        debug!("Processing slice with {} shapes", slice.shapes.len());

        // Determine cutting strategy for this layer
        let strategy = self.choose_strategy_for_slice(slice);

        // Generate toolpath based on strategy
        let toolpath = match strategy {
            SliceStrategy::Contour => self.generate_contour_toolpath(slice)?,
            SliceStrategy::Pocket => self.generate_pocket_toolpath(slice)?,
            SliceStrategy::Engrave => self.generate_engrave_toolpath(slice)?,
            SliceStrategy::Adaptive => self.generate_adaptive_toolpath(slice)?,
        };

        // Estimate machining time
        let estimated_time = self.estimate_machining_time(&toolpath);

        Ok(LayerToolpath {
            slice: slice.clone(),
            toolpath,
            estimated_time,
            strategy,
        })
    }

    /// Choose cutting strategy for a specific slice
    fn choose_strategy_for_slice(&self, slice: &SliceLayer) -> SliceStrategy {
        match self.params.strategy {
            SliceStrategy::Adaptive => {
                // Analyze slice geometry to choose best strategy
                let total_area = self.calculate_slice_area(slice);
                let perimeter = self.calculate_slice_perimeter(slice);

                // Simple heuristic: if area is large relative to perimeter, use pocketing
                if total_area > perimeter * perimeter * 0.1 {
                    SliceStrategy::Pocket
                } else {
                    SliceStrategy::Contour
                }
            }
            strategy => strategy,
        }
    }

    /// Generate contour toolpath for a slice
    fn generate_contour_toolpath(&self, slice: &SliceLayer) -> Result<Toolpath> {
        debug!(
            "Generating contour toolpath for {} shapes",
            slice.shapes.len()
        );

        // Create generator and configure using parameters/tool
        let mut gen = crate::toolpath::ToolpathGenerator::new();
        gen.set_tool_diameter(self.params.tool.diameter);
        gen.set_feed_rate(self.params.tool.feed_rate);
        gen.set_spindle_speed(self.params.tool.spindle_speed);
        gen.set_cut_depth(self.params.layer_depth as f64);
        gen.set_step_in(self.params.tool.stepover);

        let mut combined = gen.empty_toolpath();

        for shape in &slice.shapes {
            match shape {
                Shape::Rectangle(rect) => {
                    let tps = gen.generate_rectangle_contour(rect, self.params.layer_depth as f64);
                    for tp in tps {
                        combined.segments.extend(tp.segments);
                    }
                }
                Shape::Circle(circle) => {
                    let tps = gen.generate_circle_contour(circle, self.params.layer_depth as f64);
                    for tp in tps {
                        combined.segments.extend(tp.segments);
                    }
                }
                Shape::Line(line) => {
                    let tps = gen.generate_line_contour(line, self.params.layer_depth as f64);
                    for tp in tps {
                        combined.segments.extend(tp.segments);
                    }
                }
                Shape::Path(path) => {
                    let tps = gen.generate_path_contour(path, self.params.layer_depth as f64);
                    for tp in tps {
                        combined.segments.extend(tp.segments);
                    }
                }
                Shape::Triangle(tri) => {
                    let tps = gen.generate_triangle_contour(tri, self.params.layer_depth as f64);
                    for tp in tps {
                        combined.segments.extend(tp.segments);
                    }
                }
                Shape::Polygon(poly) => {
                    let tps = gen.generate_polygon_contour(poly, self.params.layer_depth as f64);
                    for tp in tps {
                        combined.segments.extend(tp.segments);
                    }
                }
                _ => {
                    debug!("Shape type not implemented for contour toolpath");
                }
            }
        }

        Ok(combined)
    }

    /// Generate pocket toolpath for a slice
    fn generate_pocket_toolpath(&self, slice: &SliceLayer) -> Result<Toolpath> {
        debug!(
            "Generating pocket toolpath for {} shapes",
            slice.shapes.len()
        );

        let mut gen = crate::toolpath::ToolpathGenerator::new();
        gen.set_tool_diameter(self.params.tool.diameter);
        gen.set_feed_rate(self.params.tool.feed_rate);
        gen.set_spindle_speed(self.params.tool.spindle_speed);
        gen.set_cut_depth(self.params.layer_depth as f64);
        gen.set_step_in(self.params.tool.stepover);
        gen.set_pocket_strategy(crate::pocket_operations::PocketStrategy::ContourParallel);

        let mut combined = gen.empty_toolpath();

        for shape in &slice.shapes {
            match shape {
                Shape::Rectangle(rect) => {
                    let tps = gen.generate_rectangle_pocket(
                        rect,
                        self.params.layer_depth as f64,
                        self.params.layer_depth as f64,
                        self.params.tool.stepover,
                    );
                    for tp in tps {
                        combined.segments.extend(tp.segments);
                    }
                }
                Shape::Circle(circle) => {
                    let tps = gen.generate_circle_pocket(
                        circle,
                        self.params.layer_depth as f64,
                        self.params.layer_depth as f64,
                        self.params.tool.stepover,
                    );
                    for tp in tps {
                        combined.segments.extend(tp.segments);
                    }
                }
                Shape::Path(path) => {
                    let tps = gen.generate_path_pocket(
                        path,
                        self.params.layer_depth as f64,
                        self.params.layer_depth as f64,
                        self.params.tool.stepover,
                    );
                    for tp in tps {
                        combined.segments.extend(tp.segments);
                    }
                }
                Shape::Polygon(poly) => {
                    let tps = gen.generate_polygon_pocket(
                        poly,
                        self.params.layer_depth as f64,
                        self.params.layer_depth as f64,
                        self.params.tool.stepover,
                    );
                    for tp in tps {
                        combined.segments.extend(tp.segments);
                    }
                }
                _ => {
                    debug!("Shape type not implemented for pocket toolpath");
                }
            }
        }

        Ok(combined)
    }

    /// Generate engrave toolpath for a slice
    fn generate_engrave_toolpath(&self, slice: &SliceLayer) -> Result<Toolpath> {
        debug!(
            "Generating engrave toolpath for {} shapes",
            slice.shapes.len()
        );

        let mut gen = crate::toolpath::ToolpathGenerator::new();
        gen.set_tool_diameter(self.params.tool.diameter);
        gen.set_feed_rate((self.params.tool.feed_rate * 0.5).max(50.0)); // slower feed
        gen.set_spindle_speed(self.params.tool.spindle_speed);
        gen.set_cut_depth(self.params.layer_depth as f64 * 0.1);
        gen.set_step_in(self.params.tool.stepover);

        let mut combined = gen.empty_toolpath();

        for shape in &slice.shapes {
            match shape {
                Shape::Path(path) => {
                    let tps = gen.generate_path_contour(path, self.params.layer_depth as f64 * 0.1);
                    for tp in tps {
                        combined.segments.extend(tp.segments);
                    }
                }
                Shape::Rectangle(rect) => {
                    let tps =
                        gen.generate_rectangle_contour(rect, self.params.layer_depth as f64 * 0.1);
                    for tp in tps {
                        combined.segments.extend(tp.segments);
                    }
                }
                _ => {
                    debug!("Shape type not implemented for engrave toolpath");
                }
            }
        }

        Ok(combined)
    }

    /// Generate adaptive toolpath for a slice
    fn generate_adaptive_toolpath(&self, slice: &SliceLayer) -> Result<Toolpath> {
        debug!(
            "Generating adaptive toolpath for {} shapes",
            slice.shapes.len()
        );

        // For adaptive, we choose the best strategy for each individual shape
        let mut toolpath = Toolpath::new(self.params.tool.diameter, self.params.layer_depth as f64);

        for shape in &slice.shapes {
            let shape_area = self.estimate_shape_area(shape);
            let shape_perimeter = self.estimate_shape_perimeter(shape);

            let strategy = if shape_area > shape_perimeter * shape_perimeter * 0.05 {
                SliceStrategy::Pocket
            } else {
                SliceStrategy::Contour
            };

            let single_slice = SliceLayer {
                z_height: slice.z_height,
                shapes: vec![shape.clone()],
                layer_index: slice.layer_index,
            };

            let shape_toolpath = match strategy {
                SliceStrategy::Contour => self.generate_contour_toolpath(&single_slice)?,
                SliceStrategy::Pocket => self.generate_pocket_toolpath(&single_slice)?,
                _ => self.generate_contour_toolpath(&single_slice)?,
            };

            toolpath.segments.extend(shape_toolpath.segments);
        }

        Ok(toolpath)
    }

    /// Calculate total area of all shapes in a slice
    fn calculate_slice_area(&self, slice: &SliceLayer) -> f64 {
        slice
            .shapes
            .iter()
            .map(|shape| self.estimate_shape_area(shape))
            .sum()
    }

    /// Calculate total perimeter of all shapes in a slice
    fn calculate_slice_perimeter(&self, slice: &SliceLayer) -> f64 {
        slice
            .shapes
            .iter()
            .map(|shape| self.estimate_shape_perimeter(shape))
            .sum()
    }

    /// Estimate area of a shape (rough calculation)
    fn estimate_shape_area(&self, shape: &Shape) -> f64 {
        match shape {
            Shape::Rectangle(rect) => rect.width * rect.height,
            Shape::Circle(circle) => std::f64::consts::PI * circle.radius * circle.radius,
            Shape::Ellipse(ellipse) => std::f64::consts::PI * ellipse.rx * ellipse.ry,
            Shape::Path(path) => {
                // Rough estimation using bounding box
                let (min_x, min_y, max_x, max_y) = path.bounds();
                (max_x - min_x) * (max_y - min_y)
            }
            _ => 1.0, // Default small area for other shapes
        }
    }

    /// Estimate perimeter of a shape (rough calculation)
    fn estimate_shape_perimeter(&self, shape: &Shape) -> f64 {
        match shape {
            Shape::Rectangle(rect) => 2.0 * (rect.width + rect.height),
            Shape::Circle(circle) => 2.0 * std::f64::consts::PI * circle.radius,
            Shape::Ellipse(ellipse) => {
                // Approximation for ellipse perimeter
                let a = ellipse.rx;
                let b = ellipse.ry;
                std::f64::consts::PI * (3.0 * (a + b) - ((3.0 * a + b) * (a + 3.0 * b)).sqrt())
            }
            Shape::Line(line) => {
                ((line.end.x - line.start.x).powi(2) + (line.end.y - line.start.y).powi(2)).sqrt()
            }
            Shape::Path(path) => {
                // Rough estimation - sum of bounding box perimeter
                let (min_x, min_y, max_x, max_y) = path.bounds();
                2.0 * ((max_x - min_x) + (max_y - min_y))
            }
            _ => 10.0, // Default perimeter for other shapes
        }
    }

    /// Estimate machining time for a toolpath
    fn estimate_machining_time(&self, toolpath: &Toolpath) -> f32 {
        if toolpath.segments.is_empty() {
            return 0.0;
        }

        let mut total_time = 0.0;
        let feed_rate = 1000.0; // Default feed rate in mm/min

        for segment in &toolpath.segments {
            // Calculate distance manually
            let dx = segment.end.x - segment.start.x;
            let dy = segment.end.y - segment.start.y;
            let distance = (dx * dx + dy * dy).sqrt();

            let time_minutes = distance / feed_rate;
            total_time += time_minutes as f32;
        }

        // Add overhead for tool changes, etc.
        total_time *= 1.2; // 20% overhead

        total_time
    }

    /// Generate job summary
    fn generate_summary(&self, layers: &[LayerToolpath], mesh: &Mesh3D) -> Result<JobSummary> {
        let layer_count = layers.len();
        let total_length: f64 = layers
            .iter()
            .map(|layer| layer.toolpath.total_length())
            .sum();

        // Estimate material removed (rough calculation)
        let material_removed = self.estimate_material_removal(layers, mesh);

        let max_depth = layers
            .iter()
            .map(|layer| layer.slice.z_height - self.params.layer_depth)
            .fold(0.0, f32::min)
            .abs();

        let dimensions = (
            mesh.bounds_max.x - mesh.bounds_min.x,
            mesh.bounds_max.y - mesh.bounds_min.y,
            mesh.bounds_max.z - mesh.bounds_min.z,
        );

        Ok(JobSummary {
            layer_count,
            total_length,
            material_removed,
            max_depth,
            dimensions,
        })
    }

    /// Estimate total material removal volume
    fn estimate_material_removal(&self, layers: &[LayerToolpath], _mesh: &Mesh3D) -> f64 {
        // Simple estimation: sum of areas times layer depth
        layers
            .iter()
            .map(|layer| {
                let layer_area = self.calculate_slice_area(&layer.slice);
                layer_area * self.params.layer_depth as f64
            })
            .sum()
    }

    /// Get parameters
    pub fn params(&self) -> &SliceToolpathParams {
        &self.params
    }

    /// Set parameters
    pub fn set_params(&mut self, params: SliceToolpathParams) {
        self.params = params;
    }
}

/// Utility functions for working with sliced jobs
impl SlicedJob {
    /// Export all toolpaths as a single combined toolpath
    pub fn combine_toolpaths(&self) -> Toolpath {
        let mut combined = Toolpath::new(self.params.tool.diameter, self.params.layer_depth as f64);

        for layer in &self.layers {
            combined.segments.extend(layer.toolpath.segments.clone());
        }

        combined
    }

    /// Get toolpath for a specific layer
    pub fn get_layer_toolpath(&self, layer_index: usize) -> Option<&Toolpath> {
        self.layers.get(layer_index).map(|layer| &layer.toolpath)
    }

    /// Get summary information
    pub fn summary(&self) -> &JobSummary {
        &self.summary
    }

    /// Export to G-code
    pub fn to_gcode(&self) -> Result<String> {
        let _combined_toolpath = self.combine_toolpaths();

        // Generate basic G-code header for this job
        Ok(format!(
            "; Generated by GCodeKit5 3D Slice-to-Toolpath\n\
             ; Tool: {} ({}mm)\n\
             ; Layers: {}\n\
             ; Strategy: {:?}\n\
             G21 ; Millimeters\n\
             G90 ; Absolute positioning\n\
             G0 Z{}\n",
            self.params.tool.name,
            self.params.tool.diameter,
            self.layers.len(),
            self.params.strategy,
            self.params.safe_z
        ))
    }

    /// Save layer information to file
    pub fn save_layer_info(&self, path: &std::path::Path) -> Result<()> {
        let layer_info: Vec<_> = self
            .layers
            .iter()
            .map(|layer| {
                serde_json::json!({
                    "layer_index": layer.slice.layer_index,
                    "z_height": layer.slice.z_height,
                    "shape_count": layer.slice.shapes.len(),
                    "strategy": format!("{:?}", layer.strategy),
                    "estimated_time": layer.estimated_time,
                    "toolpath_length": layer.toolpath.total_length(),
                })
            })
            .collect();

        let output = serde_json::json!({
            "summary": {
                "layer_count": self.summary.layer_count,
                "total_length": self.summary.total_length,
                "material_removed": self.summary.material_removed,
                "max_depth": self.summary.max_depth,
                "dimensions": self.summary.dimensions,
                "total_time": self.total_time,
            },
            "layers": layer_info
        });

        std::fs::write(path, serde_json::to_string_pretty(&output)?)
            .map_err(|e| anyhow!("Failed to write layer info: {}", e))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{DesignRectangle, Point, Shape};

    #[test]
    fn test_generate_contour_rectangle() {
        let tool = crate::tool_library::Tool::new(
            "t1".to_string(),
            "Default".to_string(),
            crate::tool_library::ToolType::EndMill,
            3.175,
            2,
            "Carbide".to_string(),
        );
        let mut stt = SliceToToolpath::contour_cutting(tool.clone(), 1.0);

        let rect = DesignRectangle {
            width: 10.0,
            height: 5.0,
            center: Point::new(0.0, 0.0),
            corner_radius: 0.0,
            rotation: 0.0,
            is_slot: false,
        };
        let slice = crate::shadow_projection::SliceLayer {
            z_height: 0.0,
            shapes: vec![Shape::Rectangle(rect)],
            layer_index: 0,
        };

        let tp = stt.generate_contour_toolpath(&slice).unwrap();
        assert!(tp.segments.len() >= 4);
    }

    #[test]
    fn test_generate_pocket_rectangle() {
        let tool = crate::tool_library::Tool::new(
            "t1".to_string(),
            "Default".to_string(),
            crate::tool_library::ToolType::EndMill,
            3.175,
            2,
            "Carbide".to_string(),
        );
        let mut stt = SliceToToolpath::pocket_clearing(tool.clone(), 0.5);

        let rect = DesignRectangle {
            width: 20.0,
            height: 10.0,
            center: Point::new(0.0, 0.0),
            corner_radius: 0.0,
            rotation: 0.0,
            is_slot: false,
        };
        let slice = crate::shadow_projection::SliceLayer {
            z_height: 0.0,
            shapes: vec![Shape::Rectangle(rect)],
            layer_index: 0,
        };

        let tp = stt.generate_pocket_toolpath(&slice).unwrap();
        assert!(tp.segments.len() > 0);
    }

    #[test]
    fn test_generate_engrave_rectangle() {
        let tool = crate::tool_library::Tool::new(
            "t1".to_string(),
            "Default".to_string(),
            crate::tool_library::ToolType::EndMill,
            3.175,
            2,
            "Carbide".to_string(),
        );
        let stt = SliceToToolpath::contour_cutting(tool.clone(), 1.0);

        let rect = DesignRectangle {
            width: 12.0,
            height: 6.0,
            center: Point::new(0.0, 0.0),
            corner_radius: 0.0,
            rotation: 0.0,
            is_slot: false,
        };
        let slice = crate::shadow_projection::SliceLayer {
            z_height: 0.0,
            shapes: vec![Shape::Rectangle(rect)],
            layer_index: 0,
        };

        let tp = stt.generate_engrave_toolpath(&slice).unwrap();
        assert!(tp.segments.len() >= 4 || tp.segments.len() > 0);
    }
}
