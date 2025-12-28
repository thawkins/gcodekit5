//! # 3D Shadow Projection Module
//!
//! Provides advanced shadow projection capabilities for converting 3D models to 2D toolpaths.
//! This module implements the core "Shadow" feature similar to E-CAM's 3D shadow projection.
//!
//! ## Features
//! - Orthographic and perspective projection
//! - Custom projection directions and orientations
//! - Multi-layer slicing for 2.5D machining
//! - Edge detection and outline simplification
//! - Batch processing multiple Z-heights

use crate::model::{Point, Shape};
use crate::model3d::Mesh3D;
use anyhow::{anyhow, Result};
use nalgebra::{Matrix4, Point3, Vector3};
use tracing::{debug, info};

/// Projection method for shadow generation
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ProjectionMethod {
    /// Orthographic projection (parallel rays)
    Orthographic,
    /// Perspective projection (rays from a point)
    Perspective { distance: f32 },
}

/// Parameters for shadow projection
#[derive(Debug, Clone)]
pub struct ShadowProjectionParams {
    /// Projection method (orthographic or perspective)
    pub method: ProjectionMethod,
    /// Direction vector for projection
    pub direction: Vector3<f32>,
    /// Up vector for orientation
    pub up: Vector3<f32>,
    /// Whether to generate filled shapes or just outlines
    pub filled: bool,
    /// Tolerance for path simplification
    pub tolerance: f64,
    /// Whether to merge overlapping contours
    pub merge_contours: bool,
}

impl Default for ShadowProjectionParams {
    fn default() -> Self {
        Self {
            method: ProjectionMethod::Orthographic,
            direction: Vector3::new(0.0, 0.0, -1.0), // Project down along Z-axis
            up: Vector3::new(0.0, 1.0, 0.0),         // Y-axis is up
            filled: false,
            tolerance: 0.01,
            merge_contours: true,
        }
    }
}

/// Parameters for multi-layer slicing
#[derive(Debug, Clone)]
pub struct SlicingParams {
    /// Starting Z height
    pub z_start: f32,
    /// Ending Z height
    pub z_end: f32,
    /// Layer height (step size)
    pub layer_height: f32,
    /// Whether to generate adaptive layers
    pub adaptive: bool,
    /// Minimum feature size for adaptive slicing
    pub min_feature_size: f32,
}

impl Default for SlicingParams {
    fn default() -> Self {
        Self {
            z_start: 0.0,
            z_end: 10.0,
            layer_height: 0.2,
            adaptive: false,
            min_feature_size: 0.1,
        }
    }
}

/// Result of a slicing operation
#[derive(Debug, Clone)]
pub struct SliceLayer {
    /// Z height of this layer
    pub z_height: f32,
    /// 2D shapes for this layer
    pub shapes: Vec<Shape>,
    /// Layer index
    pub layer_index: usize,
}

/// 3D Shadow Projection Engine
pub struct ShadowProjector {
    params: ShadowProjectionParams,
}

impl ShadowProjector {
    pub fn new(params: ShadowProjectionParams) -> Self {
        Self { params }
    }
    
    /// Create a projector with default orthographic projection along Z-axis
    pub fn orthographic_z() -> Self {
        Self::new(ShadowProjectionParams::default())
    }
    
    /// Create a projector with perspective projection
    pub fn perspective(direction: Vector3<f32>, distance: f32) -> Self {
        let params = ShadowProjectionParams {
            method: ProjectionMethod::Perspective { distance },
            direction: direction.normalize(),
            ..Default::default()
        };
        Self::new(params)
    }
    
    /// Create a projector with custom orthographic direction
    pub fn orthographic_custom(direction: Vector3<f32>, up: Vector3<f32>) -> Self {
        let params = ShadowProjectionParams {
            direction: direction.normalize(),
            up: up.normalize(),
            ..Default::default()
        };
        Self::new(params)
    }
    
    /// Project mesh to create 2D shadow
    pub fn project_shadow(&self, mesh: &Mesh3D) -> Result<Vec<Shape>> {
        info!(
            "Projecting 3D mesh with {} triangles using {:?}",
            mesh.triangles.len(),
            self.params.method
        );
        
        match self.params.method {
            ProjectionMethod::Orthographic => self.project_orthographic(mesh),
            ProjectionMethod::Perspective { distance } => self.project_perspective(mesh, distance),
        }
    }
    
