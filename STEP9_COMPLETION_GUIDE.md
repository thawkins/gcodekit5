# Step 9 Completion Guide - 3D Stock Removal Integration

## Status: ~80% Complete

### ✅ What's Already Done:
1. **Infrastructure** (Steps 1-8):
   - VoxelGrid data structure ✅
   - StockSimulator3D engine ✅  
   - GPU shaders (StockRemovalShaderProgram) ✅
   - StockTexture3D for uploading voxel data ✅
   - Volume box geometry generation ✅

2. **UI Structure Updates** ✅:
   - Added imports for 3D stock removal components
   - Added `stock_simulator_3d` and `stock_simulation_3d_pending` fields to `GcodeVisualizer`
   - Added `stock_removal_shader`, `stock_removal_buffers`, `stock_texture` fields to `RendererState`
   - Initialized new fields in struct constructors

### 🔧 What Remains:

#### 1. Add 3D Simulation Logic to Stock Removal Checkbox Handler

Location: Around line 923 in `visualizer.rs`

Current code only handles 2D simulation. Need to add:

```rust
show_stock_removal.connect_toggled(move |checkbox| {
    if checkbox.is_active() {
        // Check mode: 2D or 3D
        let is_3d_mode = stack.visible_child_name().as_deref() == Some("3d");
        
        if is_3d_mode {
            // Run 3D simulation
            run_3d_stock_simulation(
                &visualizer_stock,
                &stock_material_stock,
                &tool_radius_stock,
                &stock_simulator_3d,
                &stock_simulation_3d_pending,
                &gl_update,
            );
        } else {
            // Existing 2D simulation code...
        }
    } else {
        // Clear simulation
        *stock_simulator_3d.borrow_mut() = None;
        *stock_simulation_3d_pending.borrow_mut() = false;
    }
});
```

#### 2. Implement `run_3d_stock_simulation` Function

Add this as a method or helper function:

```rust
fn run_3d_stock_simulation(
    visualizer: &Rc<RefCell<Visualizer>>,
    stock_material: &Rc<RefCell<Option<StockMaterial>>>,
    tool_radius: &Rc<RefCell<f32>>,
    simulator_ref: &Rc<RefCell<Option<StockSimulator3D>>>,
    pending_ref: &Rc<RefCell<bool>>,
    gl_area: &GLArea,
) {
    let vis = visualizer.borrow();
    let stock = match stock_material.borrow().as_ref() {
        Some(s) => s.clone(),
        None => return,
    };
    
    let tool_rad = *tool_radius.borrow();
    let resolution = 0.5; // 0.5mm voxel resolution
    
    // Convert GCode commands to 3D toolpath segments
    use gcodekit5_visualizer::stock_removal_3d::ToolpathSegment;
    
    let mut segments = Vec::new();
    for cmd in vis.commands() {
        match cmd {
            GCodeCommand::Move { from, to, rapid, .. } => {
                let seg_type = if *rapid {
                    gcodekit5_visualizer::stock_removal_3d::ToolpathSegmentType::RapidMove
                } else {
                    gcodekit5_visualizer::stock_removal_3d::ToolpathSegmentType::LinearMove
                };
                segments.push(ToolpathSegment {
                    segment_type: seg_type,
                    start: (from.x, from.y, from.z),
                    end: (to.x, to.y, to.z),
                    center: None,
                });
            }
            GCodeCommand::Arc { from, to, center, clockwise, .. } => {
                let seg_type = if *clockwise {
                    gcodekit5_visualizer::stock_removal_3d::ToolpathSegmentType::ArcCW
                } else {
                    gcodekit5_visualizer::stock_removal_3d::ToolpathSegmentType::ArcCCW
                };
                segments.push(ToolpathSegment {
                    segment_type: seg_type,
                    start: (from.x, from.y, from.z),
                    end: (to.x, to.y, to.z),
                    center: Some((center.x, center.y)),
                });
            }
            _ => {}
        }
    }
    
    drop(vis);
    
    // Run simulation in background thread
    let simulator_clone = simulator_ref.clone();
    let pending_clone = pending_ref.clone();
    let gl_clone = gl_area.clone();
    
    *pending_clone.borrow_mut() = true;
    
    std::thread::spawn(move || {
        let voxel_grid = VoxelGrid::new(
            stock.width,
            stock.height,
            stock.thickness,
            resolution,
        );
        
        let mut simulator = StockSimulator3D::new(voxel_grid, tool_rad);
        simulator.simulate_toolpath(&segments);
        
        // Store result on main thread
        glib::idle_add_local(move || {
            *simulator_clone.borrow_mut() = Some(simulator);
            *pending_clone.borrow_mut() = false;
            gl_clone.queue_render();
            glib::ControlFlow::Break
        });
    });
}
```

#### 3. Initialize 3D Stock Removal Resources in GLArea Render Callback

Location: In the `gl_area.connect_render` callback, after initializing the main shader

Add after line ~1163:

