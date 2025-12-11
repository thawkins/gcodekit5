use glow::*;
use std::rc::Rc;
use gcodekit5_visualizer::{Visualizer, GCodeCommand, Point3D};

pub struct RenderBuffers {
    pub vao: NativeVertexArray,
    pub vbo: NativeBuffer,
    pub vertex_count: i32,
    pub draw_mode: u32,
    gl: Rc<Context>,
}

impl RenderBuffers {
    pub fn new(gl: Rc<Context>, draw_mode: u32) -> Result<Self, String> {
        unsafe {
            let vao = gl.create_vertex_array().map_err(|e| format!("Create VAO: {}", e))?;
            let vbo = gl.create_buffer().map_err(|e| format!("Create VBO: {}", e))?;
            
            Ok(Self {
                vao,
                vbo,
                vertex_count: 0,
                draw_mode,
                gl,
            })
        }
    }

    pub fn update(&mut self, vertices: &[f32]) {
        unsafe {
            self.gl.bind_vertex_array(Some(self.vao));
            self.gl.bind_buffer(ARRAY_BUFFER, Some(self.vbo));
            
            // Upload data
            let u8_slice = std::slice::from_raw_parts(
                vertices.as_ptr() as *const u8,
                vertices.len() * std::mem::size_of::<f32>(),
            );
            self.gl.buffer_data_u8_slice(ARRAY_BUFFER, u8_slice, STATIC_DRAW);

            // Configure attributes
            // Stride: 3 floats for pos + 4 floats for color = 7 * 4 bytes
            let stride = 7 * std::mem::size_of::<f32>() as i32;
            
            // Position (loc 0)
            self.gl.enable_vertex_attrib_array(0);
            self.gl.vertex_attrib_pointer_f32(0, 3, FLOAT, false, stride, 0);
            
            // Color (loc 1)
            self.gl.enable_vertex_attrib_array(1);
            self.gl.vertex_attrib_pointer_f32(1, 4, FLOAT, false, stride, 3 * std::mem::size_of::<f32>() as i32);

            self.vertex_count = (vertices.len() / 7) as i32;
            
            self.gl.bind_vertex_array(None);
            self.gl.bind_buffer(ARRAY_BUFFER, None);
        }
    }
    
    pub fn update_volume(&mut self, vertices: &[f32]) {
        unsafe {
            self.gl.bind_vertex_array(Some(self.vao));
            self.gl.bind_buffer(ARRAY_BUFFER, Some(self.vbo));
            
            // Upload data
            let u8_slice = std::slice::from_raw_parts(
                vertices.as_ptr() as *const u8,
                vertices.len() * std::mem::size_of::<f32>(),
            );
            self.gl.buffer_data_u8_slice(ARRAY_BUFFER, u8_slice, STATIC_DRAW);

            // Configure attributes for volume rendering
            // Stride: 3 floats for pos + 3 floats for tex coords = 6 * 4 bytes
            let stride = 6 * std::mem::size_of::<f32>() as i32;
            
            // Position (loc 0)
            self.gl.enable_vertex_attrib_array(0);
            self.gl.vertex_attrib_pointer_f32(0, 3, FLOAT, false, stride, 0);
            
            // TexCoord (loc 1)
            self.gl.enable_vertex_attrib_array(1);
            self.gl.vertex_attrib_pointer_f32(1, 3, FLOAT, false, stride, 3 * std::mem::size_of::<f32>() as i32);

            self.vertex_count = (vertices.len() / 6) as i32;
            
            self.gl.bind_vertex_array(None);
            self.gl.bind_buffer(ARRAY_BUFFER, None);
        }
    }
    
    pub fn update_mesh(&mut self, vertices: &[f32]) {
        unsafe {
            self.gl.bind_vertex_array(Some(self.vao));
            self.gl.bind_buffer(ARRAY_BUFFER, Some(self.vbo));
            
            let u8_slice = std::slice::from_raw_parts(
                vertices.as_ptr() as *const u8,
                vertices.len() * std::mem::size_of::<f32>(),
            );
            self.gl.buffer_data_u8_slice(ARRAY_BUFFER, u8_slice, STATIC_DRAW);

            // Layout: pos(3) + normal(3) + color(4) = 10 floats
            let stride = 10 * std::mem::size_of::<f32>() as i32;
            
            self.gl.enable_vertex_attrib_array(0);
            self.gl.vertex_attrib_pointer_f32(0, 3, FLOAT, false, stride, 0);

            self.gl.enable_vertex_attrib_array(1);
            self.gl.vertex_attrib_pointer_f32(1, 3, FLOAT, false, stride, 3 * std::mem::size_of::<f32>() as i32);

            self.gl.enable_vertex_attrib_array(2);
            self.gl.vertex_attrib_pointer_f32(2, 4, FLOAT, false, stride, 6 * std::mem::size_of::<f32>() as i32);

            self.vertex_count = (vertices.len() / 10) as i32;

            self.gl.bind_vertex_array(None);
            self.gl.bind_buffer(ARRAY_BUFFER, None);
        }
    }
    
