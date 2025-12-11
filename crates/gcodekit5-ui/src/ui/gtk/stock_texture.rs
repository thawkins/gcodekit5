use glow::*;
use std::rc::Rc;
use gcodekit5_visualizer::visualizer::stock_removal_3d::VoxelGrid;

pub struct StockTexture3D {
    texture: NativeTexture,
    width: usize,
    height: usize,
    depth: usize,
    gl: Rc<Context>,
}

impl StockTexture3D {
    pub fn from_voxel_grid(gl: Rc<Context>, voxel_grid: &VoxelGrid) -> Result<Self, String> {
        let (width, height, depth) = voxel_grid.dimensions();
        
        unsafe {
            let texture = gl.create_texture().map_err(|e| format!("Create 3D texture: {}", e))?;
            gl.bind_texture(TEXTURE_3D, Some(texture));
            
            // Set texture parameters
            gl.tex_parameter_i32(TEXTURE_3D, TEXTURE_MIN_FILTER, LINEAR as i32);
            gl.tex_parameter_i32(TEXTURE_3D, TEXTURE_MAG_FILTER, LINEAR as i32);
            gl.tex_parameter_i32(TEXTURE_3D, TEXTURE_WRAP_S, CLAMP_TO_EDGE as i32);
            gl.tex_parameter_i32(TEXTURE_3D, TEXTURE_WRAP_T, CLAMP_TO_EDGE as i32);
            gl.tex_parameter_i32(TEXTURE_3D, TEXTURE_WRAP_R, CLAMP_TO_EDGE as i32);
            
            // Upload voxel data as 3D texture
            let data = voxel_grid.data();
            
            gl.tex_image_3d(
                TEXTURE_3D,
                0,
                R8 as i32,
                width as i32,
                height as i32,
                depth as i32,
                0,
                RED,
                UNSIGNED_BYTE,
                Some(data),
            );
            
            gl.bind_texture(TEXTURE_3D, None);
            
            Ok(Self {
                texture,
                width,
                height,
                depth,
                gl,
            })
        }
    }
    
    pub fn bind(&self) {
        unsafe {
            self.gl.bind_texture(TEXTURE_3D, Some(self.texture));
        }
    }
    
    pub fn unbind(&self) {
        unsafe {
            self.gl.bind_texture(TEXTURE_3D, None);
        }
    }
    
    pub fn texture(&self) -> NativeTexture {
        self.texture
    }
    
    pub fn dimensions(&self) -> (usize, usize, usize) {
        (self.width, self.height, self.depth)
    }
}

impl Drop for StockTexture3D {
    fn drop(&mut self) {
        unsafe {
            self.gl.delete_texture(self.texture);
        }
    }
}
