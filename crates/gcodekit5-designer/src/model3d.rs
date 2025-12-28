//! # 3D Model Module
//!
//! Provides data structures and functionality for importing and working with 3D models.
//! Supports STL mesh format and shadow projection to 2D shapes.
//!
//! ## Supported Formats
//! - STL (STereoLithography) - Triangle mesh format
//!
//! ## Features
//! - 3D model import from STL files
//! - Shadow projection to generate 2D silhouettes
//! - Model slicing for 2.5D machining
//! - Coordinate transformation and scaling

use crate::model::Point;
use crate::model::{DesignPath as PathShape, Shape};
use anyhow::{anyhow, Result};
use nalgebra::{Matrix4, Point3, Vector3};
use std::collections::HashSet;
use tracing::debug;

/// A 3D triangle made up of three vertices
#[derive(Debug, Clone, PartialEq)]
pub struct Triangle3D {
    pub vertices: [Point3<f32>; 3],
    pub normal: Vector3<f32>,
}

impl Triangle3D {
    pub fn new(v1: Point3<f32>, v2: Point3<f32>, v3: Point3<f32>) -> Self {
        // Calculate normal using cross product
        let edge1 = v2 - v1;
        let edge2 = v3 - v1;
        let normal = edge1.cross(&edge2).normalize();
        
        Self {
            vertices: [v1, v2, v3],
            normal,
        }
    }
    
    /// Get bounding box of the triangle
    pub fn bounds(&self) -> (Point3<f32>, Point3<f32>) {
        let mut min_x = self.vertices[0].x;
        let mut max_x = self.vertices[0].x;
        let mut min_y = self.vertices[0].y;
        let mut max_y = self.vertices[0].y;
        let mut min_z = self.vertices[0].z;
        let mut max_z = self.vertices[0].z;
        
        for vertex in &self.vertices[1..] {
            min_x = min_x.min(vertex.x);
            max_x = max_x.max(vertex.x);
            min_y = min_y.min(vertex.y);
            max_y = max_y.max(vertex.y);
            min_z = min_z.min(vertex.z);
            max_z = max_z.max(vertex.z);
        }
        
        (Point3::new(min_x, min_y, min_z), Point3::new(max_x, max_y, max_z))
    }
    
    /// Check if triangle intersects with a horizontal plane at given Z height
    pub fn intersects_plane_z(&self, z: f32) -> bool {
        let z_min = self.vertices[0].z.min(self.vertices[1].z).min(self.vertices[2].z);
        let z_max = self.vertices[0].z.max(self.vertices[1].z).max(self.vertices[2].z);
        z >= z_min && z <= z_max
    }
    
    /// Get intersection points with horizontal plane at Z height
    pub fn intersect_plane_z(&self, z: f32) -> Vec<Point3<f32>> {
        if !self.intersects_plane_z(z) {
            return vec![];
        }
        
        let mut intersections = Vec::new();
        
        // Check each edge for intersection with Z plane
        for i in 0..3 {
            let v1 = self.vertices[i];
            let v2 = self.vertices[(i + 1) % 3];
            
            if let Some(intersection) = intersect_edge_with_plane_z(v1, v2, z) {
                intersections.push(intersection);
            }
        }
        
        // Remove duplicates (vertices that are exactly on the plane)
        intersections.dedup_by(|a, b| {
            (a.x - b.x).abs() < 1e-6 && (a.y - b.y).abs() < 1e-6
        });
        
        intersections
    }
}

/// Intersect an edge with a horizontal plane at Z height
fn intersect_edge_with_plane_z(v1: Point3<f32>, v2: Point3<f32>, z: f32) -> Option<Point3<f32>> {
    // If both vertices are on the same side of the plane, no intersection
    if (v1.z - z) * (v2.z - z) > 0.0 {
        return None;
    }
    
    // If one vertex is exactly on the plane, return it
    if (v1.z - z).abs() < 1e-6 {
        return Some(v1);
    }
    if (v2.z - z).abs() < 1e-6 {
        return Some(v2);
    }
    
    // Calculate intersection point using linear interpolation
    let t = (z - v1.z) / (v2.z - v1.z);
    let x = v1.x + t * (v2.x - v1.x);
    let y = v1.y + t * (v2.y - v1.y);
    
    Some(Point3::new(x, y, z))
}