    pub fn draw(&self) {
        if self.vertex_count > 0 {
            unsafe {
                self.gl.bind_vertex_array(Some(self.vao));
                self.gl.draw_arrays(self.draw_mode, 0, self.vertex_count);
                self.gl.bind_vertex_array(None);
            }
        }
    }
}

impl Drop for RenderBuffers {
    fn drop(&mut self) {
        unsafe {
            self.gl.delete_vertex_array(self.vao);
            self.gl.delete_buffer(self.vbo);
        }
    }
}

pub fn generate_vertex_data(visualizer: &Visualizer) -> (Vec<f32>, Vec<f32>) {
    let mut rapid_vertices = Vec::new();
    let mut cut_vertices = Vec::new();
    
    // Colors
    let rapid_color = [0.0, 0.8, 1.0, 1.0]; // Cyan (matches 2D)
    let cut_color = [1.0, 1.0, 0.0, 1.0];   // Yellow (matches 2D)
    let arc_color = [1.0, 1.0, 0.0, 1.0];   // Yellow (matches 2D)

    for cmd in visualizer.commands() {
        match cmd {
            GCodeCommand::Move { from, to, rapid, .. } => {
                if *rapid {
                    push_line(&mut rapid_vertices, from, to, rapid_color);
                } else {
                    push_line(&mut cut_vertices, from, to, cut_color);
                }
            }
            GCodeCommand::Arc { from, to, center, clockwise, .. } => {
                push_arc(&mut cut_vertices, from, to, center, *clockwise, arc_color);
            }
            GCodeCommand::Dwell { .. } => {
                // Ignore dwell for now in 3D
            }
        }
    }
    
    (rapid_vertices, cut_vertices)
}

pub fn generate_bounds_data(min_x: f32, max_x: f32, min_y: f32, max_y: f32, min_z: f32, max_z: f32) -> Vec<f32> {
    let mut vertices = Vec::new();
    let color = [0.0, 0.5, 1.0, 1.0]; // Bright Blue
    
    // Bottom face (Z = min_z)
    let p1 = Point3D::new(min_x, min_y, min_z);
    let p2 = Point3D::new(max_x, min_y, min_z);
    let p3 = Point3D::new(max_x, max_y, min_z);
    let p4 = Point3D::new(min_x, max_y, min_z);
    
    push_line(&mut vertices, &p1, &p2, color);
    push_line(&mut vertices, &p2, &p3, color);
    push_line(&mut vertices, &p3, &p4, color);
    push_line(&mut vertices, &p4, &p1, color);
    
    // Top face (Z = max_z)
    let p5 = Point3D::new(min_x, min_y, max_z);
    let p6 = Point3D::new(max_x, min_y, max_z);
    let p7 = Point3D::new(max_x, max_y, max_z);
    let p8 = Point3D::new(min_x, max_y, max_z);
    
    push_line(&mut vertices, &p5, &p6, color);
    push_line(&mut vertices, &p6, &p7, color);
    push_line(&mut vertices, &p7, &p8, color);
    push_line(&mut vertices, &p8, &p5, color);
    
    // Vertical edges
    push_line(&mut vertices, &p1, &p5, color);
    push_line(&mut vertices, &p2, &p6, color);
    push_line(&mut vertices, &p3, &p7, color);
    push_line(&mut vertices, &p4, &p8, color);
    
    vertices
}

