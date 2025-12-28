//! # 3D Mesh Rendering Module
//!
//! Renders 3D models (STL meshes) in the visualizer alongside G-code toolpaths.
//! Integrates with existing OpenGL infrastructure.

use glam::{Mat4, Vec3};

/// A renderable 3D mesh for visualization
#[derive(Debug, Clone)]
pub struct RenderableMesh {
    /// Vertex data: [x, y, z, nx, ny, nz, r, g, b, a] per vertex
    pub vertices: Vec<f32>,
    /// Index data for triangles
    pub indices: Vec<u32>,
    /// Mesh bounds for culling
    pub bounds_min: Vec3,
    pub bounds_max: Vec3,
    /// Material properties
    pub material: MeshMaterial,
}

/// Material properties for mesh rendering
#[derive(Debug, Clone)]
pub struct MeshMaterial {
    /// Diffuse color (RGBA)
    pub diffuse_color: [f32; 4],
    /// Ambient color (RGBA)  
    pub ambient_color: [f32; 4],
    /// Specular color (RGBA)
    pub specular_color: [f32; 4],
    /// Shininess factor
    pub shininess: f32,
    /// Transparency (0.0 = fully transparent, 1.0 = fully opaque)
    pub alpha: f32,
    /// Whether to render wireframe
    pub wireframe: bool,
}

impl Default for MeshMaterial {
    fn default() -> Self {
        Self {
            diffuse_color: [0.7, 0.7, 0.8, 1.0],  // Light gray
            ambient_color: [0.2, 0.2, 0.2, 1.0],  // Dark gray
            specular_color: [1.0, 1.0, 1.0, 1.0], // White
            shininess: 32.0,
            alpha: 0.8,  // Slightly transparent so toolpaths show through
            wireframe: false,
        }
    }
}

impl MeshMaterial {
    /// Create a material for imported STL models
    pub fn stl_model() -> Self {
        Self {
            diffuse_color: [0.6, 0.8, 1.0, 1.0],  // Light blue
            ambient_color: [0.1, 0.2, 0.3, 1.0],  // Dark blue
            alpha: 0.7,  // Semi-transparent
            ..Default::default()
        }
    }
    
    /// Create a wireframe material
    pub fn wireframe() -> Self {
        Self {
            diffuse_color: [0.0, 1.0, 0.0, 1.0],  // Green
            wireframe: true,
            alpha: 1.0,
            ..Default::default()
        }
    }
    
    /// Create a solid material
    pub fn solid(color: [f32; 4]) -> Self {
        Self {
            diffuse_color: color,
            ambient_color: [color[0] * 0.3, color[1] * 0.3, color[2] * 0.3, color[3]],
            alpha: color[3],
            ..Default::default()
        }
    }
}

impl RenderableMesh {
    /// Create a new empty mesh
    pub fn new() -> Self {
        Self {
            vertices: Vec::new(),
            indices: Vec::new(),
            bounds_min: Vec3::ZERO,
            bounds_max: Vec3::ZERO,
            material: MeshMaterial::default(),
        }
    }
    
    /// Create mesh from STL mesh data
    pub fn from_stl_mesh(mesh: &gcodekit5_designer::model3d::Mesh3D) -> Self {
        let mut vertices = Vec::new();
        let mut indices = Vec::new();
        let mut index_counter = 0u32;
        
        for triangle in &mesh.triangles {
            // Add vertices for this triangle
            for vertex in &triangle.vertices {
                // Position
                vertices.push(vertex.x);
                vertices.push(vertex.y);
                vertices.push(vertex.z);
                
                // Normal
                vertices.push(triangle.normal.x);
                vertices.push(triangle.normal.y);
                vertices.push(triangle.normal.z);
                
                // Color (will be overridden by material)
                vertices.push(0.7); // R
                vertices.push(0.7); // G
                vertices.push(0.8); // B
                vertices.push(1.0); // A
            }
            
            // Add indices for this triangle
            indices.push(index_counter);
            indices.push(index_counter + 1);
            indices.push(index_counter + 2);
            
            index_counter += 3;
        }
        
        // Calculate bounds
        let bounds_min = Vec3::new(mesh.bounds_min.x, mesh.bounds_min.y, mesh.bounds_min.z);
        let bounds_max = Vec3::new(mesh.bounds_max.x, mesh.bounds_max.y, mesh.bounds_max.z);
        
        Self {
            vertices,
            indices,
            bounds_min,
            bounds_max,
            material: MeshMaterial::stl_model(),
        }
    }
    
