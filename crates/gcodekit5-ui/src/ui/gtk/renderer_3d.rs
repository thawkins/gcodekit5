//! # 3D Renderer
//!
//! OpenGL-based 3D rendering pipeline for the G-code visualizer.
//! Handles shader compilation, buffer management, and draw calls
//! for toolpath, grid, axis, and stock visualization.

use gcodekit5_visualizer::gcode::arc;
use gcodekit5_visualizer::{GCodeCommand, Point3D, Visualizer};
use glow::{
    ARRAY_BUFFER, Context, FLOAT, HasContext, NativeBuffer, NativeVertexArray, STATIC_DRAW,
};
use std::rc::Rc;

pub struct RenderBuffers {
    pub vao: NativeVertexArray,
    pub vbo: NativeBuffer,
    pub vertex_count: i32,
    pub draw_mode: u32,
    gl: Rc<Context>,
}

impl RenderBuffers {
    pub fn new(gl: Rc<Context>, draw_mode: u32) -> Result<Self, String> {
        // SAFETY: GL context is valid (Rc-shared from GLArea). Creating VAO/VBO
        // are standard GL resource allocation calls that cannot cause UB if the
        // context is current.
        unsafe {
            let vao = gl
                .create_vertex_array()
                .map_err(|e| format!("Create VAO: {}", e))?;
            let vbo = gl
                .create_buffer()
                .map_err(|e| format!("Create VBO: {}", e))?;

            Ok(Self {
                vao,
                vbo,
                vertex_count: 0,
                draw_mode,
                gl,
            })
        }
    }

    pub fn update(&mut self, vertex: &[f32]) {
        // SAFETY: GL context is valid. The from_raw_parts reinterprets the f32
        // slice as u8 for buffer upload; this is safe because the pointer and
        // length are derived from a valid slice and f32 has no alignment issues
        // when viewed as bytes.
        unsafe {
            self.gl.bind_vertex_array(Some(self.vao));
            self.gl.bind_buffer(ARRAY_BUFFER, Some(self.vbo));

            // Upload data
            let u8_slice = std::slice::from_raw_parts(
                vertex.as_ptr() as *const u8,
                std::mem::size_of_val(vertex),
            );
            self.gl
                .buffer_data_u8_slice(ARRAY_BUFFER, u8_slice, STATIC_DRAW);

            // Configure attributes
            // Stride: 3 floats for pos + 4 floats for color = 7 * 4 bytes
            let stride = 7 * std::mem::size_of::<f32>() as i32;

            // Position (loc 0)
            self.gl.enable_vertex_attrib_array(0);
            self.gl
                .vertex_attrib_pointer_f32(0, 3, FLOAT, false, stride, 0);

            // Color (loc 1)
            self.gl.enable_vertex_attrib_array(1);
            self.gl.vertex_attrib_pointer_f32(
                1,
                4,
                FLOAT,
                false,
                stride,
                3 * std::mem::size_of::<f32>() as i32,
            );

            self.vertex_count = (vertex.len() / 7) as i32;

            self.gl.bind_vertex_array(None);
            self.gl.bind_buffer(ARRAY_BUFFER, None);
        }
    }

    pub fn update_mesh(&mut self, vertex: &[f32]) {
        // SAFETY: GL context is valid. The from_raw_parts reinterprets f32 as u8
        // for GL buffer upload; pointer and length come from a valid slice.
        unsafe {
            self.gl.bind_vertex_array(Some(self.vao));
            self.gl.bind_buffer(ARRAY_BUFFER, Some(self.vbo));

            let u8_slice = std::slice::from_raw_parts(
                vertex.as_ptr() as *const u8,
                std::mem::size_of_val(vertex),
            );
            self.gl
                .buffer_data_u8_slice(ARRAY_BUFFER, u8_slice, STATIC_DRAW);

            // Layout: pos(3) + normal(3) + color(4) = 10 floats
            let stride = 10 * std::mem::size_of::<f32>() as i32;

            self.gl.enable_vertex_attrib_array(0);
            self.gl
                .vertex_attrib_pointer_f32(0, 3, FLOAT, false, stride, 0);

            self.gl.enable_vertex_attrib_array(1);
            self.gl.vertex_attrib_pointer_f32(
                1,
                3,
                FLOAT,
                false,
                stride,
                3 * std::mem::size_of::<f32>() as i32,
            );

            self.gl.enable_vertex_attrib_array(2);
            self.gl.vertex_attrib_pointer_f32(
                2,
                4,
                FLOAT,
                false,
                stride,
                6 * std::mem::size_of::<f32>() as i32,
            );

            self.vertex_count = (vertex.len() / 10) as i32;

            self.gl.bind_vertex_array(None);
            self.gl.bind_buffer(ARRAY_BUFFER, None);
        }
    }

