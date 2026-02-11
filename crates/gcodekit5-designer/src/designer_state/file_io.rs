//! File I/O operations (save, load, new) for designer state.

use super::DesignerState;

impl DesignerState {
    /// Save design to file.
    pub fn save_to_file(&mut self, path: impl AsRef<std::path::Path>) -> anyhow::Result<()> {
        use crate::serialization::DesignFile;

        let mut design = DesignFile::new(&self.design_name);

        // Save viewport state
        design.viewport.zoom = self.canvas.zoom();
        design.viewport.pan_x = self.canvas.pan_x();
        design.viewport.pan_y = self.canvas.pan_y();

        // Save all shapes
        for obj in self.canvas.shapes() {
            design.shapes.push(DesignFile::from_drawing_object(obj));
        }

        // Save default properties
        design.default_properties = Some(DesignFile::from_drawing_object(
            &self.default_properties_shape,
        ));

        // Save tool settings
        design.toolpath_params.feed_rate = self.tool_settings.feed_rate;
        design.toolpath_params.spindle_speed = self.tool_settings.spindle_speed as f64;
        design.toolpath_params.tool_diameter = self.tool_settings.tool_diameter;
        design.toolpath_params.cut_depth = self.tool_settings.cut_depth;

        // Save stock settings
        if let Some(stock) = &self.stock_material {
            design.toolpath_params.stock_width = stock.width;
            design.toolpath_params.stock_height = stock.height;
            design.toolpath_params.stock_thickness = stock.thickness;
            design.toolpath_params.safe_z_height = stock.safe_z;
        }

        // Save to file
        design.save_to_file(&path)?;

        // Update state
        self.current_file_path = Some(path.as_ref().to_path_buf());
        self.is_modified = false;

        Ok(())
    }

    /// Load design from file.
    pub fn load_from_file(&mut self, path: impl AsRef<std::path::Path>) -> anyhow::Result<()> {
        use crate::serialization::DesignFile;
        use crate::stock_removal::StockMaterial;

        let design = DesignFile::load_from_file(&path)?;

        // Clear existing shapes
        self.canvas.clear();

        // Restore viewport
        self.canvas.set_zoom(design.viewport.zoom);
        self.canvas
            .set_pan(design.viewport.pan_x, design.viewport.pan_y);

        // Restore shapes with full DrawingObject (preserves all properties)
        for shape_data in &design.shapes {
            if let Ok(obj) = DesignFile::to_drawing_object(shape_data, shape_data.id) {
                self.canvas.restore_shape(obj);
            }
        }

        // Restore default properties
        if let Some(default_props) = &design.default_properties {
            if let Ok(obj) = DesignFile::to_drawing_object(default_props, 0) {
                self.default_properties_shape = obj;
            }
        }

        // Restore tool settings
        self.tool_settings.feed_rate = design.toolpath_params.feed_rate;
        self.tool_settings.spindle_speed = design.toolpath_params.spindle_speed as u32;
        self.tool_settings.tool_diameter = design.toolpath_params.tool_diameter;
        self.tool_settings.cut_depth = design.toolpath_params.cut_depth;

        // Also update the toolpath generator to match
        self.toolpath_generator
            .set_feed_rate(design.toolpath_params.feed_rate);
        self.toolpath_generator
            .set_spindle_speed(design.toolpath_params.spindle_speed as u32);
        self.toolpath_generator
            .set_tool_diameter(design.toolpath_params.tool_diameter);
        self.toolpath_generator
            .set_cut_depth(design.toolpath_params.cut_depth);

        // Restore stock settings
        self.stock_material = Some(StockMaterial {
            width: design.toolpath_params.stock_width,
            height: design.toolpath_params.stock_height,
            thickness: design.toolpath_params.stock_thickness,
            origin: (0.0, 0.0, 0.0),
            safe_z: design.toolpath_params.safe_z_height,
        });

        // Update state
        self.design_name = design.metadata.name.clone();
        self.current_file_path = Some(path.as_ref().to_path_buf());
        self.is_modified = false;
        self.clear_history();

        Ok(())
    }

    /// Create new design (clear all).
    pub fn new_design(&mut self) {
        self.canvas.clear();
        self.generated_gcode.clear();
        self.gcode_generated = false;
        self.current_file_path = None;
        self.is_modified = false;
        self.design_name = "Untitled".to_string();
        self.clear_history();
    }

    /// Mark design as modified.
    pub fn mark_modified(&mut self) {
        self.is_modified = true;
    }

    /// Get display name for the design.
    pub fn display_name(&self) -> String {
        let name = if let Some(path) = &self.current_file_path {
            path.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or(&self.design_name)
        } else {
            &self.design_name
        };

        if self.is_modified {
            format!("{}*", name)
        } else {
            name.to_string()
        }
    }
}