    /// Orthographic projection (parallel rays)
    fn project_orthographic(&self, mesh: &Mesh3D) -> Result<Vec<Shape>> {
        debug!("Performing orthographic projection");
        
        // For orthographic projection, we simply project all points onto a plane
        // perpendicular to the projection direction
        
        // Create transformation matrix that aligns projection direction with Z-axis
        let transform = self.create_view_transform();
        
        // Apply transformation to mesh
        let mut transformed_mesh = mesh.clone();
        transformed_mesh.transform(&transform);
        
        // Now project to XY plane (Z = 0)
        transformed_mesh.project_shadow_z()
    }
    
    /// Perspective projection (rays from a point)
    fn project_perspective(&self, mesh: &Mesh3D, distance: f32) -> Result<Vec<Shape>> {
        debug!("Performing perspective projection with distance {}", distance);
        
        // For perspective projection, we need to scale points based on their distance
        // from the projection point
        
        let projection_point = Point3::from(-self.params.direction * distance);
        let mut projected_segments = Vec::new();
        
        for triangle in &mesh.triangles {
            // Project each triangle edge
            for i in 0..3 {
                let v1 = triangle.vertices[i];
                let v2 = triangle.vertices[(i + 1) % 3];
                
                // Project vertices to the projection plane
                let proj_v1 = self.project_point_perspective(v1, projection_point)?;
                let proj_v2 = self.project_point_perspective(v2, projection_point)?;
                
                projected_segments.push((proj_v1, proj_v2));
            }
        }
        
        // Build contours from projected segments
        self.build_contours_from_segments(&projected_segments)
    }
    
    /// Project a single point using perspective projection
    fn project_point_perspective(&self, point: Point3<f32>, projection_point: Point3<f32>) -> Result<Point> {
        // Ray from projection point through the 3D point
        let ray_dir = (point - projection_point).normalize();
        
        // Find intersection with projection plane (Z = 0 in transformed space)
        let t = -projection_point.z / ray_dir.z;
        if t < 0.0 {
            return Err(anyhow!("Point behind projection plane"));
        }
        
        let intersection = projection_point + ray_dir * t;
        Ok(Point::new(intersection.x as f64, intersection.y as f64))
    }
    
    /// Create view transformation matrix
    fn create_view_transform(&self) -> Matrix4<f32> {
        let forward = -self.params.direction.normalize();
        let right = self.params.up.cross(&forward).normalize();
        let up = forward.cross(&right);
        
        Matrix4::look_at_rh(
            &Point3::new(0.0, 0.0, 0.0),
            &Point3::from(forward),
            &up,
        )
    }
    
    /// Build contour paths from line segments
    fn build_contours_from_segments(&self, segments: &[(Point, Point)]) -> Result<Vec<Shape>> {
        crate::model3d::build_contours_from_segments(segments)
            .map(|contours| contours.into_iter().map(Shape::Path).collect())
    }
    
    /// Slice mesh into multiple layers
    pub fn slice_mesh(&self, mesh: &Mesh3D, params: &SlicingParams) -> Result<Vec<SliceLayer>> {
        info!(
            "Slicing mesh from Z={} to Z={} with layer height {}",
            params.z_start, params.z_end, params.layer_height
        );
        
        let mut layers = Vec::new();
        let mut z = params.z_start;
        let mut layer_index = 0;
        
        while z <= params.z_end {
            debug!("Processing layer {} at Z={}", layer_index, z);
            
            let shapes = mesh.slice_at_z(z)?;
            
            layers.push(SliceLayer {
                z_height: z,
                shapes,
                layer_index,
            });
            
            // Calculate next Z height
            if params.adaptive {
                z += self.calculate_adaptive_layer_height(mesh, z, params);
            } else {
                z += params.layer_height;
            }
            
            layer_index += 1;
        }
        
        info!("Generated {} layers", layers.len());
        Ok(layers)
    }
    
