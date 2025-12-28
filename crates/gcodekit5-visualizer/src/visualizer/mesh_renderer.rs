//! # OpenGL Mesh Renderer
//!
//! Handles OpenGL rendering operations for 3D meshes with proper lighting and materials.

use crate::visualizer::mesh_rendering::{RenderableMesh, MeshCollection};
use crate::visualizer::mesh_shaders::{MESH_VERTEX_SHADER, MESH_FRAGMENT_SHADER, 
                                       WIREFRAME_VERTEX_SHADER, WIREFRAME_FRAGMENT_SHADER};
use glam::{Mat4, Vec3};
use glow::HasContext;
use std::collections::HashMap;

/// Error type for mesh rendering operations
#[derive(Debug, thiserror::Error)]
pub enum MeshRenderError {
    #[error("OpenGL error: {0}")]
    OpenGLError(String),
    #[error("Shader compilation error: {0}")]
    ShaderError(String),
    #[error("Buffer creation error: {0}")]
    BufferError(String),
}

type Result<T> = std::result::Result<T, MeshRenderError>;

/// OpenGL resources for a single mesh
#[derive(Debug)]
struct MeshGLResources {
    vao: glow::VertexArray,
    vbo: glow::Buffer,
    ebo: glow::Buffer,
    index_count: i32,
    is_wireframe: bool,
}

/// Lighting parameters for mesh rendering
#[derive(Debug, Clone)]
pub struct LightingParams {
    /// Light direction (normalized)
    pub light_direction: Vec3,
    /// Light color (RGB)
    pub light_color: Vec3,
    /// Ambient light color (RGB)
    pub ambient_color: Vec3,
    /// Camera position for specular highlights
    pub camera_position: Vec3,
}

impl Default for LightingParams {
    fn default() -> Self {
        Self {
            light_direction: Vec3::new(-0.3, -1.0, -0.7).normalize(),
            light_color: Vec3::new(1.0, 1.0, 1.0),
            ambient_color: Vec3::new(0.2, 0.2, 0.2),
            camera_position: Vec3::new(0.0, 0.0, 10.0),
        }
    }
}

/// OpenGL mesh renderer
pub struct MeshRenderer {
    gl: glow::Context,
    mesh_shader_program: glow::Program,
    wireframe_shader_program: glow::Program,
    mesh_resources: HashMap<usize, MeshGLResources>,
    lighting: LightingParams,
    wireframe_mode: bool,
}

impl MeshRenderer {
    /// Create a new mesh renderer
    pub fn new(gl: glow::Context) -> Result<Self> {
        // Compile shaders
        let mesh_shader_program = Self::create_shader_program(
            &gl, 
            MESH_VERTEX_SHADER, 
            MESH_FRAGMENT_SHADER
        )?;
        
        let wireframe_shader_program = Self::create_shader_program(
            &gl, 
            WIREFRAME_VERTEX_SHADER, 
            WIREFRAME_FRAGMENT_SHADER
        )?;
        
        Ok(Self {
            gl,
            mesh_shader_program,
            wireframe_shader_program,
            mesh_resources: HashMap::new(),
            lighting: LightingParams::default(),
            wireframe_mode: false,
        })
    }
    
    /// Set lighting parameters
    pub fn set_lighting(&mut self, lighting: LightingParams) {
        self.lighting = lighting;
    }
    
    /// Toggle wireframe mode
    pub fn set_wireframe_mode(&mut self, wireframe: bool) {
        self.wireframe_mode = wireframe;
    }
    