/// A 3D mesh model
#[derive(Debug, Clone)]
pub struct Mesh3D {
    pub triangles: Vec<Triangle3D>,
    pub bounds_min: Point3<f32>,
    pub bounds_max: Point3<f32>,
}

impl Mesh3D {
    pub fn new(triangles: Vec<Triangle3D>) -> Self {
        let mut mesh = Self {
            triangles,
            bounds_min: Point3::new(0.0, 0.0, 0.0),
            bounds_max: Point3::new(0.0, 0.0, 0.0),
        };
        mesh.calculate_bounds();
        mesh
    }
    
    pub fn from_stl_mesh(stl_mesh: &stl_io::IndexedMesh) -> Self {
        let mut triangles = Vec::new();
        
        // Convert indexed mesh to triangles
        for face in &stl_mesh.faces {
            let v1_idx = face.vertices[0] as usize;
            let v2_idx = face.vertices[1] as usize;
            let v3_idx = face.vertices[2] as usize;
            
            if v1_idx < stl_mesh.vertices.len() 
                && v2_idx < stl_mesh.vertices.len() 
                && v3_idx < stl_mesh.vertices.len() {
                
                let v1 = stl_mesh.vertices[v1_idx];
                let v2 = stl_mesh.vertices[v2_idx];
                let v3 = stl_mesh.vertices[v3_idx];
                
                triangles.push(Triangle3D::new(
                    Point3::new(v1[0], v1[1], v1[2]),
                    Point3::new(v2[0], v2[1], v2[2]),
                    Point3::new(v3[0], v3[1], v3[2]),
                ));
            }
        }
        
        Self::new(triangles)
    }

    pub fn from_stl_triangles(stl_triangles: &[stl_io::Triangle]) -> Self {
        let triangles: Vec<Triangle3D> = stl_triangles
            .iter()
            .map(|tri| {
                Triangle3D::new(
                    Point3::new(tri.vertices[0][0], tri.vertices[0][1], tri.vertices[0][2]),
                    Point3::new(tri.vertices[1][0], tri.vertices[1][1], tri.vertices[1][2]),
                    Point3::new(tri.vertices[2][0], tri.vertices[2][1], tri.vertices[2][2]),
                )
            })
            .collect();
            
        Self::new(triangles)
    }
    
    fn calculate_bounds(&mut self) {
        if self.triangles.is_empty() {
            return;
        }
        
        let first_bounds = self.triangles[0].bounds();
        let mut min = first_bounds.0;
        let mut max = first_bounds.1;
        
        for triangle in &self.triangles[1..] {
            let (tri_min, tri_max) = triangle.bounds();
            min.x = min.x.min(tri_min.x);
            min.y = min.y.min(tri_min.y);
            min.z = min.z.min(tri_min.z);
            max.x = max.x.max(tri_max.x);
            max.y = max.y.max(tri_max.y);
            max.z = max.z.max(tri_max.z);
        }
        
        self.bounds_min = min;
        self.bounds_max = max;
    }
    
    /// Transform the mesh using a 4x4 transformation matrix
    pub fn transform(&mut self, transform: &Matrix4<f32>) {
        for triangle in &mut self.triangles {
            for vertex in &mut triangle.vertices {
                let transformed = transform.transform_point(vertex);
                *vertex = transformed;
            }
            
            // Recalculate normal after transformation
            let edge1 = triangle.vertices[1] - triangle.vertices[0];
            let edge2 = triangle.vertices[2] - triangle.vertices[0];
            triangle.normal = edge1.cross(&edge2).normalize();
        }
        
        self.calculate_bounds();
    }
    
    /// Scale the mesh uniformly
    pub fn scale(&mut self, factor: f32) {
        let transform = Matrix4::new_scaling(factor);
        self.transform(&transform);
    }
    
    /// Translate the mesh
    pub fn translate(&mut self, offset: Vector3<f32>) {
        let transform = Matrix4::new_translation(&offset);
        self.transform(&transform);
    }
    
    /// Center the mesh at the origin
    pub fn center(&mut self) {
        let center = (self.bounds_min + self.bounds_max.coords) * 0.5;
        self.translate(-center.coords);
    }
    
    /// Get all triangles that intersect with a horizontal plane at Z height
    pub fn get_intersecting_triangles(&self, z: f32) -> Vec<&Triangle3D> {
        self.triangles
            .iter()
            .filter(|tri| tri.intersects_plane_z(z))
            .collect()
    }
    