    /// Create a simple box mesh for testing
    pub fn create_box(width: f32, height: f32, depth: f32) -> Self {
        let half_w = width * 0.5;
        let half_h = height * 0.5;
        let half_d = depth * 0.5;
        
        // Box vertices with normals and colors
        #[rustfmt::skip]
        let vertices = vec![
            // Front face (Z+)
            -half_w, -half_h,  half_d,  0.0,  0.0,  1.0,  1.0, 0.0, 0.0, 1.0,
             half_w, -half_h,  half_d,  0.0,  0.0,  1.0,  1.0, 0.0, 0.0, 1.0,
             half_w,  half_h,  half_d,  0.0,  0.0,  1.0,  1.0, 0.0, 0.0, 1.0,
            -half_w,  half_h,  half_d,  0.0,  0.0,  1.0,  1.0, 0.0, 0.0, 1.0,
            
            // Back face (Z-)
            -half_w, -half_h, -half_d,  0.0,  0.0, -1.0,  0.0, 1.0, 0.0, 1.0,
             half_w, -half_h, -half_d,  0.0,  0.0, -1.0,  0.0, 1.0, 0.0, 1.0,
             half_w,  half_h, -half_d,  0.0,  0.0, -1.0,  0.0, 1.0, 0.0, 1.0,
            -half_w,  half_h, -half_d,  0.0,  0.0, -1.0,  0.0, 1.0, 0.0, 1.0,
            
            // Left face (X-)
            -half_w, -half_h, -half_d, -1.0,  0.0,  0.0,  0.0, 0.0, 1.0, 1.0,
            -half_w, -half_h,  half_d, -1.0,  0.0,  0.0,  0.0, 0.0, 1.0, 1.0,
            -half_w,  half_h,  half_d, -1.0,  0.0,  0.0,  0.0, 0.0, 1.0, 1.0,
            -half_w,  half_h, -half_d, -1.0,  0.0,  0.0,  0.0, 0.0, 1.0, 1.0,
            
            // Right face (X+)
             half_w, -half_h, -half_d,  1.0,  0.0,  0.0,  1.0, 1.0, 0.0, 1.0,
             half_w, -half_h,  half_d,  1.0,  0.0,  0.0,  1.0, 1.0, 0.0, 1.0,
             half_w,  half_h,  half_d,  1.0,  0.0,  0.0,  1.0, 1.0, 0.0, 1.0,
             half_w,  half_h, -half_d,  1.0,  0.0,  0.0,  1.0, 1.0, 0.0, 1.0,
            
            // Top face (Y+)
            -half_w,  half_h, -half_d,  0.0,  1.0,  0.0,  1.0, 0.0, 1.0, 1.0,
             half_w,  half_h, -half_d,  0.0,  1.0,  0.0,  1.0, 0.0, 1.0, 1.0,
             half_w,  half_h,  half_d,  0.0,  1.0,  0.0,  1.0, 0.0, 1.0, 1.0,
            -half_w,  half_h,  half_d,  0.0,  1.0,  0.0,  1.0, 0.0, 1.0, 1.0,
            
            // Bottom face (Y-)
            -half_w, -half_h, -half_d,  0.0, -1.0,  0.0,  0.5, 0.5, 0.5, 1.0,
             half_w, -half_h, -half_d,  0.0, -1.0,  0.0,  0.5, 0.5, 0.5, 1.0,
             half_w, -half_h,  half_d,  0.0, -1.0,  0.0,  0.5, 0.5, 0.5, 1.0,
            -half_w, -half_h,  half_d,  0.0, -1.0,  0.0,  0.5, 0.5, 0.5, 1.0,
        ];
        
        #[rustfmt::skip]
        let indices = vec![
            // Front
            0, 1, 2,  2, 3, 0,
            // Back
            6, 5, 4,  4, 7, 6,
            // Left
            8, 9, 10,  10, 11, 8,
            // Right
            14, 13, 12,  12, 15, 14,
            // Top
            16, 17, 18,  18, 19, 16,
            // Bottom
            22, 21, 20,  20, 23, 22,
        ];
        
        Self {
            vertices,
            indices,
            bounds_min: Vec3::new(-half_w, -half_h, -half_d),
            bounds_max: Vec3::new(half_w, half_h, half_d),
            material: MeshMaterial::default(),
        }
    }
    
    /// Set material properties
    pub fn with_material(mut self, material: MeshMaterial) -> Self {
        self.material = material;
        self
    }
    