    pub fn draw(&self) {
        if self.vertex_count > 0 {
            // SAFETY: GL context is valid; VAO was successfully created in new()
            // and configured in update(). draw_arrays reads from bound buffers.
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
        // SAFETY: GL context is valid; VAO and VBO handles are owned by this
        // struct and will not be used after deletion.
        unsafe {
            self.gl.delete_vertex_array(self.vao);
            self.gl.delete_buffer(self.vbo);
        }
    }
}
/// Colour of rapid moves and of feed moves made with the spindle/laser off.
const TRAVEL_COLOR: [f32; 4] = [0.5, 0.5, 0.5, 1.0];

/// Build line vertex data (x, y, z, r, g, b, a per vertex) for the toolpath.
///
/// Returns `(travel, cut)`: rapids and moves made with the spindle/laser off
/// (`intensity == None`, i.e. after M5) go to the travel buffer in grey; powered
/// feed moves and arcs go to the cut buffer, coloured by intensity.
pub fn generate_vertex_data(vis: &Visualizer) -> (Vec<f32>, Vec<f32>) {
    let mut rapid_vertex = Vec::new();
    let mut cut_vertex = Vec::new();

    let cut_color = |intensity: f32| -> [f32; 4] {
        let (r, g, b) = vis.get_color_for_intensity(intensity);
        [r, g, b, 1.0]
    };

    for cmd in vis.commands() {
        match cmd {
            GCodeCommand::Move {
                from,
                to,
                rapid,
                intensity,
            } => match (*rapid, intensity) {
                (false, Some(s)) => push_line(&mut cut_vertex, from, to, cut_color(*s)),
                _ => push_line(&mut rapid_vertex, from, to, TRAVEL_COLOR),
            },
            GCodeCommand::Arc {
                from,
                to,
                center,
                clockwise,
                intensity,
            } => match intensity {
                Some(s) => push_arc(&mut cut_vertex, from, to, center, *clockwise, cut_color(*s)),
                None => push_arc(
                    &mut rapid_vertex,
                    from,
                    to,
                    center,
                    *clockwise,
                    TRAVEL_COLOR,
                ),
            },
            GCodeCommand::Dwell { .. } => {}
        }
    }

    (rapid_vertex, cut_vertex)
}

/*
pub fn generate_vertex_data(visualizer: &Visualizer) -> (Vec<f32>, Vec<f32>) {
    let mut rapid_vertex = Vec::new();
    let mut cut_vertex = Vec::new();

    // Colors
    let rapid_color = [0.0, 0.8, 1.0, 1.0]; // Cyan (matches 2D)
    let cut_color = [1.0, 1.0, 0.0, 1.0]; // Yellow (matches 2D)
    let arc_color = [1.0, 1.0, 0.0, 1.0]; // Yellow (matches 2D)

    for cmd in visualizer.commands() {
        match cmd {
            GCodeCommand::Move {
                from, to, rapid, ..
            } => {
                if *rapid {
                    push_line(&mut rapid_vertex, from, to, rapid_color);
                } else {
                    push_line(&mut cut_vertex, from, to, cut_color);
                }
            }
            GCodeCommand::Arc {
                from,
                to,
                center,
                clockwise,
                ..
            } => {
                push_arc(&mut cut_vertex, from, to, center, *clockwise, arc_color);
            }
            GCodeCommand::Dwell { .. } => {
                // Ignore dwell for now in 3D
            }
        }
    }

    (rapid_vertex, cut_vertex)
}
*/

pub fn generate_bounds_data(
    min_x: f32,
    max_x: f32,
    min_y: f32,
    max_y: f32,
    min_z: f32,
    max_z: f32,
) -> Vec<f32> {
    let mut vertex = Vec::new();
    let color = [0.0, 0.5, 1.0, 1.0]; // Bright Blue

    // Bottom face (Z = min_z)
    let p1 = Point3D::new(min_x, min_y, min_z);
    let p2 = Point3D::new(max_x, min_y, min_z);
    let p3 = Point3D::new(max_x, max_y, min_z);
    let p4 = Point3D::new(min_x, max_y, min_z);

    push_line(&mut vertex, &p1, &p2, color);
    push_line(&mut vertex, &p2, &p3, color);
    push_line(&mut vertex, &p3, &p4, color);
    push_line(&mut vertex, &p4, &p1, color);

    // Top face (Z = max_z)
    let p5 = Point3D::new(min_x, min_y, max_z);
    let p6 = Point3D::new(max_x, min_y, max_z);
    let p7 = Point3D::new(max_x, max_y, max_z);
    let p8 = Point3D::new(min_x, max_y, max_z);

    push_line(&mut vertex, &p5, &p6, color);
    push_line(&mut vertex, &p6, &p7, color);
    push_line(&mut vertex, &p7, &p8, color);
    push_line(&mut vertex, &p8, &p5, color);

    // Vertical edges
    push_line(&mut vertex, &p1, &p5, color);
    push_line(&mut vertex, &p2, &p6, color);
    push_line(&mut vertex, &p3, &p7, color);
    push_line(&mut vertex, &p4, &p8, color);

    vertex
}

/// Live machine tool marker: a translucent red cone with its tip at the origin.
pub fn generate_tool_marker_data() -> Vec<f32> {
    generate_tool_marker_data_with_color([1.0, 0.0, 0.0, 0.40]) // Red with 60% transparency
}

/// Tool marker cone (tip at the origin, base at Z = 10 mm) in the given RGBA colour.
///
/// # Arguments
/// * `color` - RGBA colour for every vertex.
pub fn generate_tool_marker_data_with_color(color: [f32; 4]) -> Vec<f32> {
    let mut vertex = Vec::new();

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
        push_triangle(&mut vertex, &tip, &p1, &p2, color);

        // Triangle base: BaseCenter -> p2 -> p1
        push_triangle(&mut vertex, &base_center, &p2, &p1, color);
    }