    /// Upload mesh data to GPU
    pub fn upload_mesh(&mut self, mesh_id: usize, mesh: &RenderableMesh) -> Result<()> {
        // Clean up existing resources if any
        if let Some(old_resources) = self.mesh_resources.remove(&mesh_id) {
            self.cleanup_mesh_resources(&old_resources);
        }
        
        // Skip empty meshes
        if mesh.is_empty() {
            return Ok(());
        }
        
        unsafe {
            // Create VAO
            let vao = self.gl.create_vertex_array()
                .map_err(|e| MeshRenderError::BufferError(e))?;
            self.gl.bind_vertex_array(Some(vao));
            
            // Create VBO
            let vbo = self.gl.create_buffer()
                .map_err(|e| MeshRenderError::BufferError(e))?;
            self.gl.bind_buffer(glow::ARRAY_BUFFER, Some(vbo));
            self.gl.buffer_data_u8_slice(
                glow::ARRAY_BUFFER,
                bytemuck::cast_slice(&mesh.vertices),
                glow::STATIC_DRAW
            );
            
            // Create EBO
            let ebo = self.gl.create_buffer()
                .map_err(|e| MeshRenderError::BufferError(e))?;
            self.gl.bind_buffer(glow::ELEMENT_ARRAY_BUFFER, Some(ebo));
            self.gl.buffer_data_u8_slice(
                glow::ELEMENT_ARRAY_BUFFER,
                bytemuck::cast_slice(&mesh.indices),
                glow::STATIC_DRAW
            );
            
            // Setup vertex attributes
            // Position (location 0)
            self.gl.vertex_attrib_pointer_f32(
                0, 3, glow::FLOAT, false, 40, 0
            );
            self.gl.enable_vertex_attrib_array(0);
            
            // Normal (location 1)
            self.gl.vertex_attrib_pointer_f32(
                1, 3, glow::FLOAT, false, 40, 12
            );
            self.gl.enable_vertex_attrib_array(1);
            
            // Color (location 2)
            self.gl.vertex_attrib_pointer_f32(
                2, 4, glow::FLOAT, false, 40, 24
            );
            self.gl.enable_vertex_attrib_array(2);
            
            // Unbind
            self.gl.bind_vertex_array(None);
            self.gl.bind_buffer(glow::ARRAY_BUFFER, None);
            self.gl.bind_buffer(glow::ELEMENT_ARRAY_BUFFER, None);
            
            // Store resources
            let resources = MeshGLResources {
                vao,
                vbo,
                ebo,
                index_count: mesh.indices.len() as i32,
                is_wireframe: mesh.material.wireframe,
            };
            
            self.mesh_resources.insert(mesh_id, resources);
        }
        
        Ok(())
    }
    
