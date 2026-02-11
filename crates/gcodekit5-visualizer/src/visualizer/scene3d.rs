//! # 3D Visualization Integration
//!
//! Integrates 3D model rendering with existing toolpath visualization.
//! Provides a unified interface for rendering both imported STL models and G-code toolpaths.

use crate::visualizer::{
    Camera3D, LightingParams, MeshCollection, MeshRenderError, MeshRenderer, RenderableMesh,
    Visualizer,
};
use gcodekit5_designer::model::{DesignerShape, Shape};
use gcodekit5_designer::model3d::Mesh3D;
use gcodekit5_designer::shadow_projection::ShadowProjector;
use glam::{Mat4, Vec3};
use std::collections::HashMap;
use tracing::debug;

/// Unified 3D scene containing both meshes and toolpaths
#[derive(Debug)]
pub struct Scene3D {
    /// Imported 3D models (STL meshes)
    pub mesh_collections: HashMap<String, MeshCollection>,

    /// G-code toolpath visualizer
    pub toolpath_visualizer: Option<Visualizer>,

    /// Shadow projections from 3D models
    pub shadow_projections: HashMap<String, Vec<lyon::path::Path>>,

    /// Scene visibility flags
    pub show_meshes: bool,
    pub show_toolpaths: bool,
    pub show_shadows: bool,

    /// Scene bounds
    pub bounds_min: Vec3,
    pub bounds_max: Vec3,
}

impl Default for Scene3D {
    fn default() -> Self {
        Self {
            mesh_collections: HashMap::new(),
            toolpath_visualizer: None,
            shadow_projections: HashMap::new(),
            show_meshes: true,
            show_toolpaths: true,
            show_shadows: false,
            bounds_min: Vec3::ZERO,
            bounds_max: Vec3::ZERO,
        }
    }
}

impl Scene3D {
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a 3D mesh to the scene
    pub fn add_mesh(&mut self, name: String, mesh_3d: &Mesh3D) {
        let renderable_mesh = RenderableMesh::from_stl_mesh(mesh_3d);
        let mut collection = MeshCollection::new();
        collection.add_mesh(renderable_mesh);

        self.mesh_collections.insert(name.clone(), collection);

        // Generate shadow projection for toolpath planning
        let projector = ShadowProjector::orthographic_z();
        if let Ok(shapes) = projector.project_shadow(mesh_3d) {
            // Convert to lyon paths for toolpath generation
            let paths = self.projection_to_lyon_paths(&[shapes]);
            self.shadow_projections.insert(name, paths);
        }

        self.update_bounds();
    }

    /// Remove a mesh from the scene
    pub fn remove_mesh(&mut self, name: &str) {
        self.mesh_collections.remove(name);
        self.shadow_projections.remove(name);
        self.update_bounds();
    }

    /// Set toolpath visualizer
    pub fn set_toolpath_visualizer(&mut self, visualizer: Visualizer) {
        self.toolpath_visualizer = Some(visualizer);
        self.update_bounds();
    }

    /// Get mesh collection by name
    pub fn get_mesh_collection(&self, name: &str) -> Option<&MeshCollection> {
        self.mesh_collections.get(name)
    }

    /// Get mutable mesh collection by name
    pub fn get_mesh_collection_mut(&mut self, name: &str) -> Option<&mut MeshCollection> {
        self.mesh_collections.get_mut(name)
    }

    /// Get all mesh names
    pub fn mesh_names(&self) -> Vec<String> {
        self.mesh_collections.keys().cloned().collect()
    }

    /// Get shadow projection paths for a mesh
    pub fn get_shadow_projection(&self, name: &str) -> Option<&Vec<lyon::path::Path>> {
        self.shadow_projections.get(name)
    }

    /// Toggle mesh visibility
    pub fn set_meshes_visible(&mut self, visible: bool) {
        self.show_meshes = visible;
        for collection in self.mesh_collections.values_mut() {
            collection.set_visible(visible);
        }
    }

    /// Toggle toolpath visibility
    pub fn set_toolpaths_visible(&mut self, visible: bool) {
        self.show_toolpaths = visible;
    }