    vertex
}

fn push_triangle(vertex: &mut Vec<f32>, p1: &Point3D, p2: &Point3D, p3: &Point3D, color: [f32; 4]) {
    // Point 1
    vertex.push(p1.x);
    vertex.push(p1.y);
    vertex.push(p1.z);
    vertex.extend_from_slice(&color);

    // Point 2
    vertex.push(p2.x);
    vertex.push(p2.y);
    vertex.push(p2.z);
    vertex.extend_from_slice(&color);

    // Point 3
    vertex.push(p3.x);
    vertex.push(p3.y);
    vertex.push(p3.z);
    vertex.extend_from_slice(&color);
}

fn push_line(vertex: &mut Vec<f32>, from: &Point3D, to: &Point3D, color: [f32; 4]) {
    // Point 1
    vertex.push(from.x);
    vertex.push(from.y);
    vertex.push(from.z);
    vertex.extend_from_slice(&color);

    // Point 2
    vertex.push(to.x);
    vertex.push(to.y);
    vertex.push(to.z);
    vertex.extend_from_slice(&color);
}

/// Push an XY arc as line segments.
///
/// The sweep comes from the shared arc maths in
/// `gcodekit5_visualizer::gcode::arc`, so the 3D view draws exactly the arc
/// the parser computed bounds for (including full circles).
fn push_arc(
    vertex: &mut Vec<f32>,
    from: &Point3D,
    to: &Point3D,
    center: &Point3D,
    clockwise: bool,
    color: [f32; 4],
) {
    let center_xy = (center.x as f64, center.y as f64);
    let Some(sweep) = arc::arc_sweep(
        center_xy,
        (from.x as f64, from.y as f64),
        (to.x as f64, to.y as f64),
        clockwise,
    ) else {
        // Degenerate arc (zero radius): draw it as a straight move.
        push_line(vertex, from, to, color);
        return;
    };

    // At most 5 degrees per segment, 4..=128 segments.
    let segments = sweep.segment_count(std::f64::consts::PI / 36.0, 4, 128);
    let z_diff = to.z - from.z;
    let mut prev = *from;
    for i in 1..=segments {
        let next = if i == segments {
            *to
        } else {
            let t = i as f64 / segments as f64;
            let (x, y) = sweep.point_at(center_xy, t);
            Point3D::new(x as f32, y as f32, from.z + z_diff * t as f32)
        };
        push_line(vertex, &prev, &next, color);
        prev = next;
    }
}

pub fn generate_grid_data(size: f32, step: f32) -> Vec<f32> {
    let mut vertex = Vec::new();
    let color = [0.3, 0.3, 0.3, 1.0]; // Dark gray

    let half_size = size / 2.0;
    let steps = (size / step) as i32;

    for i in 0..=steps {
        let pos = -half_size + (i as f32 * step);

        // X-parallel lines (varying Y)
        // Start
        vertex.push(-half_size);
        vertex.push(pos);
        vertex.push(0.0);
        vertex.extend_from_slice(&color);

        // End
        vertex.push(half_size);
        vertex.push(pos);
        vertex.push(0.0);
        vertex.extend_from_slice(&color);

        // Y-parallel lines (varying X)
        // Start
        vertex.push(pos);
        vertex.push(-half_size);
        vertex.push(0.0);
        vertex.extend_from_slice(&color);

        // End
        vertex.push(pos);
        vertex.push(half_size);
        vertex.push(0.0);
        vertex.extend_from_slice(&color);
    }

    vertex
}

pub fn generate_axis_data(length: f32) -> Vec<f32> {
    let mut vertex = Vec::new();
    let origin = Point3D::new(0.0, 0.0, 0.0);

    // X Axis - Red
    push_line(
        &mut vertex,
        &origin,
        &Point3D::new(length, 0.0, 0.0),
        [1.0, 0.0, 0.0, 1.0],
    );

    // Y Axis - Green
    push_line(
        &mut vertex,
        &origin,
        &Point3D::new(0.0, length, 0.0),
        [0.0, 1.0, 0.0, 1.0],
    );

    // Z Axis - Blue
    push_line(
        &mut vertex,
        &origin,
        &Point3D::new(0.0, 0.0, length),
        [0.0, 0.0, 1.0, 1.0],
    );

    vertex
}