    /// Get vertex count
    pub fn vertex_count(&self) -> usize {
        self.vertices.len() / 10  // 10 floats per vertex
    }
    
    /// Get triangle count
    pub fn triangle_count(&self) -> usize {
        self.indices.len() / 3
    }
    
    /// Check if mesh is empty
    pub fn is_empty(&self) -> bool {
        self.vertices.is_empty() || self.indices.is_empty()
    }
    
    /// Get bounds as (min, max) Vec3 tuple
    pub fn bounds(&self) -> (Vec3, Vec3) {
        (self.bounds_min, self.bounds_max)
    }
    
    /// Transform mesh vertices by a matrix
    pub fn transform(&mut self, transform: &Mat4) {
        let normal_matrix = transform.inverse().transpose();
        
        // Transform vertices in chunks of 10 (pos + normal + color)
        for chunk in self.vertices.chunks_mut(10) {
            // Transform position
            let pos = Vec3::new(chunk[0], chunk[1], chunk[2]);
            let transformed_pos = transform.transform_point3(pos);
            chunk[0] = transformed_pos.x;
            chunk[1] = transformed_pos.y;
            chunk[2] = transformed_pos.z;
            
            // Transform normal
            let normal = Vec3::new(chunk[3], chunk[4], chunk[5]);
            let transformed_normal = normal_matrix.transform_vector3(normal).normalize();
            chunk[3] = transformed_normal.x;
            chunk[4] = transformed_normal.y;
            chunk[5] = transformed_normal.z;
        }
        
        // Recalculate bounds
        let mut min_pos = Vec3::splat(f32::MAX);
        let mut max_pos = Vec3::splat(f32::MIN);
        
        for chunk in self.vertices.chunks(10) {
            let pos = Vec3::new(chunk[0], chunk[1], chunk[2]);
            min_pos = min_pos.min(pos);
            max_pos = max_pos.max(pos);
        }
        
        self.bounds_min = min_pos;
        self.bounds_max = max_pos;
    }
    
    /// Create wireframe version of the mesh
    pub fn to_wireframe(&self) -> Self {
        // For wireframe, we need to convert triangles to lines
        let mut wireframe_vertices = Vec::new();
        let mut wireframe_indices = Vec::new();
        let mut index_counter = 0u32;
        
        // Process triangles in groups of 3 indices
        for triangle_indices in self.indices.chunks(3) {
            if triangle_indices.len() != 3 {
                continue;
            }
            
            // Get the three vertices of this triangle
            for &vertex_idx in triangle_indices {
                let vertex_start = (vertex_idx as usize) * 10;
                if vertex_start + 9 < self.vertices.len() {
                    // Copy vertex data
                    for i in 0..10 {
                        wireframe_vertices.push(self.vertices[vertex_start + i]);
                    }
                }
            }
            
            // Create line indices for the triangle edges
            wireframe_indices.push(index_counter);     // Edge 0-1
            wireframe_indices.push(index_counter + 1);
            
            wireframe_indices.push(index_counter + 1); // Edge 1-2
            wireframe_indices.push(index_counter + 2);
            
            wireframe_indices.push(index_counter + 2); // Edge 2-0
            wireframe_indices.push(index_counter);
            
            index_counter += 3;
        }
        
        Self {
            vertices: wireframe_vertices,
            indices: wireframe_indices,
            bounds_min: self.bounds_min,
            bounds_max: self.bounds_max,
            material: MeshMaterial::wireframe(),
        }
    }
}

/// Collection of meshes for rendering
#[derive(Debug, Clone)]
pub struct MeshCollection {
    pub meshes: Vec<RenderableMesh>,
    pub visible: bool,
}

impl MeshCollection {
    pub fn new() -> Self {
        Self {
            meshes: Vec::new(),
            visible: true,
        }
    }
    
    pub fn add_mesh(&mut self, mesh: RenderableMesh) {
        self.meshes.push(mesh);
    }
    
    pub fn clear(&mut self) {
        self.meshes.clear();
    }
    
    pub fn is_empty(&self) -> bool {
        self.meshes.is_empty()
    }
    
    pub fn total_vertices(&self) -> usize {
        self.meshes.iter().map(|m| m.vertex_count()).sum()
    }
    
    pub fn total_triangles(&self) -> usize {
        self.meshes.iter().map(|m| m.triangle_count()).sum()
    }
    
    pub fn set_visible(&mut self, visible: bool) {
        self.visible = visible;
    }
    