pub fn generate_tool_marker_data() -> Vec<f32> {
    let mut vertices = Vec::new();
    let color = [1.0, 0.0, 0.0, 0.25]; // Red with 75% transparency
    
    // Simple pyramid/cone pointing down to (0,0,0)
    // Tip at (0,0,0)
    // Base at Z=10.0
    
    let tip = Point3D::new(0.0, 0.0, 0.0);
    let height = 10.0;
    let radius = 1.5;
    let base_center = Point3D::new(0.0, 0.0, height);
    
    let segments = 16; // More segments for smoother cone
    use std::f32::consts::PI;
    
    for i in 0..segments {
        let angle1 = (i as f32 / segments as f32) * 2.0 * PI;
        let angle2 = ((i + 1) as f32 / segments as f32) * 2.0 * PI;
        
        let p1 = Point3D::new(radius * angle1.cos(), radius * angle1.sin(), height);
        let p2 = Point3D::new(radius * angle2.cos(), radius * angle2.sin(), height);
        
        // Triangle side: Tip -> p1 -> p2
        push_triangle(&mut vertices, &tip, &p1, &p2, color);
        
        // Triangle base: BaseCenter -> p2 -> p1
        push_triangle(&mut vertices, &base_center, &p2, &p1, color);
    }
    
    vertices
}

fn push_triangle(vertices: &mut Vec<f32>, p1: &Point3D, p2: &Point3D, p3: &Point3D, color: [f32; 4]) {
    // Point 1
    vertices.push(p1.x);
    vertices.push(p1.y);
    vertices.push(p1.z);
    vertices.extend_from_slice(&color);
    
    // Point 2
    vertices.push(p2.x);
    vertices.push(p2.y);
    vertices.push(p2.z);
    vertices.extend_from_slice(&color);
    
    // Point 3
    vertices.push(p3.x);
    vertices.push(p3.y);
    vertices.push(p3.z);
    vertices.extend_from_slice(&color);
}

fn push_line(vertices: &mut Vec<f32>, from: &Point3D, to: &Point3D, color: [f32; 4]) {
    // Point 1
    vertices.push(from.x);
    vertices.push(from.y);
    vertices.push(from.z);
    vertices.extend_from_slice(&color);
    
    // Point 2
    vertices.push(to.x);
    vertices.push(to.y);
    vertices.push(to.z);
    vertices.extend_from_slice(&color);
}

fn push_arc(vertices: &mut Vec<f32>, from: &Point3D, to: &Point3D, center: &Point3D, clockwise: bool, color: [f32; 4]) {
    let radius = ((from.x - center.x).powi(2) + (from.y - center.y).powi(2)).sqrt();
    let start_angle = (from.y - center.y).atan2(from.x - center.x);
    let end_angle = (to.y - center.y).atan2(to.x - center.x);
    
    let mut angle_diff = end_angle - start_angle;
    use std::f32::consts::PI;
    
    if clockwise {
        if angle_diff > 0.0 {
            angle_diff -= 2.0 * PI;
        }
    } else {
        if angle_diff < 0.0 {
            angle_diff += 2.0 * PI;
        }
    }
    
    let segments = (angle_diff.abs() * radius * 2.0).ceil() as i32; // Adaptive segments
    let segments = segments.max(4).min(100); // Clamp
    
    let z_diff = to.z - from.z;
    
    let mut prev_x = from.x;
    let mut prev_y = from.y;
    let mut prev_z = from.z;
    
    for i in 1..=segments {
        let t = i as f32 / segments as f32;
        let angle = start_angle + angle_diff * t;
        
        let x = center.x + radius * angle.cos();
        let y = center.y + radius * angle.sin();
        let z = from.z + z_diff * t;
        
        // Line segment
        vertices.push(prev_x);
        vertices.push(prev_y);
        vertices.push(prev_z);
        vertices.extend_from_slice(&color);
        
        vertices.push(x);
        vertices.push(y);
        vertices.push(z);
        vertices.extend_from_slice(&color);
        
        prev_x = x;
        prev_y = y;
        prev_z = z;
    }
}

pub fn generate_grid_data(size: f32, step: f32) -> Vec<f32> {
    let mut vertices = Vec::new();
    let color = [0.3, 0.3, 0.3, 1.0]; // Dark gray
    
    let half_size = size / 2.0;
    let steps = (size / step) as i32;
    
    for i in 0..=steps {
        let pos = -half_size + (i as f32 * step);
        
        // X-parallel lines (varying Y)
        // Start
        vertices.push(-half_size);
        vertices.push(pos);
        vertices.push(0.0);
        vertices.extend_from_slice(&color);
        
        // End
        vertices.push(half_size);
        vertices.push(pos);
        vertices.push(0.0);
        vertices.extend_from_slice(&color);
        
        // Y-parallel lines (varying X)
        // Start
        vertices.push(pos);
        vertices.push(-half_size);
        vertices.push(0.0);
        vertices.extend_from_slice(&color);
        
        // End
        vertices.push(pos);
        vertices.push(half_size);
        vertices.push(0.0);
        vertices.extend_from_slice(&color);
    }
    
    vertices
}