    /// Project the mesh to a 2D silhouette by projecting along Z-axis
    /// Returns a collection of 2D paths representing the outline
    pub fn project_shadow_z(&self) -> Result<Vec<Shape>> {
        debug!("Projecting 3D mesh to 2D shadow along Z-axis");
        
        // Collect all edge segments from all triangles projected to XY plane
        let mut edge_segments = Vec::new();
        
        for triangle in &self.triangles {
            // Project each triangle to XY plane and add its edges
            let v1_2d = Point::new(triangle.vertices[0].x as f64, triangle.vertices[0].y as f64);
            let v2_2d = Point::new(triangle.vertices[1].x as f64, triangle.vertices[1].y as f64);
            let v3_2d = Point::new(triangle.vertices[2].x as f64, triangle.vertices[2].y as f64);
            
            // Add triangle edges as line segments
            edge_segments.push((v1_2d, v2_2d));
            edge_segments.push((v2_2d, v3_2d));
            edge_segments.push((v3_2d, v1_2d));
        }
        
        debug!("Collected {} edge segments", edge_segments.len());
        
        // Build contour paths from edge segments
        let contours = build_contours_from_segments(&edge_segments)?;
        
        debug!("Built {} contour paths", contours.len());
        
        let shapes: Vec<Shape> = contours
            .into_iter()
            .map(|path| Shape::Path(path))
            .collect();
            
        Ok(shapes)
    }
    
    /// Slice the mesh at a specific Z height and return 2D contour paths
    pub fn slice_at_z(&self, z: f32) -> Result<Vec<Shape>> {
        debug!("Slicing mesh at Z = {}", z);
        
        let mut intersection_segments = Vec::new();
        
        for triangle in &self.triangles {
            let intersections = triangle.intersect_plane_z(z);
            if intersections.len() == 2 {
                // Two intersection points form a line segment
                let p1 = Point::new(intersections[0].x as f64, intersections[0].y as f64);
                let p2 = Point::new(intersections[1].x as f64, intersections[1].y as f64);
                intersection_segments.push((p1, p2));
            }
        }
        
        debug!("Found {} intersection segments at Z = {}", intersection_segments.len(), z);
        
        // Build contour paths from intersection segments
        let contours = build_contours_from_segments(&intersection_segments)?;
        
        debug!("Built {} contour paths from slice", contours.len());
        
        let shapes: Vec<Shape> = contours
            .into_iter()
            .map(|path| Shape::Path(path))
            .collect();
            
        Ok(shapes)
    }
}

/// Build contour paths from a collection of line segments
/// This function attempts to connect segments into closed loops
pub fn build_contours_from_segments(segments: &[(Point, Point)]) -> Result<Vec<PathShape>> {
    if segments.is_empty() {
        return Ok(vec![]);
    }
    
    let mut used_segments = HashSet::new();
    let mut contours = Vec::new();
    
    for (i, &(start, end)) in segments.iter().enumerate() {
        if used_segments.contains(&i) {
            continue;
        }
        
        // Start a new contour
        let mut contour_points = vec![start];
        let mut current_end = end;
        used_segments.insert(i);
        
        // Try to extend the contour by finding connecting segments
        let mut found_connection = true;
        while found_connection {
            found_connection = false;
            
            for (j, &(seg_start, seg_end)) in segments.iter().enumerate() {
                if used_segments.contains(&j) {
                    continue;
                }
                
                let tolerance = 1e-6;
                
                // Check if this segment connects to the current end
                if (current_end.x - seg_start.x).abs() < tolerance && (current_end.y - seg_start.y).abs() < tolerance {
                    contour_points.push(seg_end);
                    current_end = seg_end;
                    used_segments.insert(j);
                    found_connection = true;
                    break;
                } else if (current_end.x - seg_end.x).abs() < tolerance && (current_end.y - seg_end.y).abs() < tolerance {
                    contour_points.push(seg_start);
                    current_end = seg_start;
                    used_segments.insert(j);
                    found_connection = true;
                    break;
                }
            }
        }
        
        // Check if the contour closes back to the start
        let tolerance = 1e-6;
        let is_closed = contour_points.len() > 2 
            && (current_end.x - start.x).abs() < tolerance 
            && (current_end.y - start.y).abs() < tolerance;
        
        // Only add contours with multiple points
        if contour_points.len() > 1 {
            let path = PathShape::from_points(&contour_points, is_closed);
            contours.push(path);
        }
    }
    
    Ok(contours)
}