    /// Calculate adaptive layer height based on mesh geometry
    fn calculate_adaptive_layer_height(&self, mesh: &Mesh3D, current_z: f32, params: &SlicingParams) -> f32 {
        // Simple adaptive algorithm: use smaller layers where there's more geometry detail
        let intersecting_triangles = mesh.get_intersecting_triangles(current_z);
        
        if intersecting_triangles.len() > 10 {
            // More triangles = more detail = smaller layer height
            params.layer_height * 0.5
        } else if intersecting_triangles.len() < 3 {
            // Fewer triangles = less detail = larger layer height
            params.layer_height * 2.0
        } else {
            params.layer_height
        }
    }
    
    /// Generate multiple shadow projections from different angles
    pub fn multi_angle_projection(&self, mesh: &Mesh3D, angles: &[f32]) -> Result<Vec<Vec<Shape>>> {
        let mut projections = Vec::new();
        
        for &angle in angles {
            info!("Generating projection at angle {} degrees", angle.to_degrees());
            
            // Rotate projection direction around Y-axis
            let rotation = Matrix4::from_axis_angle(&Vector3::y_axis(), angle);
            let rotated_direction = rotation.transform_vector(&self.params.direction);
            
            // Create projector with rotated direction
            let mut rotated_params = self.params.clone();
            rotated_params.direction = rotated_direction;
            let projector = ShadowProjector::new(rotated_params);
            
            let shapes = projector.project_shadow(mesh)?;
            projections.push(shapes);
        }
        
        Ok(projections)
    }
    
    /// Get projection parameters
    pub fn params(&self) -> &ShadowProjectionParams {
        &self.params
    }
    
    /// Set projection parameters
    pub fn set_params(&mut self, params: ShadowProjectionParams) {
        self.params = params;
    }
}

/// Utility functions for shadow projection
impl ShadowProjector {
    /// Create a "front view" projector (looking along negative Y axis)
    pub fn front_view() -> Self {
        Self::orthographic_custom(
            Vector3::new(0.0, -1.0, 0.0), // Look along -Y
            Vector3::new(0.0, 0.0, 1.0),  // Z is up
        )
    }
    
    /// Create a "side view" projector (looking along negative X axis)
    pub fn side_view() -> Self {
        Self::orthographic_custom(
            Vector3::new(-1.0, 0.0, 0.0), // Look along -X
            Vector3::new(0.0, 0.0, 1.0),  // Z is up
        )
    }
    
    /// Create a "top view" projector (looking along negative Z axis)
    pub fn top_view() -> Self {
        Self::orthographic_z()
    }
    
    /// Create an isometric view projector
    pub fn isometric_view() -> Self {
        Self::orthographic_custom(
            Vector3::new(-1.0, -1.0, -1.0), // Isometric direction
            Vector3::new(0.0, 1.0, 0.0),    // Y is up
        )
    }
}

/// Batch processing utilities
pub struct BatchProjector {
    base_projector: ShadowProjector,
}

impl BatchProjector {
    pub fn new(projector: ShadowProjector) -> Self {
        Self {
            base_projector: projector,
        }
    }
    
    /// Process multiple meshes with the same projection settings
    pub fn process_meshes(&self, meshes: &[Mesh3D]) -> Result<Vec<Vec<Shape>>> {
        let mut results = Vec::new();
        
        for (i, mesh) in meshes.iter().enumerate() {
            info!("Processing mesh {} of {}", i + 1, meshes.len());
            let shapes = self.base_projector.project_shadow(mesh)?;
            results.push(shapes);
        }
        
        Ok(results)
    }
    
    /// Generate standard engineering views (front, side, top) for a mesh
    pub fn standard_views(&self, mesh: &Mesh3D) -> Result<Vec<(String, Vec<Shape>)>> {
        let views = vec![
            ("Top", ShadowProjector::top_view()),
            ("Front", ShadowProjector::front_view()),
            ("Side", ShadowProjector::side_view()),
            ("Isometric", ShadowProjector::isometric_view()),
        ];
        
        let mut results = Vec::new();
        
        for (name, projector) in views {
            info!("Generating {} view", name);
            let shapes = projector.project_shadow(mesh)?;
            results.push((name.to_string(), shapes));
        }
        
        Ok(results)
    }
}