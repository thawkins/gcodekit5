//! Designer state manager for UI integration.
//! Manages the designer canvas state and handles UI callbacks.
//!
//! This module is split into submodules for better organization:
//! - `history`: Undo/redo functionality
//! - `viewport`: Zoom, pan, grid controls  
//! - `selection`: Shape selection operations
//! - `shapes`: Shape creation, deletion, clipboard
//! - `transforms`: Move, resize, align, mirror
//! - `properties`: Property setters for selected shapes
//! - `gcode`: G-code generation
//! - `file_io`: Save/load operations

mod file_io;
mod gcode;
mod history;
mod properties;
mod selection;
mod shapes;
mod transforms;
mod viewport;

use crate::commands::DesignerCommand;
use crate::stock_removal::{SimulationResult, StockMaterial};
use crate::{Canvas, ToolpathGenerator};

/// Tool settings for the designer
#[derive(Clone, Debug)]
pub struct ToolSettings {
    pub feed_rate: f64,
    pub spindle_speed: u32,
    pub tool_diameter: f64,
    pub cut_depth: f64,
    pub start_depth: f64,
    pub step_down: f64,
    pub machine_mode: MachineMode,
}

impl Default for ToolSettings {
    fn default() -> Self {
        Self {
            feed_rate: 2000.0,
            spindle_speed: 200,
            tool_diameter: 0.1,
            cut_depth: 0.0,
            start_depth: 0.0,
            step_down: 0.0,
            machine_mode: MachineMode::default(),
        }
    }
}

/// Machine operation mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MachineMode {
    Laser2D,
    Cnc3D,
}

impl Default for MachineMode {
    fn default() -> Self {
        Self::Laser2D  // By default, Laser2D
    }
}

/// Designer state for UI integration
#[derive(Clone)]
pub struct DesignerState {
    pub canvas: Canvas,
    pub toolpath_generator: ToolpathGenerator,
    pub tool_settings: ToolSettings,
    pub generated_gcode: String,
    pub gcode_generated: bool,
    pub current_file_path: Option<std::path::PathBuf>,
    pub is_modified: bool,
    pub design_name: String,
    pub show_grid: bool,
    pub grid_spacing_mm: f64,
    pub show_toolpaths: bool,
    pub snap_enabled: bool,
    pub snap_threshold_mm: f64,
    pub clipboard: Vec<crate::canvas::DrawingObject>,
    pub default_properties_shape: crate::canvas::DrawingObject,
    pub(crate) undo_stack: Vec<DesignerCommand>,
    pub(crate) redo_stack: Vec<DesignerCommand>,
    // Stock removal simulation
    pub stock_material: Option<StockMaterial>,
    pub show_stock_removal: bool,
    pub simulation_resolution: f32,
    pub simulation_result: Option<SimulationResult>,
    /// Number of axes on the active device (default 3).
    pub num_axes: u8,
}

impl DesignerState {
    /// Creates a new designer state.
    pub fn new() -> Self {
        Self {
            canvas: Canvas::with_size(800.0, 600.0),
            toolpath_generator: ToolpathGenerator::new(),
            tool_settings: ToolSettings::default(),
            generated_gcode: String::new(),
            gcode_generated: false,
            current_file_path: None,
            is_modified: false,
            design_name: "Untitled".to_string(),
            show_grid: true,
            grid_spacing_mm: 10.0,
            show_toolpaths: false,
            snap_enabled: false,
            snap_threshold_mm: 0.5,
            clipboard: Vec::new(),
            default_properties_shape: crate::canvas::DrawingObject::new(
                0,
                crate::model::Shape::Rectangle(crate::model::DesignRectangle::new(
                    0.0, 0.0, 0.0, 0.0,
                )),
            ),
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            stock_material: Some(StockMaterial {
                width: 200.0,
                height: 200.0,
                thickness: 10.0,
                origin: (0.0, 0.0, 0.0),
                safe_z: 10.0,
            }),
            show_stock_removal: false,
            simulation_resolution: 0.1,
            simulation_result: None,
            num_axes: 3,
        }
    }

    /// Sets the drawing mode.
    pub fn set_mode(&mut self, mode: i32) {
        use crate::DrawingMode;
        let drawing_mode = match mode {
            0 => DrawingMode::Select,
            1 => DrawingMode::Rectangle,
            2 => DrawingMode::Circle,
            3 => DrawingMode::Line,
            4 => DrawingMode::Ellipse,
            5 => DrawingMode::Polyline,
            6 => DrawingMode::Text,
            7 => DrawingMode::Triangle,
            8 => DrawingMode::Polygon,
            9 => DrawingMode::Gear,
            10 => DrawingMode::Sprocket,
            11 => DrawingMode::Pan,
            unknown => {
                tracing::warn!("Unknown drawing mode {}, defaulting to Select", unknown);
                DrawingMode::Select
            }
        };
        self.canvas.set_mode(drawing_mode);
    }

    /// Sets the feed rate for toolpath generation.
    pub fn set_feed_rate(&mut self, rate: f64) {
        debug_assert!(
            rate.is_finite() && rate > 0.0,
            "feed_rate must be positive and finite, got {rate}"
        );
        self.tool_settings.feed_rate = rate;
        self.toolpath_generator.set_feed_rate(rate);
        self.gcode_generated = false;
    }

    /// Sets the spindle speed for toolpath generation.
    pub fn set_spindle_speed(&mut self, speed: u32) {
        self.tool_settings.spindle_speed = speed;
        self.toolpath_generator.set_spindle_speed(speed);
        self.gcode_generated = false;
    }

    /// Sets the tool diameter for toolpath generation.
    pub fn set_tool_diameter(&mut self, diameter: f64) {
        debug_assert!(
            diameter.is_finite() && diameter > 0.0,
            "tool_diameter must be positive and finite, got {diameter}"
        );
        self.tool_settings.tool_diameter = diameter;
        self.toolpath_generator.set_tool_diameter(diameter);
        self.gcode_generated = false;
    }

    /// Sets the cut depth for toolpath generation.
    pub fn set_cut_depth(&mut self, depth: f64) {
        debug_assert!(depth.is_finite(), "cut_depth must be finite, got {depth}");
        self.tool_settings.cut_depth = depth;
        self.toolpath_generator.set_cut_depth(depth);
        self.gcode_generated = false;
    }

    /// Sets the step-down for toolpath generation.
    pub fn set_step_down(&mut self, step: f64) {
        debug_assert!(
            step.is_finite() && step > 0.0,
            "step_down must be positive and finite, got {step}"
        );
        self.tool_settings.step_down = step;
        self.gcode_generated = false;
    }

/// Gets the current machine mode
    pub fn machine_mode(&self) -> MachineMode {
        self.tool_settings.machine_mode
    }

    /// Sets the machine mode
    pub fn set_machine_mode(&mut self, mode: MachineMode) {
        self.tool_settings.machine_mode = mode;
        self.gcode_generated = false;
    }


}

impl Default for DesignerState {
    fn default() -> Self {
        Self::new()
    }
}