    /// Get combined bounds of all meshes
    pub fn bounds(&self) -> Option<(Vec3, Vec3)> {
        if self.meshes.is_empty() {
            return None;
        }
        
        let mut min_bounds = Vec3::splat(f32::MAX);
        let mut max_bounds = Vec3::splat(f32::MIN);
        
        for mesh in &self.meshes {
            min_bounds = min_bounds.min(mesh.bounds_min);
            max_bounds = max_bounds.max(mesh.bounds_max);
        }
        
        Some((min_bounds, max_bounds))
    }
}

impl Default for MeshCollection {
    fn default() -> Self {
        Self::new()
    }
}

/// Utility functions for mesh generation
pub mod mesh_utils {
    use super::*;
    
    /// Generate a grid mesh for visualization
    pub fn create_grid_mesh(size: f32, divisions: u32) -> RenderableMesh {
        let mut vertices = Vec::new();
        let mut indices = Vec::new();
        
        let step = size / divisions as f32;
        let half_size = size * 0.5;
        
        // Generate grid lines in X direction
        for i in 0..=divisions {
            let x = -half_size + i as f32 * step;
            
            // Start point
            vertices.extend_from_slice(&[
                x, 0.0, -half_size,  // Position
                0.0, 1.0, 0.0,       // Normal (up)
                0.5, 0.5, 0.5, 1.0,  // Color (gray)
            ]);
            
            // End point  
            vertices.extend_from_slice(&[
                x, 0.0, half_size,   // Position
                0.0, 1.0, 0.0,       // Normal (up)
                0.5, 0.5, 0.5, 1.0,  // Color (gray)
            ]);
        }
        
        // Generate grid lines in Z direction
        for i in 0..=divisions {
            let z = -half_size + i as f32 * step;
            
            // Start point
            vertices.extend_from_slice(&[
                -half_size, 0.0, z,  // Position
                0.0, 1.0, 0.0,       // Normal (up)
                0.5, 0.5, 0.5, 1.0,  // Color (gray)
            ]);
            
            // End point
            vertices.extend_from_slice(&[
                half_size, 0.0, z,   // Position
                0.0, 1.0, 0.0,       // Normal (up)
                0.5, 0.5, 0.5, 1.0,  // Color (gray)
            ]);
        }
        
        // Generate line indices
        for i in 0..(vertices.len() / 10) {
            if i % 2 == 0 {
                indices.push(i as u32);
                indices.push(i as u32 + 1);
            }
        }
        
        RenderableMesh {
            vertices,
            indices,
            bounds_min: Vec3::new(-half_size, 0.0, -half_size),
            bounds_max: Vec3::new(half_size, 0.0, half_size),
            material: MeshMaterial::wireframe(),
        }
    }
    
    /// Generate coordinate axes mesh
    pub fn create_axes_mesh(length: f32) -> RenderableMesh {
        let mut vertices = Vec::new();
        let mut indices = Vec::new();
        
        // X axis (red)
        vertices.extend_from_slice(&[
            0.0, 0.0, 0.0,       // Origin
            0.0, 1.0, 0.0,       // Normal
            1.0, 0.0, 0.0, 1.0,  // Red
        ]);
        vertices.extend_from_slice(&[
            length, 0.0, 0.0,    // X end
            0.0, 1.0, 0.0,       // Normal
            1.0, 0.0, 0.0, 1.0,  // Red
        ]);
        
        // Y axis (green)
        vertices.extend_from_slice(&[
            0.0, 0.0, 0.0,       // Origin
            0.0, 1.0, 0.0,       // Normal
            0.0, 1.0, 0.0, 1.0,  // Green
        ]);
        vertices.extend_from_slice(&[
            0.0, length, 0.0,    // Y end
            0.0, 1.0, 0.0,       // Normal
            0.0, 1.0, 0.0, 1.0,  // Green
        ]);
        
        // Z axis (blue)
        vertices.extend_from_slice(&[
            0.0, 0.0, 0.0,       // Origin
            0.0, 1.0, 0.0,       // Normal
            0.0, 0.0, 1.0, 1.0,  // Blue
        ]);
        vertices.extend_from_slice(&[
            0.0, 0.0, length,    // Z end
            0.0, 1.0, 0.0,       // Normal
            0.0, 0.0, 1.0, 1.0,  // Blue
        ]);
        
        // Line indices
        indices.extend_from_slice(&[0, 1, 2, 3, 4, 5]);
        
        RenderableMesh {
            vertices,
            indices,
            bounds_min: Vec3::ZERO,
            bounds_max: Vec3::splat(length),
            material: MeshMaterial::wireframe(),
        }
    }
}