    /// Render a single mesh
    pub fn render_mesh(&self, 
                       mesh_id: usize, 
                       mesh: &RenderableMesh,
                       model_matrix: &Mat4,
                       view_matrix: &Mat4,
                       projection_matrix: &Mat4) -> Result<()> {
        
        let resources = match self.mesh_resources.get(&mesh_id) {
            Some(r) => r,
            None => return Ok(()), // Mesh not uploaded yet
        };
        
        if resources.index_count == 0 {
            return Ok(());
        }
        
        unsafe {
            // Choose shader program
            let shader_program = if self.wireframe_mode || mesh.material.wireframe {
                self.wireframe_shader_program
            } else {
                self.mesh_shader_program
            };
            
            self.gl.use_program(Some(shader_program));
            
            // Calculate matrices
            let mvp_matrix = *projection_matrix * *view_matrix * *model_matrix;
            let normal_matrix = model_matrix.inverse().transpose();
            
            // Convert Mat4 to Mat3 for normal matrix
            let normal_mat3 = glam::Mat3::from_mat4(normal_matrix);
            let normal_array = normal_mat3.to_cols_array();
            
            // Set matrix uniforms
            self.set_uniform_mat4(&shader_program, "mvp_matrix", &mvp_matrix)?;
            self.set_uniform_mat4(&shader_program, "model_matrix", model_matrix)?;
            self.set_uniform_mat4(&shader_program, "view_matrix", view_matrix)?;
            self.set_uniform_mat4(&shader_program, "projection_matrix", projection_matrix)?;
            self.set_uniform_mat3(&shader_program, "normal_matrix", &normal_array)?;
            
            if !self.wireframe_mode && !mesh.material.wireframe {
                // Set lighting uniforms
                self.set_uniform_vec3(&shader_program, "light_direction", &self.lighting.light_direction)?;
                self.set_uniform_vec3(&shader_program, "light_color", &self.lighting.light_color)?;
                self.set_uniform_vec3(&shader_program, "ambient_color", &self.lighting.ambient_color)?;
                self.set_uniform_vec3(&shader_program, "camera_position", &self.lighting.camera_position)?;
                
                // Set material uniforms
                self.set_uniform_vec4(&shader_program, "material_diffuse", &mesh.material.diffuse_color)?;
                self.set_uniform_vec4(&shader_program, "material_ambient", &mesh.material.ambient_color)?;
                self.set_uniform_vec4(&shader_program, "material_specular", &mesh.material.specular_color)?;
                self.set_uniform_f32(&shader_program, "material_shininess", mesh.material.shininess)?;
                self.set_uniform_f32(&shader_program, "material_alpha", mesh.material.alpha)?;
                self.set_uniform_bool(&shader_program, "wireframe_mode", false)?;
                
                // Enable depth testing and blending for solid meshes
                self.gl.enable(glow::DEPTH_TEST);
                if mesh.material.alpha < 1.0 {
                    self.gl.enable(glow::BLEND);
                    self.gl.blend_func(glow::SRC_ALPHA, glow::ONE_MINUS_SRC_ALPHA);
                }
            } else {
                // Wireframe mode
                self.set_uniform_vec4(&shader_program, "line_color", &mesh.material.diffuse_color)?;
                self.gl.line_width(2.0);
            }
            
            // Bind and render
            self.gl.bind_vertex_array(Some(resources.vao));
            
            if self.wireframe_mode || mesh.material.wireframe {
                self.gl.draw_elements(glow::LINES, resources.index_count, glow::UNSIGNED_INT, 0);
            } else {
                self.gl.draw_elements(glow::TRIANGLES, resources.index_count, glow::UNSIGNED_INT, 0);
            }
            
            self.gl.bind_vertex_array(None);
            
            // Cleanup state
            if mesh.material.alpha < 1.0 && !self.wireframe_mode {
                self.gl.disable(glow::BLEND);
            }
        }
        
        Ok(())
    }
    
    /// Render a collection of meshes
    pub fn render_mesh_collection(&self,
                                 collection: &MeshCollection,
                                 model_matrix: &Mat4,
                                 view_matrix: &Mat4,
                                 projection_matrix: &Mat4) -> Result<()> {
        if !collection.visible || collection.is_empty() {
            return Ok(());
        }
        
        for (i, mesh) in collection.meshes.iter().enumerate() {
            self.render_mesh(i, mesh, model_matrix, view_matrix, projection_matrix)?;
        }
        
        Ok(())
    }
    
    /// Remove mesh from GPU memory
    pub fn remove_mesh(&mut self, mesh_id: usize) {
        if let Some(resources) = self.mesh_resources.remove(&mesh_id) {
            self.cleanup_mesh_resources(&resources);
        }
    }
    
    /// Clear all meshes
    pub fn clear_all_meshes(&mut self) {
        let resources_to_clean: Vec<_> = self.mesh_resources.drain().collect();
        for (_, resources) in resources_to_clean {
            self.cleanup_mesh_resources(&resources);
        }
    }
    
    // Helper methods
    