    /// Toggle shadow projection visibility
    pub fn set_shadows_visible(&mut self, visible: bool) {
        self.show_shadows = visible;
    }

    /// Get scene statistics
    pub fn get_stats(&self) -> Scene3DStats {
        let mesh_count = self.mesh_collections.len();
        let total_vertices: usize = self
            .mesh_collections
            .values()
            .map(|c| c.total_vertices())
            .sum();
        let total_triangles: usize = self
            .mesh_collections
            .values()
            .map(|c| c.total_triangles())
            .sum();

        Scene3DStats {
            mesh_count,
            total_vertices,
            total_triangles,
            has_toolpaths: self.toolpath_visualizer.is_some(),
            shadow_projections: self.shadow_projections.len(),
            bounds: (self.bounds_min, self.bounds_max),
        }
    }

    /// Clear all content
    pub fn clear(&mut self) {
        self.mesh_collections.clear();
        self.toolpath_visualizer = None;
        self.shadow_projections.clear();
        self.bounds_min = Vec3::ZERO;
        self.bounds_max = Vec3::ZERO;
    }

    /// Update scene bounds to encompass all objects
    fn update_bounds(&mut self) {
        let mut min_bounds = Vec3::splat(f32::MAX);
        let mut max_bounds = Vec3::splat(f32::MIN);
        let mut has_objects = false;

        // Include mesh bounds
        for collection in self.mesh_collections.values() {
            if let Some((mesh_min, mesh_max)) = collection.bounds() {
                min_bounds = min_bounds.min(mesh_min);
                max_bounds = max_bounds.max(mesh_max);
                has_objects = true;
            }
        }

        // TODO(#19): Include toolpath bounds when available

        if has_objects {
            self.bounds_min = min_bounds;
            self.bounds_max = max_bounds;
        } else {
            self.bounds_min = Vec3::ZERO;
            self.bounds_max = Vec3::ZERO;
        }
    }

    /// Convert shadow projection shapes to lyon paths
    fn projection_to_lyon_paths(&self, shapes: &[Vec<Shape>]) -> Vec<lyon::path::Path> {
        let mut paths = Vec::new();

        for shape_group in shapes {
            for shape in shape_group {
                match shape {
                    Shape::Path(path) => {
                        // Convert DesignPath sketch to lyon path using the render method
                        let lyon_path = path.render();
                        paths.push(lyon_path);
                    }
                    _ => {
                        // TODO(#19): Handle other shape types (circles, rectangles, etc.)
                        debug!("Shape type not yet supported for lyon path conversion");
                    }
                }
            }
        }

        paths
    }
}

/// Statistics about the 3D scene
#[derive(Debug, Clone)]
pub struct Scene3DStats {
    pub mesh_count: usize,
    pub total_vertices: usize,
    pub total_triangles: usize,
    pub has_toolpaths: bool,
    pub shadow_projections: usize,
    pub bounds: (Vec3, Vec3),
}

/// Integrated 3D renderer that handles both meshes and toolpaths
pub struct Renderer3D {
    mesh_renderer: MeshRenderer,
    scene: Scene3D,
    camera: Camera3D,
    lighting: LightingParams,
}

impl Renderer3D {
    /// Create a new 3D renderer
    pub fn new(gl: glow::Context) -> Result<Self, MeshRenderError> {
        let mesh_renderer = MeshRenderer::new(gl)?;

        Ok(Self {
            mesh_renderer,
            scene: Scene3D::new(),
            camera: Camera3D::new(Vec3::ZERO, 100.0),
            lighting: LightingParams::default(),
        })
    }

    /// Get reference to the scene
    pub fn scene(&self) -> &Scene3D {
        &self.scene
    }

    /// Get mutable reference to the scene
    pub fn scene_mut(&mut self) -> &mut Scene3D {
        &mut self.scene
    }

    /// Set camera
    pub fn set_camera(&mut self, camera: Camera3D) {
        self.camera = camera;
    }

    /// Set lighting parameters
    pub fn set_lighting(&mut self, lighting: LightingParams) {
        self.lighting = lighting.clone();
        self.mesh_renderer.set_lighting(lighting);
    }