pub fn generate_axis_data(length: f32) -> Vec<f32> {
    let mut vertices = Vec::new();
    let origin = Point3D::new(0.0, 0.0, 0.0);
    
    // X Axis - Red
    push_line(&mut vertices, &origin, &Point3D::new(length, 0.0, 0.0), [1.0, 0.0, 0.0, 1.0]);
    
    // Y Axis - Green
    push_line(&mut vertices, &origin, &Point3D::new(0.0, length, 0.0), [0.0, 1.0, 0.0, 1.0]);
    
    // Z Axis - Blue
    push_line(&mut vertices, &origin, &Point3D::new(0.0, 0.0, length), [0.0, 0.0, 1.0, 1.0]);
    
    vertices
}

// Generate a bounding box for volumetric rendering
// Returns vertices for a cube with position (loc 0) and tex coords (loc 1)
pub fn generate_volume_box_data(min_x: f32, max_x: f32, min_y: f32, max_y: f32, min_z: f32, max_z: f32) -> Vec<f32> {
    // Each vertex: 3 floats (position) + 3 floats (tex coord)
    let vertices = vec![
        // Front face (Z = max_z)
        min_x, min_y, max_z,  0.0, 0.0, 1.0,
        max_x, min_y, max_z,  1.0, 0.0, 1.0,
        max_x, max_y, max_z,  1.0, 1.0, 1.0,
        min_x, min_y, max_z,  0.0, 0.0, 1.0,
        max_x, max_y, max_z,  1.0, 1.0, 1.0,
        min_x, max_y, max_z,  0.0, 1.0, 1.0,
        
        // Back face (Z = min_z)
        max_x, min_y, min_z,  1.0, 0.0, 0.0,
        min_x, min_y, min_z,  0.0, 0.0, 0.0,
        min_x, max_y, min_z,  0.0, 1.0, 0.0,
        max_x, min_y, min_z,  1.0, 0.0, 0.0,
        min_x, max_y, min_z,  0.0, 1.0, 0.0,
        max_x, max_y, min_z,  1.0, 1.0, 0.0,
        
        // Top face (Y = max_y)
        min_x, max_y, max_z,  0.0, 1.0, 1.0,
        max_x, max_y, max_z,  1.0, 1.0, 1.0,
        max_x, max_y, min_z,  1.0, 1.0, 0.0,
        min_x, max_y, max_z,  0.0, 1.0, 1.0,
        max_x, max_y, min_z,  1.0, 1.0, 0.0,
        min_x, max_y, min_z,  0.0, 1.0, 0.0,
        
        // Bottom face (Y = min_y)
        min_x, min_y, min_z,  0.0, 0.0, 0.0,
        max_x, min_y, min_z,  1.0, 0.0, 0.0,
        max_x, min_y, max_z,  1.0, 0.0, 1.0,
        min_x, min_y, min_z,  0.0, 0.0, 0.0,
        max_x, min_y, max_z,  1.0, 0.0, 1.0,
        min_x, min_y, max_z,  0.0, 0.0, 1.0,
        
        // Right face (X = max_x)
        max_x, min_y, max_z,  1.0, 0.0, 1.0,
        max_x, min_y, min_z,  1.0, 0.0, 0.0,
        max_x, max_y, min_z,  1.0, 1.0, 0.0,
        max_x, min_y, max_z,  1.0, 0.0, 1.0,
        max_x, max_y, min_z,  1.0, 1.0, 0.0,
        max_x, max_y, max_z,  1.0, 1.0, 1.0,
        
        // Left face (X = min_x)
        min_x, min_y, min_z,  0.0, 0.0, 0.0,
        min_x, min_y, max_z,  0.0, 0.0, 1.0,
        min_x, max_y, max_z,  0.0, 1.0, 1.0,
        min_x, min_y, min_z,  0.0, 0.0, 0.0,
        min_x, max_y, max_z,  0.0, 1.0, 1.0,
        min_x, max_y, min_z,  0.0, 1.0, 0.0,
    ];
    
    vertices
}