/// Supported 3D file formats
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Model3DFormat {
    /// STL (STereoLithography) format
    Stl,
}

/// 3D model importer for converting 3D files to mesh representations
pub struct Model3DImporter {
    pub scale: f32,
    pub center_model: bool,
}

impl Model3DImporter {
    pub fn new() -> Self {
        Self {
            scale: 1.0,
            center_model: true,
        }
    }
    
    pub fn with_scale(mut self, scale: f32) -> Self {
        self.scale = scale;
        self
    }
    
    pub fn with_centering(mut self, center: bool) -> Self {
        self.center_model = center;
        self
    }
    
    /// Import 3D model from file path
    pub fn import_file(&self, path: &str) -> Result<Mesh3D> {
        let format = self.detect_format(path)?;
        
        match format {
            Model3DFormat::Stl => self.import_stl_file(path),
        }
    }
    
    /// Import STL from file path
    pub fn import_stl_file(&self, path: &str) -> Result<Mesh3D> {
        debug!("Importing STL file: {}", path);
        
        let mut file = std::fs::File::open(path)
            .map_err(|e| anyhow!("Failed to open STL file {}: {}", path, e))?;
            
        let stl = stl_io::read_stl(&mut file)
            .map_err(|e| anyhow!("Failed to parse STL file {}: {}", path, e))?;
            
        debug!("STL contains {} faces", stl.faces.len());
        
        let mut mesh = Mesh3D::from_stl_mesh(&stl);
        
        // Apply transformations
        if self.scale != 1.0 {
            debug!("Scaling mesh by factor {}", self.scale);
            mesh.scale(self.scale);
        }
        
        if self.center_model {
            debug!("Centering mesh at origin");
            mesh.center();
        }
        
        debug!("Final mesh bounds: {:?} to {:?}", mesh.bounds_min, mesh.bounds_max);
        
        Ok(mesh)
    }
    
    /// Import STL from string content (binary STL data)
    pub fn import_stl_data(&self, data: &[u8]) -> Result<Mesh3D> {
        debug!("Importing STL from binary data ({} bytes)", data.len());
        
        let mut cursor = std::io::Cursor::new(data);
        let stl = stl_io::read_stl(&mut cursor)
            .map_err(|e| anyhow!("Failed to parse STL data: {}", e))?;
            
        debug!("STL contains {} faces", stl.faces.len());
        
        let mut mesh = Mesh3D::from_stl_mesh(&stl);
        
        // Apply transformations
        if self.scale != 1.0 {
            debug!("Scaling mesh by factor {}", self.scale);
            mesh.scale(self.scale);
        }
        
        if self.center_model {
            debug!("Centering mesh at origin");
            mesh.center();
        }
        
        debug!("Final mesh bounds: {:?} to {:?}", mesh.bounds_min, mesh.bounds_max);
        
        Ok(mesh)
    }
    
    /// Detect file format from file extension
    fn detect_format(&self, path: &str) -> Result<Model3DFormat> {
        let extension = std::path::Path::new(path)
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|s| s.to_lowercase())
            .ok_or_else(|| anyhow!("Unable to determine file extension for: {}", path))?;
            
        match extension.as_str() {
            "stl" => Ok(Model3DFormat::Stl),
            _ => Err(anyhow!("Unsupported 3D file format: .{}", extension)),
        }
    }
}

impl Default for Model3DImporter {
    fn default() -> Self {
        Self::new()
    }
}

/// Projection parameters for shadow generation
#[derive(Debug, Clone)]
pub struct ProjectionParams {
    /// Direction vector for projection (default: along negative Z-axis)
    pub direction: Vector3<f32>,
    /// Whether to generate filled silhouette or just outline
    pub filled: bool,
    /// Simplification tolerance for generated paths
    pub tolerance: f64,
}

impl Default for ProjectionParams {
    fn default() -> Self {
        Self {
            direction: Vector3::new(0.0, 0.0, -1.0), // Project along Z-axis
            filled: false,
            tolerance: 0.01,
        }
    }
}