    /// Upload all scene meshes to GPU
    pub fn upload_scene_meshes(&mut self) -> Result<(), MeshRenderError> {
        let mut mesh_id = 0;

        for collection in self.scene.mesh_collections.values() {
            for mesh in &collection.meshes {
                self.mesh_renderer.upload_mesh(mesh_id, mesh)?;
                mesh_id += 1;
            }
        }

        Ok(())
    }

    /// Render the entire scene
    pub fn render_scene(
        &self,
        view_matrix: &Mat4,
        projection_matrix: &Mat4,
    ) -> Result<(), MeshRenderError> {
        let model_matrix = Mat4::IDENTITY;

        // Render meshes
        if self.scene.show_meshes {
            let mut mesh_id = 0;
            for collection in self.scene.mesh_collections.values() {
                if collection.visible {
                    for mesh in &collection.meshes {
                        self.mesh_renderer.render_mesh(
                            mesh_id,
                            mesh,
                            &model_matrix,
                            view_matrix,
                            projection_matrix,
                        )?;
                        mesh_id += 1;
                    }
                }
            }
        }

        // TODO(#19): Render toolpaths using existing visualizer
        // TODO(#19): Render shadow projections as 2D overlays

        Ok(())
    }

    /// Add a mesh from STL data
    pub fn add_stl_mesh(&mut self, name: String, mesh_3d: &Mesh3D) -> Result<(), MeshRenderError> {
        self.scene.add_mesh(name, mesh_3d);
        self.upload_scene_meshes()?;
        Ok(())
    }

    /// Remove a mesh by name
    pub fn remove_mesh(&mut self, name: &str) {
        // Remove from renderer
        // TODO(#19): Track mesh IDs properly for removal

        // Remove from scene
        self.scene.remove_mesh(name);
    }

    /// Set wireframe mode for all meshes
    pub fn set_wireframe_mode(&mut self, wireframe: bool) {
        self.mesh_renderer.set_wireframe_mode(wireframe);
    }

    /// Fit camera to scene bounds
    pub fn fit_camera_to_scene(&mut self) {
        let stats = self.scene.get_stats();
        let (min_bounds, max_bounds) = stats.bounds;

        if min_bounds != max_bounds {
            let center = (min_bounds + max_bounds) * 0.5;
            let size = (max_bounds - min_bounds).length();

            // Position camera to view the entire scene
            let distance = size * 2.0;
            let _camera_pos = center + Vec3::new(distance, distance, distance);

            // Update camera to look at scene center
            // TODO(#19): Implement camera positioning based on existing Camera3D interface
        }
    }
}

impl Drop for Renderer3D {
    fn drop(&mut self) {
        // Cleanup is handled by MeshRenderer's Drop implementation
    }
}

/// Convenience functions for STL integration
pub mod stl_integration {
    use super::*;

    /// Load an STL file and add it to the scene
    pub fn load_stl_file<P: AsRef<std::path::Path>>(
        renderer: &mut Renderer3D,
        file_path: P,
        name: String,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let importer = gcodekit5_designer::import::StlImporter::new();
        let design = importer.import_file(file_path.as_ref().to_str().unwrap_or(""))?;

        // Extract 3D mesh from the imported design
        if let Some(mesh_3d) = design.mesh_3d {
            renderer.add_stl_mesh(name, &mesh_3d)?;
        }
        Ok(())
    }

    /// Generate toolpaths from shadow projection
    pub fn generate_shadow_toolpaths(
        renderer: &Renderer3D,
        mesh_name: &str,
        _cutting_params: &gcodekit5_designer::slice_toolpath::SliceToolpathParams,
    ) -> Result<Vec<lyon::path::Path>, Box<dyn std::error::Error>> {
        if let Some(projections) = renderer.scene().get_shadow_projection(mesh_name) {
            // TODO(#19): Convert lyon paths to actual toolpaths using slice_toolpath module
            Ok(projections.clone())
        } else {
            Err(format!("No shadow projection found for mesh: {}", mesh_name).into())
        }
    }
}