    fn create_shader_program(gl: &glow::Context, vs_source: &str, fs_source: &str) -> Result<glow::Program> {
        unsafe {
            let vs = gl.create_shader(glow::VERTEX_SHADER)
                .map_err(|e| MeshRenderError::ShaderError(e))?;
            gl.shader_source(vs, vs_source);
            gl.compile_shader(vs);
            
            if !gl.get_shader_compile_status(vs) {
                let info = gl.get_shader_info_log(vs);
                gl.delete_shader(vs);
                return Err(MeshRenderError::ShaderError(format!("Vertex shader: {}", info)));
            }
            
            let fs = gl.create_shader(glow::FRAGMENT_SHADER)
                .map_err(|e| MeshRenderError::ShaderError(e))?;
            gl.shader_source(fs, fs_source);
            gl.compile_shader(fs);
            
            if !gl.get_shader_compile_status(fs) {
                let info = gl.get_shader_info_log(fs);
                gl.delete_shader(vs);
                gl.delete_shader(fs);
                return Err(MeshRenderError::ShaderError(format!("Fragment shader: {}", info)));
            }
            
            let program = gl.create_program()
                .map_err(|e| MeshRenderError::ShaderError(e))?;
            gl.attach_shader(program, vs);
            gl.attach_shader(program, fs);
            gl.link_program(program);
            
            if !gl.get_program_link_status(program) {
                let info = gl.get_program_info_log(program);
                gl.delete_shader(vs);
                gl.delete_shader(fs);
                gl.delete_program(program);
                return Err(MeshRenderError::ShaderError(format!("Program linking: {}", info)));
            }
            
            gl.delete_shader(vs);
            gl.delete_shader(fs);
            
            Ok(program)
        }
    }
    
    fn cleanup_mesh_resources(&self, resources: &MeshGLResources) {
        unsafe {
            self.gl.delete_vertex_array(resources.vao);
            self.gl.delete_buffer(resources.vbo);
            self.gl.delete_buffer(resources.ebo);
        }
    }
    
    fn set_uniform_mat4(&self, program: &glow::Program, name: &str, matrix: &Mat4) -> Result<()> {
        unsafe {
            let location = self.gl.get_uniform_location(*program, name);
            if let Some(loc) = location {
                self.gl.uniform_matrix_4_f32_slice(Some(&loc), false, &matrix.to_cols_array());
            }
            Ok(())
        }
    }
    
    fn set_uniform_mat3(&self, program: &glow::Program, name: &str, matrix: &[f32; 9]) -> Result<()> {
        unsafe {
            let location = self.gl.get_uniform_location(*program, name);
            if let Some(loc) = location {
                self.gl.uniform_matrix_3_f32_slice(Some(&loc), false, matrix);
            }
            Ok(())
        }
    }
    
    fn set_uniform_vec3(&self, program: &glow::Program, name: &str, vec: &Vec3) -> Result<()> {
        unsafe {
            let location = self.gl.get_uniform_location(*program, name);
            if let Some(loc) = location {
                self.gl.uniform_3_f32(Some(&loc), vec.x, vec.y, vec.z);
            }
            Ok(())
        }
    }
    
    fn set_uniform_vec4(&self, program: &glow::Program, name: &str, vec: &[f32; 4]) -> Result<()> {
        unsafe {
            let location = self.gl.get_uniform_location(*program, name);
            if let Some(loc) = location {
                self.gl.uniform_4_f32(Some(&loc), vec[0], vec[1], vec[2], vec[3]);
            }
            Ok(())
        }
    }
    
    fn set_uniform_f32(&self, program: &glow::Program, name: &str, value: f32) -> Result<()> {
        unsafe {
            let location = self.gl.get_uniform_location(*program, name);
            if let Some(loc) = location {
                self.gl.uniform_1_f32(Some(&loc), value);
            }
            Ok(())
        }
    }
    
    fn set_uniform_bool(&self, program: &glow::Program, name: &str, value: bool) -> Result<()> {
        unsafe {
            let location = self.gl.get_uniform_location(*program, name);
            if let Some(loc) = location {
                self.gl.uniform_1_i32(Some(&loc), if value { 1 } else { 0 });
            }
            Ok(())
        }
    }
}

impl Drop for MeshRenderer {
    fn drop(&mut self) {
        self.clear_all_meshes();
        
        unsafe {
            self.gl.delete_program(self.mesh_shader_program);
            self.gl.delete_program(self.wireframe_shader_program);
        }
    }
}