```rust
// Initialize stock removal shader and buffers
if state.stock_removal_shader.is_none() {
    match StockRemovalShaderProgram::new(gl.clone()) {
        Ok(stock_shader) => {
            state.stock_removal_shader = Some(stock_shader);
        }
        Err(e) => {
            eprintln!("Stock removal shader init failed: {}", e);
        }
    }
}

if state.stock_removal_buffers.is_none() {
    match RenderBuffers::new(gl.clone(), glow::TRIANGLES) {
        Ok(buffers) => {
            use gcodekit5_visualizer::renderer_3d::generate_volume_box_data;
            // Generate unit cube, will be scaled by stock dimensions
            let volume_data = generate_volume_box_data(1.0, 1.0, 1.0);
            buffers.update(&volume_data);
            state.stock_removal_buffers = Some(buffers);
        }
        Err(e) => {
            eprintln!("Stock removal buffers init failed: {}", e);
        }
    }
}
```

#### 4. Render 3D Stock Removal

Location: In the `gl_area.connect_render` callback, after drawing tool marker (around line 1257)

Add:

```rust
// Draw 3D Stock Removal if enabled and available
if show_stock_removal_3d.is_active() {
    if let Some(simulator) = stock_simulator_3d_ref.borrow().as_ref() {
        // Create or update texture if needed
        if state.stock_texture.is_none() {
            match StockTexture3D::from_voxel_grid(gl.clone(), simulator.get_grid()) {
                Ok(texture) => {
                    state.stock_texture = Some(texture);
                }
                Err(e) => {
                    eprintln!("Failed to create stock texture: {}", e);
                }
            }
        }
        
        // Render stock removal
        if let (Some(stock_shader), Some(stock_buffers), Some(stock_texture)) = (
            &state.stock_removal_shader,
            &state.stock_removal_buffers,
            &state.stock_texture,
        ) {
            // Get stock material for dimensions
            if let Some(stock) = stock_material_3d.borrow().as_ref() {
                stock_shader.bind();
                stock_texture.bind(0);
                
                // Set uniforms
                let model = glam::Mat4::from_scale_rotation_translation(
                    glam::Vec3::new(stock.width, stock.height, stock.thickness),
                    glam::Quat::IDENTITY,
                    glam::Vec3::new(stock.origin.0, stock.origin.1, stock.origin.2),
                );
                let mvp_stock = proj * view * model;
                
                if let Some(loc) = stock_shader.get_uniform_location("uModelViewProjection") {
                    unsafe {
                        gl.uniform_matrix_4_f32_slice(Some(&loc), false, &mvp_stock.to_cols_array());
                    }
                }
                
                if let Some(loc) = stock_shader.get_uniform_location("uStockTexture") {
                    unsafe {
                        gl.uniform_1_i32(Some(&loc), 0);
                    }
                }
                
                if let Some(loc) = stock_shader.get_uniform_location("uStockDimensions") {
                    unsafe {
                        gl.uniform_3_f32(Some(&loc), stock.width, stock.height, stock.thickness);
                    }
                }
                
                if let Some(loc) = stock_shader.get_uniform_location("uShowRemoved") {
                    unsafe {
                        gl.uniform_1_i32(Some(&loc), 1); // Show removed material
                    }
                }
                
                // Enable blending for transparency
                unsafe {
                    gl.enable(glow::BLEND);
                    gl.blend_func(glow::SRC_ALPHA, glow::ONE_MINUS_SRC_ALPHA);
                }
                
                stock_buffers.draw();
                
                unsafe {
                    gl.disable(glow::BLEND);
                }
                
                stock_shader.unbind();
            }
        }
    }
}
```

#### 5. Clear 3D Stock Removal on New G-Code Load

In the `set_gcode` method, add:

```rust
// Clear 3D simulation
*self.stock_simulator_3d.borrow_mut() = None;
*self.stock_simulation_3d_pending.borrow_mut() = false;

// Clear 3D resources in renderer state
if let Some(state) = self.renderer_state.borrow_mut().as_mut() {
    state.stock_texture = None;
}
```

### Testing Checklist:

1. ☐ Load G-code file
2. ☐ Switch to 3D view
3. ☐ Configure stock dimensions (Width, Height, Thickness, Tool Radius)
4. ☐ Enable "Show Stock Removal" checkbox
5. ☐ Verify simulation runs in background (UI remains responsive)
6. ☐ Verify 3D volume rendering appears with depth-based colors
7. ☐ Test camera rotation/zoom with stock removal visible
8. ☐ Disable "Show Stock Removal" - verify it disappears
9. ☐ Switch back to 2D view - verify 2D stock removal still works

### Performance Notes:

- **Voxel Resolution**: Currently set to 0.5mm. Adjust based on performance:
  - 0.5mm = good detail, ~16MB for 200x200x50mm stock
  - 1.0mm = lower detail, ~2MB for same stock
  - 0.25mm = high detail, ~128MB for same stock

- **Texture Upload**: Only happens once when simulation completes
- **Rendering**: GPU-accelerated volumetric rendering, real-time capable

### Known Limitations:

1. **Single Pass Rendering**: Current implementation uses simple alpha blending. For better quality, could implement:
   - Ray marching with multiple samples per pixel
   - Order-independent transparency
   - Depth peeling

2. **Memory**: Large voxel grids (fine resolution, large stock) can use significant GPU memory

3. **Simulation Time**: Background thread prevents UI blocking, but large toolpaths take time to simulate

## Next Steps to Complete:

1. Add the signal handler logic to detect 3D mode
2. Implement the helper function for 3D simulation
3. Add resource initialization in GLArea render
4. Add rendering code for 3D stock removal
5. Test thoroughly!

The infrastructure is 100% complete - just need these final integration points!
